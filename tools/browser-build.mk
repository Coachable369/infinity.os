# Experimental until installed browser acceptance passes. Both live and installed
# kernels always select the same mode; configuration changes invalidate stamps.
NATIVE_BROWSER ?= 0
BROWSER_SOURCES := $(shell find sdk/infinity-browser-core sdk/infinity-browser-servo sdk/servo-runtime-primitives sdk/servo-std tools/servo-platform-probe -type d \( -name target -o -name __pycache__ \) -prune -o -type f -print) $(wildcard assets/apps/infinity-browser-*) tools/browser-build.mk
.PHONY: browser-mode-check
$(BUILD)/browser-mode: browser-mode-check
	@python3 tools/browser-build-mode.py $(NATIVE_BROWSER)

ifeq ($(NATIVE_BROWSER),1)
BROWSER_FEATURE := --features native-browser
BROWSER_LINK := --strip-debug --undefined=infinity_browser_private_infinity_browser_run
BROWSER_ARM := $(BUILD)/servo-platform-probe/browser-private-aarch64.o
BROWSER_X86 := $(BUILD)/servo-platform-probe/browser-private-x86_64.o
$(BROWSER_ARM): $(BROWSER_SOURCES)
	python3 tools/servo-platform-probe/link-engine.py --arch aarch64 --component
$(BROWSER_X86): $(BROWSER_SOURCES)
	python3 tools/servo-platform-probe/link-engine.py --arch x86_64 --component
endif

$(BUILD)/aarch64/installed-kernel.stamp $(BUILD)/aarch64/kernel.stamp $(BUILD)/x86_64/installed-kernel.o $(BUILD)/x86_64/kernel.o: $(BUILD)/browser-mode $(BROWSER_SOURCES)
$(BUILD)/aarch64/installed-kernel.elf $(BUILD)/aarch64/kernel.elf $(BUILD)/aarch64/kernel-qemu.elf: $(BROWSER_ARM)
$(BUILD)/x86_64/installed-kernel.elf $(BUILD)/x86_64/kernel.elf: $(BROWSER_X86)
