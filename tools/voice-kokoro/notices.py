"""Assemble upstream notices from the exact source trees used by the native build."""
from pathlib import Path
import hashlib
import json


# ------------------------=
# FUNC: assemble
# DESC: Retains dependency license bytes and their provenance; never substitutes a license label for full notices.
# ------------------=
def assemble(root, output):
    deps = root / "build/voice-kokoro/reference/_deps"
    sources = {
        "kokopop": root / "build/kokopop-port-audit/LICENSE",
        "ggml": deps / "ggml-src/LICENSE",
        "yyjson": deps / "yyjson-src/LICENSE",
        "espeak-ng": deps / "espeak-src/COPYING",
        "espeak-ng-apache": deps / "espeak-src/COPYING.APACHE",
        "espeak-ng-bsd": deps / "espeak-src/COPYING.BSD2",
        "espeak-ng-unicode": deps / "espeak-src/COPYING.UCD",
        "ucd-tools": deps / "espeak-src/src/ucd-tools/COPYING",
        "ucd-tools-unicode": deps / "espeak-src/src/ucd-tools/COPYING.UCD",
        "llvm": root / "build/voice-kokoro/llvm-src/LICENSE.TXT",
        "libcxx": root / "build/voice-kokoro/llvm-src/libcxx/LICENSE.TXT",
        "libcxxabi": root / "build/voice-kokoro/llvm-src/libcxxabi/LICENSE.TXT",
        "newlib": root / "build/newlib-4.6.0.20260123/COPYING.NEWLIB",
    }
    document = bytearray(b"InfinityOS native Kokoro dependency notices\n"
        b"Native port modifications: InfinityOS, September 2026.\n"
        b"Full corresponding-source distribution remains a separate release requirement.\n\n")
    inventory = []
    for component, path in sources.items():
        content = path.read_bytes()
        if not content:
            raise ValueError(f"Empty required license: {path}")
        document.extend(("\n===== " + component + " =====\n").encode())
        document.extend(content)
        document.extend(b"\n")
        inventory.append(dict(component=component, source=str(path.relative_to(root)),
                              sha256=hashlib.sha256(content).hexdigest(), bytes=len(content)))
    output.mkdir(parents=True, exist_ok=True)
    notice = output / "THIRD-PARTY-NOTICES.txt"
    notice.write_bytes(document)
    (output / "notice-inventory.json").write_text(json.dumps(inventory, indent=2) + "\n")
    return notice
