#!/usr/bin/env python3
"""Verify embedded native speech resources in actual linked live/installed ELF images."""
from pathlib import Path
import hashlib
import json
import struct
import subprocess
import sys
from reference import MODEL_SHA256

ROOT = Path(__file__).resolve().parents[2]


# ------------------------=
# FUNC: verify
# DESC: Checks immutable resource bytes in loadable segments and executable constructor destinations.
# ------------------=
def verify(path):
    image = Path(path).read_bytes()
    assert image[:6] == b"\x7fELF\x02\x01"
    machine = struct.unpack_from("<H", image, 18)[0]
    assert machine in (62, 183)
    arch = "aarch64" if machine == 183 else "x86_64"
    phoff, shoff = struct.unpack_from("<QQ", image, 32)
    phsize, phcount, shsize, shcount, names_index = struct.unpack_from("<5H", image, 54)
    loads = [struct.unpack_from("<II6Q", image, phoff + i * phsize) for i in range(phcount)]
    loads = [entry for entry in loads if entry[0] == 1]
    if machine == 62:
        # PC firmware reserves the low-memory PCI aperture. The native arena
        # must be a separate zero-fill segment, never an extent across it.
        arenas = [p for p in loads if p[3] == 0x100000000]
        assert len(arenas) == 1
        assert arenas[0][1] == 6 and arenas[0][5] == 0
        assert arenas[0][6] == 1536 * 1024 * 1024
        assert all(p[3] == p[4] and
                   (p[3] + p[6] <= 0xC0000000 or p == arenas[0]) for p in loads)
    sections = [struct.unpack_from("<II4QII2Q", image, shoff + i * shsize) for i in range(shcount)]
    names = sections[names_index]
    names = image[names[4]:names[4] + names[5]]
    constructors = None
    for section in sections:
        name = names[section[0]:].split(b"\0", 1)[0]
        if name == b".kokoro_init_array":
            constructors = image[section[4]:section[4] + section[5]]
    assert constructors and len(constructors) % 8 == 0
    for (address,) in struct.iter_unpack("<Q", constructors):
        assert any(p[1] & 1 and p[3] <= address < p[3] + p[5] for p in loads)
    model = ROOT / "model-cache/kokoro-v1_0.gguf"
    assert hashlib.sha256(model.read_bytes()).hexdigest() == MODEL_SHA256
    resources = [model, ROOT / "build/voice-kokoro" / arch / "THIRD-PARTY-NOTICES.txt"]
    resources += sorted(p for p in (ROOT / "build/voice-kokoro/reference/_deps/espeak-build/espeak-ng-data").rglob("*") if p.is_file())
    total = 0
    for resource in resources:
        content = resource.read_bytes()
        assert content, resource
        offset = image.find(content)
        assert offset >= 0, resource
        assert any(p[2] <= offset and offset + len(content) <= p[2] + p[5] for p in loads), resource
        total += len(content)
    return dict(image=str(path), resources=len(resources), resource_bytes=total,
                model_sha256=hashlib.sha256(model.read_bytes()).hexdigest(),
                constructors=len(constructors) // 8)


# ------------------------=
# FUNC: main
# DESC: Fails packaging if either runtime omits real speech model, phonemizer, notices or constructor data.
# ------------------=
def main():
    arguments = sys.argv[1:]
    embedded = "--embedded-install" in arguments
    arguments = [argument for argument in arguments if argument != "--embedded-install"]
    media = None
    if "--installer-media" in arguments:
        index = arguments.index("--installer-media")
        if index + 1 >= len(arguments):
            raise SystemExit("--installer-media requires a FAT image")
        media = arguments[index + 1]
        del arguments[index:index + 2]
    if not arguments:
        raise SystemExit("Provide linked kernel ELF paths")
    results = [verify(path) for path in arguments]
    if embedded:
        assert len(arguments) == 2
        installed, live = [Path(path).read_bytes() for path in arguments]
        assert live.find(installed) >= 0, "Live installer does not embed the verified installed kernel"
    if media:
        assert len(arguments) == 2 and not embedded
        installed = Path(arguments[0]).read_bytes()
        chunk = 512 * 1024 * 1024
        for part, offset in enumerate(range(0, len(installed), chunk)):
            actual = subprocess.check_output(["mtype", "-i", media,
                f"::/EFI/INFINITY/PAYLOAD/P1-{part:03}.BIN"])
            assert actual == installed[offset:offset + chunk], (media, part)
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
