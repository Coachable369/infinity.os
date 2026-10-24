#!/usr/bin/env python3
"""Fetch hash-pinned face data and leaf C source, never execute remote code."""
import hashlib
import os
from pathlib import Path
import urllib.request

ROOT=Path(__file__).resolve().parents[1]
ZOO="https://raw.githubusercontent.com/opencv/opencv_zoo/48b10fa505aeb51fbd127fc60a5c660040e823b7/models/"
MEDIA=ZOO.replace("raw.githubusercontent.com/", "media.githubusercontent.com/media/")
ASSETS=(
    ("model-cache/face/licenses/YuNet-LICENSE",ZOO+"face_detection_yunet/LICENSE",
     "c83b8120c50ccbd4c4f96edf53141bdd566ebb8f8e9227e415326aa1b1aba958",1085),
    ("model-cache/face/licenses/SFace-LICENSE",ZOO+"face_recognition_sface/LICENSE",
     "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30",11358),
    ("model-cache/face/yunet.onnx",MEDIA+"face_detection_yunet/face_detection_yunet_2023mar.onnx",
     "8f2383e4dd3cfbb4553ea8718107fc0423210dc964f9f4280604804ed2552fa4",232589),
    ("model-cache/face/sface.onnx",MEDIA+"face_recognition_sface/face_recognition_sface_2021dec.onnx",
     "0ba9fbfa01b5270c96627c4ef784da859931e02f04419c829e83484087c34e79",38696353),
    ("build/face/native-deps/stb_image.h","https://raw.githubusercontent.com/nothings/stb/003501f6d7a8668ce8b6d875e34dd97ad0d5d5b5/stb_image.h",
     "594c2fe35d49488b4382dbfaec8f98366defca819d916ac95becf3e75f4200b3",None),
)

# ------------------------=
# FUNC: fetch
# DESC: Downloads to a bounded temporary file and atomically publishes only data matching its audited hash and size.
# ------------------=
def fetch(relative,url,digest,size):
    path=ROOT/relative
    if path.exists() and hashlib.sha256(path.read_bytes()).hexdigest()==digest:
        return
    path.parent.mkdir(parents=True,exist_ok=True)
    temporary=path.with_suffix(path.suffix+".download")
    try:
        checksum=hashlib.sha256();total=0
        with urllib.request.urlopen(url,timeout=30) as response,temporary.open("wb") as output:
            while chunk:=response.read(65536):
                total+=len(chunk)
                if total>(size if size is not None else 1024*1024):
                    raise ValueError("asset exceeded its byte budget")
                checksum.update(chunk);output.write(chunk)
        if checksum.hexdigest()!=digest or (size is not None and total!=size):
            raise ValueError("asset hash or size mismatch")
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)

# ------------------------=
# FUNC: main
# DESC: Runs the pinned asset fetch only under repository build authority; no dependency manager or host inference runtime is installed.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE")!="1":
        raise SystemExit("Use ./build-kit run python3 tools/face-assets-fetch.py")
    for arguments in ASSETS:
        fetch(*arguments)

if __name__=="__main__":
    main()
