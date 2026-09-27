# Experimental until installed browser acceptance passes. Both live and installed
# kernels always select the same mode; configuration changes invalidate stamps.
NATIVE_BROWSER ?= 0
BROWSER_SOURCES := $(shell find sdk/infinity-browser-core sdk/infinity-browser-servo sdk/servo-runtime-primitives sdk/servo-std tools/servo-platform-probe -type d \( -name target -o -name __pycache__ \) -prune -o -type f -print) $(wildcard assets/apps/infinity-browser-*) tools/browser-build.mk
.PHONY: browser-mode-check
$(BUILD)/browser-mode: browser-mode-check
	@python3 tools/browser-build-mode.py $(NATIVE_BROWSER)

ifeq ($(NATIVE_BROWSER),1)
BROWSER_FEATURE := --features native-browser
BROWSER_INSTALL_FEATURE := --features browser-installer-payload
BROWSER_STAGE_ARM = cp -R $(BUILD)/browser-payload/aarch64/PAYLOAD $(BUILD)/fat-aarch64/EFI/INFINITY/
BROWSER_STAGE_QEMU = cp -R $(BUILD)/browser-payload/aarch64/PAYLOAD $(BUILD)/fat-aarch64-qemu/EFI/INFINITY/
BROWSER_STAGE_X86 = cp -R $(BUILD)/browser-payload/x86_64/PAYLOAD $(BUILD)/fat/EFI/INFINITY/
BROWSER_LINK := --strip-debug --undefined=infinity_browser_private_infinity_browser_run
BROWSER_ARM := $(BUILD)/servo-platform-probe/browser-private-aarch64.o
BROWSER_X86 := $(BUILD)/servo-platform-probe/browser-private-x86_64.o
$(BROWSER_ARM): $(BROWSER_SOURCES)
	python3 tools/servo-platform-probe/link-engine.py --arch aarch64 --component
$(BROWSER_X86): $(BROWSER_SOURCES)
	python3 tools/servo-platform-probe/link-engine.py --arch x86_64 --component
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
