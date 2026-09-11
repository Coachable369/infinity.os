"""Fresh-install editor/assistant acceptance using real QEMU input and typed read-only state."""
import importlib.util
import json
import mmap
import pathlib
import shutil
import struct
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("installed_guest", ROOT / "tools/ms9-installed-acceptance.py")
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)

# ------------------------=
# FUNC: verify_artifacts
# DESC: Verifies the actual code-font bytes and typed feature contract are packaged in both live and installed architectures.
# ------------------=
def verify_artifacts():
    font = (ROOT / "assets/fonts/InfinityEditor-Mono-14.atlas").read_bytes()
    ui_fonts = [(ROOT / "assets/fonts" / name).read_bytes() for name in ("InfinityUI-Regular-16.atlas", "InfinityUI-Semibold-16.atlas")]
    assert len(font) == 14 * 20 * 95 and any(font)
    for architecture in ("x86_64", "aarch64"):
        for name in ("kernel.elf", "installed-kernel.elf"):
            path = ROOT / "build" / architecture / name
            _, size = base.symbol(path, "INFINITY_APP_FEATURE_SNAPSHOT")
            assert size == 256
            with path.open("rb") as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as binary:
                assert binary.find(font) >= 0, (architecture, name, "missing code font")
                assert all(binary.find(atlas) >= 0 for atlas in ui_fonts), (architecture, name, "missing compact UI typography")

# ------------------------=
# FUNC: features
# DESC: Reads the production app capability snapshot without inspecting rendered prose or injecting guest memory.
# ------------------=
def features(guest):
    elf = guest.work.parent / "artifacts" / ("kernel.elf" if guest.installer else "installed-kernel.elf")
    address, size = base.symbol(elf, "INFINITY_APP_FEATURE_SNAPSHOT")
    assert size == 256
    guest.qmp("stop")
    try:
        state = struct.unpack("<32Q", guest.memory(address, size))
    finally:
        guest.qmp("cont")
    assert state[0:2] == (0x494e464150505331, 1)
    assert state[2] == state[31] and state[2] % 2 == 0
    assert state[3:7] == (0x1ff, 16384, 8, 9)
    return state

# ------------------------=
# FUNC: wait_feature
# DESC: Waits for a structured application state transition with a fixed acceptance deadline.
# ------------------=
def wait_feature(guest, predicate):
    deadline = time.monotonic() + 30
    while True:
        state = features(guest)
        if predicate(state):
            return state
        assert time.monotonic() < deadline, state
        time.sleep(.2)

# ------------------------=
# FUNC: text
# DESC: Types through actual keyboard events, including punctuation used by the code fixture.
# ------------------=
def text(guest, value):
    aliases = {" ": "spc", "-": "minus", ".": "dot", "=": "equal", "/": "slash", ";": "semicolon", "\n": "ret", "\t": "tab", ",": "comma"}
    shifted = {"(": "9", ")": "0", "{": "bracket_left", "}": "bracket_right", '"': "apostrophe", "!": "1", ":": "semicolon"}
    for c in value:
        guest.key(*(("shift", shifted[c]) if c in shifted else ("shift", c.lower()) if c.isupper() else (aliases.get(c, c),)))

# ------------------------=
# FUNC: click
# DESC: Moves the real relative pointer to a pixel coordinate and verifies press/release processing.
# ------------------=
def click(guest, x, y, button="left"):
    display = guest.wait(lambda s: s[11] > 0 and s[12] > 0, "display geometry")
    target = (x * 1000 // display[11], y * 1000 // display[12])
    for _ in range(60):
        state = guest.state()
        delta = [target[0] - state[13], target[1] - state[14]]
        if all(abs(d) < 3 for d in delta):
            break
        events = [{"type": "rel", "data": {"axis": axis, "value": max(-60, min(60, int(d / 3) or (1 if d > 0 else -1)))}}
                  for axis, d in zip(("x", "y"), delta) if abs(d) >= 3]
        guest.qmp("input-send-event", {"events": events})
        guest.wait(lambda v: v[13:15] != state[13:15], "pointer movement")
    else:
        raise AssertionError("Pointer did not reach requested hit region")
    mask = {"left": 1, "right": 2}[button]
    guest.qmp("input-send-event", {"events": [{"type": "btn", "data": {"button": button, "down": True}}]})
    guest.wait(lambda s: s[15] & mask, "pressed")
    guest.qmp("input-send-event", {"events": [{"type": "btn", "data": {"button": button, "down": False}}]})
    guest.wait(lambda s: not s[15] & mask, "released")

# ------------------------=
# FUNC: panel_click
# DESC: Uses independently computed documented panel dimensions against the actual owner bounds.
# ------------------=
def panel_click(guest, control):
    p = features(guest)
    x, y, w, h = p[21:25]
    display = guest.wait(lambda s: s[11] > 0 and s[12] > 0, "panel display geometry")
    scale = 2 if display[11] >= 2560 and display[12] >= 1440 else 1
    width = min(max(w * 336 // 1000, 320 * scale), 480 * scale, w - 280 * scale)
    left, top, bottom = x + w - width - scale, y + 48 * scale, y + h - scale
    if control == "toggle":
        click(guest, x + w - 18 * scale, top + 24 * scale if p[13] else y + max(h // 3, 64 * scale) + 24 * scale)
    elif control == "apply":
        click(guest, left + 12 * scale + (width - 32 * scale) // 4, bottom - 80 * scale)
    elif control == "composer":
        click(guest, left + 40 * scale, bottom - 32 * scale)

# ------------------------=
# FUNC: main
# DESC: Installs to a new disposable disk, cold-boots without ISO, edits/saves with real input and verifies the universal panel.
# ------------------=
def main():
    verify_artifacts()
    if sys.argv[1:] == ["--artifacts-only"]:
        print("Both architectures package the editor font and app feature contract")
        return
    live_only = sys.argv[2:] == ["--live-only"]
    assert len(sys.argv) == 2 or live_only
    work = pathlib.Path(sys.argv[1]).resolve()
    work.mkdir(exist_ok=False)
    artifacts = work / "artifacts"
    artifacts.mkdir()
    for source, name in [("build/infinity-x86_64.iso", "installer.iso"), ("build/x86_64/kernel.elf", "kernel.elf"), ("build/x86_64/installed-kernel.elf", "installed-kernel.elf")]:
        shutil.copyfile(ROOT / source, artifacts / name)
    guest = base.Guest(work, 1, "/opt/homebrew/share/qemu/edk2-x86_64-code.fd", width=1920, height=1080)
    try:
        identity = None
        if live_only:
            print("Live-only UI check; this does not verify installation or saving", flush=True)
            guest.boot(True)
            guest.wait(lambda s: s[4] == 0 and s[3] == 0, "live startup")
            guest.text("3")
            guest.key("ret")
            guest.text("task launch editor")
            guest.key("ret")
        else:
            print("Installing fresh ISO", flush=True)
            guest.install()
            features(guest)
            guest.stop()
            print("Configuring and cold-booting installed disk without ISO", flush=True)
            identity = guest.onboard()
            guest.launch("text", 5)
        wait_feature(guest, lambda p: p[12] == 2)
        initial = wait_feature(guest, lambda p: p[7] == 0)
        x, y, w, h = initial[21:25]
        click(guest, x + 32, y + 66)
        wait_feature(guest, lambda p: p[25] == 1)
        guest.screenshot("editor-file-menu")
        guest.key("down")
        guest.key("ret")
        wait_feature(guest, lambda p: p[12] != 2)
        guest.key("esc")
        wait_feature(guest, lambda p: p[12] == 2)
        text(guest, "let answer = 42;")
        initial = wait_feature(guest, lambda p: p[7] == 16)
        click(guest, x + w - 100, y + h - 16)
        wait_feature(guest, lambda p: p[25] == 5)
        guest.screenshot("editor-syntax-dropdown")
        guest.key("down")
        guest.key("ret")
        wait_feature(guest, lambda p: p[20] == 1)
        guest.key("ctrl", "a")
        wait_feature(guest, lambda p: p[9:11] == (0, 16))
        guest.key("ctrl", "c")
        guest.key("ctrl", "x")
        wait_feature(guest, lambda p: p[7] == 0)
        guest.key("ctrl", "z")
        wait_feature(guest, lambda p: p[16] == initial[16])
        guest.key("ctrl", "y")
        wait_feature(guest, lambda p: p[7] == 0)
        guest.key("ctrl", "v")
        wait_feature(guest, lambda p: p[16] == initial[16])
        guest.key("ctrl", "f")
        text(guest, "answer")
        guest.key("ret")
        wait_feature(guest, lambda p: p[9:11] == (4, 10))
        guest.key("esc")
        guest.screenshot("editor-syntax-selection")
        closed = features(guest)
        panel_click(guest, "toggle")
        opened = wait_feature(guest, lambda p: p[13] and p[15])
        assert opened[27] < closed[27] and opened[9:11] == closed[9:11]
        text(guest, "insert VALUE")
        guest.key("ret")
        proposed = wait_feature(guest, lambda p: p[14] == 5)
        assert proposed[16] == initial[16]
        guest.screenshot("editor-ai-proposal")
        panel_click(guest, "apply")
        applied = wait_feature(guest, lambda p: p[7] == 15 and p[14] == 0)
        assert applied[16] != initial[16]
        guest.screenshot("editor-ai-applied")
        panel_click(guest, "toggle")
        saved = wait_feature(guest, lambda p: not p[13] and p[27] == closed[27])
        if not live_only:
            guest.key("ctrl", "s")
            text(guest, "assistant-proof.rs")
            guest.key("ret")
            saved = wait_feature(guest, lambda p: p[11] and p[12] == 2)
            assert saved[16] == applied[16]
            guest.screenshot("editor-saved")
        panel_click(guest, "toggle")
        text(guest, "help")
        guest.key("ret")
        wait_feature(guest, lambda p: p[19] > 0 and p[14] == 0)
        panel_click(guest, "toggle")
        guest.key("ctrl", "a")
        text(guest, 'fn main() {\n\tlet answer = 42;\nlet message = "Hello, InfinityOS!";\nprintln!("{}", message);\n}\n')
        panel_click(guest, "toggle")
        guest.screenshot("editor-idesign-review")
        text(guest, "minimize")
        guest.key("ret")
        wait_feature(guest, lambda p: p[14] == 3)
        panel_click(guest, "apply")
        wait_feature(guest, lambda p: p[12] != 2)
        guest.launch("settings", 8)
        wait_feature(guest, lambda p: p[12] == 4 and not p[13])
        panel_click(guest, "toggle")
        wait_feature(guest, lambda p: p[13] and p[12] == 4)
        text(guest, "help")
        guest.key("ret")
        wait_feature(guest, lambda p: p[19] > 0 and p[14] == 0)
        guest.screenshot("settings-ai-expanded")
        (work / "result.json").write_text(json.dumps({"fresh_install": not live_only, "iso_detached": not live_only,
            "cold_boot_identity": identity, "editor_history_clipboard_search_syntax": True, "saved": not live_only,
            "file_menu_open_dialog": True, "named_syntax_dropdown": True, "assistant_viewport_reflow": True,
            "reviewed_ai_insert": True, "independent_settings_panel": True, "features": saved[3:7]}, indent=2))
        print("Live UI acceptance passed" if live_only else "Installed editor and shared assistant acceptance passed", flush=True)
    finally:
        guest.stop()

if __name__ == "__main__":
    main()
