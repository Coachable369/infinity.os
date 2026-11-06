"""Stage the browser mutex dependency with native cooperative parking."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tomllib

# ------------------------=
# FUNC: main
# DESC: Verifies the pinned dependency archive and selects native park/wake only for InfinityOS targets.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root=Path(__file__).resolve().parents[2]
    spec=importlib.util.spec_from_file_location("staging",Path(__file__).with_name("prepare-async-net.py"))
    helper=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    servo=root/"build/servo-port-audit"
    original=subprocess.check_output(["git","-C",str(servo),"show","HEAD:Cargo.lock"],text=True)
    packages=tomllib.loads(original)["package"]
    package=next(p for p in packages if p["name"]=="parking_lot_core")
    directory,package=helper.stage(root,packages,"parking_lot_core",package["version"])
    path=directory/"src/thread_parker/mod.rs"
    text=path.read_text()
    marker='    if #[cfg(any(target_os = "linux", target_os = "android"))] {'
    if text.count(marker)!=1:
        raise SystemExit("Unexpected parking_lot platform dispatch")
    text=text.replace(marker,'''    if #[cfg(all(target_os = "none", infinity_native))] {
        #[path = "infinity.rs"]
        mod imp;
    } else if #[cfg(any(target_os = "linux", target_os = "android"))] {''',1)
    path.write_text(text)
    path.with_name("infinity.rs").write_bytes((root/"sdk/servo-std/parking_lot.rs").read_bytes())
    header='name = "'+package["name"]+'"\nversion = "'+package["version"]+'"\n'
    entry=header+'source = "'+package["source"]+'"\nchecksum = "'+package["checksum"]+'"\n'
    lock=servo/"Cargo.lock"
    lock.write_text(lock.read_text().replace(entry,header))

if __name__=="__main__":
    main()
