"""Stage bounded WOFF2 reconstruction without treating source padding as an allocation limit."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tomllib

# ------------------------=
# FUNC: main
# DESC: Keeps OTS validation and size ceilings while allowing decoder-specific SFNT alignment padding.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root=Path(__file__).resolve().parents[2]
    spec=importlib.util.spec_from_file_location("staging",Path(__file__).with_name("prepare-async-net.py"))
    helper=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    servo=root / "build/servo-port-audit"
    packages=tomllib.loads(subprocess.check_output(["git","-C",str(servo),"show","HEAD:Cargo.lock"],text=True))["package"]
    directory,package=helper.stage(root,packages,"fontsan","0.7.0")
    path=directory / "src/deps/ots/src/ots.cc"
    text=path.read_text()
    marker="  out.SetMaxSize(decompressed_size);"
    if text.count(marker)!=1:
        raise SystemExit("Unexpected OTS reconstruction path")
    text=text.replace(marker,"""  // Reconstructed glyph alignment can differ from the source SFNT layout.
  // The header is an initial size estimate, not the decoder's exact output size.
  // Preserve both the sanitizer and destination hard limits; validate the actual
  // reconstructed bytes through ProcessTTF/ProcessTTC below.
  out.SetMaxSize(std::min(static_cast<size_t>(OTS_MAX_DECOMPRESSED_FILE_SIZE), output->size()));""")
    path.write_text(text)
    lock=servo / "Cargo.lock"
    header='name = "'+package["name"]+'"\nversion = "'+package["version"]+'"\n'
    entry=header+'source = "'+package["source"]+'"\nchecksum = "'+package["checksum"]+'"\n'
    lock.write_text(lock.read_text().replace(entry,header))

if __name__ == "__main__":
    main()
