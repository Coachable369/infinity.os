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
    parser.add_argument("--url", help="Public HTTPS destination for the native network probe")
    parser.add_argument("--assert-script", help="JavaScript boolean over real page state; required with --url")
    parser.add_argument("--network-only", action="store_true", help="Retest only site compatibility after engine fixtures have passed")
    options = parser.parse_args()
    if bool(options.url) != bool(options.assert_script) or (options.network_only and not (options.network_probe or options.url)):
        parser.error("--url requires --assert-script; --network-only requires --network-probe")
    environment=os.environ.copy()
    if options.url:
        options.network_probe=True
        environment["INFINITY_BROWSER_PROBE_URL"]=options.url
        environment["INFINITY_BROWSER_PROBE_ASSERT"]=options.assert_script
    if options.network_only:
        environment["INFINITY_BROWSER_NETWORK_ONLY"]="1"
    if options.network_probe:
        options.page_probe = True
    subprocess.run(["python3",str(Path(__file__).with_name("link-engine.py")),"--boot-probe"] +
                   (["--swgl-probe"] if options.swgl_probe else []) +
                   (["--page-probe"] if options.page_probe else []) +
                   (["--network-probe"] if options.network_probe else []),env=environment,check=True)
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
        process.wait(timeout=180 if options.network_probe else 45)
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
    dimensions=next((r[3] for r in records if r[:3]==[9,2,36]),None)
    screenshot=None
    if dimensions is not None:
        width,height=dimensions>>32,dimensions&0xffffffff
        rgba=b"".join(struct.pack("<Q",r[3]) for r in records if r[1]==5)
        if len(rgba)==width*height*4:
            from PIL import Image
            screenshot=output/"browser-page.png"
            Image.frombytes("RGBA",(width,height),rgba).save(screenshot)
    cookie_only=environment.get("INFINITY_BROWSER_COOKIE_ONLY")=="1"
    report={"passed":passed,"timed_out":timed_out,"records":[r for r in records if r[1] not in (4,5)],"diagnostics":diagnostics,"registers":registers,"installed_os":False,"page_rendered":page_passed and not cookie_only,"cookie_flow_verified":[9,2,38,1] in records,"screenshot":str(screenshot) if screenshot else None}
    report["software_raster_verified"] = raster_passed
    report["browser_interactions_verified"] = all([9,2,40,step] in records for step in range(1,9))
    report["external_https_verified"] = network_passed
    report["load_completed"] = [9,2,32,1] in records if options.network_probe else None
    report["requested_url"] = environment.get("INFINITY_BROWSER_PROBE_URL", "https://www.google.com/") if options.network_probe else None
    report["network_only"] = options.network_only
    (output / "engine-boot.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps(report,indent=2))
    return 0 if passed else 1

if __name__=="__main__":
    raise SystemExit(main())
