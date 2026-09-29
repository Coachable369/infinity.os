"""Real URL comparison from a freshly installed, ISO-detached native guest.

This first case is GET acceptance, not full curl feature acceptance.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import struct
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("installed", ROOT / "tools/ms9-installed-acceptance.py")
installed = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installed)

# ------------------------=
# FUNC: configure_nat
# DESC: Configures this isolated QEMU NAT network through real Settings fields, without guest memory writes or ambient grants.
# ------------------=
def configure_nat(guest):
    guest.launch("network", 8, 6)
    guest.key("right"); guest.key("right")
    for control, value in ((1, "10.0.2.15"), (2, "24"), (3, "10.0.2.2")):
        guest.key("down"); guest.key("ret")
        guest.wait(lambda s: bool(s[9] & 4), "network field editing")
        guest.text(value); guest.key("ret")
        guest.wait(lambda s: not s[9] & 12, "network field committed")
    guest.key("right")
    guest.key("down"); guest.key("ret")
    guest.wait(lambda s: bool(s[9] & 4), "DNS field editing")
    guest.text("10.0.2.3"); guest.key("ret")
    guest.wait(lambda s: not s[9] & 12, "DNS field committed")
    guest.key("esc")

# ------------------------=
# FUNC: main
# DESC: Runs reference curl and native geturl against one real URL; compares typed status and complete response length/hash, never console prose.
# ------------------=
def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--url", default="https://example.com/")
    parser.add_argument("--reuse-installed", action="store_true",
                        help="Reuse only this harness's existing disk and immutable artifacts")
    parser.add_argument("--diagnose-network", action="store_true")
    parser.add_argument("--configure-nat", action="store_true")
    args=parser.parse_args()
    work=args.output.resolve()
    artifacts=work / "artifacts"
    if args.reuse_installed:
        assert (work/"reference.json").is_file() and artifacts.is_dir()
    else:
        work.mkdir(parents=True,exist_ok=False)
        artifacts.mkdir()
        for source,name in [("builds/InfinityOS-x86_64.iso","installer.iso"),("build/x86_64/kernel.elf","kernel.elf"),("build/x86_64/installed-kernel.elf","installed-kernel.elf")]:
            shutil.copyfile(ROOT/source, artifacts/name)
    baseline=work/"curl-body.bin"
    reference=subprocess.run(["curl","--http1.1","--ipv4","--noproxy","*","--max-time","30","--silent","--show-error","--output",str(baseline),"--write-out","%{http_code}",args.url],capture_output=True)
    assert reference.returncode==0,{"curl_exit":reference.returncode,"error":reference.stderr.decode(errors="replace")}
    expected={"status":int(reference.stdout),"length":baseline.stat().st_size,"sha256":hashlib.sha256(baseline.read_bytes()).hexdigest()}
    (work/"reference.json").write_text(json.dumps(expected,indent=2))
    guest=installed.Guest(work,1,"/opt/homebrew/share/qemu/edk2-x86_64-code.fd",reuse=args.reuse_installed)
    receipt={"url":args.url,"reference":expected,"installed_get":False,"full_curl_parity":False,
             "reused_installation":args.reuse_installed,"configured_test_nat":args.configure_nat,
             "installed_kernel_sha256":hashlib.sha256((artifacts/"installed-kernel.elf").read_bytes()).hexdigest()}
    try:
        if not args.reuse_installed:
            guest.install(); guest.stop(); guest.onboard()
        else:
            guest.boot(False); guest.authenticate()
        guest.qmp("object-add", {"qom-type": "filter-dump", "id": "geturl-capture",
                                "netdev": "net", "file": str(work/f"https-{time.time_ns()}.pcap")})
        if args.configure_nat:
            configure_nat(guest)
        guest.launch("command",5)
        guest.fast_commands = True
        if args.diagnose_network:
            for command in ("network status", "network interface-list", "network address-list", "network route-list", "network diagnostics"):
                guest.command(command)
        guest.command("https authorize confirm=true")
        address,size=installed.symbol(artifacts/"installed-kernel.elf","INFINITY_GETURL_EXIT")
        body_address,body_size=installed.symbol(artifacts/"installed-kernel.elf","INFINITY_GETURL_RESPONSE")
        assert size==4 and body_size==48
        guest.command("geturl "+args.url)
        # TCG virtual time and cryptographic execution can lag host wall time.
        # This does not extend the actor's own thirty-second guest deadline.
        deadline=time.monotonic()+180
        while True:
            guest.qmp("stop")
            try:
                code=struct.unpack("<I",guest.memory(address,size))[0]
                response=struct.unpack("<6Q",guest.memory(body_address,body_size))
            finally: guest.qmp("cont")
            receipt["native_exit"]=code
            if code!=0xffffffff: break
            assert time.monotonic()<deadline,{"pending_timeout":True}
            time.sleep(.1)
        receipt.update({"native_exit":code,"native_status":response[0],"native_length":response[1],"native_sha256":struct.pack("<4Q",*response[2:]).hex()})
        guest.screenshot("geturl-result")
        assert code==0,receipt
        assert (response[0],response[1],receipt["native_sha256"])==(expected["status"],expected["length"],expected["sha256"]),receipt
        receipt["installed_get"]=True
    except Exception as error:
        receipt["failure"]=repr(error)
        if guest.process is not None:
            guest.screenshot("geturl-failure")
        raise
    finally:
        result_name=f"result-{time.time_ns()}.json" if args.reuse_installed else "result.json"
        (work/result_name).write_text(json.dumps(receipt,indent=2))
        guest.stop()

if __name__=="__main__": main()
