"""Reproducible disposable guest capture; run exclusively through build-kit run.

Only firmware transport evidence. Never installed-system or process-isolation proof.
Camera images remain local and must not be committed or uploaded.
"""
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]

# ------------------------=
# FUNC: command
# DESC: Runs a bounded diagnostic command with repository-local build environment.
# ------------------=
def command(*args, check=True):
    return subprocess.run([str(a) for a in args], cwd=ROOT, check=check, timeout=60,
                          text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)

# ------------------------=
# FUNC: main
# DESC: Builds a probe, captures through an owned disposable VM, and preserves raw evidence across full builds.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise RuntimeError("Use ./build-kit run python3 tools/camera-probe/run.py")
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-" + uuid.uuid4().hex[:8]
    out = ROOT / "builds/validation/camera" / run_id
    out.mkdir(parents=True)
    os.environ["INFINITY_CAMERA_OUTPUT"] = str(out)
    vm = "infinityos-camera-probe-" + run_id
    owned = False
    attached = False
    running = False
    evidence = {"vm": vm, "run_id": run_id, "installed_proof": False,
                "isolated_delivery_proof": False, "errors": []}
    try:
        command("sh", "tools/camera-probe/build.sh")
        command("python3", "tools/camera-probe/disk.py")
        command("VBoxManage", "convertfromraw", out / "probe.raw", out / "probe.vdi", "--format", "VDI")
        command("VBoxManage", "createvm", "--name", vm, "--platform-architecture", "ARM",
                "--ostype", "Other_arm64", "--basefolder", out, "--register")
        owned = True
        command("VBoxManage", "modifyvm", vm, "--memory", "1024", "--cpus", "1",
                "--firmware", "efi", "--graphicscontroller", "vmsvga", "--vram", "32",
                "--usb-xhci", "on", "--audio-enabled", "off", "--uart1", "0x3f8", "4",
                "--uartmode1", "file", out / "serial.log")
        command("VBoxManage", "storagectl", vm, "--name", "Probe", "--add", "virtio-scsi",
                "--controller", "VirtIO", "--bootable", "on")
        command("VBoxManage", "storageattach", vm, "--storagectl", "Probe", "--port", "0",
                "--device", "0", "--type", "hdd", "--medium", out / "probe.vdi")
        running = True
        command("VBoxManage", "startvm", vm, "--type", "gui")
        attached = True
        command("VBoxManage", "controlvm", vm, "webcam", "attach", ".1")
        # 60-second enumeration bound + 5-second capture bound + startup allowance.
        # Do not use serial text as an acceptance oracle.
        for _ in range(15):
            time.sleep(5)
    except Exception as error:
        evidence["errors"].append(str(error))
    finally:
        cleanup = []
        if attached:
            cleanup.append(("webcam", "detach", ".1"))
        if running:
            cleanup.append(("poweroff",))
        for action in cleanup:
            try:
                result = command("VBoxManage", "controlvm", vm, *action, check=False)
                if result.returncode:
                    evidence["errors"].append(str(action) + ": " + result.stderr)
            except Exception as error:
                evidence["errors"].append(str(action) + ": " + str(error))
        evidence["registered_vm_retained"] = owned
        (out / "run.json").write_text(json.dumps(evidence, indent=2) + "\n")
    if evidence["errors"]:
        raise RuntimeError(evidence["errors"])
    command("VBoxManage", "clonemedium", "disk", out / "probe.vdi", out / "evidence.raw", "--format", "RAW")
    for name in ("device.bin", "config.bin", "capture.bin", "probe.bin", "usb.bin", "shutdown.bin"):
        command("mcopy", "-i", str(out / "evidence.raw") + "@@1048576", "::/" + name, out / name)
    command("python3", "tools/camera-probe/inspect.py", out)
    result = command("python3", "tools/camera-probe/capture_report.py", out / "capture.bin")
    print(result.stdout, flush=True)
    result = json.loads(result.stdout)
    if result["state"] != "frame":
        raise RuntimeError("Real frame acceptance failed; see " + str(out))
    command("mcopy", "-i", str(out / "evidence.raw") + "@@1048576", "::/frame.jpg", out / "frame.jpg")
    print("Encoded frame preserved locally; decoding and installed isolation remain unverified.")

if __name__ == "__main__":
    main()
