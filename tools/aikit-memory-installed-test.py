"""Fresh installed memory CRUD across two cold boots, using native keyboard input and typed diagnostics."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import platform
import time

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("memory_advice", ROOT / "tools/llm-advice-installed-test.py")
advice = importlib.util.module_from_spec(spec)
spec.loader.exec_module(advice)
native = advice.native

# ------------------------=
# FUNC: snapshot
# DESC: Reads native structured receipts; prose and log strings are not acceptance oracles.
# ------------------=
def snapshot(guest):
    return native.read(guest, "INFINITY_MEMORY_SNAPSHOT", 10)

# ------------------------=
# FUNC: command
# DESC: Executes real terminal input and waits for a new durable memory receipt.
# ------------------=
def command(guest, text):
    previous = snapshot(guest)[2]
    guest.command("ai memory " + text)
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline:
        state = snapshot(guest)
        if state[:2] == (0x494d454d303031, 1) and state[2] != previous:
            assert state[3] != 3, (text, state)
            return state
        time.sleep(.2)
    raise AssertionError((text, snapshot(guest)))

# ------------------------=
# FUNC: main
# DESC: Installs unchanged media into a private disk, then verifies committed records and deletions after actual cold boots without the ISO.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1"
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arch", choices=("aarch64", "x86_64"), required=True)
    parser.add_argument("--iso", type=Path, required=True)
    parser.add_argument("--install-timeout", type=int, default=1800,
                        help="Maximum seconds per installation stage on the emulated test host")
    args = parser.parse_args()
    if args.install_timeout <= 0:
        parser.error("--install-timeout must be positive")
    iso = args.iso.resolve()
    assert iso.is_relative_to(ROOT) and iso.is_file()
    work = ROOT / "build" / f"aikit-memory-installed-{args.arch}-{time.time_ns()}"
    artifacts = work / "artifacts"
    artifacts.mkdir(parents=True)
    native.desktop.copy_artifact(iso, artifacts / "installer.iso")
    media = native.desktop.extract_media_kernels(iso, artifacts, args.arch)
    firmware = Path(f"/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd")
    guest = advice.AdviceGuest(work, 1, str(firmware), reuse=False, width=1024, height=768, memory_mb=20480)
    guest.arch = args.arch
    guest.acceleration = "hvf" if args.arch == "aarch64" and platform.system() == "Darwin" and platform.machine() == "arm64" else "tcg"
    guest.patched = True
    guest.install_timeout_seconds = args.install_timeout
    guest.boot_timeout_seconds = 600
    proof = dict(architecture=args.arch, media=media, source=str(iso), accepted=False, screenshots=[])
    destination = ROOT / "builds/validation/aikit-memory" / args.arch / "installed.json"
    destination.parent.mkdir(parents=True, exist_ok=True)
    try:
        guest.install()
        guest.stop()
        proof["identity"] = guest.onboard()
        proof["detached_devices"] = advice.detached(guest)
        guest.launch("command", 5)
        before = command(guest, "status")[4]
        saved = command(guest, "remember Moonquartz preference is Rust")
        assert saved[3] == 0 and saved[4] == before + 1, saved
        found = command(guest, "search Moonquartz")
        assert found[5] == 1 and found[6] != 0, found
        identity = found[6]
        command(guest, f"update {identity} to Moonquartz preference is Python")
        assert command(guest, "search Rust")[5] == 0
        proof["screenshots"].append(str(guest.screenshot("memory-before-reboot")))
        guest.stop()
        guest.onboard()
        advice.detached(guest)
        guest.launch("command", 5)
        restored = command(guest, "search Moonquartz")
        assert restored[5:7] == (1, identity), restored
        assert command(guest, "search Python")[5] == 1
        removed = command(guest, f"forget {identity}")
        assert removed[3] == 1 and removed[4] == before
        guest.stop()
        guest.onboard()
        advice.detached(guest)
        guest.launch("command", 5)
        assert command(guest, "search Moonquartz")[5] == 0
        proof["screenshots"].append(str(guest.screenshot("memory-forgotten-after-reboot")))
        proof.update(accepted=True, cold_boots=2, record_id=identity)
    except BaseException as error:
        proof["failure"] = str(error)
        raise
    finally:
        guest.stop()
        destination.write_text(json.dumps(proof, indent=2) + "\n")
        print(str(destination), flush=True)

if __name__ == "__main__":
    main()
