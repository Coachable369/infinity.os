"""Real save / Documents / Open With acceptance on a disposable installed system."""
import importlib.util
import json
import pathlib
import shutil
import struct
import sys
import time
import zlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("editor_acceptance", ROOT / "tools/editor-assistant-installed-test.py")
helpers = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helpers)
base = helpers.base
DOCUMENTS = b"/home/default/documents"
FILE = DOCUMENTS + b"/helloworld.txt"

# ------------------------=
# FUNC: navigator
# DESC: Reads coherent structured state without injecting guest memory or parsing rendered text.
# ------------------=
def navigator(guest):
    address, size = base.symbol(guest.work.parent / "artifacts/installed-kernel.elf", "INFINITY_NAVIGATOR_DIAGNOSTIC_SNAPSHOT")
    assert size == 512
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        state = struct.unpack("<64Q", guest.memory(address, size))
        if state[:2] == (0x494e464e41563131, 1) and state[2] == state[63] and not state[2] & 1:
            return state
        time.sleep(.1)
    raise AssertionError("Incoherent Navigator state")

# ------------------------=
# FUNC: wait_navigator
# DESC: Requires a bounded observable picker or navigator state transition.
# ------------------=
def wait_navigator(guest, predicate):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        state = navigator(guest)
        if predicate(state):
            return state
        time.sleep(.2)
    raise AssertionError(state)

# ------------------------=
# FUNC: documents
# DESC: Opens File Navigator and clicks its real Documents favorite.
# ------------------=
def documents(guest):
    display = guest.state()
    width, height = display[11:13]
    scale = 2 if width >= 2560 and height >= 1440 else 1
    dock_width = width * 54 // 100
    # File Navigator is the second entry in the persistent eight-item native dock.
    helpers.click(guest, (width - dock_width) // 2 + (dock_width // 8) * 3 // 2,
                  height - 46 * scale)
    state = wait_navigator(guest, lambda s: s[3])
    x, y, w, h = state[4:8]
    scale = state[8]
    helpers.click(guest, x + 80 * scale, y + 154 * scale)
    return wait_navigator(guest, lambda s: s[14] == zlib.crc32(DOCUMENTS))

# ------------------------=
# FUNC: open_with
# DESC: Right-clicks the saved row, chooses Open With, and invokes the real Text Editor app.
# ------------------=
def open_with(guest, expected_hash, capture):
    state = documents(guest)
    assert state[15] == 1, state
    guest.screenshot(capture + "-documents")
    x, y, w, h = state[4:8]
    scale = state[8]
    helpers.click(guest, x + w * 27 // 100 + 55 * scale, y + 168 * scale, button="right")
    state = wait_navigator(guest, lambda s: s[17] and s[16] == 2 and s[27] == zlib.crc32(FILE))
    identity = state[28:30]
    assert any(identity) and state[19] == 1
    guest.screenshot(capture + "-context")
    mx, my, mw, mh = state[23:27]
    helpers.click(guest, mx + 70 * scale, my + 48 * scale)
    state = wait_navigator(guest, lambda s: s[18] == 1)
    guest.screenshot(capture + "-open-with")
    mx, my, mw, mh = state[23:27]
    helpers.click(guest, mx + 70 * scale, my + 20 * scale)
    helpers.wait_feature(guest, lambda s: s[12] == 2 and s[11] and s[16] == expected_hash)
    wait_navigator(guest, lambda s: s[13] == zlib.crc32(FILE) and not s[17])
    guest.screenshot(capture + "-opened")
    return identity

# ------------------------=
# FUNC: main
# DESC: Installs the new ISO, verifies save/browse/open, then repeats on a cold boot with no ISO attached.
# ------------------=
def main():
    work = pathlib.Path(sys.argv[1]).resolve()
    resume = sys.argv[2:] == ["--resume"]
    if not resume:
        work.mkdir(exist_ok=False)
    artifacts = work / "artifacts"
    if not resume:
        artifacts.mkdir()
        for source, name in [("build/infinity-x86_64.iso", "installer.iso"), ("build/x86_64/kernel.elf", "kernel.elf"), ("build/x86_64/installed-kernel.elf", "installed-kernel.elf")]:
            shutil.copyfile(ROOT / source, artifacts / name)
    for arch, machine in (("x86_64", 62), ("aarch64", 183)):
        for kind in ("kernel.elf", "installed-kernel.elf"):
            with (ROOT / "build" / arch / kind).open("rb") as stream:
                header = stream.read(64)
            assert header[:6] == b"\x7fELF\x02\x01" and struct.unpack_from("<H", header, 18)[0] == machine
    guest = base.Guest(work, 1, "/opt/homebrew/share/qemu/edk2-x86_64-code.fd", reuse=resume)
    try:
        if resume:
            guest.boot(False)
            guest.authenticate()
            expected = 1469598103934665603
            for byte in b"Hello world":
                expected = ((expected ^ byte) * 1099511628211) & ((1 << 64) - 1)
            file_id = open_with(guest, expected, "resumed")
            guest.stop()
            guest.boot(False)
            guest.authenticate()
            assert open_with(guest, expected, "cold-boot") == file_id
            (work / "result.json").write_text(json.dumps({"resumed_disposable_install": True,
                "iso_detached": True, "navigator_lists_saved_file": True, "right_click_open_with": True,
                "exact_content_reopened": True, "cold_boot_same_object": True}, indent=2))
            return
        print("Installing to disposable disk", flush=True)
        guest.install()
        guest.stop()
        identity = guest.onboard()
        guest.launch("text", 5)
        helpers.wait_feature(guest, lambda s: s[12] == 2 and s[7] == 0)
        helpers.text(guest, "Hello world")
        expected = 1469598103934665603
        for byte in b"Hello world":
            expected = ((expected ^ byte) * 1099511628211) & ((1 << 64) - 1)
        helpers.wait_feature(guest, lambda s: s[7] == 11 and s[16] == expected)
        guest.key("ctrl", "s")
        wait_navigator(guest, lambda s: s[9] == 1 and s[12] == zlib.crc32(DOCUMENTS))
        helpers.text(guest, "helloworld.txt")
        guest.key("ret")
        wait_navigator(guest, lambda s: s[9] == 0 and s[13] == zlib.crc32(FILE))
        helpers.wait_feature(guest, lambda s: s[11])
        guest.screenshot("saved-in-documents")
        guest.key("ctrl", "n")
        helpers.wait_feature(guest, lambda s: s[7] == 0)
        file_id = open_with(guest, expected, "first")
        print("Save / browse / right-click Open With passed; checking cold boot", flush=True)
        guest.stop()
        guest.boot(False)
        guest.authenticate()
        assert open_with(guest, expected, "cold-boot") == file_id
        result = {"fresh_install": True, "iso_detached": True, "save_to_documents": True,
                  "navigator_lists_saved_file": True, "right_click_open_with": True,
                  "exact_content_reopened": True, "cold_boot_same_object": True,
                  "both_architectures_built": True, "node_identity": identity}
        (work / "result.json").write_text(json.dumps(result, indent=2))
        print("Installed Documents acceptance passed", flush=True)
    finally:
        guest.stop()

if __name__ == "__main__":
    main()
