"""Settings acceptance on a fresh disposable install, using real pointer events and read-only state."""
import importlib.util
import argparse
import json
import mmap
import pathlib
import shutil
import struct
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("editor_acceptance", ROOT / "tools/editor-assistant-installed-test.py")
helpers = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helpers)
base = helpers.base
native_spec = importlib.util.spec_from_file_location("settings_native_guest", ROOT / "tools/native-editor-installed-test.py")
native = importlib.util.module_from_spec(native_spec)
native_spec.loader.exec_module(native)
helpers.click = native.adaptive_click
base.symbol = native.base.symbol

# ------------------------=
# FUNC: settings
# DESC: Reads coherent non-secret Settings state without injecting input or modifying guest memory.
# ------------------=
def settings(guest):
    elf = guest.work.parent / "artifacts" / "installed-kernel.elf"
    address, size = base.symbol(elf, "INFINITY_SETTINGS_DIAGNOSTIC_SNAPSHOT")
    assert size == 1024
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        values = struct.unpack("<128Q", guest.memory(address, size))
        if values[0:2] == (0x494e465345545331, 1) and values[2] == values[127] and not values[2] & 1:
            return values
        time.sleep(.1)
    raise AssertionError("Settings snapshot did not become coherent")

# ------------------------=
# FUNC: wait_settings
# DESC: Requires a bounded, observable live Settings state transition following real input.
# ------------------=
def wait_settings(guest, predicate):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        state = settings(guest)
        if predicate(state):
            return state
        time.sleep(.1)
    guest.screenshot("settings-failure")
    raise AssertionError(state)

# ------------------------=
# FUNC: disclosure
# DESC: Clicks the visible disclosure, checks the resulting state, and verifies subsequent rows clear the detail well.
# ------------------=
def disclosure(guest, index, opening):
    state = settings(guest)
    x, y, w, h = state[29 + index * 8:33 + index * 8]
    vx, vy, vw, vh = state[13:17]
    assert vx <= x and x + w <= vx + vw and vy <= y and y + h <= vy + vh
    helpers.click(guest, x + w // 2, y + h // 2)
    state = wait_settings(guest, lambda s: s[5] == (index + 1 if opening else 0))
    if opening and index + 1 < state[8]:
        assert state[112] + state[114] <= state[26 + (index + 1) * 8]
    return state

# ------------------------=
# FUNC: verify_artifacts
# DESC: Verifies the exact Settings template and diagnostic contract are embedded in live and installed kernels on both architectures.
# ------------------=
def verify_artifacts():
    template = (ROOT / "assets/boot/settings-screens.iuit").read_bytes()
    for architecture in ("x86_64", "aarch64"):
        for name in ("kernel.elf", "installed-kernel.elf"):
            path = ROOT / "build" / architecture / name
            assert base.symbol(path, "INFINITY_SETTINGS_DIAGNOSTIC_SNAPSHOT")[1] == 1024
            with path.open("rb") as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as binary:
                assert binary.find(template) >= 0, (architecture, name)

# ------------------------=
# FUNC: review_sections
# DESC: Visits every real Settings category through pointer input and captures the production rendered result.
# ------------------=
def review_sections(guest):
    reviewed = []
    for section in range(11):
        state = settings(guest)
        helpers.click(guest, state[89 + section * 2], state[90 + section * 2])
        state = wait_settings(guest, lambda s: s[3] == 1 and s[4] == section)
        assert state[13] >= state[9] and state[13] + state[15] <= state[9] + state[11]
        assert state[14] >= state[10] and state[14] + state[16] <= state[10] + state[12]
        guest.screenshot(f"settings-category-{section:02d}")
        if section not in (0, 1, 3, 6, 7, 10):
            disclosure(guest, 0, True)
            guest.screenshot(f"settings-category-{section:02d}-expanded")
            disclosure(guest, 0, False)
        reviewed.append(section)
    return reviewed

# ------------------------=
# FUNC: verify_voice_controls
# DESC: Uses native keyboard reveal and real pointer clicks to verify five circular voices and a reversible speech checkbox.
# ------------------=
def verify_voice_controls(guest):
    state = settings(guest)
    helpers.click(guest, state[95], state[96])
    wait_settings(guest, lambda s: s[4] == 3)
    guest.key("tab")
    guest.key("end")
    state = wait_settings(guest, lambda s: s[118] == 135)
    original = state[115]
    for step in range(1, 6):
        x, y, w, h = state[81:85]
        vx, vy, vw, vh = state[13:17]
        assert vx <= x and y >= vy and y + h <= vy + vh
        helpers.click(guest, x + w // 2, y + h // 2)
        state = wait_settings(guest, lambda s: s[115] == (original + step) % 5 and s[5] == 0)
    guest.key("up")
    state = wait_settings(guest, lambda s: s[118] == 134)
    original_speech = state[119]
    for expected in (1 - original_speech, original_speech):
        x, y, w, h = state[73:77]
        helpers.click(guest, x + w // 2, y + h // 2)
        state = wait_settings(guest, lambda s: s[119] == expected and s[5] == 0)
    guest.screenshot("settings-voice-controls")

# ------------------------=
# FUNC: main
# DESC: Installs the current ISO, removes installation media, and checks native expanders, shifted click targets, scrolling and screenshots.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("work", type=pathlib.Path)
    parser.add_argument("--arch", choices=("x86_64", "aarch64"), default="x86_64")
    parser.add_argument("--iso", type=pathlib.Path)
    parser.add_argument("--review-existing", action="store_true")
    parser.add_argument("--resume-onboarding", action="store_true")
    parser.add_argument("--accel", choices=("hvf", "tcg"))
    args = parser.parse_args()
    if args.arch == "aarch64" and args.iso is None:
        parser.error("ARM QEMU requires explicit QEMU-linked --iso media")
    verify_artifacts()
    work = args.work.resolve()
    work.relative_to(ROOT / "build")
    if args.review_existing:
        guest = base.Guest(work, 1, "/opt/homebrew/share/qemu/edk2-x86_64-code.fd",
                           reuse=True, width=1920, height=1080)
        try:
            guest.boot(False)
            guest.authenticate()
            guest.launch("settings", 8)
            reviewed = review_sections(guest)
            (work / "category-review.json").write_text(json.dumps({
                "iso_detached": True, "resolution": list(guest.state()[11:13]),
                "category_navigation": reviewed,
                "note": "Uses the retained installed fixture and its archived kernel, not a new install."}, indent=2))
        finally:
            guest.stop()
        return
    work.mkdir(exist_ok=args.resume_onboarding)
    artifacts = work / "artifacts"
    artifacts.mkdir(exist_ok=args.resume_onboarding)
    media = (args.iso or ROOT / f"builds/InfinityOS-{args.arch}.iso").resolve()
    media.relative_to(ROOT)
    if not args.resume_onboarding:
        shutil.copyfile(media, artifacts / "installer.iso")
    packaged = native.desktop.extract_media_kernels(artifacts / "installer.iso", artifacts, args.arch)
    guest = native.NativeGuest(work, 1, f"/opt/homebrew/share/qemu/edk2-{args.arch}-code.fd",
                               width=1920, height=1080, memory_mb=12288, reuse=args.resume_onboarding)
    guest.arch = args.arch
    guest.acceleration = args.accel or ("hvf" if args.arch == "aarch64" else "tcg")
    guest.patched = True
    guest.boot_timeout_seconds = 600
    try:
        print("Fresh Settings installation", flush=True)
        if not args.resume_onboarding:
            guest.install()
            guest.stop()
        identity = guest.onboard()
        guest.launch("settings", 8)
        state = wait_settings(guest, lambda s: s[3] == 1 and s[4] == 0 and s[5] == 0)
        guest.screenshot("settings-general-collapsed")
        disclosure(guest, 1, True)
        guest.screenshot("settings-general-expanded")
        # The second row has moved. Its actual arrow must still work, replacing the open well.
        disclosure(guest, 2, True)
        disclosure(guest, 2, False)
        state = settings(guest)
        helpers.click(guest, state[91], state[92])
        wait_settings(guest, lambda s: s[4] == 1 and s[5] == 0)
        disclosure(guest, 6, True)
        guest.screenshot("settings-theme-color-expanded")
        state = settings(guest)
        assert state[7] > 0
        x, y, w, h = state[17:21]
        helpers.click(guest, x + w // 2, y + h - 3)
        wait_settings(guest, lambda s: s[6] > 0)
        guest.screenshot("settings-theme-scrolled")
        disclosure(guest, 7, True)
        disclosure(guest, 7, False)
        state = settings(guest)
        helpers.click(guest, state[97], state[98])
        wait_settings(guest, lambda s: s[4] == 4 and s[5] == 0)
        disclosure(guest, 0, True)
        guest.screenshot("settings-privacy-timeout")
        reviewed = review_sections(guest)
        verify_voice_controls(guest)
        (work / "result.json").write_text(json.dumps({"fresh_install": True, "iso_detached": True,
            "architecture": args.arch, "source_media": str(media), "packaged": packaged,
            "identity": identity, "disclosures_toggle": True, "moved_row_click": True,
            "scroll_and_lower_row_click": True, "detail_nonoverlap": True,
            "saved_template_in_both_architectures": True,
            "voice_choice_five_step_wrap": True, "speech_checkbox_round_trip": True,
            "keyboard_row_focus": True,
            "category_navigation": reviewed}, indent=2))
        print("Installed Settings behavior passed", flush=True)
    finally:
        guest.stop()

if __name__ == "__main__":
    main()
