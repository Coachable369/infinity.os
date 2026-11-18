#!/usr/bin/env python3
"""Fetch only the pinned speaker model; no changes to speech recognition or TTS assets."""
import hashlib
import os
from pathlib import Path
import urllib.request
PIN='5ef208a9da1453335308a6b6f4e6dfbd7e183a38b604de0a57664f45d257fe94'
URL='https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/wespeaker_en_voxceleb_resnet34.onnx'
# ------------------------=
# FUNC: main
# DESC: Downloads the fixed speaker embedding model atomically and rejects altered or oversized payloads.
# ------------------=
def main():
    if os.environ.get('INFINITY_BUILD_KIT_ACTIVE')!='1':raise SystemExit('Use the build kit')
    target=Path(__file__).resolve().parents[1]/'model-cache/speaker/resnet34.onnx'
    if target.exists() and hashlib.sha256(target.read_bytes()).hexdigest()==PIN:return
    target.parent.mkdir(parents=True,exist_ok=True)
    with urllib.request.urlopen(URL,timeout=60) as response:data=response.read(27*1024*1024)
    if len(data)!=26534365 or hashlib.sha256(data).hexdigest()!=PIN:raise ValueError('speaker model pin mismatch')
    pending=target.with_suffix('.download');pending.write_bytes(data);pending.replace(target)
if __name__=='__main__':main()
