#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

font_count=$(find assets/fonts -maxdepth 1 -type f -name '*.ttf' | wc -l | tr -d ' ')
test "$font_count" -ge 47 || { echo "FAIL: live and installed UI payload must contain Roboto plus the existing type library" >&2; exit 1; }
test -s assets/fonts/Roboto-Variable.ttf
test -s assets/fonts/Arimo-Regular.ttf
test -s assets/fonts/Arimo-Bold.ttf
test -s assets/desktop/infinity-default-dark-wallpaper-v2.png
test -s assets/desktop/infinity-shell-wallpaper-v3.png
test -s assets/desktop/infinity-onboarding-wallpaper-v1.png
test -s assets/fonts/InfinityUI-Regular-24.atlas
test -s assets/fonts/InfinityUI-Semibold-24.atlas
test -s assets/fonts/InfinityUI-Regular-24.metrics
test -s assets/fonts/InfinityUI-Semibold-24.metrics
test -s assets/fonts/InfinityUI-Regular-24.kern
test -s assets/fonts/InfinityUI-Semibold-24.kern
test -s assets/fonts/InfinityInstaller-Regular-24.atlas
test -s assets/fonts/InfinityInstaller-Semibold-24.atlas
test -s assets/fonts/InfinityInstaller-Regular-24.metrics
test -s assets/fonts/InfinityInstaller-Semibold-24.metrics
test -s assets/fonts/InfinityInstaller-Regular-24.kern
test -s assets/fonts/InfinityInstaller-Semibold-24.kern
test -s assets/fonts/InfinityInstaller-Semibold-32.atlas
test -s assets/fonts/InfinityInstaller-Semibold-32.metrics
test -s assets/fonts/InfinityInstaller-Semibold-32.kern
test -s assets/boot/infinity-disk-discovery-vision-v1.bmp
test -s assets/boot/infinity-storage-device-v1.bmp
test -s assets/boot/infinity-date-time-world-v1.bmp
test -s assets/fonts/InfinityInstaller-Regular-19.atlas
test -s assets/fonts/InfinityInstaller-Semibold-19.atlas
test -s assets/fonts/InfinityInstaller-Regular-19.metrics
test -s assets/fonts/InfinityInstaller-Semibold-19.metrics
test -s assets/fonts/InfinityInstaller-Regular-19.kern
test -s assets/fonts/InfinityInstaller-Semibold-19.kern
for atlas in assets/fonts/InfinityUI-Regular-24.atlas assets/fonts/InfinityUI-Semibold-24.atlas assets/fonts/InfinityInstaller-Regular-24.atlas assets/fonts/InfinityInstaller-Semibold-24.atlas; do
    test "$(wc -c < "$atlas" | tr -d ' ')" -eq 63840 || { echo "FAIL: invalid Roboto atlas geometry: $atlas" >&2; exit 1; }
done
for metrics in assets/fonts/InfinityUI-Regular-24.metrics assets/fonts/InfinityUI-Semibold-24.metrics assets/fonts/InfinityInstaller-Regular-24.metrics assets/fonts/InfinityInstaller-Semibold-24.metrics; do
    test "$(wc -c < "$metrics" | tr -d ' ')" -eq 95 || { echo "FAIL: invalid proportional metrics table: $metrics" >&2; exit 1; }
done
for kerning in assets/fonts/InfinityUI-Regular-24.kern assets/fonts/InfinityUI-Semibold-24.kern assets/fonts/InfinityInstaller-Regular-24.kern assets/fonts/InfinityInstaller-Semibold-24.kern; do
    test "$(wc -c < "$kerning" | tr -d ' ')" -eq 9025 || { echo "FAIL: invalid pair-kerning table: $kerning" >&2; exit 1; }
done
rg -q 'font_pair_adjustment' kernel/core/bootstrap.rs
rg -q 'InfinityOS is a distributed operating system\.' kernel/core/bootstrap.rs
rg -q 'You.re in control\. Always\.' kernel/core/bootstrap.rs
rg -q 'See how InfinityPool works' kernel/core/bootstrap.rs
rg -q 'FUNC: clock_tick' kernel/core/console.rs
rg -q 'BOOT_PARTICLE_Y_OFFSET: i32 = 50' kernel/core/bootstrap.rs
rg -q 'HomeControl' kernel/core/console.rs kernel/ui/system_layout.rs
rg -q 'SettingsTarget::WindowControl' kernel/core/console.rs
rg -q 'HomeSidebar' kernel/core/console.rs kernel/ui/system_layout.rs
rg -q 'notes.txt moved by drag and drop' kernel/core/console.rs
rg -q 'last_system_clock' kernel/core/bootstrap.rs
test "$(grep -c '<symbol id=' assets/skins/infinity.default.dark/icons/semantic-icons.svg)" -ge 50

for tree in installed-fat installed-fat-aarch64 fat fat-aarch64 fat-aarch64-qemu; do
    rg -q "assets/fonts/\\*\\.ttf.*$tree" Makefile || { echo "FAIL: fonts missing from $tree recipe" >&2; exit 1; }
    rg -q "assets/desktop/\\*\\.png.*$tree.*/Wallpapers" Makefile || { echo "FAIL: wallpapers missing from $tree recipe" >&2; exit 1; }
    rg -q "assets/skins/\\..*$tree" Makefile || { echo "FAIL: skins missing from $tree recipe" >&2; exit 1; }
done

rg -q 'assets/desktop/\*\.png.*iso-x86/System/InfinityUI/Wallpapers' Makefile
echo "PASS UI install parity: fresh System Generations include live window controls, menus, drag/drop, clock updates, skins, fonts, and wallpapers"
