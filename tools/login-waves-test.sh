#!/bin/sh
set -eu
. tools/require-build-kit.sh
out=builds/.studio-work/login-waves
mkdir -p "$out"
rustc --edition=2021 --test kernel/ui/login_waves.rs -o "$out/envelope-test"
"$out/envelope-test"
rustc --edition=2021 --test -O -Awarnings tools/audio-poll-test.rs -o "$out/audio-test"
"$out/audio-test" --test-threads=1
rustc --edition=2021 --test -O kernel/ui/authentication_motion.rs -o "$out/authentication-test"
"$out/authentication-test"
rustc --edition=2021 --test -O -Awarnings tools/voice-toggle-test.rs -o "$out/voice-test"
"$out/voice-test" --test-threads=1
rustc --edition=2021 --crate-name infinity_browser_core --crate-type rlib -C opt-level=z sdk/infinity-browser-core/lib.rs -o "$out/libinfinity_browser_core.rlib"
rustc --edition=2021 -O -Awarnings --extern infinity_browser_core="$out/libinfinity_browser_core.rlib" tools/active-painter-test.rs -o "$out/painter-test"
"$out/painter-test" --login-waves-only
for source in builds/validation/login-waves/*.ppm; do
    sips -s format png "$source" --out "${source%.ppm}.png" >/dev/null
done
