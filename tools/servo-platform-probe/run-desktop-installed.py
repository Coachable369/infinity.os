"""Exercise the integrated browser in a disposable installed native desktop.

Default diagnostic runs use an offline experimental kernel update. Only
--iso-parity installs matching browser media unchanged and verifies the exact
installed kernel before its media-detached boot.
"""
import importlib.util
import argparse
import base64
import json
import hashlib
import os
from pathlib import Path
import shutil
import socket
import subprocess
import time
import tempfile
from contextlib import contextmanager
from urllib.parse import quote

ROOT = Path(__file__).resolve().parents[2]

# ------------------------=
# FUNC: acceptance_case
# DESC: Records independent behavioral failures without skipping later checks; transport and infrastructure exceptions remain fatal.
# ------------------=
@contextmanager
def acceptance_case(receipt, name):
    result = {"case": name, "passed": False}
    receipt.setdefault("cases", []).append(result)
    try:
        yield result
    except AssertionError as error:
        result["failure"] = str(error)
    else:
        result["passed"] = True

# ------------------------=
# FUNC: extract_media_kernels
# DESC: Uses the tested ISO's own live ELF and ordered payload shards, never a loose artifact overwritten by a later build.
# ------------------=
def extract_media_kernels(iso, artifacts, arch):
    assert artifacts.resolve().is_relative_to(ROOT / "build")
    machine={"x86_64":62,"aarch64":183}[arch]
    with tempfile.TemporaryDirectory(prefix="browser-media-",dir=ROOT / "build" / "tmp") as temporary:
        temporary=Path(temporary)
        subprocess.run(["xorriso","-osirrox","on","-indev",str(iso),
            "-extract","/EFI/INFINITY/KERNEL.ELF",str(temporary/"live.elf"),
            "-extract","/EFI/INFINITY/PAYLOAD",str(temporary/"payload")],check=True)
        parts=sorted((temporary/"payload").glob("P1-*.BIN"))
        assert parts and len(parts)<=8
        assert [part.name for part in parts]==[f"P1-{index:03}.BIN" for index in range(len(parts))]
        assert 0<sum(part.stat().st_size for part in parts)<=1024**3
        with (temporary/"installed.elf").open("wb") as output:
            for part in parts:
                with part.open("rb") as source: shutil.copyfileobj(source,output,1024*1024)
        receipt={}
        for source,name in [(temporary/"live.elf","kernel.elf"),(temporary/"installed.elf","installed-kernel.elf")]:
            with source.open("rb") as stream:
                header=stream.read(20)
                assert header[:6]==b"\x7fELF\x02\x01" and int.from_bytes(header[18:20],"little")==machine
                stream.seek(0)
                receipt[name]=dict(bytes=source.stat().st_size,sha256=hashlib.file_digest(stream,"sha256").hexdigest())
            shutil.copyfile(source,artifacts/name)
        return receipt

# ------------------------=
# FUNC: interaction_url
# DESC: Serves a minimal HTML test document from a real public HTTPS endpoint, not an injected engine response.
# ------------------=
def interaction_url():
    document=b'''<body style="margin:0;background:#123456"><input oninput="document.body.style.background='#00ff00'"><img style="position:absolute;top:40px;left:0" src="https://httpbingo.org/image/png"><div style="height:100vh"></div><div style="height:100vh;background:#ff0000"></div>'''
    return "https://httpbun.com/mix/h=Content-Type:text%2Fhtml/b64="+quote(base64.b64encode(document).decode(),safe="")

# ------------------------=
# FUNC: navigation_urls
# DESC: Provides real HTTPS pages with a deterministic link and distinct rendered colors for navigation assertions.
# ------------------=
def navigation_urls():
    prefix="https://httpbun.com/mix/h=Content-Type:text%2Fhtml/b64="
    second=prefix+quote(base64.b64encode(b'<title>Destination</title><body style=margin:0;background:#46505a>Next').decode(),safe="")
    first=prefix+quote(base64.b64encode(('<body style=margin:0;background:#123456><a style=display:block;margin:24px;width:200px;height:48px href="'+second+'">Next</a>').encode()).decode(),safe="")
    assert len("browser "+first)<512, "Navigation fixture exceeds native Console capacity"
    return first,second

# ------------------------=
# FUNC: browser_text
# DESC: Types into native browser chrome using keyboard/event-loop acknowledgement, not Console editor counters.
# ------------------=
def browser_text(guest,value):
    aliases={" ":"spc","-":"minus",".":"dot","=":"equal","/":"slash"}
    shifted={":":"semicolon","?":"slash","%":"5","&":"7","_":"minus"}
    for character in value:
        if character in shifted: guest.key("shift",shifted[character])
        elif character.isupper(): guest.key("shift",character.lower())
        else: guest.key(aliases.get(character,character))

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
# FUNC: wait_image
# DESC: Waits for asynchronous image decode and compositing rather than treating document completion as image readiness.
# ------------------=
def wait_image(guest, left, top):
    deadline=time.monotonic()+40
    while True:
        width,height,pixels=read_pixels(guest.screenshot("browser-image"))
        assert left>=0 and top>=0 and left+98<width and top+139<height
        colors={pixels[(y*width+x)*3:(y*width+x)*3+3]
            for y in range(top+44,top+139) for x in range(left+4,left+98)}
        if len(colors)>32:
            return {"distinct_colors":len(colors)}
        if time.monotonic()>=deadline:
            raise AssertionError({"image_colors":len(colors),"decode_present_timeout_seconds":40})
        time.sleep(.5)

# ------------------------=
# FUNC: page_color_bounds
# DESC: Finds the actual test page's unique CSS pixels so interaction proof is independent of firmware resolution.
# ------------------=
def page_color_bounds(guest):
    deadline=time.monotonic()+60
    while time.monotonic()<deadline:
        width,height,pixels=read_pixels(guest.screenshot("browser-css"))
        locations=[at//3 for at in range(0,len(pixels),3) if pixels[at:at+3]==b"\x12\x34\x56"]
        if len(locations)>1000:
            guest.width,guest.height=width,height
            return min(at%width for at in locations),min(at//width for at in locations),max(at%width for at in locations),max(at//width for at in locations)
        time.sleep(.5)
    raise AssertionError("Real CSS background did not render")

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
        assert (width,height)==(guest.width,guest.height)
        return b"".join(pixels[(y*width+width-84)*3:(y*width+width)*3]
                        for y in range(height-98,height-18))
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
                     "NETWORK_FAILURE", "NETWORK_STATUS", "NETWORK_COMPLETED", "FAILED_ALLOCATION", "LOCATION_HASH", "DOWNLOAD_STATE",
                     "FAVORITES_COUNT","FAVORITES_ERROR","FAVORITE_SAVED","FAVORITE_TITLE_HASH","FAVORITES_HASH","FOOTER_STATUS","DISPLAY_ADDRESS_HASH","SETTINGS","MENU"):
            if fields[3] in ("infinity_kernel::runtime::browser::" + name,
                "infinity_kernel::runtime::browser::" + name + " (.0)", "INFINITY_BROWSER_" + name):
                result[name] = (int(fields[0], 16), int(fields[1], 16))
    required={"STATE","FAILURE","FRAME_REVISION","LOAD_REVISION","LOADING","PAGE_ERROR","HISTORY",
              "NETWORK_FAILURE","NETWORK_STATUS","NETWORK_COMPLETED","FAILED_ALLOCATION","LOCATION_HASH","DOWNLOAD_STATE"}
    assert required.issubset(result), {"missing_counters":sorted(required-set(result))}
    return result

class Guest(base.Guest):
    # ------------------------=
    # FUNC: click
    # DESC: Requires observed pointer motion and paired button transitions before accepting a desktop click.
    # ------------------=
    def click(self, x, y, press=True):
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
        if not press:return
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
        acceleration=getattr(self,"acceleration","tcg")
        command = ["qemu-system-aarch64", "-machine", "virt", "-accel", acceleration, "-cpu", "host" if acceleration=="hvf" else "max",
            "-smp", "4", "-m", "12G", "-bios", self.firmware, "-device", "ramfb",
            "-device", "virtio-rng-pci",
            "-device", "qemu-xhci", "-device", "usb-kbd", "-device", "usb-mouse",
            "-device", "virtio-scsi-pci", "-drive", f"if=none,id=disk,format=raw,file={self.disk}",
            "-device", "scsi-hd,drive=disk,bootindex=1", "-netdev", "user,id=net",
            "-device", "e1000,netdev=net", "-qmp", f"unix:{qmp},server=on,wait=off",
            "-display", "none", "-serial", "stdio", "-monitor", "none", "-no-reboot"]
        if self.arch=="x86_64":
            command=["qemu-system-x86_64","-machine","pc","-accel","tcg","-cpu","max",
                "-smp","4","-m","12G","-vga","none","-device","VGA,xres=1024,yres=768,xmax=1024,ymax=768",
                "-drive",f"if=pflash,format=raw,readonly=on,file={self.firmware}",
                "-drive",f"if=ide,index=0,format=raw,file={self.disk}",
                "-netdev","user,id=net","-device","e1000,netdev=net",
                "-object","rng-random,id=rng0,filename=/dev/urandom","-device","virtio-rng-pci,rng=rng0",
                "-qmp",f"unix:{qmp},server=on,wait=off","-display","none","-serial","stdio","-no-reboot"]
            command += ["-cdrom",str(artifacts/"installer.iso"),"-boot","order=d"] if installer else ["-boot","order=c"]
        elif installer:
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
    parser.add_argument("--arch",choices=("aarch64","x86_64"),default="aarch64")
    parser.add_argument("--accel",choices=("tcg","hvf"),default="tcg",help="Use host ARM virtualization for hardware-accelerated latency measurements")
    parser.add_argument("--reuse-installed", type=Path)
    parser.add_argument("--resume-onboarding",action="store_true",help="Resume a completed disposable installation, reverify against its original ISO and finish onboarding")
    parser.add_argument("--iso-parity", action="store_true", help="Cold-install the browser QEMU ISO without any offline kernel replacement")
    parser.add_argument("--update-kernel", type=Path, help="Update only this harness's disposable disk from a repository-local kernel")
    parser.add_argument("--navigation", action="store_true", help="Verify real HTTPS link/back/forward/reload through URL state and distinct page pixels")
    parser.add_argument("--download", action="store_true", help="Fetch a real HTTPS attachment, click native Save, and verify the stored object after shutdown")
    parser.add_argument("--launcher", action="store_true", help="Launch through the installed catalog under the user's existing Network Settings policy")
    parser.add_argument("--open-url", action="store_true", help="Verify the OS default web association under the user's existing Network Settings policy")
    parser.add_argument("--interaction", action="store_true", help="Verify real HTTPS image, CSS, JavaScript input and scrolling by framebuffer pixels")
    parser.add_argument("--tabs", action="store_true", help="With interaction, exercise native create/select/close controls and independent page pixels")
    parser.add_argument("--invalid-tls", action="store_true", help="Require a certificate-validation rejection from a real expired HTTPS endpoint")
    parser.add_argument("--lifecycle", action="store_true", help="Check maximize/restore/minimize/close/reopen after the interaction page")
    parser.add_argument("--measure", action="store_true", help="Measure submission-to-real-page pixels and completion on the cold installed engine")
    parser.add_argument("--address", action="store_true", help="Type a new URL into the native address bar and require its real rendered page")
    parser.add_argument("--favorites",action="store_true",help="Exercise durable favorites and detached cold reboot persistence")
    parser.add_argument("--footer",action="store_true",help="Retry only footer loading, failed destination and recovery on an existing saved favorite")
    parser.add_argument("--favorites-overflow",action="store_true",help="Exercise favorites overflow paging on an installed browser with one saved page")
    parser.add_argument("--favorites-label",action="store_true",help="Verify same-title navigation preserves the document title when saving another favorite")
    parser.add_argument("--settings",action="store_true",help="Exercise native settings controls, save state, and detached reboot persistence")
    parser.add_argument("--chrome",action="store_true",help="Check File/Settings menus, URL gear and attached AI panel")
    parser.add_argument("--reopen", action="store_true", help="Retest only close/reopen without repeating passing resize and minimize checks")
    args = parser.parse_args()
    if args.accel=="hvf" and args.arch!="aarch64":
        parser.error("HVF verification is supported only for the native ARM target")
    if args.open_url and any((args.launcher,args.interaction,args.tabs,args.address,args.measure,args.navigation,args.download,args.invalid_tls,args.lifecycle,args.reopen)):
        parser.error("Default web association acceptance is a separate bounded run")
    if args.tabs and not args.interaction:
        parser.error("Tab acceptance requires --interaction")
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
    if args.resume_onboarding and (not reuse or args.update_kernel):
        parser.error("Onboarding resume requires a reused disk without a kernel update")
    if args.arch=="x86_64" and not (reuse or args.iso_parity):
        parser.error("x86 verification requires unmodified ISO installation or a previously verified disk")
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
        ("builds/InfinityOS-x86_64-bootstrap-test.iso" if args.arch=="x86_64" else "builds/InfinityOS-aarch64-qemu-test.iso", "installer.iso"),
        ("build/x86_64/kernel.elf" if args.arch=="x86_64" else "build/aarch64/kernel-qemu.elf", "kernel.elf"),
        ("build/x86_64/installed-kernel.elf" if args.arch=="x86_64" else "build/aarch64/installed-kernel-qemu.elf" if args.iso_parity else "build/servo-platform-probe/kernel-aarch64/qemu-kernel.elf", "installed-kernel.elf")]):
        shutil.copyfile(ROOT / source, artifacts / name)
    media_kernels=extract_media_kernels(artifacts/"installer.iso",artifacts,args.arch) if args.iso_parity or args.resume_onboarding else None
    if args.update_kernel:
        shutil.copyfile(args.update_kernel.resolve(), artifacts / "installed-kernel.elf")
    subprocess.run(["/opt/homebrew/opt/llvm/bin/llvm-objcopy", "--strip-debug",
        str(artifacts / "installed-kernel.elf"), str(artifacts / "installed-stripped.elf")], check=True)
    guest = Guest(work, 1, f"/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd", reuse=reuse, width=1024, height=768, memory_mb=12288)
    guest.arch=args.arch
    guest.acceleration=args.accel
    # x86 PIO under cross-architecture TCG must read back the entire large payload.
    # This bounds the harness only; it does not relax any installed-byte checks.
    guest.install_timeout_seconds=1800 if args.arch=="x86_64" else 900
    # PS/2 emulation cannot reliably ingest the rapid four-key batches while
    # the 2048px x86 desktop is painting. Acknowledge each key on that target.
    guest.fast_commands = args.interaction and args.arch != "x86_64"
    guest.patched = args.iso_parity or (reuse and args.update_kernel is None)
    receipt = dict(architecture=args.arch,acceleration=args.accel,installed=reuse, browser_iso_parity=False, browser_interactive=False)
    if media_kernels is not None: receipt["iso_kernel_artifacts"]=media_kernels
    try:
        if not reuse:
            guest.install()
            receipt["installed"] = True
            guest.stop()
            if args.iso_parity:
                subprocess.run(["python3",str(ROOT/"tools/update-installed-clone.py"),str(guest.disk),
                    str(artifacts/"installed-kernel.elf"),"--verify-only"],check=True)
                receipt["browser_iso_parity"]=True
        if args.resume_onboarding:
            subprocess.run(["python3",str(ROOT/"tools/update-installed-clone.py"),str(guest.disk),
                str(artifacts/"installed-kernel.elf"),"--verify-only"],check=True)
            receipt["browser_iso_parity"]=True
            receipt["resumed_completed_installation"]=True
            guest.onboard()
        elif reuse:
            guest.boot(False)
            guest.authenticate()
        else:
            guest.onboard()
        if not reuse or args.resume_onboarding:
            network.configure_nat(guest)
        counters = browser_symbols(artifacts / "installed-kernel.elf")
        if args.measure or args.lifecycle or args.reopen:
            assert "PEAK" in counters, "This optimized kernel does not expose peak-memory diagnostics"
        if args.launcher or args.open_url:
            if args.open_url:
                guest.launch("command",5)
                guest.command("open https://example.com/")
            else:
                guest.launch("browser",5)
            # Launch now uses the signed-in user's persisted Network Settings;
            # there is no separate consent card to click or Console grant.
            receipt["catalog_launch_without_console_grant"]=True
            if args.open_url: receipt["default_web_association_submitted"]=True
        else:
            guest.launch("command", 5)
            guest.command("browser authorize confirm=true")
            command="browser " + ("https://expired-isrgrootx1.letsencrypt.org/" if args.invalid_tls else interaction_url() if args.interaction else "https://httpbun.com/mix/h=Content-Type:text%2Fhtml/b64=PGJvZHkgc3R5bGU9YmFja2dyb3VuZDpyZWQ%2B" if args.lifecycle else "https://httpbingo.org/response-headers?Content-Disposition=attachment%3B%20filename%3Dnative-browser-test.txt&Content-Type=text%2Fplain" if args.download else "https://example.com/")
            if args.navigation: command="browser "+navigation_urls()[0]
            if args.measure:
                assert int.from_bytes(guest.memory(*counters["STATE"]),"little")==0
                guest.text(command)
                guest.width,guest.height,_=read_pixels(guest.screenshot("browser-timing-display"))
                guest.click(guest.width-54,guest.height-68,press=False)
                cpu_started=process_cpu_seconds(guest.process.pid)
                submitted=time.monotonic()
                guest.key("ret")
                receipt["pointer_during_load_seconds"]=[]
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
                if values["LOADING"] and values["FRAME_REVISION"]:
                    samples=receipt["pointer_during_load_seconds"]
                    samples.append(pointer_pixel_latency(guest,1 if len(samples)%2==0 else -1))
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
        if args.measure:
            assert "page_complete_seconds" in receipt and values["PAGE_ERROR"]==0,receipt
            receipt["timing_boundary"]=f"QMP Enter submission to observed framebuffer/engine completion; {args.arch} {args.accel.upper()}, 4 vCPU, 12 GiB; polling upper bounds"
            guest.click(guest.width-54,guest.height-68,press=False)
            receipt["pointer_visible_roundtrip_seconds"]=[pointer_pixel_latency(guest,1 if index%2==0 else -1) for index in range(10)]
            guest.key("ctrl","w")
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
            left,top,right,bottom=page_color_bounds(guest)
            check_x,check_y=right-32,min(bottom-32,top+210)
            with acceptance_case(receipt,"https_css_image"):
                wait_color(guest,"browser-css",check_x,check_y,(18,52,86))
                receipt["image_pixels"]=wait_image(guest,left,top)
                receipt["https_image_and_css"]=True
            with acceptance_case(receipt,"keyboard_javascript"):
                guest.click(left+43,top+10)
                guest.key("a")
                wait_color(guest,"browser-js-input",check_x,check_y,(0,255,0))
                receipt["keyboard_javascript_dom_mutation"]=True
            with acceptance_case(receipt,"scroll"):
                # Keep the software cursor away from the exact CSS sample.
                guest.click(check_x-48,check_y)
                for _ in range(32):
                    for down in (True,False):
                        guest.qmp("input-send-event",{"events":[{"type":"btn","data":{"button":"wheel-down","down":down}}]})
                    time.sleep(.1)
                wait_color(guest,"browser-scroll",check_x,check_y,(255,0,0))
                receipt["scroll_pixels"]=True
            receipt["browser_interactive"]=all(case["passed"] for case in receipt["cases"])
            if args.tabs:
                # Bounds come from rendered page pixels, not a guessed desktop position.
                # Kit chrome: 48px tabs, 32px menus, 48px navigation, 36px favorites.
                tab_y=top-134
                guest.click(left+352,tab_y)
                time.sleep(1)
                guest.click(left+300,top-66)
                guest.key("end")
                for _ in range(len("about:blank")):
                    guest.key("backspace")
                # A short real URL exercises the same chrome-to-HTTPS path without
                # spending minutes typing an encoded fixture under cross-ISA TCG.
                browser_text(guest,"https://example.com/")
                guest.key("ret")
                wait_color(guest,"browser-second-tab",check_x,check_y,(238,238,238))
                guest.click(left+174,tab_y)
                wait_color(guest,"browser-first-tab-restored",check_x,check_y,(255,0,0))
                slot=min((right-left-264)//2,232)
                guest.click(left+104+slot+50,tab_y)
                wait_color(guest,"browser-second-tab-restored",check_x,check_y,(238,238,238))
                # Center of the second tab's 24px close target in Layout::tab.
                guest.click(left+104+slot+(slot-8)-44+12,tab_y)
                wait_color(guest,"browser-tab-closed",check_x,check_y,(255,0,0))
                receipt["native_tab_create_select_close_pixels"]=True
                receipt["background_tab_scroll_preserved"]=True
        # Run address replacement last so it cannot destroy the interaction fixture.
        if args.address:
            guest.key("ctrl","l")
            browser_text(guest,"https://example.com/")
            guest.key("ret")
            wait_color(guest,"browser-address-navigation",400,400,(238,238,238))
            receipt["native_address_keyboard_navigation"]=True
            receipt["omnibox_shortcut_replaces_previous_address"]=True
        if args.favorites:
            # ------------------------=
            # FUNC: value
            # DESC: Reads the installed browser's typed state rather than matching visible prose.
            # ------------------=
            def value(name): return int.from_bytes(guest.memory(*counters[name]),"little")
            # ------------------------=
            # FUNC: expect
            # DESC: Waits for a specific observable state transition with an unchanged bounded deadline.
            # ------------------=
            def expect(name,wanted):
                deadline=time.monotonic()+30
                while time.monotonic()<deadline:
                    actual=value(name)
                    if actual==wanted:return
                    time.sleep(.1)
                raise AssertionError(dict(counter=name,actual=actual,expected=wanted))
            guest.key("ctrl","l");browser_text(guest,"https://example.com/");guest.key("ret")
            wait_color(guest,"favorites-initial-page",400,400,(238,238,238));expect("FOOTER_STATUS",2)
            expect("FAVORITES_COUNT",0)
            guest.key("ctrl","d");expect("FAVORITES_COUNT",1);expect("FAVORITE_SAVED",1);expect("FAVORITES_ERROR",0)
            # The browser is at the standard installed geometry; header row hit
            # targets remain fixed independently of the document's CSS colors.
            guest.click(132,239);expect("FAVORITES_COUNT",0);expect("FAVORITE_SAVED",0)
            guest.click(132,239);expect("FAVORITES_COUNT",1)
            saved_hash=value("FAVORITES_HASH")
            guest.key("ctrl","l");browser_text(guest,"https://example.com/?other=1");guest.key("ret")
            expect("FAVORITE_SAVED",0)
            guest.click(214,239);expect("FAVORITE_SAVED",1);expect("FOOTER_STATUS",2)
            guest.screenshot("browser-favorites")
            receipt["favorites_add_remove_and_navigate"]=True
            guest.stop();guest.boot(False);guest.authenticate();guest.launch("browser",5)
            expect("FAVORITES_COUNT",1);expect("FAVORITES_HASH",saved_hash);expect("FAVORITES_ERROR",0)
            guest.click(214,239);expect("FAVORITE_SAVED",1)
            wait_color(guest,"browser-favorites-cold-reboot",400,400,(238,238,238));expect("FOOTER_STATUS",2)
            receipt["favorites_detached_cold_reboot"]=True
            receipt["favorites_payload_hash"]=saved_hash
        if args.favorites or args.footer:
            # ------------------------=
            # FUNC: expect_footer
            # DESC: Requires precise footer and destination state without depending on rendered wording.
            # ------------------=
            def expect_footer(name,wanted):
                deadline=time.monotonic()+30
                while time.monotonic()<deadline:
                    actual=int.from_bytes(guest.memory(*counters[name]),"little")
                    if actual==wanted:return
                    time.sleep(.1)
                raise AssertionError(dict(counter=name,actual=actual,expected=wanted))
            failed_url="https://expired-isrgrootx1.letsencrypt.org/"
            target_hash=0xcbf29ce484222325
            for byte in failed_url.encode():target_hash=((target_hash^byte)*0x100000001b3)&((1<<64)-1)
            guest.key("ctrl","l");browser_text(guest,failed_url);guest.key("ret")
            expect_footer("FOOTER_STATUS",1);expect_footer("DISPLAY_ADDRESS_HASH",target_hash)
            guest.screenshot("browser-footer-loading")
            expect_footer("FOOTER_STATUS",8);expect_footer("PAGE_ERROR",4);expect_footer("DISPLAY_ADDRESS_HASH",target_hash)
            guest.screenshot("browser-footer-error")
            guest.click(214,239);expect_footer("FAVORITE_SAVED",1);expect_footer("FOOTER_STATUS",2);expect_footer("PAGE_ERROR",0)
            wait_color(guest,"browser-footer-recovered",400,400,(238,238,238))
            receipt["footer_loading_error_and_recovery"]=True
            receipt["footer_failed_destination_matches_requested_url"]=True
        if args.favorites_overflow:
            # ------------------------=
            # FUNC: wait_state
            # DESC: Waits for a specific installed favorites or navigation state before the next gesture.
            # ------------------=
            def wait_state(name,wanted):
                deadline=time.monotonic()+30
                while time.monotonic()<deadline:
                    actual=int.from_bytes(guest.memory(*counters[name]),"little")
                    if actual==wanted:return
                    time.sleep(.1)
                raise AssertionError(dict(counter=name,actual=actual,expected=wanted))
            # ------------------------=
            # FUNC: url_hash
            # DESC: Computes the shared observable URL identity for overflow navigation assertions.
            # ------------------=
            def url_hash(url):
                value=0xcbf29ce484222325
                for byte in url.encode():value=((value^byte)*0x100000001b3)&((1<<64)-1)
                return value
            wait_state("FAVORITES_COUNT",1)
            for index in range(4):
                url=f"https://example.com/?favorite={index}"
                guest.key("ctrl","l");browser_text(guest,url);guest.key("ret")
                wait_state("LOCATION_HASH",url_hash(url));wait_state("LOADING",0)
                guest.key("ctrl","d");wait_state("FAVORITES_COUNT",index+2);wait_state("FAVORITES_ERROR",0)
            guest.click(214,239);wait_state("LOCATION_HASH",url_hash("https://example.com/"))
            guest.click(890,239);guest.click(214,239)
            wait_state("LOCATION_HASH",url_hash("https://example.com/?favorite=3"))
            guest.screenshot("browser-favorites-overflow")
            guest.click(854,239);guest.click(214,239)
            wait_state("LOCATION_HASH",url_hash("https://example.com/"));wait_state("LOADING",0)
            guest.screenshot("browser-favorites-full-row")
            receipt["favorites_overflow_paging_and_navigation"]=True
        if args.favorites_label:
            # ------------------------=
            # FUNC: read_favorite
            # DESC: Reads installed favorite data identity for a same-title navigation regression.
            # ------------------=
            def read_favorite(name):return int.from_bytes(guest.memory(*counters[name]),"little")
            # ------------------------=
            # FUNC: await_favorite
            # DESC: Waits for an actual saved data transition rather than matching screen text.
            # ------------------=
            def await_favorite(name,wanted):
                deadline=time.monotonic()+30
                while time.monotonic()<deadline:
                    actual=read_favorite(name)
                    if actual==wanted:return
                    time.sleep(.1)
                raise AssertionError(dict(counter=name,actual=actual,expected=wanted))
            await_favorite("FAVORITE_SAVED",1);await_favorite("LOADING",0)
            title_hash=read_favorite("FAVORITE_TITLE_HASH");count=read_favorite("FAVORITES_COUNT")
            assert title_hash!=0
            guest.key("ctrl","l");browser_text(guest,"https://example.com/?favorite=label");guest.key("ret")
            await_favorite("FAVORITE_SAVED",0);await_favorite("FOOTER_STATUS",2)
            guest.key("ctrl","d");await_favorite("FAVORITES_COUNT",count+1)
            await_favorite("FAVORITE_TITLE_HASH",title_hash);await_favorite("FAVORITES_ERROR",0)
            guest.click(890,239);guest.screenshot("browser-favorite-title-preserved")
            receipt["same_title_navigation_preserves_favorite_title"]=True
        if args.chrome:
            # ------------------------=
            # FUNC: chrome_value
            # DESC: Waits for real native menu and settings state after pointer or keyboard input.
            # ------------------=
            def chrome_value(name,wanted):
                deadline=time.monotonic()+12
                while time.monotonic()<deadline:
                    actual=int.from_bytes(guest.memory(*counters[name]),"little")
                    if actual==wanted:return
                raise AssertionError(dict(counter=name,actual=actual,wanted=wanted))
            guest.click(220,157);chrome_value("MENU",2)
            guest.screenshot("browser-settings-menu")
            guest.key("ret");chrome_value("SETTINGS",0x10100);chrome_value("MENU",0)
            guest.key("esc");chrome_value("SETTINGS",256)
            guest.click(808,197);chrome_value("SETTINGS",0x10100)
            guest.screenshot("browser-chrome-settings")
            guest.key("esc");chrome_value("SETTINGS",256)
            guest.click(140,157);chrome_value("MENU",1)
            guest.screenshot("browser-file-menu")
            guest.key("esc");chrome_value("MENU",0)
            width,height,before=read_pixels(guest.screenshot("browser-ai-collapsed"))
            guest.click(934,286)
            width,height,expanded=read_pixels(guest.screenshot("browser-ai-expanded"))
            at=(350*width+850)*3
            assert before[at:at+3]!=expanded[at:at+3]
            guest.click(934,286)
            width,height,collapsed=read_pixels(guest.screenshot("browser-ai-restored"))
            assert collapsed[at:at+3]==before[at:at+3]
            receipt["menus_url_gear_and_ai_toggle"]=True
        if args.settings:
            if int.from_bytes(guest.memory(*counters["FAVORITES_COUNT"]),"little")==0:
                guest.key("ctrl","d")
            assert int.from_bytes(guest.memory(*counters["FAVORITES_COUNT"]),"little")>0
            # ------------------------=
            # FUNC: setting_value
            # DESC: Waits for committed native preference state after real UI input.
            # ------------------=
            def setting_value(expected):
                deadline=time.monotonic()+12
                while time.monotonic()<deadline:
                    actual=int.from_bytes(guest.memory(*counters["SETTINGS"]),"little")
                    if actual==expected: return
                raise AssertionError(dict(expected=expected,actual=actual))
            guest.click(808,197);setting_value(0x10100)
            guest.click(350,361);setting_value(0x1010101)
            guest.click(500,417);setting_value(0x1010001)
            guest.screenshot("browser-settings-hidden-favorites")
            guest.key("esc");setting_value(1)
            guest.stop();guest.boot(False);guest.authenticate();guest.launch("browser",5)
            setting_value(1)
            guest.click(808,197);setting_value(0x10001)
            guest.click(500,381);setting_value(0x1010101)
            guest.click(150,537);setting_value(0x1010100)
            guest.screenshot("browser-settings-defaults")
            guest.key("tab");guest.key("ret");setting_value(0x1010100)
            guest.key("tab");guest.key("ret");setting_value(0x1010101)
            guest.key("tab");guest.key("ret");setting_value(0x1010102)
            before=int.from_bytes(guest.memory(*counters["FAVORITES_COUNT"]),"little")
            guest.click(500,473);setting_value(0x10102)
            assert int.from_bytes(guest.memory(*counters["FAVORITES_COUNT"]),"little")==before
            guest.key("esc");setting_value(258)
            guest.click(808,197);setting_value(0x10102)
            guest.click(500,473);guest.click(500,473);setting_value(0x1010102)
            assert int.from_bytes(guest.memory(*counters["FAVORITES_COUNT"]),"little")==0
            guest.click(150,537);setting_value(0x1010100)
            guest.click(500,537);setting_value(256)
            receipt["settings_controls_and_detached_persistence"]=True
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
            left,top,right,bottom=page_color_bounds(guest)
            first,second=navigation_urls()
            for label, x, y in (("link", left+40, top+32), ("back", left+38, top-66),
                               ("forward", left+90, top-66), ("reload", left+140, top-66)):
                target_url = first if label == "back" else second
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
                wait_color(guest,"browser-"+label,right-32,bottom-32,(18,52,86) if label=="back" else (70,80,90))
            receipt["https_navigation_state_and_pixels"] = True
        print(json.dumps(dict(work=str(work), **receipt)), flush=True)
        assert all(case["passed"] for case in receipt.get("cases", [])), receipt.get("cases")
    finally:
        (work / "result.json").write_text(json.dumps(receipt, indent=2) + "\n")
        guest.stop()

if __name__ == "__main__":
    main()
