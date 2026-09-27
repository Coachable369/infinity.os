"""Exercise the integrated browser in a disposable installed ARM desktop.

Default diagnostic runs use an offline experimental kernel update. Only
--iso-parity installs matching browser media unchanged and verifies the exact
installed kernel before its media-detached boot.
"""
import importlib.util
import argparse
import base64
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import time
from urllib.parse import quote

ROOT = Path(__file__).resolve().parents[2]

# ------------------------=
# FUNC: interaction_url
# DESC: Serves a minimal HTML test document from a real public HTTPS endpoint, not an injected engine response.
# ------------------=
def interaction_url():
    document=b'''<body style="margin:0;background:#123456"><input oninput="document.body.style.background='#00ff00'"><img style="position:absolute;top:40px;left:0" src="https://httpbingo.org/image/png"><div style="height:900px"></div><div style="height:900px;background:#ff0000"></div>'''
    return "https://httpbun.com/mix/h=Content-Type:text%2Fhtml/b64="+quote(base64.b64encode(document).decode(),safe="")

# ------------------------=
# FUNC: read_pixels
# DESC: Decodes the actual QEMU framebuffer capture for behavioral color and image assertions.
# ------------------=
def read_pixels(path):
    magic,size,maximum,pixels=path.read_bytes().split(b"\n",3)
    width,height=map(int,size.split())
    assert magic==b"P6" and maximum==b"255" and len(pixels)==width*height*3
    return width,height,pixels

# ------------------------=
# FUNC: wait_color
# DESC: Waits for real CSS or JavaScript output pixels instead of accepting page text or an engine callback alone.
# ------------------=
def wait_color(guest,label,x,y,color):
    deadline=time.monotonic()+40
    while time.monotonic()<deadline:
        width,height,pixels=read_pixels(guest.screenshot(label))
        assert x<width and y<height
        at=(y*width+x)*3
        if pixels[at:at+3]==bytes(color): return
        time.sleep(.5)
    raise AssertionError({"pixel_stage":label,"actual":list(pixels[at:at+3]),"expected":color})

# ------------------------=
# FUNC: process_cpu_seconds
# DESC: Reads host QEMU CPU time for explicitly labeled emulator utilization, not fabricated guest hardware utilization.
# ------------------=
def process_cpu_seconds(pid):
    value=subprocess.check_output(["ps","-p",str(pid),"-o","time="],text=True).strip()
    total=0.0
    for part in value.split(":"): total=total*60+float(part)
    return total

# ------------------------=
# FUNC: pointer_pixel_latency
# DESC: Measures a visible cursor change in a quiet screen corner, independent of coarse diagnostic snapshot publication.
# ------------------=
def pointer_pixel_latency(guest, direction):
    # ------------------------=
    # FUNC: region
    # DESC: Captures only the quiet corner containing the software cursor.
    # ------------------=
    def region():
        width,height,pixels=read_pixels(guest.screenshot("browser-pointer-timing"))
        assert (width,height)==(1024,768)
        return b"".join(pixels[(y*width+940)*3:(y*width+1024)*3] for y in range(670,750))
    before=region()
    began=time.monotonic()
    guest.qmp("input-send-event",{"events":[{"type":"rel","data":{"axis":"x","value":direction*2}}]})
    deadline=began+5
    while time.monotonic()<deadline:
        if region()!=before:return time.monotonic()-began
        time.sleep(.02)
    raise AssertionError("No visible pointer movement within five seconds")
spec = importlib.util.spec_from_file_location("installed", ROOT / "tools/ms9-installed-acceptance.py")
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)
net_spec = importlib.util.spec_from_file_location("geturl_installed", ROOT / "tools/geturl-installed-test.py")
network = importlib.util.module_from_spec(net_spec)
net_spec.loader.exec_module(network)

# ------------------------=
# FUNC: browser_symbols
# DESC: Locates read-only engine counters in ELF metadata; acceptance uses their guest values, not symbol strings.
# ------------------=
def browser_symbols(elf):
    output = subprocess.check_output(["/opt/homebrew/opt/llvm/bin/llvm-nm", "-S",
        "--defined-only", "--demangle", str(elf)], text=True)
    result = {}
    for line in output.splitlines():
        fields = line.split(maxsplit=3)
        if len(fields) != 4:
            continue
        for name in ("STATE", "FAILURE", "FRAME_REVISION", "PEAK", "LOAD_REVISION", "LOADING", "PAGE_ERROR", "HISTORY",
                     "NETWORK_FAILURE", "NETWORK_STATUS", "NETWORK_COMPLETED", "FAILED_ALLOCATION", "LOCATION_HASH", "DOWNLOAD_STATE"):
            if fields[3] in ("infinity_kernel::runtime::browser::" + name,
                "infinity_kernel::runtime::browser::" + name + " (.0)", "INFINITY_BROWSER_" + name):
                result[name] = (int(fields[0], 16), int(fields[1], 16))
    assert len(result) == 14
    return result

class Guest(base.Guest):
    # ------------------------=
    # FUNC: click
    # DESC: Requires observed pointer motion and paired button transitions before accepting a desktop click.
    # ------------------=
    def click(self, x, y):
        target = (x * 1000 // self.width, y * 1000 // self.height)
        previous=(0,0)
        divisor=[3,3]
        for _ in range(80):
            state = self.state()
            assert state is not None
            deltas = (target[0] - state[13], target[1] - state[14])
            if max(map(abs, deltas)) <= 4:
                break
            for axis in range(2):
                if deltas[axis]*previous[axis]<0:
                    divisor[axis]*=2
            previous=deltas
            events = [{"type": "rel", "data": {"axis": axis,
                "value": max(-40, min(40, int(delta / divisor[index]) or (1 if delta > 0 else -1)))}}
                for index,(axis, delta) in enumerate(zip(("x", "y"), deltas)) if abs(delta) > 4]
            self.qmp("input-send-event", {"events": events})
            self.wait(lambda s: s[13:15] != state[13:15], "browser pointer moved", timeout=30)
        else:
            raise AssertionError({"pointer_target":target,"pointer_actual":state[13:15]})
        for down in (True, False):
            self.qmp("input-send-event", {"events": [
                {"type": "btn", "data": {"button": "left", "down": down}}]})
            self.wait(lambda s: bool(s[15] & 1) == down, "browser pointer button", timeout=30)

    # ------------------------=
    # FUNC: boot
    # DESC: Boots actual UEFI media and patches only this disposable installation before its first detached boot.
    # ------------------=
    def boot(self, installer):
        assert self.process is None
        artifacts = self.work.parent / "artifacts"
        if not installer and not getattr(self, "patched", False):
            subprocess.run(["python3", str(ROOT / "tools/update-installed-clone.py"),
                str(self.disk), str(artifacts / "installed-stripped.elf"), "--apply"], check=True)
            self.patched = True
        self.installer = installer
        elf = artifacts / ("kernel.elf" if installer else "installed-kernel.elf")
        self.address, self.length = base.symbol(elf, "INFINITY_DIAGNOSTIC_SNAPSHOT")
        self.frames_address, self.frames_length = base.symbol(elf, "INFINITY_DIAGNOSTIC_FRAMES")
        self.compute_address = None
        qmp = self.work / "qmp.sock"
        qmp.unlink(missing_ok=True)
        self.log = (self.work / ("installer.log" if installer else "installed.log")).open("ab")
        command = ["qemu-system-aarch64", "-machine", "virt", "-accel", "tcg", "-cpu", "max",
            "-smp", "4", "-m", "12G", "-bios", self.firmware, "-device", "ramfb",
            "-device", "qemu-xhci", "-device", "usb-kbd", "-device", "usb-mouse",
            "-device", "virtio-scsi-pci", "-drive", f"if=none,id=disk,format=raw,file={self.disk}",
            "-device", "scsi-hd,drive=disk,bootindex=1", "-netdev", "user,id=net",
            "-device", "e1000,netdev=net", "-qmp", f"unix:{qmp},server=on,wait=off",
            "-display", "none", "-serial", "stdio", "-monitor", "none", "-no-reboot"]
        if installer:
            command += ["-drive", f"if=none,id=cd,format=raw,media=cdrom,file={artifacts / 'installer.iso'}",
                "-device", "scsi-cd,drive=cd,bootindex=0"]
        self.process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=self.log, stderr=self.log)
        deadline = time.monotonic() + 20
        while not qmp.exists():
            if self.process.poll() is not None or time.monotonic() >= deadline:
                if self.process.poll() is None:
                    self.process.kill()
                self.process.wait()
                self.process = None
                self.log.close()
                raise RuntimeError("QEMU management startup failed; inspect guest log")
            time.sleep(.1)
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.settimeout(20)
        self.sock.connect(str(qmp))
        self.channel = self.sock.makefile("rwb")
        json.loads(self.channel.readline())
        self.qmp("qmp_capabilities")

# ------------------------=
# FUNC: main
# DESC: Installs on a new private disk and captures native browser launch without claiming release packaging acceptance.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reuse-installed", type=Path)
    parser.add_argument("--iso-parity", action="store_true", help="Cold-install the browser QEMU ISO without any offline kernel replacement")
    parser.add_argument("--update-kernel", type=Path, help="Update only this harness's disposable disk from a repository-local kernel")
    parser.add_argument("--navigation", action="store_true", help="Capture real link/history interaction for manual review; not an automatic navigation pass")
    parser.add_argument("--download", action="store_true", help="Fetch a real HTTPS attachment, click native Save, and verify the stored object after shutdown")
    parser.add_argument("--launcher", action="store_true", help="Launch through the installed catalog and approve native network consent without Console authorization")
    parser.add_argument("--interaction", action="store_true", help="Verify real HTTPS image, CSS, JavaScript input and scrolling by framebuffer pixels")
    parser.add_argument("--invalid-tls", action="store_true", help="Require a certificate-validation rejection from a real expired HTTPS endpoint")
    parser.add_argument("--lifecycle", action="store_true", help="Check maximize/restore/minimize/close/reopen after the interaction page")
    parser.add_argument("--measure", action="store_true", help="Measure submission-to-real-page pixels and completion on the cold installed engine")
    parser.add_argument("--address", action="store_true", help="Type a new URL into the native address bar and require its real rendered page")
    parser.add_argument("--reopen", action="store_true", help="Retest only close/reopen without repeating passing resize and minimize checks")
    args = parser.parse_args()
    if args.lifecycle and (args.invalid_tls or args.download or args.launcher or args.navigation):
        parser.error("Lifecycle acceptance is a separate bounded run")
    if args.reopen and (args.interaction or args.invalid_tls or args.download or args.launcher or args.navigation or args.lifecycle):
        parser.error("Reopen acceptance is a separate bounded run")
    if args.measure and (args.reopen or args.lifecycle or args.interaction or args.invalid_tls or args.download or args.launcher or args.navigation):
        parser.error("Timing acceptance is a separate bounded run")
    if args.invalid_tls and (args.interaction or args.download or args.launcher or args.navigation):
        parser.error("Certificate acceptance is a separate bounded run")
    if args.download and (args.launcher or args.navigation):
        parser.error("Download acceptance is a separate bounded run")
    if args.interaction and (args.download or args.launcher or args.navigation):
        parser.error("Interaction acceptance is a separate bounded run")
    reuse = args.reuse_installed is not None
    if args.iso_parity and (reuse or args.update_kernel):
        parser.error("ISO parity requires a new unmodified installation")
    if args.update_kernel and (not reuse or not args.update_kernel.resolve().is_relative_to(ROOT / "build")):
        parser.error("Kernel updates require a reused disposable installation and a build-local image")
    work = args.reuse_installed.resolve() if reuse else ROOT / "build" / ("browser-installed-" + str(time.time_ns()))
    assert work.parent == ROOT / "build" and work.name.startswith("browser-installed-")
    artifacts = work / "artifacts"
    if not reuse:
        artifacts.mkdir(parents=True)
    for source, name in ([] if reuse else [
        ("builds/InfinityOS-aarch64-qemu-test.iso", "installer.iso"),
        ("build/aarch64/kernel-qemu.elf", "kernel.elf"),
        ("build/aarch64/installed-kernel-qemu.elf" if args.iso_parity else "build/servo-platform-probe/kernel-aarch64/qemu-kernel.elf", "installed-kernel.elf")]):
        shutil.copyfile(ROOT / source, artifacts / name)
    if args.update_kernel:
        shutil.copyfile(args.update_kernel.resolve(), artifacts / "installed-kernel.elf")
    subprocess.run(["/opt/homebrew/opt/llvm/bin/llvm-objcopy", "--strip-debug",
        str(artifacts / "installed-kernel.elf"), str(artifacts / "installed-stripped.elf")], check=True)
    guest = Guest(work, 1, "/opt/homebrew/share/qemu/edk2-aarch64-code.fd", reuse=reuse, width=1024, height=768, memory_mb=12288)
    guest.fast_commands = args.interaction
    guest.patched = args.iso_parity or (reuse and args.update_kernel is None)
    receipt = dict(installed=reuse, browser_iso_parity=False, browser_interactive=False)
    try:
        if not reuse:
            guest.install()
            receipt["installed"] = True
            guest.stop()
            if args.iso_parity:
                subprocess.run(["python3",str(ROOT/"tools/update-installed-clone.py"),str(guest.disk),
                    str(artifacts/"installed-kernel.elf"),"--verify-only"],check=True)
                receipt["browser_iso_parity"]=True
        if reuse:
            guest.boot(False)
            guest.authenticate()
        else:
            guest.onboard()
        if not reuse:
            network.configure_nat(guest)
        counters = browser_symbols(artifacts / "installed-kernel.elf")
        if args.launcher:
            guest.launch("browser",5)
            time.sleep(.5)
            assert int.from_bytes(guest.memory(*counters["STATE"]),"little")==0
            assert int.from_bytes(guest.memory(*counters["NETWORK_COMPLETED"]),"little")==0
            guest.screenshot("browser-network-consent")
            guest.click(740,595)
            receipt["launcher_with_explicit_consent"]=True
        else:
            guest.launch("command", 5)
            guest.command("browser authorize confirm=true")
            command="browser " + ("https://expired-isrgrootx1.letsencrypt.org/" if args.invalid_tls else interaction_url() if args.interaction else "https://httpbun.com/mix/h=Content-Type:text%2Fhtml/b64=PGJvZHkgc3R5bGU9YmFja2dyb3VuZDpyZWQ%2B" if args.lifecycle else "https://httpbingo.org/response-headers?Content-Disposition=attachment%3B%20filename%3Dnative-browser-test.txt&Content-Type=text%2Fplain" if args.download else "https://example.com/")
            if args.measure:
                assert int.from_bytes(guest.memory(*counters["STATE"]),"little")==0
                guest.text(command)
                cpu_started=process_cpu_seconds(guest.process.pid)
                submitted=time.monotonic()
                guest.key("ret")
            else:
                guest.command(command)
        started = time.monotonic()
        deadline = started + 90
        while time.monotonic() < deadline:
            values = {name: int.from_bytes(guest.memory(address, size), "little")
                for name, (address, size) in counters.items()}
            receipt["engine"] = values
            if args.measure:
                elapsed=time.monotonic()-submitted
                if values["FRAME_REVISION"] and "first_engine_frame_seconds" not in receipt:
                    receipt["first_engine_frame_seconds"]=elapsed
                width,height,pixels=read_pixels(guest.screenshot("browser-timing"))
                at=(500*width+400)*3
                if pixels[at:at+3]==bytes((238,238,238)) and values["NETWORK_COMPLETED"]:
                    receipt.setdefault("first_page_paint_seconds",time.monotonic()-submitted)
                if values["NETWORK_COMPLETED"] and values["LOADING"]==0 and "first_page_paint_seconds" in receipt:
                    receipt["page_complete_seconds"]=time.monotonic()-submitted
                    receipt["qemu_host_cpu_seconds"]=process_cpu_seconds(guest.process.pid)-cpu_started
                    receipt["qemu_host_cpu_percent"]=100*receipt["qemu_host_cpu_seconds"]/receipt["page_complete_seconds"]
                    break
            if values["STATE"] == 3 or values["FAILURE"]:
                break
            if args.download and values["DOWNLOAD_STATE"]==1:
                break
            if not args.measure and not args.download and values["STATE"] == 2 and values["FRAME_REVISION"] >= 2 and time.monotonic() - started >= 15:
                break
            time.sleep(.25)
        guest.screenshot("browser-launch")
        receipt["observation_seconds"] = time.monotonic() - started
        assert values["STATE"] == 2 and values["FAILURE"] == 0 and values["FRAME_REVISION"] >= 2, receipt
        receipt["engine_running_with_frames"] = True
        receipt["launch_command_submitted"] = True
        if args.address:
            guest.click(480,160)
            guest.key("end")
            for _ in range(len("https://example.com/")):
                guest.key("backspace")
            guest.text("https://httpbun.com/mix/h=Content-Type:text%2Fhtml/b64=PGJvZHkgc3R5bGU9YmFja2dyb3VuZDpyZWQ%2B")
            guest.key("ret")
            wait_color(guest,"browser-address-navigation",400,400,(255,0,0))
            receipt["native_address_keyboard_navigation"]=True
        if args.measure:
            assert "page_complete_seconds" in receipt and values["PAGE_ERROR"]==0,receipt
            receipt["timing_boundary"]="QMP Enter submission to observed framebuffer/engine completion; ARM TCG, 4 vCPU, 12 GiB; polling upper bounds"
            guest.click(970,700)
            receipt["pointer_visible_roundtrip_seconds"]=[pointer_pixel_latency(guest,1 if index%2==0 else -1) for index in range(10)]
            guest.click(889,111)
            deadline=time.monotonic()+30
            while time.monotonic()<deadline:
                peak=int.from_bytes(guest.memory(*counters["PEAK"]),"little")
                if peak:break
                time.sleep(.25)
            assert peak>0,receipt
            receipt["engine_peak_allocated_bytes"]=peak
        if args.invalid_tls:
            assert values["NETWORK_FAILURE"] == 9, receipt
            assert values["NETWORK_COMPLETED"] == 0 and values["PAGE_ERROR"] != 0, receipt
            receipt["expired_certificate_rejected_before_http"] = True
        if args.interaction:
            wait_color(guest,"browser-css",400,400,(18,52,86))
            width,height,pixels=read_pixels(guest.screenshot("browser-image"))
            image_colors={pixels[(y*width+x)*3:(y*width+x)*3+3] for y in range(235,330) for x in range(106,200)}
            assert len(image_colors)>32,{"image_colors":len(image_colors)}
            receipt["https_image_and_css"]=True
            guest.click(145,201)
            guest.key("a")
            wait_color(guest,"browser-js-input",400,400,(0,255,0))
            receipt["keyboard_javascript_dom_mutation"]=True
            guest.click(450,350)
            for _ in range(32):
                for down in (True,False):
                    guest.qmp("input-send-event",{"events":[{"type":"btn","data":{"button":"wheel-down","down":down}}]})
                time.sleep(.1)
            wait_color(guest,"browser-scroll",400,400,(255,0,0))
            receipt["scroll_pixels"]=True
            receipt["browser_interactive"]=True
        if args.lifecycle:
            wait_color(guest,"browser-lifecycle-page",400,400,(255,0,0))
            guest.click(849,111)
            wait_color(guest,"browser-maximized",970,400,(255,0,0))
            guest.click(944,65)
            wait_color(guest,"browser-restored",400,400,(255,0,0))
            receipt["maximize_restore_pixels"]=True
            guest.click(809,111)
            time.sleep(1)
            width,height,pixels=read_pixels(guest.screenshot("browser-minimized"))
            at=(400*width+400)*3
            assert pixels[at:at+3]!=bytes((255,0,0)), "Minimize did not hide browser surface"
            receipt["minimize_hides_surface"]=True
            guest.click(45,206)
            wait_color(guest,"browser-shelf-restored",400,400,(255,0,0))
            receipt["shelf_restores_live_page"]=True
        if args.lifecycle or args.reopen:
            guest.click(889,111)
            deadline=time.monotonic()+30
            while time.monotonic()<deadline:
                peak=int.from_bytes(guest.memory(*counters["PEAK"]),"little")
                if peak: break
                time.sleep(.25)
            assert peak>0, "Close did not acknowledge engine teardown and heap accounting"
            receipt["engine_peak_allocated_bytes"]=peak
            guest.screenshot("browser-closed")
            guest.launch("browser",5)
            guest.click(740,595)
            wait_color(guest,"browser-open-after-close",400,500,(238,238,238))
            receipt["close_reopen_pixels"]=True
        if args.download:
            assert values["DOWNLOAD_STATE"] == 1, receipt
            time.sleep(.5)
            guest.screenshot("browser-download-consent")
            guest.click(740,595)
            deadline=time.monotonic()+30
            while time.monotonic()<deadline:
                status=int.from_bytes(guest.memory(*counters["DOWNLOAD_STATE"]),"little")
                if status in (2,3): break
                time.sleep(.25)
            receipt["download_state"]=status
            guest.screenshot("browser-download-saved")
            assert status==2,receipt
            guest.stop()
            checker=ROOT/"build/tests/browser-installed-object-test"
            subprocess.run(["rustc","--edition=2021","-O",str(ROOT/"tools/installed-object-test.rs"),"-o",str(checker)],check=True)
            subprocess.run([str(checker),str(work/"node-1/installed.raw"),"--browser-download"],check=True)
            receipt["download_persisted"]=True
        if args.navigation:
            receipt["navigation"] = []
            for label, x, y in (("link", 303, 368), ("back", 139, 161),
                               ("forward", 192, 161), ("reload", 242, 161)):
                target_url = "https://example.com/" if label == "back" else "https://www.iana.org/help/example-domains"
                target_hash = 0xcbf29ce484222325
                for byte in target_url.encode():
                    target_hash = ((target_hash ^ byte) * 0x100000001b3) & ((1 << 64) - 1)
                before = int.from_bytes(guest.memory(*counters["LOAD_REVISION"]), "little")
                guest.click(x, y)
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    values = {name: int.from_bytes(guest.memory(address, size), "little")
                        for name, (address, size) in counters.items()}
                    if values["FAILURE"] or values["PAGE_ERROR"]:
                        break
                    if values["LOAD_REVISION"] > before and values["LOADING"] == 0 and values["LOCATION_HASH"] == target_hash:
                        break
                    time.sleep(.25)
                receipt["navigation"].append(dict(action=label, **values))
                # Allow the BSP compositor to present the observed status before
                # capturing it; the state assertions below remain authoritative.
                time.sleep(.5)
                guest.screenshot("browser-" + label)
                assert values["FAILURE"] == 0 and values["PAGE_ERROR"] == 0, receipt
                assert values["LOAD_REVISION"] > before and values["LOADING"] == 0, receipt
                assert values["LOCATION_HASH"] == target_hash, receipt
            receipt["navigation_captures_for_review"] = True
        print(json.dumps(dict(work=str(work), **receipt)), flush=True)
    finally:
        (work / "result.json").write_text(json.dumps(receipt, indent=2) + "\n")
        guest.stop()

if __name__ == "__main__":
    main()
