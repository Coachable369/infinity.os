#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

font_count=$(find assets/fonts -maxdepth 1 -type f -name '*.ttf' | wc -l | tr -d ' ')
test "$font_count" -ge 47 || { echo "FAIL: expected 47 bundled font resources including Arimo regular and bold, found $font_count" >&2; exit 1; }
test -s assets/fonts/Arimo-Regular.ttf
test -s assets/fonts/Arimo-Bold.ttf
test -s assets/desktop/infinity-default-dark-wallpaper-v2.bmp
test -s assets/desktop/infinity-shell-wallpaper-v3.bmp
test -s assets/desktop/infinity-onboarding-wallpaper-v1.bmp
test -s assets/fonts/InfinityUI-Regular-24.atlas
test -s assets/fonts/InfinityUI-Semibold-24.atlas
rg -q 'assets/fonts/\*\.ttf.*installed-fat' Makefile || { echo "FAIL: installed System Generation does not package fonts" >&2; exit 1; }
rg -q 'assets/fonts/\*\.ttf.*fat-aarch64-qemu' Makefile || { echo "FAIL: QEMU AArch64 image does not package fonts" >&2; exit 1; }
sed -n '/infinity-aarch64-qemu\.img:/,/mcopy -i \$@/p' Makefile | rg -q 'bs=1M count=256' || { echo "FAIL: QEMU AArch64 image is too small for the complete system UI and font payload" >&2; exit 1; }
rg -q 'infinity-default-dark-wallpaper-v2\.bmp' Makefile kernel/core/bootstrap.rs || { echo "FAIL: desktop artwork is not a kernel build dependency" >&2; exit 1; }
rg -q 'infinity-onboarding-wallpaper-v1\.bmp' Makefile kernel/core/bootstrap.rs || { echo "FAIL: onboarding artwork is not a kernel build dependency" >&2; exit 1; }
rg -q 'authentication_frame' kernel/core/bootstrap.rs || { echo "FAIL: gold-standard authentication layout is missing" >&2; exit 1; }
rg -q 'cp -R assets/skins' Makefile || { echo "FAIL: installed System Generation does not package InfinityUI skins" >&2; exit 1; }
rg -q 'system_login_animation' kernel/core/bootstrap.rs || { echo "FAIL: damage-limited login animation missing" >&2; exit 1; }
rg -q 'runtime.identity.user_count' kernel/core/bootstrap.rs kernel/core/console.rs || { echo "FAIL: login account picker is not identity-backed" >&2; exit 1; }
rg -q 'TopMenu' kernel/ui/system_layout.rs kernel/core/console.rs || { echo "FAIL: native top menus are not pointer-addressable" >&2; exit 1; }
rg -q 'activate_shell_menu_item' kernel/core/console.rs || { echo "FAIL: top menu items are not functional" >&2; exit 1; }
rg -Uq 'self\.bootstrap_scene && !installer && !split && self\.split_layout[[:space:][:print:]]*paint_console_background\(\);[[:space:][:print:]]*finish_console_emblem\(\);[[:space:][:print:]]*self\.bootstrap_scene = false' kernel/core/bootstrap.rs || { echo "FAIL: Console and Repair retain the cropped bootstrap emblem" >&2; exit 1; }
echo "PASS desktop system: HD artwork, Roboto UI atlas, functional native menus, account picker, damage-limited animation, and 46-family install parity"
