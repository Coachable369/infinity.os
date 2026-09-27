"""Build the existing native HTTP service with the engine's freestanding ABI."""
import json
import os
from pathlib import Path
import subprocess


# ------------------------=
# FUNC: main
# DESC: Builds native transport against the same core/std metadata as the pinned engine.
# ------------------=
def main():
    if os.environ.get("INFINITY_BUILD_KIT_ACTIVE") != "1":
        raise SystemExit("Run through build-kit")
    root = Path(__file__).resolve().parents[2]
    environment = dict(os.environ, RUSTC_BOOTSTRAP="1",
        CARGO_HOME=str(root / "build/servo-cargo-home"),
        __CARGO_TESTS_ONLY_SRC_ROOT=str(root / "build/servo-rust-src/library"),
        RUSTFLAGS="--cfg infinity_native --check-cfg=cfg(infinity_native) --check-cfg=cfg(infinity_certificate_test)")
    result = subprocess.run(["cargo", "build", "--locked", "-j", "4", "-Z", "build-std=std,panic_abort",
        "--target", "aarch64-unknown-none", "--manifest-path",
        str(Path(__file__).with_name("network") / "Cargo.toml"), "--message-format=json-render-diagnostics"],
        env=environment, cwd=root, stdout=subprocess.PIPE, text=True)
    artifacts = []
    for line in result.stdout.splitlines():
        event = json.loads(line)
        if event.get("reason") == "compiler-artifact" and event["target"]["name"] == "infinity_browser_native_network":
            artifacts += [name for name in event["filenames"] if name.endswith(".rlib")]
    if result.returncode or len(artifacts) != 1:
        raise SystemExit(result.returncode or 1)
    (root / "build/servo-platform-probe/network.json").write_text(json.dumps({"archive": artifacts[0]}) + "\n")


if __name__ == "__main__":
    main()
