"""Exercise the native launcher, real GUI edits and AIKit recall on private installed media."""
import argparse
import importlib.util
import json
import os
import subprocess
from pathlib import Path
import time

ROOT = Path(__file__).resolve().parents[1]

# ------------------------=
# FUNC: load
# DESC: Reuses the repository's keyboard, compositor and installed guest harnesses.
# ------------------=
def load(name, relative):
    spec = importlib.util.spec_from_file_location(name, ROOT / relative)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

memory = load("browser_memory_fixture", "tools/aikit-memory-installed-test.py")
tk = load("browser_tk_fixture", "tools/native-python/tk-installed.py")
python = load("browser_python_fixture", "tools/native-python/installed.py")
native, advice = memory.native, memory.advice

# ------------------------=
# FUNC: window
# DESC: Requires the expected native surface and a live successful Python task.
# ------------------=
def window(guest):
    return tk.wait(guest, python, lambda: tk.gui(guest, native),
                   lambda g: len(g["windows"]) == 1 and g["windows"][0]["visible"]
                   and g["windows"][0]["width"] >= 640, timeout=90)["windows"][0]

# ------------------------=
# FUNC: main
# DESC: Runs launcher, recall and reboot checks using real inputs against the installed account.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1"
    parser = argparse.ArgumentParser()
    parser.add_argument("--iso", type=Path, required=True)
    parser.add_argument("--work", type=Path, required=True)
    parser.add_argument("--reuse", action="store_true")
    parser.add_argument("--kernel", type=Path, help="Test a corrected kernel on the stopped private QA disk")
    args = parser.parse_args()
    work = args.work.resolve()
    assert work.parent == ROOT / "build" and work.name.startswith("memory-browser-")
    artifacts = work / "artifacts"
    image = args.iso.resolve()
    assert image.is_relative_to(ROOT)
    if not args.reuse:
        artifacts.mkdir(parents=True)
        native.desktop.copy_artifact(image, artifacts / "installer.iso")
        native.desktop.extract_media_kernels(image, artifacts, "aarch64")
    else:
        assert advice.sha256(image) == advice.sha256(artifacts / "installer.iso")
    if args.kernel:
        assert args.reuse and args.kernel.resolve().is_relative_to(ROOT / "build")
        disk=work/"node-1/installed.raw"
        opened=subprocess.run(["lsof","-t",str(disk)],capture_output=True)
        assert opened.returncode==1 and not opened.stdout
        subprocess.run(["python3",str(ROOT/"tools/update-installed-clone.py"),str(disk),str(args.kernel),"--apply"],check=True)
        (artifacts/"installed-kernel.elf").unlink()
        native.desktop.copy_artifact(args.kernel.resolve(), artifacts/"installed-kernel.elf")
    guest = advice.AdviceGuest(work, 1, "/opt/homebrew/share/qemu/edk2-aarch64-code.fd",
                               reuse=args.reuse, width=1024, height=768, memory_mb=20480)
    guest.arch, guest.acceleration, guest.patched = "aarch64", "hvf", True
    guest.boot_timeout_seconds, guest.install_timeout_seconds = 600, 1800
    proof = {"accepted":False, "updated_private_kernel":str(args.kernel) if args.kernel else None}
    output = ROOT / "builds/validation/aikit-memory/browser-installed.json"
    try:
        if not args.reuse:
            guest.install()
            guest.stop()
        guest.onboard()
        proof["devices"] = advice.detached(guest)
        guest.launch("command", 5)
        if args.reuse:
            # This disposable test owns these synthetic markers.
            for _ in range(16):
                prior=memory.command(guest,"search ORBIT")
                if prior[5]==0:
                    break
                assert memory.command(guest,f"forget {prior[6]}")[3]==1
        guest.command("ai new")
        repair=memory.command(guest,"remember Install the missing compiler dependency to fix the build")
        assert repair[3]==0
        command_bounds=native.features(guest)[21:25]
        previous = python.snapshot(guest)[2]
        guest.launch("Infinity Memory", 5)
        bounds = window(guest)
        time.sleep(2)
        proof["window"]=bounds
        wx,wy,ww,wh=(bounds[k] for k in ("x","y","width","height"))
        proof["initial"] = str(guest.screenshot("memory-browser-initial"))
        native.adaptive_click(guest, wx+142,wy+97)
        guest.text("Compilation failed because a library was absent")
        time.sleep(1)
        proof["semantic"]=str(guest.screenshot("memory-browser-semantic"))
        native.adaptive_click(guest,wx+ww-65,wy+33)
        native.adaptive_click(guest,wx+ww-170,wy+wh//2)
        guest.text("The diagnostic project code is ORBIT-7331.")
        native.adaptive_click(guest,wx+ww-88,wy+wh-66)
        time.sleep(1)
        native.adaptive_click(guest,wx+ww-170,wy+wh//2)
        guest.key("end")
        for _ in range(5):
            guest.key("backspace")
        guest.text("7429.")
        native.adaptive_click(guest,wx+ww-88,wy+wh-66)
        time.sleep(1)
        proof["saved"] = str(guest.screenshot("memory-browser-record"))
        native.adaptive_click(guest, *tk.close_point(guest,window(guest)))
        proof["reclaimed"] = list(tk.finished(guest,python,previous))
        proof["after_close"]=str(guest.screenshot("memory-browser-after-close"))
        assert command_bounds[2]>200 and command_bounds[3]>100,command_bounds
        native.adaptive_click(guest, command_bounds[0]+100, command_bounds[1]+command_bounds[3]-40)
        lookup=memory.command(guest,"search ORBIT")
        assert lookup[5]==1 and lookup[6]>0,lookup
        identity=lookup[6]
        before=advice.wait(guest,lambda s:s["native_ready"]==s["selected_ready"]==1,"model available",300)
        guest.command("ai ask What is the diagnostic project code")
        admitted=advice.wait(guest,lambda s:s["admitted_turn"]>before["admitted_turn"],"separate question admitted",60)
        proof["answer_receipt"]=advice.completed(guest,admitted,(before["audit_count"],before["audit_fingerprint"]),240,True)
        proof["answer"]=str(guest.screenshot("memory-browser-model-answer"))
        guest.stop()
        guest.boot(False)
        guest.authenticate()
        guest.launch("command",5)
        assert memory.command(guest,"search ORBIT")[5:7] == (1,identity)
        assert memory.command(guest,f"forget {identity}")[3] == 1
        guest.stop()
        guest.boot(False)
        guest.authenticate()
        guest.launch("command",5)
        assert memory.command(guest,"search ORBIT")[5] == 0
        other=memory.command(guest,"search compiler")
        assert other[5]==1 and memory.command(guest,f"forget {other[6]}")[3]==1
        proof["accepted"]=True
    except BaseException as error:
        proof["failure"]=str(error)
        if guest.process and guest.process.poll() is None:
            proof["failure_image"]=str(guest.screenshot("memory-browser-failure"))
        raise
    finally:
        guest.stop()
        output.write_text(json.dumps(proof,indent=2)+"\n")

if __name__=="__main__":
    main()
