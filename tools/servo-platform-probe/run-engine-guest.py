"""Run real Servo initialization with native callbacks in a disposable guest."""
import json
import os
from pathlib import Path
import struct
import subprocess
import socket

# ------------------------=
# FUNC: main
# DESC: Builds the boot fixture through the active kit and verifies structured guest results.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    subprocess.run(["python3",str(Path(__file__).with_name("link-engine.py")),"--boot-probe"],check=True)
    output=root / "build/servo-platform-probe"
    result=output / "engine-boot.bin"
    result.unlink(missing_ok=True)
    timed_out=False
    registers=None
    monitor=output / "engine-qmp.sock"
    monitor.unlink(missing_ok=True)
    process=subprocess.Popen(["qemu-system-aarch64","-machine","virt","-accel","tcg","-cpu","max",
            "-m","2G","-display","none","-serial","file:"+str(result),"-monitor","none",
            "-qmp","unix:"+str(monitor)+",server=on,wait=off",
            "-kernel",str(output / "engine-boot.elf")])
    try:
        process.wait(timeout=45)
    except subprocess.TimeoutExpired:
        timed_out=True
        with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as connection:
            connection.settimeout(3)
            connection.connect(str(monitor))
            stream=connection.makefile("rwb",buffering=0)
            stream.readline()
            for command in ({"execute":"qmp_capabilities"},
                            {"execute":"human-monitor-command","arguments":{"command-line":"info registers"}}):
                stream.write(json.dumps(command).encode()+b"\n")
                while True:
                    response=json.loads(stream.readline())
                    if "return" in response:
                        registers=response["return"]
                        break
    finally:
        if process.poll() is None:
            process.kill()
        process.wait()
    data=result.read_bytes()
    records=[list(struct.unpack("<4Q",data[i:i+32])) for i in range(0,len(data)-31,32)]
    passed=not timed_out and bool(records) and records[-1][:3]==[9,0,5]
    report={"passed":passed,"timed_out":timed_out,"records":records,"registers":registers,"installed_os":False,"page_rendered":False}
    (output / "engine-boot.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps(report,indent=2))
    return 0 if passed else 1

if __name__=="__main__":
    raise SystemExit(main())
