"""Run real Servo initialization with native callbacks in a disposable guest."""
import json
import os
from pathlib import Path
import struct
import subprocess
import socket
import argparse

# ------------------------=
# FUNC: main
# DESC: Builds the boot fixture through the active kit and verifies structured guest results.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--swgl-probe", action="store_true")
    parser.add_argument("--page-probe", action="store_true")
    parser.add_argument("--network-probe", action="store_true")
    options = parser.parse_args()
    if options.network_probe:
        options.page_probe = True
    subprocess.run(["python3",str(Path(__file__).with_name("link-engine.py")),"--boot-probe"] +
                   (["--swgl-probe"] if options.swgl_probe else []) +
                   (["--page-probe"] if options.page_probe else []) +
                   (["--network-probe"] if options.network_probe else []),check=True)
    output=root / "build/servo-platform-probe"
    result=output / "engine-boot.bin"
    result.unlink(missing_ok=True)
    timed_out=False
    registers=None
    monitor=output / "engine-qmp.sock"
    monitor.unlink(missing_ok=True)
    process=subprocess.Popen(["qemu-system-aarch64","-machine","virt,highmem=off","-accel","tcg","-cpu","max",
            "-m","2G","-display","none","-serial","file:"+str(result),"-monitor","none",
            "-qmp","unix:"+str(monitor)+",server=on,wait=off",
            "-kernel",str(output / "engine-boot.elf")] +
            (["-netdev","user,id=browser","-device","e1000,netdev=browser,addr=1",
              "-object","filter-dump,id=packets,netdev=browser,file="+str(output / "browser-network.pcap")]
             if options.network_probe else []))
    try:
        process.wait(timeout=90 if options.network_probe else 45)
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
    raster_passed = [9,2,8,1] in records
    if options.swgl_probe:
        passed = passed and raster_passed
    page_passed = [9,2,9,1] in records
    if options.page_probe:
        passed = passed and page_passed
    network_passed = [9,2,16,1] in records
    if options.network_probe:
        passed = passed and network_passed
    diagnostics={str(kind):b"".join(struct.pack("<Q",r[3]) for r in records if r[:3]==[9,4,kind]).rstrip(b"\0").decode(errors="replace") for kind in (6,7)}
    report={"passed":passed,"timed_out":timed_out,"records":[r for r in records if r[1]!=4],"diagnostics":diagnostics,"registers":registers,"installed_os":False,"page_rendered":page_passed}
    report["software_raster_verified"] = raster_passed
    report["external_https_verified"] = network_passed
    (output / "engine-boot.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps(report,indent=2))
    return 0 if passed else 1

if __name__=="__main__":
    raise SystemExit(main())
