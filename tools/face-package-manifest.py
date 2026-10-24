#!/usr/bin/env python3
"""Record exact native verification artifacts; no runtime or security claims."""
import hashlib
import json
import os
from pathlib import Path
import sys

# ------------------------=
# FUNC: main
# DESC: Records binary hashes and licenses for the shared native face package only after all required artifacts exist.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Use the repository build kit")
    root=Path(__file__).resolve().parents[1]
    target=sys.argv[1]
    if target not in ("aarch64","x86_64"):
        raise SystemExit("Unsupported target")
    names=[f"build/face/{target}/face-worker.elf","build/face/models/yunet.iface",
           "build/face/models/sface.iface","model-cache/face/licenses/YuNet-LICENSE",
           "model-cache/face/licenses/SFace-LICENSE","build/face/native-deps/stb_image.h"]
    artifacts=[]
    for name in names:
        data=(root/name).read_bytes()
        artifacts.append(dict(path=name,bytes=len(data),sha256=hashlib.sha256(data).hexdigest()))
    manifest=dict(version=1,target=target,purpose="temporary-camera-comparison",login_enabled=False,
        runtime="native-isolated-rust-c",artifacts=artifacts,
        security_qualified=False,installed_verification=False)
    (root/f"build/face/{target}/manifest.json").write_text(json.dumps(manifest,indent=2)+"\n")

if __name__=="__main__":
    main()
