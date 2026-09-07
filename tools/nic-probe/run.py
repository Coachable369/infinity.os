"""Native QEMU NIC-engine regression, NOT installed-system/M9 acceptance."""
import json
import pathlib
import shutil
import socket
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]

# ------------------------=
# FUNC: run
# DESC: Boots independent native probes on a shared QEMU link and asserts binary guest outcomes.
# ------------------=
def run(nodes=(1, 2), expected=(33, 33)):
    with tempfile.TemporaryDirectory(prefix="infinity-nic-") as temporary:
        work = pathlib.Path(temporary)
        volume = work / "volume"
        (volume / "EFI/BOOT").mkdir(parents=True)
        (volume / "EFI/INFINITY").mkdir(parents=True)
        shutil.copyfile(ROOT / "build/x86_64/BOOTX64.EFI", volume / "EFI/BOOT/BOOTX64.EFI")
        shutil.copyfile(ROOT / "build/nic-probe.elf", volume / "EFI/INFINITY/KERNEL.ELF")
        with socket.socket() as reservation:
            reservation.bind(("127.0.0.1", 0))
            port = reservation.getsockname()[1]
        children, logs = [], []
        try:
            for node in nodes:
                node_volume = work / f"volume-{node}"
                shutil.copytree(volume, node_volume)
                log = open(work / f"node-{node}.log", "wb")
                logs.append(log)
                network = f"socket,id=link,{'listen' if node == 1 else 'connect'}=127.0.0.1:{port}"
                command = ["qemu-system-x86_64", "-machine", "pc", "-m", "512M",
                           "-drive", "if=pflash,format=raw,readonly=on,file=/opt/homebrew/share/qemu/edk2-x86_64-code.fd",
                           "-drive", f"format=raw,file=fat:rw:{node_volume}", "-boot", "order=c",
                           "-netdev", network, "-device", f"e1000,netdev=link,mac=02:00:00:00:00:0{node}",
                           "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04", "-display", "none",
                           "-serial", "stdio", "-no-reboot"]
                children.append(subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT))
                if node == 1:
                    # Wait for QEMU's listener, not guest text or network response substitution.
                    import time
                    time.sleep(0.5)
            outcomes = [child.wait(timeout=40) for child in children]
            print(json.dumps({"test": "native-qemu-nic-engine", "guest_exit_codes": outcomes,
                              "installed_system_acceptance": False}))
            if outcomes != list(expected):
                for log in logs:
                    log.flush()
                for path in work.glob("*.log"):
                    print(path.read_text(errors="replace")[-3000:])
                raise SystemExit(1)
        finally:
            for child in children:
                if child.poll() is None:
                    child.terminate()
                    try:
                        child.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        child.kill()
                        child.wait()
            for log in logs:
                log.close()

if __name__ == "__main__":
    run()
    run(nodes=(1,), expected=(35,))
