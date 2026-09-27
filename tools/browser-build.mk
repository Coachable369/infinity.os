# Experimental until installed browser acceptance passes. Both live and installed
# kernels always select the same mode; configuration changes invalidate stamps.
NATIVE_BROWSER ?= 0
QEMU_KERNEL_STAMP = $(BUILD)/aarch64/kernel.stamp
QEMU_KERNEL_LIBRARY = $(BUILD)/aarch64/libkernel.a
BROWSER_SOURCES := $(shell find sdk/infinity-browser-core sdk/infinity-browser-servo sdk/servo-runtime-primitives sdk/servo-std tools/servo-platform-probe -type d \( -name target -o -name __pycache__ \) -prune -o -type f -print) $(wildcard assets/apps/infinity-browser-*) tools/browser-build.mk
.PHONY: browser-mode-check
$(BUILD)/browser-mode: browser-mode-check
	@python3 tools/browser-build-mode.py $(NATIVE_BROWSER)

ifeq ($(NATIVE_BROWSER),1)
BROWSER_FEATURE := --features native-browser
BROWSER_INSTALL_FEATURE := --features browser-installer-payload
BROWSER_STAGE_ARM = cp -R $(BUILD)/browser-payload/aarch64/PAYLOAD $(BUILD)/fat-aarch64/EFI/INFINITY/
BROWSER_STAGE_QEMU = cp -R $(BUILD)/browser-payload/aarch64-qemu/PAYLOAD $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/
BROWSER_STAGE_X86 = cp -R $(BUILD)/browser-payload/x86_64/PAYLOAD $(BUILD)/fat/EFI/INFINITY/
BROWSER_LINK := --strip-debug --undefined=infinity_browser_private_infinity_browser_run
BROWSER_ARM := $(BUILD)/servo-platform-probe/browser-private-aarch64.o
BROWSER_X86 := $(BUILD)/servo-platform-probe/browser-private-x86_64.o
QEMU_KERNEL_STAMP = $(BUILD)/aarch64/kernel-browser-qemu.stamp
QEMU_KERNEL_LIBRARY = $(BUILD)/aarch64/libkernel-browser-qemu.a

# One shared installed library, linked at the selected machine's native RAM
# address. QEMU media installs this exact variant without an offline patch.
$(BUILD)/aarch64/installed-kernel-qemu.elf: $(BUILD)/aarch64/installed-kernel.stamp linker/aarch64-qemu.ld $(BUILD)/aarch64/qwen-math.o $(BUILD)/voice-kokoro/aarch64/private-native.o $(BUILD)/voice-pocketsphinx-arm/private-native.o $(BROWSER_ARM)
	$(LD_LLD) -nostdlib -static $(BROWSER_LINK) -T linker/aarch64-qemu.ld -o $@ $(BUILD)/aarch64/libinstalled-kernel.a $(BUILD)/aarch64/qwen-math.o $(BUILD)/voice-kokoro/aarch64/private-native.o $(BUILD)/voice-pocketsphinx-arm/private-native.o $(BROWSER_ARM)

$(BUILD)/browser-payload/aarch64-qemu/manifest.rs: $(BUILD)/aarch64/installed-kernel-qemu.elf $(BUILD)/aarch64/installed-esp.img tools/qwen-pack.rs tools/browser-build.mk
	rm -rf $(BUILD)/browser-payload/aarch64-qemu/PAYLOAD
	@mkdir -p $(@D)/PAYLOAD
	CARGO_TARGET_DIR=$(BUILD)/behavior-harness $(CARGO) run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin qwen-pack -- install $(BUILD)/aarch64/installed-esp.img $(BUILD)/aarch64/installed-kernel-qemu.elf $(@D)/PAYLOAD $@.partial
	cat $(@D)/PAYLOAD/P1-*.BIN | cmp - $(BUILD)/aarch64/installed-kernel-qemu.elf
	cat $(@D)/PAYLOAD/P0-*.BIN | cmp - $(BUILD)/aarch64/installed-esp.img
	mv $@.partial $@

$(BUILD)/aarch64/kernel-browser-qemu.stamp: $(BUILD)/browser-mode $(BROWSER_SOURCES) $(BUILD)/browser-payload/aarch64-qemu/manifest.rs
	RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=$(BUILD)/cargo-browser-qemu $(CARGO) build --release -Z build-std=core --target aarch64-unknown-none-softfloat --features installer,native-browser,browser-qemu-payload
	cp $(BUILD)/cargo-browser-qemu/aarch64-unknown-none-softfloat/release/libinfinity_kernel.a $(QEMU_KERNEL_LIBRARY)
	touch $@
$(BROWSER_ARM): $(BROWSER_SOURCES) $(BUILD)/voice-kokoro/aarch64/private-native.o
	python3 tools/servo-platform-probe/build-component.py --arch aarch64
$(BROWSER_X86): $(BROWSER_SOURCES) $(BUILD)/voice-kokoro/x86_64/private-native.o
	python3 tools/servo-platform-probe/build-component.py --arch x86_64
$(BUILD)/browser-payload/%/manifest.rs: $(BUILD)/%/installed-kernel.elf $(BUILD)/%/installed-esp.img tools/qwen-pack.rs tools/browser-build.mk
	rm -rf $(@D)/PAYLOAD
	@mkdir -p $(@D)/PAYLOAD
	CARGO_TARGET_DIR=$(BUILD)/behavior-harness $(CARGO) run --quiet --release --manifest-path tools/behavior-harness/Cargo.toml --bin qwen-pack -- install $(BUILD)/$*/installed-esp.img $(BUILD)/$*/installed-kernel.elf $(@D)/PAYLOAD $@.partial
	cat $(@D)/PAYLOAD/P1-*.BIN | cmp - $(BUILD)/$*/installed-kernel.elf
	cat $(@D)/PAYLOAD/P0-*.BIN | cmp - $(BUILD)/$*/installed-esp.img
	mv $@.partial $@
$(BUILD)/aarch64/kernel.stamp: $(BUILD)/browser-payload/aarch64/manifest.rs
$(BUILD)/x86_64/kernel.o: $(BUILD)/browser-payload/x86_64/manifest.rs
endif

$(BUILD)/aarch64/installed-kernel.stamp $(BUILD)/aarch64/kernel.stamp $(BUILD)/x86_64/installed-kernel.o $(BUILD)/x86_64/kernel.o: $(BUILD)/browser-mode $(BROWSER_SOURCES)
$(BUILD)/aarch64/installed-kernel.elf $(BUILD)/aarch64/kernel.elf $(BUILD)/aarch64/kernel-qemu.elf: $(BROWSER_ARM)
$(BUILD)/x86_64/installed-kernel.elf $(BUILD)/x86_64/kernel.elf: $(BROWSER_X86)
