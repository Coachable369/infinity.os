"""Reproduce the pinned native browser component after a clean repository build."""
import argparse
import io
import os
from pathlib import Path
import subprocess
import tarfile

REVISION="d05154e2b4def11a9fefe412898a0a6c8925a9cd"

# ------------------------=
# FUNC: main
# DESC: Prepares pinned sources and dependencies before native code generation and linkage, with all outputs inside the build workspace.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE")!="1":
        raise SystemExit("Run through build-kit")
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--arch",choices=("aarch64","x86_64"),required=True)
    options=parser.parse_args()
    root=Path(__file__).resolve().parents[2]
    source=root/"build/servo-port-audit"
    if not source.exists():
        source.mkdir(parents=True)
        subprocess.run(["git","init",str(source)],check=True)
        subprocess.run(["git","-C",str(source),"fetch","--depth=1","https://github.com/servo/servo.git",REVISION],check=True)
        subprocess.run(["git","-C",str(source),"checkout","--detach",REVISION],check=True)
    actual=subprocess.check_output(["git","-C",str(source),"rev-parse","HEAD"],text=True).strip()
    if actual!=REVISION:
        raise SystemExit("Refusing an unreviewed Servo source revision")
    ready=root/"build/servo-platform-probe/source-ready"
    if not ready.exists():
        # Fetch from the original tree, never a previous mutable native overlay.
        fetch=root/"build/servo-fetch-source"
        fetch.mkdir(parents=True,exist_ok=True)
        tree=subprocess.check_output(["git","-C",str(source),"archive",REVISION])
        with tarfile.open(fileobj=io.BytesIO(tree)) as archive:
            archive.extractall(fetch,filter="data")
        environment=dict(os.environ,CARGO_HOME=str(root/"build/servo-cargo-home"))
        subprocess.run(["cargo","fetch","--locked","--manifest-path",str(fetch/"Cargo.toml")],env=environment,check=True)
        ready.parent.mkdir(parents=True,exist_ok=True)
        ready.write_text(REVISION+"\n")
    scripts=Path(__file__).parent
    subprocess.run(["python3",str(scripts/"prepare-std.py")],check=True)
    subprocess.run(["python3",str(scripts/"check-servo.py"),"--arch",options.arch,"--codegen"],check=True)
    subprocess.run(["python3",str(scripts/"link-engine.py"),"--arch",options.arch,"--component"],check=True)

if __name__=="__main__":
    main()
