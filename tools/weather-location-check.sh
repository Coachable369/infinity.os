#!/bin/sh
set -eu
. tools/require-build-kit.sh
out=builds/validation/weather-location
mkdir -p "$out" build
rustc --edition=2021 --test kernel/core/weather_place.rs -o "$out/place-test"
"$out/place-test"
rustc --edition=2021 --crate-name infinity_browser_core --crate-type rlib sdk/infinity-browser-core/lib.rs -o "$out/libinfinity_browser_core.rlib"
rustc --edition=2021 --test -Awarnings --extern infinity_browser_core="$out/libinfinity_browser_core.rlib" tools/active-painter-test.rs -o "$out/painter"
WEATHER_SETUP_PPM="$out/setup.ppm" "$out/painter" forecast_and_meter_pixels
WEATHER_REVIEW_PPM="$out/widgets.ppm" "$out/painter" weather_outlook_and_chooser_render
rustc --edition=2021 --test tools/desktop-widgets-test.rs -o "$out/layout-test"
"$out/layout-test" --test-threads=1
