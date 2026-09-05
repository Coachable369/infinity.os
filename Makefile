SHELL := /bin/sh

BUILD := build
LLVM := /opt/homebrew/opt/llvm/bin
LLD := /opt/homebrew/opt/lld/bin
CLANG := $(LLVM)/clang
LD_LLD := $(LLD)/ld.lld
LLD_LINK := $(LLD)/lld-link
OBJCOPY := $(LLVM)/llvm-objcopy
RUSTC := rustc
CARGO := cargo
QEMU_X64 := qemu-system-x86_64
QEMU_AARCH64 := qemu-system-aarch64
OVMF_CODE := $(firstword $(wildcard /opt/homebrew/share/qemu/edk2-x86_64-code.fd /opt/homebrew/share/qemu/edk2-x86_64-code.fd))
AAVMF_CODE := $(firstword $(wildcard /opt/homebrew/share/qemu/edk2-aarch64-code.fd))
KERNEL_SOURCES := $(shell find kernel -type f)
FONT_ASSETS := $(wildcard assets/fonts/*.ttf) $(wildcard assets/fonts/OFL-*.txt)
ICON_ASSETS := $(shell find assets/icons -type f)
ICON_RUNTIME_ASSETS := assets/icons/runtime/crystal-blue-glass-base.bmp assets/icons/runtime/crystal-blue-glass-actions.bmp \
	assets/icons/runtime/luminous-obsidian-base.bmp assets/icons/runtime/luminous-obsidian-actions.bmp \
	assets/icons/runtime/frosted-quartz-base.bmp assets/icons/runtime/frosted-quartz-actions.bmp
WALLPAPER_ASSETS := assets/desktop/infinity-default-dark-wallpaper-v2.png \
	assets/desktop/infinity-shell-wallpaper-v3.png \
	assets/desktop/infinity-onboarding-wallpaper-v1.png
UI_ASSETS := $(shell find assets/skins -type f) $(WALLPAPER_ASSETS)
INSTALLER_UI_ASSETS := assets/boot/infinity-installer-mesh-diagram-v1.png
CRASH_ASSETS := $(shell find assets/crash -type f)
APPLICATION_ASSETS := $(shell find assets/apps -type f)
# The BIOS payload begins at 0x8000 and must retain 16 KiB below the 0x9c000 bootstrap stack.
X86_PAYLOAD_MAX_SECTORS := 1152
ICON_THEME_SOURCES := $(shell find assets/icons -maxdepth 2 -type f -name 'master-*.png') tools/build-icon-themes.sh tools/slice-icon-atlas.py
SPLASH_ASSET := assets/boot/infinity-eclipse-header-v1.bmp assets/boot/infinity-console-background-v1.bmp \
	assets/boot/infinity-emblem-v2.bmp \
	assets/desktop/infinity-default-dark-wallpaper-v2.bmp assets/desktop/infinity-shell-wallpaper-v3.bmp \
	assets/desktop/infinity-onboarding-wallpaper-v1.bmp assets/desktop/infinity-topbar-icon-v2.bmp \
	assets/fonts/InfinityUI-Regular-24.atlas \
	assets/fonts/InfinityUI-Semibold-24.atlas assets/fonts/InfinityUI-Regular-24.metrics \
	assets/fonts/InfinityUI-Semibold-24.metrics assets/fonts/InfinityUI-Regular-24.kern \
	assets/fonts/InfinityUI-Semibold-24.kern assets/fonts/InfinityInstaller-Regular-24.atlas \
	assets/fonts/InfinityInstaller-Semibold-24.atlas assets/fonts/InfinityInstaller-Regular-24.metrics \
	assets/fonts/InfinityInstaller-Semibold-24.metrics assets/fonts/InfinityInstaller-Regular-24.kern \
	assets/fonts/InfinityInstaller-Semibold-24.kern \
	assets/fonts/InfinityInstaller-Semibold-32.atlas assets/fonts/InfinityInstaller-Semibold-32.metrics \
	assets/fonts/InfinityInstaller-Semibold-32.kern \
	assets/fonts/InfinityInstaller-Regular-19.atlas assets/fonts/InfinityInstaller-Semibold-19.atlas \
	assets/fonts/InfinityInstaller-Regular-19.metrics assets/fonts/InfinityInstaller-Semibold-19.metrics \
	assets/fonts/InfinityInstaller-Regular-19.kern assets/fonts/InfinityInstaller-Semibold-19.kern \
	assets/boot/infinity-installer-background-v2.bmp assets/boot/infinity-cursor-v1.bmp \
	assets/boot/infinity-installer-masthead-v1.bmp \
	assets/boot/infinity-installer-masthead-v2.bmp \
	assets/boot/infinity-installer-mesh-hero-v1.bmp assets/boot/infinity-installer-mesh-overview-v1.bmp \
	assets/boot/infinity-installer-mesh-diagram-v1.bmp \
	assets/boot/infinity-installer-activation-v2.bmp \
	assets/boot/infinity-installer-progress-hero-v1.bmp \
	assets/boot/infinity-storage-hierarchy-v3.bmp \
	assets/boot/infinity-disk-discovery-vision-v1.bmp \
	assets/boot/infinity-storage-device-v1.bmp assets/boot/infinity-time-zone-map-v1.bmp \
	$(CRASH_ASSETS)

.PHONY: all x86_64 x86 aarch64 run-x86_64 run-x86 run-aarch64 test test-x86 test-x86_64 test-aarch64 test-console test-mouse-menu test-installer-safety installer-capacity-test component-manifest-test object-test namespace-test crash-recovery-test crash-screen-test object-vm-test milestone-3b-test runtime-test runtime-vm-test iop-test event-test capability-test service-crash-test milestone-4-test ai-test milestone-6-test milestone-6-5-test milestone-7-test milestone-7x-test milestone-7c-test milestone-8-test network-test icon-theme-test settings-color-test desktop-system-test ui-install-parity-test input-regression-test installed-object-test vm-disk reset-test-disk install-test install-boot-test installed-console-test system-generation-test boot-installed clean check-tools

crash-screen-test:
	@tools/crash-screen-test.sh

milestone-7x-test:
	@tools/skin-compiler-test.sh
	@tools/infinity-ui-test.sh

milestone-7c-test:
	@tools/milestone-7c-test.sh

icon-theme-test:
	@tools/icon-theme-test.sh

settings-color-test:
	@mkdir -p build/tools
	@rustc --edition 2021 -A warnings tools/settings-color-test.rs -o build/tools/settings-color-test
	@build/tools/settings-color-test
	@echo "Independent Primary and Secondary theme controls: PASS"

all: x86_64

check-tools:
	@tools="$(CLANG) $(LD_LLD) $(LLD_LINK) $(OBJCOPY) $(RUSTC) $(CARGO) python3 ffmpeg mformat mcopy xorriso $(QEMU_X64) $(QEMU_AARCH64)"; \
	for tool in $$tools; do command -v $$tool >/dev/null 2>&1 || { echo "ERROR: required tool not found: $$tool"; exit 1; }; done
	@test -f "$(OVMF_CODE)" || { echo "ERROR: OVMF firmware not found (install qemu)"; exit 1; }
	@test -f "$(AAVMF_CODE)" || { echo "ERROR: AArch64 UEFI firmware not found (install qemu)"; exit 1; }

$(ICON_RUNTIME_ASSETS): $(ICON_THEME_SOURCES)
	@tools/build-icon-themes.sh

$(BUILD)/x86_64/installed-kernel.o: $(KERNEL_SOURCES) $(SPLASH_ASSET) $(ICON_RUNTIME_ASSETS)
	@mkdir -p $(@D)
	RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=$(BUILD)/cargo-installed $(CARGO) build --release \
		-Z build-std=core --target x86_64-unknown-none
	cp $(BUILD)/cargo-installed/x86_64-unknown-none/release/libinfinity_kernel.a $(BUILD)/x86_64/libinstalled-kernel.a
	touch $@

$(BUILD)/x86_64/installed-kernel.elf: $(BUILD)/x86_64/installed-kernel.o linker/x86_64.ld
	$(LD_LLD) -nostdlib -static -T linker/x86_64.ld -o $@ $(BUILD)/x86_64/libinstalled-kernel.a

$(BUILD)/x86_64/kernel.o: $(KERNEL_SOURCES) $(SPLASH_ASSET) $(BUILD)/x86_64/installed-esp.img $(BUILD)/x86_64/installed-kernel.elf
	@mkdir -p $(@D)
	RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=$(BUILD)/cargo $(CARGO) build --release \
		-Z build-std=core --target x86_64-unknown-none --features installer
	cp $(BUILD)/cargo/x86_64-unknown-none/release/libinfinity_kernel.a $(BUILD)/x86_64/libkernel.a
	touch $@

$(BUILD)/x86_64/kernel.elf: $(BUILD)/x86_64/kernel.o linker/x86_64.ld
	$(LD_LLD) -nostdlib -static -T linker/x86_64.ld -o $@ $(BUILD)/x86_64/libkernel.a

$(BUILD)/x86_64/loader.obj: boot/common/uefi_loader.c boot/common/boot_info.h
	@mkdir -p $(@D)
	$(CLANG) --target=x86_64-pc-windows-msvc -ffreestanding -fshort-wchar -fno-stack-protector \
		-mno-red-zone -O2 -Wall -Wextra -Werror -c $< -o $@

$(BUILD)/x86_64/handoff.obj: boot/x86_64/handoff.asm
	@mkdir -p $(@D)
	nasm -f win64 $< -o $@

$(BUILD)/x86_64/BOOTX64.EFI: $(BUILD)/x86_64/loader.obj $(BUILD)/x86_64/handoff.obj
	$(LLD_LINK) /subsystem:efi_application /entry:efi_main /nodefaultlib /machine:x64 /out:$@ $^

$(BUILD)/x86_64/installed-esp.img: $(BUILD)/x86_64/BOOTX64.EFI $(FONT_ASSETS) $(UI_ASSETS) $(ICON_ASSETS) $(INSTALLER_UI_ASSETS) $(CRASH_ASSETS) $(APPLICATION_ASSETS)
	rm -rf $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Icons $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Wallpapers $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Crash
	@mkdir -p $(BUILD)/installed-fat/EFI/BOOT $(BUILD)/installed-fat/EFI/InfinityOS/Fonts $(BUILD)/installed-fat/EFI/InfinityOS/FontLicenses $(BUILD)/installed-fat/EFI/InfinityOS/Applications $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Wallpapers $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Installer $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Icons $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Crash
	cp $(BUILD)/x86_64/BOOTX64.EFI $(BUILD)/installed-fat/EFI/BOOT/BOOTX64.EFI
	cp $(BUILD)/x86_64/BOOTX64.EFI $(BUILD)/installed-fat/EFI/InfinityOS/infinity.efi
	cp assets/fonts/*.ttf $(BUILD)/installed-fat/EFI/InfinityOS/Fonts/
	cp assets/fonts/OFL-*.txt $(BUILD)/installed-fat/EFI/InfinityOS/FontLicenses/
	cp -R assets/skins/. $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/
	cp $(WALLPAPER_ASSETS) $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Wallpapers/
	cp -R assets/icons/. $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Icons/
	cp $(INSTALLER_UI_ASSETS) $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Installer/
	cp $(CRASH_ASSETS) $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Crash/
	cp $(APPLICATION_ASSETS) $(BUILD)/installed-fat/EFI/InfinityOS/Applications/
	dd if=/dev/zero of=$@ bs=1M count=128 status=none
	mformat -i $@ -v INFINITYEFI ::
	mcopy -i $@ -s $(BUILD)/installed-fat/EFI ::

$(BUILD)/infinity-x86_64.img: $(BUILD)/x86_64/BOOTX64.EFI $(BUILD)/x86_64/kernel.elf $(FONT_ASSETS) $(UI_ASSETS) $(INSTALLER_UI_ASSETS) $(CRASH_ASSETS) $(APPLICATION_ASSETS)
	rm -rf $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Icons $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Wallpapers $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Crash
	@mkdir -p $(BUILD)/fat/EFI/BOOT $(BUILD)/fat/EFI/INFINITY/FONTS $(BUILD)/fat/EFI/INFINITY/FONT-LICENSES $(BUILD)/fat/EFI/INFINITY/APPLICATIONS $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Wallpapers $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Installer $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Crash
	cp $(BUILD)/x86_64/BOOTX64.EFI $(BUILD)/fat/EFI/BOOT/BOOTX64.EFI
	cp $(BUILD)/x86_64/kernel.elf $(BUILD)/fat/EFI/INFINITY/KERNEL.ELF
	cp assets/fonts/*.ttf $(BUILD)/fat/EFI/INFINITY/FONTS/
	cp assets/fonts/OFL-*.txt $(BUILD)/fat/EFI/INFINITY/FONT-LICENSES/
	cp -R assets/skins/. $(BUILD)/fat/EFI/INFINITY/INFINITYUI/
	cp $(WALLPAPER_ASSETS) $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Wallpapers/
	cp $(INSTALLER_UI_ASSETS) $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Installer/
	cp $(CRASH_ASSETS) $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Crash/
	cp $(APPLICATION_ASSETS) $(BUILD)/fat/EFI/INFINITY/APPLICATIONS/
	dd if=/dev/zero of=$@ bs=1M count=320 status=none
	mformat -i $@ ::
	mcopy -i $@ -s $(BUILD)/fat/EFI ::

$(BUILD)/infinity-x86_64.iso: $(BUILD)/infinity-x86_64.img
	@mkdir -p $(BUILD)/iso/EFI
	cp $< $(BUILD)/iso/efi.img
	cp -R $(BUILD)/fat/EFI/BOOT $(BUILD)/fat/EFI/INFINITY $(BUILD)/iso/EFI/
	xorriso -as mkisofs -R -V INFINITYOS -e efi.img -no-emul-boot -o $@ $(BUILD)/iso

x86_64: check-tools $(BUILD)/infinity-x86_64.iso
	@echo "Built VMware/QEMU boot image: $(BUILD)/infinity-x86_64.iso"

run-x86_64: x86_64
	$(QEMU_X64) -machine q35 -m 512M -drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
		-cdrom $(BUILD)/infinity-x86_64.iso -serial stdio -display none -no-reboot

test-x86_64: x86_64
	@tools/smoke-test.sh x86_64

test-aarch64: aarch64
	@tools/smoke-test.sh aarch64

test-console: x86 x86_64 aarch64
	@tools/console-test.sh x86
	@tools/console-test.sh x86_64
	@tools/console-test.sh aarch64

test-installer-safety:
	@tools/installer-safety-test.sh

installer-capacity-test:
	@tools/installer-capacity-test.sh

component-manifest-test:
	@mkdir -p build/behavior-tests
	@rustc --edition=2021 -C opt-level=2 -A warnings tools/component-manifest-test.rs -o build/behavior-tests/component-manifest-test
	@build/behavior-tests/component-manifest-test

object-test namespace-test crash-recovery-test:
	@tools/object-store-test.sh

object-vm-test:
	@tools/object-vm-test.sh

milestone-3b-test: object-test install-test object-vm-test

runtime-test iop-test event-test capability-test service-crash-test:
	@tools/runtime-test.sh

network-test:
	@tools/network-test.sh

milestone-8-test: network-test runtime-test
	@tools/milestone-8-object-navigation-test.sh
	@tools/milestone-8-install-parity-test.sh

ai-test:
	@tools/ai-test.sh

milestone-6-test: ai-test object-test runtime-test

milestone-6-5-test:
	@tools/milestone-6-5-test.sh

milestone-7-test:
	@tools/milestone-7-test.sh

desktop-system-test: milestone-7-test
	@tools/desktop-system-test.sh

ui-install-parity-test: x86 x86_64 aarch64
	@tools/ui-install-parity-test.sh

input-regression-test:
	@tools/input-regression-test.sh

installed-object-test:
	@rustc --edition=2021 -C opt-level=2 tools/installed-object-test.rs -o build/installed-object-test
	@build/installed-object-test build/infinity-test-disk.raw

runtime-vm-test:
	@tools/runtime-vm-test.sh

milestone-4-test: runtime-test install-test runtime-vm-test object-vm-test

test-mouse-menu: x86_64 aarch64
	@tools/mouse-menu-test.sh x86_64
	@tools/mouse-menu-test.sh aarch64

test: test-x86 test-x86_64 test-aarch64 test-console test-mouse-menu test-installer-safety

vm-disk:
	@test -s $(BUILD)/infinity-test-disk.raw || dd if=/dev/zero of=$(BUILD)/infinity-test-disk.raw bs=1M count=512 status=none
	@echo "Disposable InfinityOS test disk: $(BUILD)/infinity-test-disk.raw"

reset-test-disk:
	@mkdir -p $(BUILD)
	dd if=/dev/zero of=$(BUILD)/infinity-test-disk.raw bs=1M count=512 status=none
	@echo "Reset only known test artifact: $(BUILD)/infinity-test-disk.raw"

install-test: x86_64
	@tools/install-test.sh

install-boot-test: install-test
	@tools/installed-console-test.sh
	@tools/system-generation-test.sh

installed-console-test:
	@tools/installed-console-test.sh

system-generation-test:
	@tools/system-generation-test.sh

boot-installed:
	@tools/install-test.sh --boot-only

$(BUILD)/x86/kernel.stamp: $(KERNEL_SOURCES) targets/i686-infinity.json
	@mkdir -p $(@D)
	RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=$(BUILD)/cargo $(CARGO) build --release \
		-Z build-std=core -Z json-target-spec --target targets/i686-infinity.json
	cp $(BUILD)/cargo/i686-infinity/release/libinfinity_kernel.a $(BUILD)/x86/libkernel.a
	touch $@

$(BUILD)/x86/kernel.elf: $(BUILD)/x86/kernel.stamp linker/x86.ld
	$(LD_LLD) -m elf_i386 -nostdlib -static -T linker/x86.ld -o $@ $(BUILD)/x86/libkernel.a

$(BUILD)/x86/bootstrap.bin: boot/x86/bootstrap.asm $(BUILD)/x86/kernel.elf
	@mkdir -p $(@D)
	nasm -f bin $< -o $@

$(BUILD)/x86/boot-sector.bin: boot/x86/boot_sector.asm $(BUILD)/x86/bootstrap.bin
	@mkdir -p $(@D)
	@bytes=$$(wc -c < $(BUILD)/x86/bootstrap.bin); sectors=$$((($$bytes + 511) / 512)); \
	if test $$sectors -gt $(X86_PAYLOAD_MAX_SECTORS); then echo "ERROR: x86 bootstrap exceeds bounded region below the kernel stack"; exit 1; fi; \
	nasm -f bin -DPAYLOAD_SECTORS=$$sectors $< -o $@

$(BUILD)/infinity-x86.img: $(BUILD)/x86/boot-sector.bin $(BUILD)/x86/bootstrap.bin
	dd if=/dev/zero of=$@ bs=1440k count=1 status=none
	dd if=$(BUILD)/x86/boot-sector.bin of=$@ conv=notrunc status=none
	dd if=$(BUILD)/x86/bootstrap.bin of=$@ bs=512 seek=1 conv=notrunc status=none

$(BUILD)/infinity-x86.iso: $(BUILD)/infinity-x86.img $(UI_ASSETS) $(INSTALLER_UI_ASSETS) $(CRASH_ASSETS) $(APPLICATION_ASSETS)
	rm -rf $(BUILD)/iso-x86/System/InfinityUI/Icons $(BUILD)/iso-x86/System/InfinityUI/Wallpapers $(BUILD)/iso-x86/System/InfinityUI/Crash
	@mkdir -p $(BUILD)/iso-x86/System/Fonts $(BUILD)/iso-x86/System/FontLicenses $(BUILD)/iso-x86/System/Applications $(BUILD)/iso-x86/System/InfinityUI/Wallpapers $(BUILD)/iso-x86/System/InfinityUI/Installer $(BUILD)/iso-x86/System/InfinityUI/Crash
	cp $< $(BUILD)/iso-x86/x86-boot.img
	cp assets/fonts/*.ttf $(BUILD)/iso-x86/System/Fonts/
	cp assets/fonts/OFL-*.txt $(BUILD)/iso-x86/System/FontLicenses/
	cp -R assets/skins/. $(BUILD)/iso-x86/System/InfinityUI/
	cp $(WALLPAPER_ASSETS) $(BUILD)/iso-x86/System/InfinityUI/Wallpapers/
	cp $(INSTALLER_UI_ASSETS) $(BUILD)/iso-x86/System/InfinityUI/Installer/
	cp $(CRASH_ASSETS) $(BUILD)/iso-x86/System/InfinityUI/Crash/
	cp $(APPLICATION_ASSETS) $(BUILD)/iso-x86/System/Applications/
	xorriso -as mkisofs -R -V INFINITYOS_X86 -b x86-boot.img -c boot.cat -o $@ $(BUILD)/iso-x86

x86: check-tools $(BUILD)/infinity-x86.iso
	@echo "Built BIOS x86 boot image: $(BUILD)/infinity-x86.iso"

test-x86: x86
	@tools/smoke-test.sh x86

$(BUILD)/aarch64/installed-kernel.stamp: $(KERNEL_SOURCES) $(SPLASH_ASSET) $(ICON_RUNTIME_ASSETS)
	@mkdir -p $(@D)
	RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=$(BUILD)/cargo-installed-aarch64 $(CARGO) build --release \
		-Z build-std=core --target aarch64-unknown-none-softfloat
	cp $(BUILD)/cargo-installed-aarch64/aarch64-unknown-none-softfloat/release/libinfinity_kernel.a $(BUILD)/aarch64/libinstalled-kernel.a
	touch $@

$(BUILD)/aarch64/installed-kernel.elf: $(BUILD)/aarch64/installed-kernel.stamp linker/aarch64.ld
	$(LD_LLD) -nostdlib -static -T linker/aarch64.ld -o $@ $(BUILD)/aarch64/libinstalled-kernel.a

$(BUILD)/aarch64/installed-esp.img: $(BUILD)/aarch64/BOOTAA64.EFI $(FONT_ASSETS) $(UI_ASSETS) $(ICON_ASSETS) $(INSTALLER_UI_ASSETS) $(CRASH_ASSETS) $(APPLICATION_ASSETS)
	rm -rf $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Icons $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Wallpapers $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Crash
	@mkdir -p $(BUILD)/installed-fat-aarch64/EFI/BOOT $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/Fonts $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/FontLicenses $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/Applications $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Wallpapers $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Installer $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Icons $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Crash
	cp $(BUILD)/aarch64/BOOTAA64.EFI $(BUILD)/installed-fat-aarch64/EFI/BOOT/BOOTAA64.EFI
	cp $(BUILD)/aarch64/BOOTAA64.EFI $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/infinity.efi
	cp assets/fonts/*.ttf $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/Fonts/
	cp assets/fonts/OFL-*.txt $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/FontLicenses/
	cp -R assets/skins/. $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/
	cp $(WALLPAPER_ASSETS) $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Wallpapers/
	cp -R assets/icons/. $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Icons/
	cp $(INSTALLER_UI_ASSETS) $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Installer/
	cp $(CRASH_ASSETS) $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Crash/
	cp $(APPLICATION_ASSETS) $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/Applications/
	dd if=/dev/zero of=$@ bs=1M count=128 status=none
	mformat -i $@ -v INFINITYEFI ::
	mcopy -i $@ -s $(BUILD)/installed-fat-aarch64/EFI ::

$(BUILD)/aarch64/kernel.stamp: $(KERNEL_SOURCES) $(SPLASH_ASSET) $(BUILD)/aarch64/installed-esp.img $(BUILD)/aarch64/installed-kernel.elf
	@mkdir -p $(@D)
	RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=$(BUILD)/cargo $(CARGO) build --release \
		-Z build-std=core --target aarch64-unknown-none-softfloat --features installer
	cp $(BUILD)/cargo/aarch64-unknown-none-softfloat/release/libinfinity_kernel.a $(BUILD)/aarch64/libkernel.a
	touch $@

$(BUILD)/aarch64/kernel.elf: $(BUILD)/aarch64/kernel.stamp linker/aarch64.ld
	$(LD_LLD) -nostdlib -static -T linker/aarch64.ld -o $@ $(BUILD)/aarch64/libkernel.a

$(BUILD)/aarch64/kernel-qemu.elf: $(BUILD)/aarch64/kernel.stamp linker/aarch64-qemu.ld
	$(LD_LLD) -nostdlib -static -T linker/aarch64-qemu.ld -o $@ $(BUILD)/aarch64/libkernel.a

$(BUILD)/aarch64/loader.obj: boot/common/uefi_loader.c boot/common/boot_info.h
	@mkdir -p $(@D)
	$(CLANG) --target=aarch64-pc-windows-msvc -DINFINITY_AARCH64 -ffreestanding -fshort-wchar \
		-fno-stack-protector -fno-builtin -O2 -Wall -Wextra -Werror -c $< -o $@

$(BUILD)/aarch64/handoff.obj: boot/aarch64/handoff.S
	@mkdir -p $(@D)
	$(CLANG) --target=aarch64-pc-windows-msvc -c $< -o $@

$(BUILD)/aarch64/BOOTAA64.EFI: $(BUILD)/aarch64/loader.obj $(BUILD)/aarch64/handoff.obj
	$(LLD_LINK) /subsystem:efi_application /entry:efi_main /nodefaultlib /machine:arm64 /out:$@ $^

$(BUILD)/infinity-aarch64.img: $(BUILD)/aarch64/BOOTAA64.EFI $(BUILD)/aarch64/kernel.elf $(FONT_ASSETS) $(UI_ASSETS) $(INSTALLER_UI_ASSETS) $(CRASH_ASSETS)
	rm -rf $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Icons $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Wallpapers $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Crash
	@mkdir -p $(BUILD)/fat-aarch64/EFI/BOOT $(BUILD)/fat-aarch64/EFI/INFINITY/FONTS $(BUILD)/fat-aarch64/EFI/INFINITY/FONT-LICENSES $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Wallpapers $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Installer $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Crash
	cp $(BUILD)/aarch64/BOOTAA64.EFI $(BUILD)/fat-aarch64/EFI/BOOT/BOOTAA64.EFI
	cp $(BUILD)/aarch64/kernel.elf $(BUILD)/fat-aarch64/EFI/INFINITY/KERNEL.ELF
	cp assets/fonts/*.ttf $(BUILD)/fat-aarch64/EFI/INFINITY/FONTS/
	cp assets/fonts/OFL-*.txt $(BUILD)/fat-aarch64/EFI/INFINITY/FONT-LICENSES/
	cp -R assets/skins/. $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/
	cp $(WALLPAPER_ASSETS) $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Wallpapers/
	cp $(INSTALLER_UI_ASSETS) $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Installer/
	cp $(CRASH_ASSETS) $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Crash/
	dd if=/dev/zero of=$@ bs=1M count=320 status=none
	mformat -i $@ ::
	mcopy -i $@ -s $(BUILD)/fat-aarch64/EFI ::

$(BUILD)/infinity-aarch64.iso: $(BUILD)/infinity-aarch64.img
	@mkdir -p $(BUILD)/iso-aarch64/EFI
	cp $< $(BUILD)/iso-aarch64/efi.img
	cp -R $(BUILD)/fat-aarch64/EFI/BOOT $(BUILD)/fat-aarch64/EFI/INFINITY $(BUILD)/iso-aarch64/EFI/
	xorriso -as mkisofs -R -V INFINITYOS_ARM64 -e efi.img -no-emul-boot -o $@ $(BUILD)/iso-aarch64

$(BUILD)/infinity-aarch64-qemu.img: $(BUILD)/aarch64/BOOTAA64.EFI $(BUILD)/aarch64/kernel-qemu.elf $(FONT_ASSETS) $(UI_ASSETS) $(INSTALLER_UI_ASSETS) $(CRASH_ASSETS)
	rm -rf $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Icons $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Wallpapers $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Crash
	@mkdir -p $(BUILD)/fat-aarch64-qemu/EFI/BOOT $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/FONTS $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/FONT-LICENSES $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Wallpapers $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Installer $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Crash
	cp $(BUILD)/aarch64/BOOTAA64.EFI $(BUILD)/fat-aarch64-qemu/EFI/BOOT/BOOTAA64.EFI
	cp $(BUILD)/aarch64/kernel-qemu.elf $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/KERNEL.ELF
	cp assets/fonts/*.ttf $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/FONTS/
	cp assets/fonts/OFL-*.txt $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/FONT-LICENSES/
	cp -R assets/skins/. $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/
	cp $(WALLPAPER_ASSETS) $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Wallpapers/
	cp $(INSTALLER_UI_ASSETS) $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Installer/
	cp $(CRASH_ASSETS) $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Crash/
	dd if=/dev/zero of=$@ bs=1M count=320 status=none
	mformat -i $@ ::
	mcopy -i $@ -s $(BUILD)/fat-aarch64-qemu/EFI ::

$(BUILD)/infinity-aarch64-qemu.iso: $(BUILD)/infinity-aarch64-qemu.img
	@mkdir -p $(BUILD)/iso-aarch64-qemu/EFI
	cp $< $(BUILD)/iso-aarch64-qemu/efi.img
	cp -R $(BUILD)/fat-aarch64-qemu/EFI/BOOT $(BUILD)/fat-aarch64-qemu/EFI/INFINITY $(BUILD)/iso-aarch64-qemu/EFI/
	xorriso -as mkisofs -R -V INFINITYOS_ARM64 -e efi.img -no-emul-boot -o $@ $(BUILD)/iso-aarch64-qemu

aarch64: check-tools $(BUILD)/infinity-aarch64.iso $(BUILD)/infinity-aarch64-qemu.iso
	@echo "Built VirtualBox ARM64 image: $(BUILD)/infinity-aarch64.iso"
	@echo "Built QEMU ARM64 test image: $(BUILD)/infinity-aarch64-qemu.iso"

run-x86: x86
	qemu-system-i386 -machine pc -m 128M -cdrom $(BUILD)/infinity-x86.iso \
		-boot d -serial stdio -display none -no-reboot
run-aarch64: aarch64
	$(QEMU_AARCH64) -machine virt -cpu cortex-a72 -m 512M -bios $(AAVMF_CODE) \
		-device ramfb -device virtio-scsi-pci -drive if=none,id=cd,format=raw,media=cdrom,file=$(BUILD)/infinity-aarch64-qemu.iso \
		-device scsi-cd,drive=cd,bootindex=0 -serial stdio -display none -no-reboot

clean:
	rm -rf $(BUILD)
