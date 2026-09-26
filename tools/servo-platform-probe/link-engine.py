"""Expose unresolved native engine dependencies; this is not a runnable browser."""
import os
from pathlib import Path
import subprocess
import json

# ------------------------=
# FUNC: main
# DESC: Links actual Servo initialization without supplying fake native service definitions.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    target = root / "build/cargo/aarch64-unknown-none-softfloat/debug"
    output = root / "build/servo-platform-probe"
    flags = ["--cfg", "infinity_native", "--check-cfg=cfg(infinity_native)",
             "--check-cfg=cfg(infinity_certificate_test)"]
    native_externs = []
    for name in ("core", "panic_abort"):
        archives = []
        for fingerprint in (target / ".fingerprint").glob(name + "-*/lib-" + name + ".json"):
            archive = target / "deps" / ("lib" + fingerprint.parent.name + ".rlib")
            if json.loads(fingerprint.read_text())["rustflags"] == flags and archive.exists():
                archives.append(archive)
        if len(archives) != 1:
            raise SystemExit("Expected one native code-generated " + name + " archive")
        native_externs += ["--extern", name + "=" + str(archives[0])]
    command = ["rustc", "--edition=2021", "--target", "aarch64-unknown-none-softfloat",
               "--cfg", "infinity_native", "-C", "panic=abort",
               "-C", "linker=/opt/homebrew/opt/lld/bin/ld.lld",
               "-C", "link-arg=--entry=infinity_browser_link_probe",
               "-C", "link-arg=--error-limit=0",
               "--extern", "servo=" + str(target / "libservo.rlib"),
               *native_externs,
               "-L", "dependency=" + str(target / "deps"),
               "-L", "dependency=" + str(root / "build/cargo/debug/deps"),
               str(Path(__file__).with_name("engine-link.rs")),
               "-o", str(output / "engine-link-only.elf")]
    with (output / "engine-link.log").open("w") as log:
        result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
    report = {"link_exit_status": result.returncode, "executed": False,
              "purpose": "link diagnostics only; native providers are not initialized"}
    (output / "engine-link.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))
    return result.returncode

if __name__ == "__main__":
    raise SystemExit(main())
