SHELL := /bin/sh
.DEFAULT_GOAL := all

ifndef INFINITY_BUILD_KIT_ACTIVE
$(error Direct Make invocation is disabled; use ./build-kit with a named profile or ./build-kit run make TARGET)
endif

SYSTEM_SOUND_SOURCES := assets/sounds/boot.mp3 assets/sounds/login.mp3
SYSTEM_SOUND_ASSETS := assets/sounds/boot.pcm assets/sounds/login.pcm

.PHONY: audio-test audio-hardware-test system-sound-test system-sound-install-parity-test
audio-test:
	cargo run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin audio-test

audio-hardware-test:
	sh tools/audio-probe/run.sh

system-sound-test: $(SYSTEM_SOUND_ASSETS)
	@mkdir -p build/tools
	$(RUSTC) --test kernel/runtime/system_sound_policy.rs -o build/tools/system-sound-policy-test
	build/tools/system-sound-policy-test
	python3 tools/system-sound-assets-test.py

system-sound-install-parity-test: build/x86_64/kernel.elf build/x86_64/installed-kernel.elf build/aarch64/kernel.elf build/aarch64/installed-kernel.elf
	python3 tools/system-sound-install-parity.py

.PHONY: voice-synthesis-test voice-synthesis-hardware-test
voice-synthesis-test:
	python3 tools/voice-flite/build.py --target host
	$(CLANG) -O2 tools/voice-flite/host-test.c build/voice-flite/host/libflite.a -Wl,-dead_strip -o build/voice-flite/host/test
	build/voice-flite/host/test build/voice-flite/host/native-voice.wav
	$(RUSTC) --test kernel/drivers/speech_pcm.rs -o build/voice-flite/speech-pcm-test
	build/voice-flite/speech-pcm-test

voice-synthesis-hardware-test:
	sh tools/voice-flite/probe/run.sh tcg

.PHONY: voice-recognition-test voice-indicator-test voice-output-test

.PHONY: payload-cache-test
payload-cache-test:
	@mkdir -p $(BUILD)/tools
	$(CLANG) -std=c11 -Wall -Wextra -Werror tools/payload-cache-test.c -o $(BUILD)/tools/payload-cache-test
	$(BUILD)/tools/payload-cache-test
voice-output-test:
	$(RUSTC) --edition=2021 --test tools/voice-output-test.rs -o build/voice-output-test
	build/voice-output-test

voice-recognition-test: build/voice-kokoro/aarch64/private-native.o
	python3 tools/voice-whisper/probe/run.py

voice-indicator-test:
	$(RUSTC) --test tools/voice-indicator-test.rs -o build/voice-indicator-test
	build/voice-indicator-test

BUILD := build
LLVM := /opt/homebrew/opt/llvm/bin
LLD := /opt/homebrew/opt/lld/bin
CLANG := $(LLVM)/clang
LD_LLD := $(LLD)/ld.lld
LLD_LINK := $(LLD)/lld-link
OBJCOPY := $(LLVM)/llvm-objcopy
RUSTC := rustc
CARGO := cargo
include $(dir $(lastword $(MAKEFILE_LIST)))tools/browser-build.mk
QEMU_X64 := qemu-system-x86_64

QEMU_AARCH64 := qemu-system-aarch64
OVMF_CODE := $(firstword $(wildcard /opt/homebrew/share/qemu/edk2-x86_64-code.fd /opt/homebrew/share/qemu/edk2-x86_64-code.fd))
AAVMF_CODE := $(firstword $(wildcard /opt/homebrew/share/qemu/edk2-aarch64-code.fd))
AUTHENTICATION_MOTION_ASSETS := assets/desktop/infinity-auth-success-stage-v1.bmp \
	assets/desktop/infinity-auth-success-orb-v1.bmp \
	assets/desktop/infinity-auth-success-ripple-v1.bmp \
	assets/desktop/infinity-auth-success-splash-v1.bmp
KERNEL_SOURCES := $(shell find kernel -type d -name target -prune -o -type f -print) Cargo.toml Cargo.lock $(wildcard assets/fonts/*.atlas assets/fonts/*.metrics assets/fonts/*.kern assets/desktop/spatial-world-*.bmp assets/desktop/worldshift-hero-v1.bmp assets/cursors/*.rgba assets/cursors/hotspots.rs) $(AUTHENTICATION_MOTION_ASSETS) $(SYSTEM_SOUND_ASSETS) sdk/c/examples/hello.c
FONT_ASSETS := $(wildcard assets/fonts/*.ttf) $(wildcard assets/fonts/OFL-*.txt)
ICON_ASSETS := $(shell find assets/icons -type f)
ICON_RUNTIME_ASSETS := assets/icons/runtime/crystal-blue-glass-base.bmp assets/icons/runtime/crystal-blue-glass-actions.bmp \
	assets/icons/runtime/luminous-obsidian-base.bmp assets/icons/runtime/luminous-obsidian-actions.bmp \
	assets/icons/runtime/frosted-quartz-base.bmp assets/icons/runtime/frosted-quartz-actions.bmp \
	assets/icons/runtime/aurora-harmony-base.bmp assets/icons/runtime/aurora-harmony-actions.bmp \
	assets/icons/runtime/aurora-harmony-launcher-256.bmp
WALLPAPER_ASSETS := assets/desktop/infinity-default-dark-wallpaper-v2.png \
	assets/desktop/infinity-shell-wallpaper-v3.png \
	assets/desktop/infinity-onboarding-wallpaper-v1.png
UI_ASSETS := $(shell find assets/skins -type f) $(WALLPAPER_ASSETS)
INSTALLER_UI_ASSETS := assets/boot/infinity-installer-mesh-diagram-v1.png \
	assets/boot/installer-screens.infinityui assets/boot/installer-screens.iuit \
	assets/boot/configuration-screens.infinityui assets/boot/configuration-screens.iuit \
	assets/boot/settings-screens.infinityui assets/boot/settings-screens.iuit
INSTALLER_IMAGE_ASSET_DIR := assets/boot/installer-assets
INSTALLER_IMAGE_ASSETS := $(shell find $(INSTALLER_IMAGE_ASSET_DIR) -type f)
INSTALLER_TEMPLATE_SOURCE := assets/boot/installer-screens.infinityui
INSTALLER_TEMPLATE_RUNTIME := assets/boot/installer-screens.iuit
CONFIGURATION_TEMPLATE_SOURCE := assets/boot/configuration-screens.infinityui
CONFIGURATION_TEMPLATE_RUNTIME := assets/boot/configuration-screens.iuit
SETTINGS_TEMPLATE_SOURCE := assets/boot/settings-screens.infinityui
SETTINGS_TEMPLATE_RUNTIME := assets/boot/settings-screens.iuit
INSTALLER_DESIGNER_SOURCES := $(shell find tools/installer-designer/Sources -type f -name '*.swift') \
	tools/installer-designer/Package.swift tools/installer-designer/compile-template.sh
CRASH_ASSETS := $(shell find assets/crash -type f)
APPLICATION_ASSETS := $(shell find assets/apps -type f)
DESIGN_KIT_ASSETS := $(shell find assets/ui-design-kit/default -type f)
KERNEL_SOURCES += $(DESIGN_KIT_ASSETS)
KERNEL_SOURCES += $(wildcard sdk/infinity-enterprise-core/*.rs) sdk/infinity-enterprise-core/Cargo.toml

$(BUILD)/x86_64/installed-esp.img $(BUILD)/aarch64/installed-esp.img: $(DESIGN_KIT_ASSETS)
NODE_ASSETS := $(shell find assets/mesh -type f)
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
	assets/boot/installer-screens.iuit assets/boot/configuration-screens.iuit assets/boot/settings-screens.iuit \
	$(CRASH_ASSETS) $(NODE_ASSETS)

.PHONY: all x86_64 x86 aarch64 run-x86_64 run-x86 run-aarch64 test test-x86 test-x86_64 test-aarch64 test-console test-mouse-menu test-installer-safety installer-capacity-test installer-layout-test installer-template-runtime installer-template-test force-installer-template-compile component-manifest-test object-test namespace-test crash-recovery-test crash-screen-test object-vm-test milestone-3b-test runtime-test runtime-vm-test iop-test event-test capability-test service-crash-test milestone-4-test ai-test milestone-6-test milestone-6-5-test milestone-7-test milestone-7x-test milestone-7c-test milestone-8-test milestone-9-test milestone-11-test resource-policy-test network-test icon-theme-test settings-color-test settings-timeout-test desktop-system-test ui-install-parity-test input-regression-test app-launcher-interaction-test worldshift-art-test task-manager-test file-navigator-workspace-test installed-object-test vm-disk reset-test-disk install-test install-boot-test installed-console-test system-generation-test boot-installed clean check-tools

assets/sounds/boot.pcm: assets/sounds/boot.mp3 tools/build-system-sound.sh
	tools/build-system-sound.sh $< $@

assets/sounds/login.pcm: assets/sounds/login.mp3 tools/build-system-sound.sh
	tools/build-system-sound.sh $< $@

app-launcher-interaction-test:
	@mkdir -p build/tools
	@rustc --test kernel/ui/launcher_navigation.rs -o build/tools/launcher-navigation-test
	@build/tools/launcher-navigation-test
	@mkdir -p build/tools
	@rustc --edition 2021 -A warnings tools/app-launcher-interaction-test.rs -o build/tools/app-launcher-interaction-test
	@build/tools/app-launcher-interaction-test
	@rustc --edition 2021 -A warnings tools/desktop-input-calendar-test.rs -o build/tools/desktop-input-calendar-test
	@build/tools/desktop-input-calendar-test

worldshift-art-test:
	@python3 tools/worldshift-art-test.py

milestone-9-test:
	@tools/milestone9-test.sh

milestone-11-test:
	@CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --manifest-path tools/behavior-harness/Cargo.toml --bin milestone-11-compute-test

.PHONY: milestone-9-service-test
milestone-9-service-test:
	@RUST_MIN_STACK=16777216 CARGO_TARGET_DIR=build/behavior-harness cargo test --quiet --manifest-path tools/behavior-harness/Cargo.toml --bin runtime-test node_reconciliation_tests
	@RUST_MIN_STACK=16777216 CARGO_TARGET_DIR=build/behavior-harness cargo test --quiet --manifest-path tools/behavior-harness/Cargo.toml --bin runtime-test node_operator::tests

.PHONY: milestone-9-durable-mutation-test milestone-9-correlation-test
milestone-9-durable-mutation-test:
	@CARGO_TARGET_DIR=build/milestone9-harness cargo test --quiet --manifest-path tools/milestone9-harness/Cargo.toml durable_
	@RUST_MIN_STACK=16777216 CARGO_TARGET_DIR=build/behavior-harness cargo test --quiet --manifest-path tools/behavior-harness/Cargo.toml --bin runtime-test node::persistence::tests

milestone-9-correlation-test:
	@CARGO_TARGET_DIR=build/milestone9-harness cargo test --quiet --manifest-path tools/milestone9-harness/Cargo.toml

.PHONY: network-wire-test
.PHONY: native-tls-test native-c-storage-test native-memory-test
native-c-storage-test:
	@mkdir -p build/behavior-tests
	@rustc --edition=2021 -A warnings tools/native-c-storage-test.rs -o build/behavior-tests/native-c-storage-test
	@build/behavior-tests/native-c-storage-test

native-memory-test:
	@mkdir -p build/behavior-tests
	@rustc --edition=2021 -A warnings tools/native-memory-test.rs -o build/behavior-tests/native-memory-test
	@build/behavior-tests/native-memory-test

.PHONY: native-https-test
native-https-test: $(BUILD)/x86_64/BOOTX64.EFI
	@python3 tools/native-https-probe/run.py

.PHONY: native-https-arm-test
native-https-arm-test: $(BUILD)/aarch64/BOOTAA64.EFI
	@python3 tools/native-https-probe/run.py --arch aarch64

.PHONY: native-https-rsa-test
native-https-rsa-test: $(BUILD)/x86_64/BOOTX64.EFI $(BUILD)/aarch64/BOOTAA64.EFI
	@python3 tools/native-https-probe/run.py --rsa
	@python3 tools/native-https-probe/run.py --arch aarch64 --rsa

native-tls-test:
	@sh tools/native-tls-probe/test.sh
.PHONY: http-transport-test
.PHONY: https-service-test
https-service-test:
	@CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --manifest-path tools/behavior-harness/Cargo.toml --bin https-service-test

http-transport-test:
	@CARGO_TARGET_DIR=$(BUILD)/http-test cargo test --manifest-path kernel/runtime/http/Cargo.toml

network-wire-test:
	@mkdir -p build/tools
	@rustc --edition 2021 tools/network-wire-test.rs -o build/tools/network-wire-test
	@build/tools/network-wire-test
	@rustc --edition 2021 -Awarnings tools/network-datagram-test.rs -o build/tools/network-datagram-test
	@build/tools/network-datagram-test

.PHONY: native-nic-test
native-nic-test: $(BUILD)/x86_64/BOOTX64.EFI
	RUSTC_BOOTSTRAP=1 cargo build --release -Z build-std=core --target x86_64-unknown-none --manifest-path tools/nic-probe/Cargo.toml
	$(LD_LLD) -nostdlib -static -T linker/x86_64.ld -o $(BUILD)/nic-probe.elf tools/nic-probe/target/x86_64-unknown-none/release/libinfinity_native_nic_probe.a
	python3 tools/nic-probe/run.py

.PHONY: milestone-9-wire-trust-test
.PHONY: milestone-9-remote-iop-test
milestone-9-remote-iop-test:
	INFINITY_9B_TEST=1 $(MAKE) milestone-9-wire-trust-test

milestone-9-wire-trust-test: $(BUILD)/x86_64/BOOTX64.EFI
	RUSTC_BOOTSTRAP=1 cargo build --release -Z build-std=core --target x86_64-unknown-none --manifest-path tools/wire-trust-probe/Cargo.toml
	$(LD_LLD) -nostdlib -static -T linker/x86_64.ld -o $(BUILD)/wire-trust-probe.elf tools/wire-trust-probe/target/x86_64-unknown-none/release/libinfinity_wire_trust_probe.a
	python3 tools/wire-trust-probe/run.py

crash-screen-test:
	@tools/crash-screen-test.sh

milestone-7x-test:
	@tools/skin-compiler-test.sh
	@tools/infinity-ui-test.sh

milestone-7c-test:
	@tools/milestone-7c-test.sh

.PHONY: performance-test
performance-test:
	@CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --release \
		--manifest-path tools/behavior-harness/Cargo.toml --bin performance-test

.PHONY: active-painter-test
active-painter-test:
	@mkdir -p build/behavior-tests
	@rustc --edition=2021 -C opt-level=z -Awarnings tools/memory-throughput-test.rs -o build/behavior-tests/memory-throughput-test
	@build/behavior-tests/memory-throughput-test
	@rustc --edition=2021 -C opt-level=z -Awarnings tools/active-painter-test.rs -o build/behavior-tests/active-painter-test
	@build/behavior-tests/active-painter-test

icon-theme-test:
	@tools/icon-theme-test.sh

settings-color-test:
	@mkdir -p build/tools
	@rustc --edition 2021 -A warnings tools/settings-color-test.rs -o build/tools/settings-color-test
	@build/tools/settings-color-test
	@echo "Independent Primary and Secondary theme controls: PASS"

settings-timeout-test:
	@CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet \
		--manifest-path tools/behavior-harness/Cargo.toml --bin settings-timeout-test

installer-layout-test:
	@mkdir -p build/tools
	@rustc --edition 2021 -A warnings tools/installer-layout-test.rs -o build/tools/installer-layout-test
	@build/tools/installer-layout-test

force-installer-template-compile:

$(INSTALLER_TEMPLATE_RUNTIME): force-installer-template-compile $(INSTALLER_TEMPLATE_SOURCE) $(INSTALLER_DESIGNER_SOURCES) $(INSTALLER_IMAGE_ASSETS)
	@tools/installer-designer/compile-template.sh

$(CONFIGURATION_TEMPLATE_RUNTIME): force-installer-template-compile $(CONFIGURATION_TEMPLATE_SOURCE) $(INSTALLER_DESIGNER_SOURCES) $(INSTALLER_IMAGE_ASSETS)
	@tools/installer-designer/compile-template.sh $(CONFIGURATION_TEMPLATE_SOURCE) $(CONFIGURATION_TEMPLATE_RUNTIME)

$(SETTINGS_TEMPLATE_RUNTIME): force-installer-template-compile $(SETTINGS_TEMPLATE_SOURCE) $(INSTALLER_DESIGNER_SOURCES) $(INSTALLER_IMAGE_ASSETS)
	@tools/installer-designer/compile-template.sh $(SETTINGS_TEMPLATE_SOURCE) $(SETTINGS_TEMPLATE_RUNTIME)

installer-template-runtime: $(INSTALLER_TEMPLATE_RUNTIME) $(CONFIGURATION_TEMPLATE_RUNTIME) $(SETTINGS_TEMPLATE_RUNTIME)

installer-template-test: $(INSTALLER_TEMPLATE_RUNTIME) $(CONFIGURATION_TEMPLATE_RUNTIME) $(SETTINGS_TEMPLATE_RUNTIME)
	@tools/installer-designer/verify-template-build.sh
	@mkdir -p build/tools
	@rustc --edition 2021 -A warnings -O tools/installer-template-test.rs -o build/tools/installer-template-test
	@build/tools/installer-template-test

all: x86_64

.PHONY: video-driver-test
video-driver-test:
	@mkdir -p build/behavior-tests
	cc -std=c11 -Wall -Wextra -Werror tools/video-mode-test.c -o build/behavior-tests/video-mode-test
	build/behavior-tests/video-mode-test
	rustc --edition=2021 tools/svga-driver-test.rs -o build/behavior-tests/svga-driver-test
	build/behavior-tests/svga-driver-test
	rustc --edition=2021 tools/svga-qtest.rs -o build/behavior-tests/svga-qtest
	build/behavior-tests/svga-qtest

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
		-Z build-std=core --target x86_64-unknown-none $(BROWSER_FEATURE)
	cp $(BUILD)/cargo-installed/x86_64-unknown-none/release/libinfinity_kernel.a $(BUILD)/x86_64/libinstalled-kernel.a
	touch $@


FLITE_PORT_SOURCES := $(wildcard tools/voice-flite/include/*.h) tools/voice-flite/port.c tools/voice-flite/jump.S tools/voice-flite/build.py tools/voice-flite/COPYING

$(BUILD)/voice-kokoro/aarch64/private-native.o: tools/voice_target.py $(wildcard tools/voice-kokoro/*.py tools/voice-kokoro/*.c tools/voice-kokoro/*.cpp tools/voice-kokoro/*.h tools/voice-kokoro/*.ld tools/voice-kokoro/*.txt) $(wildcard tools/voice-whisper/*.py tools/voice-whisper/*.cpp) $(wildcard tools/voice-native-runtime/*.py) $(wildcard sdk/compiler/*.c sdk/compiler/*.h sdk/compiler/*.patch)
	python3 tools/voice-kokoro/build.py --build-only

$(BUILD)/voice-kokoro/x86_64/private-native.o: tools/voice_target.py $(wildcard tools/voice-kokoro/*.py tools/voice-kokoro/*.c tools/voice-kokoro/*.cpp tools/voice-kokoro/*.h tools/voice-kokoro/*.ld tools/voice-kokoro/*.txt) $(wildcard tools/voice-whisper/*.py tools/voice-whisper/*.cpp) $(wildcard tools/voice-native-runtime/*.py) $(wildcard sdk/compiler/*.c sdk/compiler/*.h sdk/compiler/*.patch)
	python3 tools/voice-kokoro/build.py --target x86_64 --build-only

$(BUILD)/x86_64/qwen-math.o: kernel/runtime/ai/qwen/cpu_math.c
	@mkdir -p $(@D)
	$(CLANG) --target=x86_64-none-elf -mno-red-zone -mno-avx -ffreestanding -fno-builtin -fno-stack-protector -ffp-contract=off -O3 -c $< -o $@

$(BUILD)/voice-flite/%/libflite.a: $(FLITE_PORT_SOURCES)
	python3 tools/voice-flite/build.py --target $*

$(BUILD)/x86_64/installed-kernel.elf: $(BUILD)/x86_64/installed-kernel.o linker/x86_64.ld $(BUILD)/x86_64/qwen-math.o $(BUILD)/voice-kokoro/x86_64/private-native.o
	$(LD_LLD) -nostdlib -static $(BROWSER_LINK) -T linker/x86_64.ld -o $@ $(BUILD)/x86_64/libinstalled-kernel.a $(BUILD)/x86_64/qwen-math.o $(BUILD)/voice-kokoro/x86_64/private-native.o $(BROWSER_X86)

$(BUILD)/x86_64/kernel.o: $(KERNEL_SOURCES) $(SPLASH_ASSET) $(BUILD)/x86_64/installed-esp.img $(BUILD)/x86_64/installed-kernel.elf
	@mkdir -p $(@D)
	RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=$(BUILD)/cargo $(CARGO) build --release \
		-Z build-std=core --target x86_64-unknown-none --features installer $(BROWSER_FEATURE) $(BROWSER_INSTALL_FEATURE)
	cp $(BUILD)/cargo/x86_64-unknown-none/release/libinfinity_kernel.a $(BUILD)/x86_64/libkernel.a
	touch $@

$(BUILD)/x86_64/kernel.elf: $(BUILD)/x86_64/kernel.o linker/x86_64.ld $(BUILD)/x86_64/qwen-math.o $(BUILD)/voice-kokoro/x86_64/private-native.o
	$(LD_LLD) -nostdlib -static $(BROWSER_LINK) -T linker/x86_64.ld -o $@ $(BUILD)/x86_64/libkernel.a $(BUILD)/x86_64/qwen-math.o $(BUILD)/voice-kokoro/x86_64/private-native.o $(BROWSER_X86)

$(BUILD)/x86_64/loader.obj: boot/common/uefi_loader.c boot/common/boot_info.h boot/common/video_modes.h boot/common/tpm_random.h boot/x86_64/workers.h boot/common/payload_loader.h boot/common/payload_cache.h
	@mkdir -p $(@D)
	$(CLANG) --target=x86_64-pc-windows-msvc -ffreestanding -fshort-wchar -fno-stack-protector \
		-mno-red-zone -O2 -Wall -Wextra -Werror -c $< -o $@

$(BUILD)/x86_64/handoff.obj: boot/x86_64/handoff.asm
	@mkdir -p $(@D)
	nasm -f win64 $< -o $@

$(BUILD)/x86_64/workers.obj: boot/x86_64/workers.asm
	@mkdir -p $(@D)
	nasm -f win64 $< -o $@

$(BUILD)/x86_64/BOOTX64.EFI: $(BUILD)/x86_64/loader.obj $(BUILD)/x86_64/handoff.obj $(BUILD)/x86_64/workers.obj Makefile
	$(LLD_LINK) /subsystem:efi_application /entry:efi_main /nodefaultlib /machine:x64 /base:0x2000000 /out:$@ $(filter %.obj,$^)

$(BUILD)/x86_64/installed-esp.img: $(BUILD)/x86_64/BOOTX64.EFI $(FONT_ASSETS) $(UI_ASSETS) $(ICON_ASSETS) $(INSTALLER_UI_ASSETS) $(INSTALLER_IMAGE_ASSETS) $(CRASH_ASSETS) $(APPLICATION_ASSETS) $(NODE_ASSETS)
	rm -rf $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Icons $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Wallpapers $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Crash
	@mkdir -p $(BUILD)/installed-fat/EFI/BOOT $(BUILD)/installed-fat/EFI/InfinityOS/Fonts $(BUILD)/installed-fat/EFI/InfinityOS/FontLicenses $(BUILD)/installed-fat/EFI/InfinityOS/Applications $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Wallpapers $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Installer $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Icons $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Crash $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Mesh
	cp $(BUILD)/x86_64/BOOTX64.EFI $(BUILD)/installed-fat/EFI/BOOT/BOOTX64.EFI
	cp $(BUILD)/x86_64/BOOTX64.EFI $(BUILD)/installed-fat/EFI/InfinityOS/infinity.efi
	cp assets/fonts/*.ttf $(BUILD)/installed-fat/EFI/InfinityOS/Fonts/
	cp assets/fonts/OFL-*.txt $(BUILD)/installed-fat/EFI/InfinityOS/FontLicenses/
	cp -R assets/skins/. $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/
	cp $(WALLPAPER_ASSETS) $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Wallpapers/
	cp -R assets/icons/. $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Icons/
	cp $(INSTALLER_UI_ASSETS) $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Installer/
	cp -R $(INSTALLER_IMAGE_ASSET_DIR) $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Installer/
	cp $(CRASH_ASSETS) $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Crash/
	cp $(NODE_ASSETS) $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/Mesh/
	cp $(APPLICATION_ASSETS) $(BUILD)/installed-fat/EFI/InfinityOS/Applications/
	@mkdir -p $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/DesignKit/Default
	cp $(DESIGN_KIT_ASSETS) $(BUILD)/installed-fat/EFI/InfinityOS/InfinityUI/DesignKit/Default/
	python3 tools/iso-staging.py allocate $(BUILD)/installed-fat $@
	mformat -F -i $@ -v INFINITYEFI ::
	mcopy -i $@ -s $(BUILD)/installed-fat/EFI ::

$(BUILD)/infinity-x86_64.img: $(BUILD)/x86_64/BOOTX64.EFI $(BUILD)/x86_64/kernel.elf $(FONT_ASSETS) $(UI_ASSETS) $(INSTALLER_UI_ASSETS) $(INSTALLER_IMAGE_ASSETS) $(CRASH_ASSETS) $(APPLICATION_ASSETS) $(NODE_ASSETS)
	rm -rf $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Icons $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Wallpapers $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Crash
	@mkdir -p $(BUILD)/fat/EFI/BOOT $(BUILD)/fat/EFI/INFINITY/FONTS $(BUILD)/fat/EFI/INFINITY/FONT-LICENSES $(BUILD)/fat/EFI/INFINITY/APPLICATIONS $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Wallpapers $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Installer $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Crash $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Mesh
	cp $(BUILD)/x86_64/BOOTX64.EFI $(BUILD)/fat/EFI/BOOT/BOOTX64.EFI
	cp $(BUILD)/x86_64/kernel.elf $(BUILD)/fat/EFI/INFINITY/KERNEL.ELF
	rm -rf $(BUILD)/fat/EFI/INFINITY/PAYLOAD
	$(BROWSER_STAGE_X86)
	cp assets/fonts/*.ttf $(BUILD)/fat/EFI/INFINITY/FONTS/
	cp assets/fonts/OFL-*.txt $(BUILD)/fat/EFI/INFINITY/FONT-LICENSES/
	cp -R assets/skins/. $(BUILD)/fat/EFI/INFINITY/INFINITYUI/
	cp $(WALLPAPER_ASSETS) $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Wallpapers/
	cp $(INSTALLER_UI_ASSETS) $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Installer/
	cp -R $(INSTALLER_IMAGE_ASSET_DIR) $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Installer/
	cp $(CRASH_ASSETS) $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Crash/
	cp $(NODE_ASSETS) $(BUILD)/fat/EFI/INFINITY/INFINITYUI/Mesh/
	cp $(APPLICATION_ASSETS) $(BUILD)/fat/EFI/INFINITY/APPLICATIONS/
	python3 tools/iso-staging.py allocate $(BUILD)/fat $@
	mformat -F -i $@ ::
	mcopy -i $@ -s $(BUILD)/fat/EFI ::

.PHONY: x86-native-speech-parity
x86-native-speech-parity: $(BUILD)/x86_64/installed-kernel.elf $(BUILD)/x86_64/kernel.elf $(if $(filter 1,$(NATIVE_BROWSER)),$(BUILD)/infinity-x86_64.img)
	python3 tools/voice-kokoro/install-parity.py $(if $(filter 1,$(NATIVE_BROWSER)),--installer-media $(BUILD)/infinity-x86_64.img,--embedded-install) $(BUILD)/x86_64/installed-kernel.elf $(BUILD)/x86_64/kernel.elf

$(BUILD)/test-media/InfinityOS-x86_64.bootmedia: $(BUILD)/infinity-x86_64.img x86-native-speech-parity
	@mkdir -p $(BUILD)/test-media
	@mkdir -p $(BUILD)/iso/EFI
	cp $< $(BUILD)/iso/efi.img
	rm -rf $(BUILD)/iso/EFI/INFINITY/PAYLOAD
	cp -R $(BUILD)/fat/EFI/BOOT $(BUILD)/fat/EFI/INFINITY $(BUILD)/iso/EFI/
	@test "$(INFINITY_ISO_BUILD_AUTHORITY)" = build.sh || { echo "ERROR: boot media creation is restricted to ./build.sh" >&2; exit 2; }
	xorriso -as mkisofs -R -V INFINITYOS -e efi.img -no-emul-boot -o $@.partial $(BUILD)/iso
	mv $@.partial $@

x86_64: check-tools $(BUILD)/test-media/InfinityOS-x86_64.bootmedia
	@echo "Built x86_64 internal boot media; use ./build.sh --target x86_64 to publish an ISO"

run-x86_64: x86_64
	$(QEMU_X64) -machine q35 -smp 4 -m 16384M -drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
		-cdrom builds/InfinityOS-x86_64.iso -serial stdio -display none -no-reboot

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

.PHONY: installer-entropy-test
.PHONY: editor-window-test
.PHONY: editor-assistant-test authentication-logo-test authentication-motion-test
editor-assistant-test:
	@mkdir -p build/behavior-tests
	rustc --edition=2021 tools/editor-assistant-test.rs -o build/behavior-tests/editor-assistant-test
	build/behavior-tests/editor-assistant-test

authentication-logo-test: x86_64
	@mkdir -p $(BUILD)/tmp; work=$$(mktemp -d $(BUILD)/tmp/infinity-auth-logo.XXXXXX); rmdir "$$work"; \
		python3 tools/authentication-logo-installed-test.py "$$work"

authentication-motion-test:
	@mkdir -p build/behavior-tests
	$(RUSTC) --edition=2021 --test kernel/ui/authentication_motion.rs \
		-o build/behavior-tests/authentication-motion-test
	build/behavior-tests/authentication-motion-test

editor-window-test:
	@mkdir -p build/behavior-tests
	rustc --edition=2021 tools/editor-window-test.rs -o build/behavior-tests/editor-window-test
	build/behavior-tests/editor-window-test

installer-entropy-test:
	@mkdir -p build/behavior-tests
	cc -std=c11 -Wall -Wextra -Werror tools/tpm-random-test.c -o build/behavior-tests/tpm-random-test
	build/behavior-tests/tpm-random-test
	python3 tools/re-provision-tpm-test.py

.PHONY: fabric-test
fabric-test:
	@CARGO_TARGET_DIR=build/behavior-harness cargo test --quiet --release \
		--manifest-path tools/behavior-harness/Cargo.toml --bin fabric-test

milestone-3b-test: object-test install-test object-vm-test

runtime-test iop-test event-test capability-test service-crash-test:
	@tools/runtime-test.sh

network-test:
	@tools/network-test.sh

milestone-8-test: network-test runtime-test
	@tools/milestone-8-object-navigation-test.sh
	@tools/milestone-8-install-parity-test.sh

.PHONY: boot-media-test
.PHONY: install-boot-handoff-test
install-boot-handoff-test:
	@mkdir -p build/behavior-tests
	@rustc --edition=2021 --test kernel/core/install_boot.rs -o build/behavior-tests/install-boot-test
	@build/behavior-tests/install-boot-test
.PHONY: boot-reveal-test
boot-reveal-test:
	@mkdir -p build/behavior-tests
	@rustc --test kernel/core/bootstrap/reveal.rs -o build/behavior-tests/boot-reveal-test
	@build/behavior-tests/boot-reveal-test

boot-media-test:
	@mkdir -p build/behavior-tests
	@clang -O2 -fshort-wchar -Wno-ignored-attributes tools/boot-media-test.c -o build/behavior-tests/boot-media-test
	@build/behavior-tests/boot-media-test

.PHONY: voice-pcm-test
voice-pcm-test:
	CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet --manifest-path tools/behavior-harness/Cargo.toml --bin voice-pcm-test
	CARGO_TARGET_DIR=build/behavior-harness cargo test --quiet --manifest-path tools/behavior-harness/Cargo.toml --bin voice-pcm-test

.PHONY: voice-vad-test
voice-vad-test:
	mkdir -p build/tools
	rustc --edition=2021 -O tools/voice-vad-test.rs -o build/tools/voice-vad-test
	build/tools/voice-vad-test
	rustc --edition=2021 --test kernel/runtime/ai/voice_vad.rs -o build/tools/voice-vad-private-test
	build/tools/voice-vad-private-test

ai-test:
	@tools/ai-test.sh
	@cargo test --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin hermes-native-test causal_attention
	@mkdir -p build/behavior-tests
	@rustc --edition=2021 --test kernel/runtime/ai/qwen/pump.rs -o build/behavior-tests/qwen-pump-test
	@build/behavior-tests/qwen-pump-test
	@rustc --edition=2021 --test kernel/runtime/ai/qwen/metrics.rs -o build/behavior-tests/qwen-metrics-test
	@build/behavior-tests/qwen-metrics-test
	@clang -O3 -ffp-contract=off -c kernel/runtime/ai/qwen/cpu_math.c -o build/behavior-tests/qwen-worker-math.o
	@rustc --edition=2021 --test kernel/runtime/ai/qwen/workers.rs -C link-arg=build/behavior-tests/qwen-worker-math.o -o build/behavior-tests/qwen-workers-test
	@build/behavior-tests/qwen-workers-test
	@clang -O3 -ffp-contract=off -DQWEN_SCALAR -Dinfinity_qwen_dot=infinity_qwen_scalar_dot -Dinfinity_qwen_dot_rows=infinity_qwen_scalar_dot_rows -Dinfinity_qwen_dot_rows_cached=infinity_qwen_scalar_dot_rows_cached -c kernel/runtime/ai/qwen/cpu_math.c -o build/behavior-tests/qwen-scalar-math.o
	@clang -O3 -ffp-contract=off tools/qwen-activation-cache-test.c kernel/runtime/ai/qwen/cpu_math.c -o build/behavior-tests/qwen-activation-cache-test
	@build/behavior-tests/qwen-activation-cache-test
	@clang -O3 -ffp-contract=off -DQWEN_EXACT_Q6 -DQWEN_VERIFY_SCALAR tools/qwen-rows-bench.c kernel/runtime/ai/qwen/cpu_math.c build/behavior-tests/qwen-scalar-math.o -o build/behavior-tests/qwen-rows-test
	@build/behavior-tests/qwen-rows-test
	@clang -O3 -ffp-contract=off tools/qwen-q8-totals-test.c kernel/runtime/ai/qwen/cpu_math.c -o build/behavior-tests/qwen-q8-totals-test
	@build/behavior-tests/qwen-q8-totals-test || test $$? -eq 77
	@clang -O3 -ffp-contract=off -DQWEN_EXACT_Q6 tools/qwen-q6-scales-test.c kernel/runtime/ai/qwen/cpu_math.c build/behavior-tests/qwen-scalar-math.o -o build/behavior-tests/qwen-q6-scales-test
	@build/behavior-tests/qwen-q6-scales-test
	@clang -O3 -ffp-contract=off tools/qwen-q6-q8-test.c kernel/runtime/ai/qwen/cpu_math.c build/behavior-tests/qwen-scalar-math.o -o build/behavior-tests/qwen-q6-q8-test
	@build/behavior-tests/qwen-q6-q8-test
	@clang -O2 tools/psci-topology-test.c -o build/behavior-tests/psci-topology-test
	@build/behavior-tests/psci-topology-test

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
	@mkdir -p build/behavior-tests
	@rustc --edition=2021 -A warnings --test tools/personalization-test.rs -o build/behavior-tests/personalization-test
	@build/behavior-tests/personalization-test --test-threads=1
	@rustc --edition=2021 --test tools/desktop-widgets-test.rs -o build/behavior-tests/desktop-widgets-test
	@build/behavior-tests/desktop-widgets-test
	@rustc --edition=2021 --test tools/minimized-shelf-test.rs -o build/behavior-tests/minimized-shelf-test
	@build/behavior-tests/minimized-shelf-test
	@rustc --edition=2021 --test kernel/core/bootstrap/spatial_timing.rs -o build/behavior-tests/spatial-timing-test
	@build/behavior-tests/spatial-timing-test
	@rustc --edition=2021 --test kernel/core/bootstrap/spatial_surface.rs -o build/behavior-tests/spatial-surface-test
	@build/behavior-tests/spatial-surface-test
	@rustc --edition=2021 -A warnings --test tools/active-painter-test.rs -o build/behavior-tests/active-painter-unit-test
	@build/behavior-tests/active-painter-unit-test
	@rustc --edition=2021 --test tools/soft-stroke-test.rs -o build/behavior-tests/soft-stroke-test
	@build/behavior-tests/soft-stroke-test
	@rustc --edition=2021 --test tools/spatial-path-test.rs -o build/behavior-tests/spatial-path-test
	@build/behavior-tests/spatial-path-test
	@rustc --edition=2021 -A warnings --test tools/spatial-retained-test.rs -o build/behavior-tests/spatial-retained-test
	@build/behavior-tests/spatial-retained-test
	@rustc --edition=2021 -A warnings --test tools/spatial-state-test.rs -o build/behavior-tests/spatial-state-test
	@build/behavior-tests/spatial-state-test
	@rustc --edition=2021 -A warnings --test tools/spatial-backdrop-test.rs -o build/behavior-tests/spatial-backdrop-test
	@build/behavior-tests/spatial-backdrop-test
	@rustc --edition=2021 --test kernel/ui/motion.rs -o build/behavior-tests/motion-test
	@build/behavior-tests/motion-test
	@rustc --edition=2021 --test kernel/drivers/input/desktop_shortcuts.rs -o build/behavior-tests/desktop-shortcuts-test
	@build/behavior-tests/desktop-shortcuts-test
	@rustc --edition=2021 --test tools/window-workflows-test.rs -o build/behavior-tests/window-workflows-test
	@build/behavior-tests/window-workflows-test
	@tools/input-regression-test.sh

task-manager-test:
	@CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet \
		--manifest-path tools/behavior-harness/Cargo.toml --bin task-manager-test

resource-policy-test:
	@CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet \
		--manifest-path tools/behavior-harness/Cargo.toml --bin resource-policy-test

file-navigator-workspace-test:
	@CARGO_TARGET_DIR=build/behavior-harness cargo run --quiet \
		--manifest-path tools/behavior-harness/Cargo.toml --bin file-navigator-workspace-test

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
	@test -s $(BUILD)/infinity-test-disk.raw || dd if=/dev/zero of=$(BUILD)/infinity-test-disk.raw bs=1M count=0 seek=16384 status=none
	@echo "Disposable InfinityOS test disk: $(BUILD)/infinity-test-disk.raw"

reset-test-disk:
	@mkdir -p $(BUILD)
	dd if=/dev/zero of=$(BUILD)/infinity-test-disk.raw bs=1M count=0 seek=16384 status=none
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
	$(LD_LLD) -m elf_i386 -nostdlib -static -s -T linker/x86.ld -o $@ $(BUILD)/x86/libkernel.a

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

$(BUILD)/test-media/InfinityOS-x86.bootmedia: $(BUILD)/infinity-x86.img $(UI_ASSETS) $(INSTALLER_UI_ASSETS) $(INSTALLER_IMAGE_ASSETS) $(CRASH_ASSETS) $(APPLICATION_ASSETS)
	@mkdir -p $(BUILD)/test-media
	rm -rf $(BUILD)/iso-x86/System/InfinityUI/Icons $(BUILD)/iso-x86/System/InfinityUI/Wallpapers $(BUILD)/iso-x86/System/InfinityUI/Crash
	@mkdir -p $(BUILD)/iso-x86/System/Fonts $(BUILD)/iso-x86/System/FontLicenses $(BUILD)/iso-x86/System/Applications $(BUILD)/iso-x86/System/InfinityUI/Wallpapers $(BUILD)/iso-x86/System/InfinityUI/Installer $(BUILD)/iso-x86/System/InfinityUI/Crash
	cp $< $(BUILD)/iso-x86/x86-boot.img
	cp assets/fonts/*.ttf $(BUILD)/iso-x86/System/Fonts/
	cp assets/fonts/OFL-*.txt $(BUILD)/iso-x86/System/FontLicenses/
	cp -R assets/skins/. $(BUILD)/iso-x86/System/InfinityUI/
	cp $(WALLPAPER_ASSETS) $(BUILD)/iso-x86/System/InfinityUI/Wallpapers/
	cp $(INSTALLER_UI_ASSETS) $(BUILD)/iso-x86/System/InfinityUI/Installer/
	cp -R $(INSTALLER_IMAGE_ASSET_DIR) $(BUILD)/iso-x86/System/InfinityUI/Installer/
	cp $(CRASH_ASSETS) $(BUILD)/iso-x86/System/InfinityUI/Crash/
	cp $(APPLICATION_ASSETS) $(BUILD)/iso-x86/System/Applications/
	@test "$(INFINITY_ISO_BUILD_AUTHORITY)" = build.sh || { echo "ERROR: boot media creation is restricted to ./build.sh" >&2; exit 2; }
	xorriso -as mkisofs -R -V INFINITYOS_X86 -b x86-boot.img -c boot.cat -o $@.partial $(BUILD)/iso-x86
	mv $@.partial $@

x86: check-tools $(BUILD)/test-media/InfinityOS-x86.bootmedia
	@echo "Built BIOS x86 internal boot media: build/test-media/InfinityOS-x86.bootmedia"

test-x86: x86
	@tools/smoke-test.sh x86

$(BUILD)/aarch64/installed-kernel.stamp: $(KERNEL_SOURCES) $(SPLASH_ASSET) $(ICON_RUNTIME_ASSETS)
	@mkdir -p $(@D)
	RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=$(BUILD)/cargo-installed-aarch64 $(CARGO) build --release \
		-Z build-std=core --target aarch64-unknown-none-softfloat $(BROWSER_FEATURE)
	cp $(BUILD)/cargo-installed-aarch64/aarch64-unknown-none-softfloat/release/libinfinity_kernel.a $(BUILD)/aarch64/libinstalled-kernel.a
	touch $@

$(BUILD)/aarch64/qwen-math.o: kernel/runtime/ai/qwen/cpu_math.c
	@mkdir -p $(@D)
	$(CLANG) --target=aarch64-none-elf -ffreestanding -fno-builtin -fno-stack-protector -ffp-contract=off -O3 -c $< -o $@

$(BUILD)/aarch64/installed-kernel.elf: $(BUILD)/aarch64/installed-kernel.stamp linker/aarch64.ld $(BUILD)/aarch64/qwen-math.o $(BUILD)/voice-kokoro/aarch64/private-native.o
	$(LD_LLD) -nostdlib -static $(BROWSER_LINK) -T linker/aarch64.ld -o $@ $(BUILD)/aarch64/libinstalled-kernel.a $(BUILD)/aarch64/qwen-math.o $(BUILD)/voice-kokoro/aarch64/private-native.o $(BROWSER_ARM)

$(BUILD)/aarch64/installed-esp.img: $(BUILD)/aarch64/BOOTAA64.EFI $(FONT_ASSETS) $(UI_ASSETS) $(ICON_ASSETS) $(INSTALLER_UI_ASSETS) $(INSTALLER_IMAGE_ASSETS) $(CRASH_ASSETS) $(APPLICATION_ASSETS) $(NODE_ASSETS)
	rm -rf $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Icons $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Wallpapers $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Crash
	@mkdir -p $(BUILD)/installed-fat-aarch64/EFI/BOOT $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/Fonts $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/FontLicenses $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/Applications $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Wallpapers $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Installer $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Icons $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Crash $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Mesh
	cp $(BUILD)/aarch64/BOOTAA64.EFI $(BUILD)/installed-fat-aarch64/EFI/BOOT/BOOTAA64.EFI
	cp $(BUILD)/aarch64/BOOTAA64.EFI $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/infinity.efi
	cp assets/fonts/*.ttf $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/Fonts/
	cp assets/fonts/OFL-*.txt $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/FontLicenses/
	cp -R assets/skins/. $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/
	cp $(WALLPAPER_ASSETS) $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Wallpapers/
	cp -R assets/icons/. $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Icons/
	cp $(INSTALLER_UI_ASSETS) $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Installer/
	cp -R $(INSTALLER_IMAGE_ASSET_DIR) $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Installer/
	cp $(CRASH_ASSETS) $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Crash/
	cp $(NODE_ASSETS) $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/Mesh/
	cp $(APPLICATION_ASSETS) $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/Applications/
	@mkdir -p $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/DesignKit/Default
	cp $(DESIGN_KIT_ASSETS) $(BUILD)/installed-fat-aarch64/EFI/InfinityOS/InfinityUI/DesignKit/Default/
	python3 tools/iso-staging.py allocate $(BUILD)/installed-fat-aarch64 $@
	mformat -F -i $@ -v INFINITYEFI ::
	mcopy -i $@ -s $(BUILD)/installed-fat-aarch64/EFI ::

$(BUILD)/aarch64/kernel.stamp: $(KERNEL_SOURCES) $(SPLASH_ASSET) $(BUILD)/aarch64/installed-esp.img $(BUILD)/aarch64/installed-kernel.elf
	@mkdir -p $(@D)
	RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=$(BUILD)/cargo $(CARGO) build --release \
		-Z build-std=core --target aarch64-unknown-none-softfloat --features installer $(BROWSER_FEATURE) $(BROWSER_INSTALL_FEATURE)
	cp $(BUILD)/cargo/aarch64-unknown-none-softfloat/release/libinfinity_kernel.a $(BUILD)/aarch64/libkernel.a
	touch $@

$(BUILD)/aarch64/kernel.elf: $(BUILD)/aarch64/kernel.stamp linker/aarch64.ld $(BUILD)/aarch64/qwen-math.o $(BUILD)/voice-kokoro/aarch64/private-native.o
	$(LD_LLD) -nostdlib -static $(BROWSER_LINK) -T linker/aarch64.ld -o $@ $(BUILD)/aarch64/libkernel.a $(BUILD)/aarch64/qwen-math.o $(BUILD)/voice-kokoro/aarch64/private-native.o $(BROWSER_ARM)

$(BUILD)/aarch64/kernel-qemu.elf: $(QEMU_KERNEL_STAMP) linker/aarch64-qemu.ld $(BUILD)/aarch64/qwen-math.o $(BUILD)/voice-kokoro/aarch64/private-native.o
	$(LD_LLD) -nostdlib -static $(BROWSER_LINK) -T linker/aarch64-qemu.ld -o $@ $(QEMU_KERNEL_LIBRARY) $(BUILD)/aarch64/qwen-math.o $(BUILD)/voice-kokoro/aarch64/private-native.o $(BROWSER_ARM)

$(BUILD)/aarch64/kernel-browser-qemu.stamp: $(KERNEL_SOURCES) $(SPLASH_ASSET) $(ICON_RUNTIME_ASSETS)

$(BUILD)/aarch64/loader.obj: boot/common/uefi_loader.c boot/common/boot_info.h boot/common/video_modes.h boot/common/tpm_random.h boot/common/payload_loader.h boot/common/worker_bridge.h boot/common/psci_workers.h
	@mkdir -p $(@D)
	$(CLANG) --target=aarch64-pc-windows-msvc -DINFINITY_AARCH64 -ffreestanding -fshort-wchar \
		-fno-stack-protector -fno-builtin -O2 -Wall -Wextra -Werror -c $< -o $@

$(BUILD)/aarch64/handoff.obj: boot/aarch64/handoff.S
	@mkdir -p $(@D)
	$(CLANG) --target=aarch64-pc-windows-msvc -c $< -o $@

$(BUILD)/aarch64/BOOTAA64.EFI: $(BUILD)/aarch64/loader.obj $(BUILD)/aarch64/handoff.obj
	$(LLD_LINK) /subsystem:efi_application /entry:efi_main /nodefaultlib /machine:arm64 /out:$@ $^

$(BUILD)/infinity-aarch64.img: $(BUILD)/aarch64/BOOTAA64.EFI $(BUILD)/aarch64/kernel.elf $(FONT_ASSETS) $(UI_ASSETS) $(INSTALLER_UI_ASSETS) $(INSTALLER_IMAGE_ASSETS) $(CRASH_ASSETS) $(NODE_ASSETS)
	rm -rf $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Icons $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Wallpapers $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Crash
	@mkdir -p $(BUILD)/fat-aarch64/EFI/BOOT $(BUILD)/fat-aarch64/EFI/INFINITY/FONTS $(BUILD)/fat-aarch64/EFI/INFINITY/FONT-LICENSES $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Wallpapers $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Installer $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Crash $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Mesh
	cp $(BUILD)/aarch64/BOOTAA64.EFI $(BUILD)/fat-aarch64/EFI/BOOT/BOOTAA64.EFI
	cp $(BUILD)/aarch64/kernel.elf $(BUILD)/fat-aarch64/EFI/INFINITY/KERNEL.ELF
	rm -rf $(BUILD)/fat-aarch64/EFI/INFINITY/PAYLOAD
	$(BROWSER_STAGE_ARM)
	cp assets/fonts/*.ttf $(BUILD)/fat-aarch64/EFI/INFINITY/FONTS/
	cp assets/fonts/OFL-*.txt $(BUILD)/fat-aarch64/EFI/INFINITY/FONT-LICENSES/
	cp -R assets/skins/. $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/
	cp $(WALLPAPER_ASSETS) $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Wallpapers/
	cp $(INSTALLER_UI_ASSETS) $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Installer/
	cp -R $(INSTALLER_IMAGE_ASSET_DIR) $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Installer/
	cp $(CRASH_ASSETS) $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Crash/
	cp $(NODE_ASSETS) $(BUILD)/fat-aarch64/EFI/INFINITY/INFINITYUI/Mesh/
	python3 tools/iso-staging.py allocate $(BUILD)/fat-aarch64 $@
	mformat -F -i $@ ::
	mcopy -i $@ -s $(BUILD)/fat-aarch64/EFI ::

$(BUILD)/test-media/InfinityOS-aarch64.bootmedia: $(BUILD)/infinity-aarch64.img
	@mkdir -p $(BUILD)/test-media
	@mkdir -p $(BUILD)/iso-aarch64/EFI
	cp $< $(BUILD)/iso-aarch64/efi.img
	rm -rf $(BUILD)/iso-aarch64/EFI/INFINITY/PAYLOAD
	cp -R $(BUILD)/fat-aarch64/EFI/BOOT $(BUILD)/fat-aarch64/EFI/INFINITY $(BUILD)/iso-aarch64/EFI/
	@test "$(INFINITY_ISO_BUILD_AUTHORITY)" = build.sh || { echo "ERROR: boot media creation is restricted to ./build.sh" >&2; exit 2; }
	xorriso -as mkisofs -R -V INFINITYOS_ARM64 -e efi.img -no-emul-boot -o $@.partial $(BUILD)/iso-aarch64
	mv $@.partial $@

$(BUILD)/infinity-aarch64-qemu.img: $(BUILD)/aarch64/BOOTAA64.EFI $(BUILD)/aarch64/kernel-qemu.elf $(FONT_ASSETS) $(UI_ASSETS) $(INSTALLER_UI_ASSETS) $(INSTALLER_IMAGE_ASSETS) $(CRASH_ASSETS) $(NODE_ASSETS)
	rm -rf $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Icons $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Wallpapers $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Crash
	@mkdir -p $(BUILD)/fat-aarch64-qemu/EFI/BOOT $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/FONTS $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/FONT-LICENSES $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Wallpapers $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Installer $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Crash $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Mesh
	cp $(BUILD)/aarch64/BOOTAA64.EFI $(BUILD)/fat-aarch64-qemu/EFI/BOOT/BOOTAA64.EFI
	cp $(BUILD)/aarch64/kernel-qemu.elf $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/KERNEL.ELF
	rm -rf $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/PAYLOAD
	$(BROWSER_STAGE_QEMU)
	cp assets/fonts/*.ttf $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/FONTS/
	cp assets/fonts/OFL-*.txt $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/FONT-LICENSES/
	cp -R assets/skins/. $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/
	cp $(WALLPAPER_ASSETS) $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Wallpapers/
	cp $(INSTALLER_UI_ASSETS) $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Installer/
	cp -R $(INSTALLER_IMAGE_ASSET_DIR) $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Installer/
	cp $(CRASH_ASSETS) $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Crash/
	cp $(NODE_ASSETS) $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/INFINITYUI/Mesh/
	python3 tools/iso-staging.py allocate $(BUILD)/fat-aarch64-qemu $@
	mformat -F -i $@ ::
	mcopy -i $@ -s $(BUILD)/fat-aarch64-qemu/EFI ::

$(BUILD)/test-media/InfinityOS-aarch64-qemu.bootmedia: $(BUILD)/infinity-aarch64-qemu.img
	@mkdir -p $(BUILD)/test-media
	@mkdir -p $(BUILD)/iso-aarch64-qemu/EFI
	cp $< $(BUILD)/iso-aarch64-qemu/efi.img
	rm -rf $(BUILD)/iso-aarch64-qemu/EFI/INFINITY/PAYLOAD
	cp -R $(BUILD)/fat-aarch64-qemu/EFI/BOOT $(BUILD)/fat-aarch64-qemu/EFI/INFINITY $(BUILD)/iso-aarch64-qemu/EFI/
	@test "$(INFINITY_ISO_BUILD_AUTHORITY)" = build.sh || { echo "ERROR: boot media creation is restricted to ./build.sh" >&2; exit 2; }
	xorriso -as mkisofs -R -V INFINITYOS_ARM64 -e efi.img -no-emul-boot -o $@.partial $(BUILD)/iso-aarch64-qemu
	mv $@.partial $@

.PHONY: aarch64-bootstrap
aarch64-bootstrap: check-tools $(BUILD)/test-media/InfinityOS-aarch64.bootmedia $(BUILD)/test-media/InfinityOS-aarch64-qemu.bootmedia
	@echo "Built ARM64 internal boot media under build/test-media"

aarch64: aarch64-bootstrap
	@echo "Built AArch64 internal boot media; use ./build.sh --target aarch64 to publish an ISO"

run-x86: x86
	qemu-system-i386 -machine pc -m 128M -cdrom $(BUILD)/test-media/InfinityOS-x86.bootmedia \
		-boot d -serial stdio -display none -no-reboot
run-aarch64: aarch64
	$(QEMU_AARCH64) -machine virt -cpu cortex-a72 -m 4096M -bios $(AAVMF_CODE) \
		-device ramfb -device virtio-scsi-pci -drive if=none,id=cd,format=raw,media=cdrom,file=$(BUILD)/test-media/InfinityOS-aarch64-qemu.bootmedia \
		-device scsi-cd,drive=cd,bootindex=0 -serial stdio -display none -no-reboot

clean:
	python3 tools/build-workspace.py clean
