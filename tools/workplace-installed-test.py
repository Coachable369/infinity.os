"""Installed workplace acceptance through native input and read-only typed state."""
import hashlib
import base64
from functools import lru_cache
import importlib.util
import json
from pathlib import Path
import struct
import time
import zlib
from urllib.parse import quote

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("workplace_documents", ROOT / "tools/documents-installed-test.py")
docs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(docs)
editor = docs.helpers
# The staged ELF is immutable for this entire run, including the cold reboot.
docs.base.symbol = lru_cache(maxsize=16)(docs.base.symbol)

# ------------------------=
# FUNC: snapshot
# DESC: Reads coherent workplace policy and actual operation results without mutating the guest.
# ------------------=
def snapshot(guest):
    address, size = docs.base.symbol(guest.work.parent / "artifacts/installed-kernel.elf", "INFINITY_WORKPLACE_DIAGNOSTIC_SNAPSHOT")
    assert size == 256
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        state = struct.unpack("<32Q", guest.memory(address, size))
        if state[:2] == (0x494e46574f524b31, 1) and state[2] == state[31] and not state[2] & 1:
            return state
        time.sleep(.1)
    raise AssertionError("Incoherent workplace state")

# ------------------------=
# FUNC: command
# DESC: Invokes a real command and checks its operation identifier and result, never rendered prose.
# ------------------=
def command(guest, value, operation, success=True):
    before = snapshot(guest)[12]
    guest.command(value)
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        state = snapshot(guest)
        if state[12] != before:
            assert state[13:15] == (operation, 1 if success else 2), state
            return state
        time.sleep(.1)
    raise AssertionError((value, state))

# ------------------------=
# FUNC: text_hash
# DESC: Computes the editor's documented byte hash independently from the implementation.
# ------------------=
def text_hash(value):
    result = 1469598103934665603
    for byte in value.encode():
        result = ((result ^ byte) * 1099511628211) & ((1 << 64) - 1)
    return result

# ------------------------=
# FUNC: launch
# DESC: Opens the actual dock launcher independently of the focused app's Escape and slash bindings.
# ------------------=
def launch(guest, query):
    state = guest.state()
    width, height = state[11:13]
    guest.width, guest.height = width, height
    scale = 2 if width >= 2560 and height >= 1440 else 1
    dock_width = width * 54 // 100
    guest.click((width - dock_width) // 2 + (dock_width // 8) // 2, height - 46 * scale)
    guest.wait(lambda s: s[4] == 6, "dock launcher opened", timeout=30)
    guest.text(query)
    guest.key("ret")
    guest.wait(lambda s: s[4] == 5, "launcher selected app", timeout=30)

# ------------------------=
# FUNC: notes
# DESC: Focuses Notes through the native launcher and optionally replaces the document through actual input.
# ------------------=
def notes(guest, value=None):
    launch(guest, "text")
    editor.wait_feature(guest, lambda s: s[12] == 2)
    if value is not None:
        guest.key("ctrl", "a")
        guest.key("backspace")
        editor.text(guest, value)
        editor.wait_feature(guest, lambda s: s[7] == len(value) and s[16] == text_hash(value))

# ------------------------=
# FUNC: terminal
# DESC: Focuses Command through the native launcher without injecting a command buffer.
# ------------------=
def terminal(guest):
    launch(guest, "command")
    guest.wait(lambda s: s[71] == 1, "empty command draft")

# ------------------------=
# FUNC: digest
# DESC: Compares an actual native file checksum against an independent SHA-256 implementation.
# ------------------=
def digest(guest, path, value):
    state = command(guest, "work checksum " + path, 5)
    assert struct.pack("<4Q", *state[16:20]) == hashlib.sha256(value.encode()).digest()

# ------------------------=
# FUNC: folder
# DESC: Navigates using the real responsive sidebar and validates the resulting namespace.
# ------------------=
def folder(guest, index, path):
    state = docs.navigator(guest)
    x, y, width, height = state[4:8]
    scale = state[8]
    step = min(32 * scale, max(20 * scale, (height - 138 * scale) // 12))
    guest.click(x + 60 * scale, y + 106 * scale + index * step + (step - 4 * scale) // 2)
    return docs.wait_navigator(guest, lambda s: s[14] == zlib.crc32(path.encode()))

# ------------------------=
# FUNC: select_file
# DESC: Selects an actual file row by its full path CRC and captures the native context menu.
# ------------------=
def select_file(guest, path, keep_menu=False):
    state = docs.navigator(guest)
    x, y, width, height = state[4:8]
    scale = state[8]
    for row in range(state[15]):
        guest.click(x + width * 27 // 100 + 55 * scale, y + (168 + row * 34) * scale, press=False)
        for down in (True, False):
            guest.qmp("input-send-event", {"events": [{"type": "btn", "data": {"button": "right", "down": down}}]})
            guest.wait(lambda s: bool(s[15] & 2) == down, "file context pointer button")
        selected = docs.wait_navigator(guest, lambda s: s[17] and s[16] == row + 2)
        if selected[27] == zlib.crc32(path.encode()):
            guest.screenshot("workplace-file-context")
            if not keep_menu:
                guest.key("esc")
            return selected[28:30]
        guest.key("esc")
    raise AssertionError(("Missing file", path))

# ------------------------=
# FUNC: verify_context_menu
# DESC: Clicks Copy, Cut, and Paste in the actual eight-row menu and verifies collision safety.
# ------------------=
def verify_context_menu(guest):
    guest.width, guest.height = guest.state()[11:13]
    documents(guest)
    source = "/home/default/documents/enterprise.txt"
    identity = None
    for row, notice in ((4, 3), (5, 2), (6, 4)):
        selected = select_file(guest, source, keep_menu=True)
        if identity is None:
            identity = selected
        assert selected == identity
        state = docs.navigator(guest)
        left, top, width, height = state[23:27]
        scale = state[8]
        assert height == 236 * scale and width >= 100 * scale
        guest.click(left + 20 * scale, top + (6 + row * 28 + 14) * scale)
        docs.wait_navigator(guest, lambda s: not s[17] and s[30] == notice)
    assert select_file(guest, source, keep_menu=True) == identity
    guest.screenshot("workplace-context-eight-rows")
    guest.key("esc")
    terminal(guest)
    command(guest, "work clipboard clear", 2)

# ------------------------=
# FUNC: documents
# DESC: Opens the actual File Navigator dock item and its responsive Documents favorite.
# ------------------=
def documents(guest):
    before = guest.state()
    width, height = before[11:13]
    scale = 2 if width >= 2560 and height >= 1440 else 1
    dock_width = width * 54 // 100
    guest.click((width - dock_width) // 2 + (dock_width // 8) * 3 // 2, height - 46 * scale)
    released = guest.state()[2]
    docs.wait_navigator(guest, lambda s: s[3] and s[2] > released)
    return folder(guest, 2, "/home/default/documents")

# ------------------------=
# FUNC: open_source
# DESC: Opens the persisted test document through its native Open With menu and verifies its complete editor bytes.
# ------------------=
def open_source(guest, path, expected):
    documents(guest)
    select_file(guest, path, keep_menu=True)
    state = docs.navigator(guest)
    x, y = state[23:25]
    guest.click(x + 70 * state[8], y + 48 * state[8])
    state = docs.wait_navigator(guest, lambda s: s[18] == 1)
    x, y = state[23:25]
    guest.click(x + 70 * state[8], y + 20 * state[8])
    editor.wait_feature(guest, lambda s: s[12] == 2 and s[11] and s[16] == text_hash(expected))
    docs.wait_navigator(guest, lambda s: s[13] == zlib.crc32(path.encode()) and not s[17])

# ------------------------=
# FUNC: verify
# DESC: Exercises cross-app editing, guarded storage workflows, history, policy, lock clearing and detached reboot persistence.
# ------------------=
def verify(guest, browser, result=None, resume_storage=False):
    if result is None:
        result = {}
    guest.width, guest.height = guest.state()[11:13]
    original = "Enterprise clipboard"
    source = "/home/default/documents/enterprise.txt"
    backup = "/home/default/documents/enterprise-backup.txt"
    report = "/home/default/documents/enterprise-report.json"
    previous = {}
    if resume_storage:
        result["resumed_storage_only"] = True
        previous = json.loads((guest.work.parent / "result.json").read_text()).get("workplace", {})
        result["previous_run_checks"] = previous
        inherited = previous
        previous = {}
        while inherited:
            previous.update({key: value for key, value in inherited.items() if value is True})
            inherited = inherited.get("previous_run_checks", {})
        if not previous.get("checksums_comparison_verified_backup_restore"):
            open_source(guest, source, original)
    else:
        launch(guest, "text")
        guest.key("ctrl", "n")
        notes(guest, original)
        guest.key("ctrl", "a")
        guest.key("ctrl", "c")
        terminal(guest)
        guest.key("ctrl", "v")
        guest.wait(lambda s: s[71] == len(original) + 1, "cross-app terminal paste")
        guest.key("ctrl", "x")
        guest.wait(lambda s: s[71] == 1, "terminal cut")
        notes(guest, "")
        guest.key("ctrl", "v")
        editor.wait_feature(guest, lambda s: s[16] == text_hash(original))
        guest.key("ctrl", "s")
        docs.wait_navigator(guest, lambda s: s[9] == 1)
        editor.text(guest, "enterprise.txt")
        guest.key("ret")
        docs.wait_navigator(guest, lambda s: s[9] == 0 and s[13] == zlib.crc32(source.encode()))
        editor.wait_feature(guest, lambda s: s[11])
        result["cross_app_copy_cut_paste"] = True
    terminal(guest)
    digest(guest, source, original)
    expected = hashlib.sha256(original.encode()).hexdigest()
    if not previous.get("checksums_comparison_verified_backup_restore"):
        state = command(guest, "work backup " + source + " " + backup, 7)
        assert struct.pack("<4Q", *state[16:20]).hex() == expected
        command(guest, "work backup " + source + " " + backup, 7, False)
        assert command(guest, "work compare " + source + " " + backup, 6)[15] == 1
        notes(guest, "Changed content")
        guest.key("ctrl", "s")
        editor.wait_feature(guest, lambda s: s[11])
        terminal(guest)
        assert command(guest, "work compare " + source + " " + backup, 6)[15] == 0
        command(guest, "work restore " + backup + " " + source + " " + "0" * 64, 8, False)
        digest(guest, source, "Changed content")
        assert command(guest, "work restore " + backup + " " + source + " " + expected, 8)[15] > 1
        digest(guest, source, original)
        result["checksums_comparison_verified_backup_restore"] = True
    if not previous.get("support_report_created_readback_verified"):
        state = command(guest, "work report " + report, 9)
        assert 100 < state[15] <= 512 and state[16] != 0
        command(guest, "work report " + report, 9, False)
        result["support_report_created_readback_verified"] = True
    documents(guest)
    source_id = select_file(guest, source)
    guest.key("ctrl", "c")
    docs.wait_navigator(guest, lambda s: s[30] == 3)
    folder(guest, 3, "/home/default/downloads")
    guest.key("ctrl", "v")
    docs.wait_navigator(guest, lambda s: s[30] == 1)
    copy_id = select_file(guest, "/home/default/downloads/enterprise.txt")
    assert copy_id != source_id
    guest.key("ctrl", "x")
    docs.wait_navigator(guest, lambda s: s[30] == 2)
    folder(guest, 2, "/home/default/documents")
    guest.key("ctrl", "v")
    docs.wait_navigator(guest, lambda s: s[30] == 4)
    folder(guest, 4, "/home/default/pictures")
    guest.key("ctrl", "v")
    docs.wait_navigator(guest, lambda s: s[30] == 1)
    assert select_file(guest, "/home/default/pictures/enterprise.txt") == copy_id
    terminal(guest)
    digest(guest, source, original)
    digest(guest, "/home/default/pictures/enterprise.txt", original)
    command(guest, "work checksum /home/default/downloads/enterprise.txt", 5, False)
    result["file_copy_cut_paste_collision_and_identity"] = True
    verify_context_menu(guest)
    result["context_menu_copy_cut_paste_and_bounds"] = True
    command(guest, "work history clear", 4)
    assert snapshot(guest)[4] == 0
    guest.command("system info")
    guest.command("system status")
    guest.key("up")
    guest.wait(lambda s: s[71] == len("system status") + 1, "history recall")
    guest.key("down")
    guest.wait(lambda s: s[71] == 1, "history draft restoration")
    guest.text("info")
    guest.key("ctrl", "r")
    guest.wait(lambda s: s[71] == len("system info") + 1, "history search")
    guest.key("ctrl", "x")
    guest.text("work check")
    guest.key("tab")
    guest.wait(lambda s: s[71] == len("work checksum ") + 1, "command completion")
    guest.key("ctrl", "x")
    command(guest, "work private on", 3)
    guest.command("system info")
    assert snapshot(guest)[3:5] == (1, 0)
    command(guest, "work private off", 3)
    result["history_search_completion_private_mode"] = True
    notes(guest, "one\ntwo")
    guest.key("ctrl", "a")
    guest.key("ctrl", "c")
    terminal(guest)
    guest.text("draft")
    guest.key("ctrl", "v")
    guest.wait(lambda s: s[71] == 6, "multiline paste rejected atomically")
    guest.key("ctrl", "x")
    command(guest, "work clipboard browser-read off", 2)
    command(guest, "work clipboard browser-write off", 2)
    assert snapshot(guest)[6:8] == (0, 0)
    command(guest, "work clipboard browser-read on", 2)
    command(guest, "work clipboard browser-write on", 2)
    command(guest, "work clipboard ttl 1", 2)
    notes(guest, "Expire")
    guest.key("ctrl", "a")
    guest.key("ctrl", "c")
    time.sleep(2)
    guest.key("ctrl", "v")
    editor.wait_feature(guest, lambda s: s[16] == text_hash("Expire"))
    terminal(guest)
    command(guest, "work clipboard ttl 0", 2)
    notes(guest, "OS")
    guest.key("ctrl", "a")
    guest.key("ctrl", "c")
    terminal(guest)
    document = b'''<body style="margin:0;background:#123456"><input value="B" oninput="document.body.style.background=this.value==='OS'?'#00ff00':this.value==='web'?'#0000ff':'#ff0000'">'''
    url = "https://httpbun.com/mix/h=Content-Type:text%2Fhtml/b64=" + quote(base64.b64encode(document).decode(), safe="")
    guest.command("browser " + url)
    left, top, right, bottom = browser["page_color_bounds"](guest)
    guest.click(left + 40, top + 8)
    guest.key("ctrl", "a")
    guest.key("ctrl", "v")
    browser["wait_color"](guest, "workplace-browser-paste", left + 20, top + 40, (0, 255, 0))
    guest.key("ctrl", "a")
    browser["browser_text"](guest, "web")
    browser["wait_color"](guest, "workplace-browser-edited", left + 20, top + 40, (0, 0, 255))
    guest.key("ctrl", "a")
    guest.key("ctrl", "c")
    notes(guest, "")
    guest.key("ctrl", "v")
    editor.wait_feature(guest, lambda s: s[7] == 3 and s[16] == text_hash("web"))
    terminal(guest)
    command(guest, "work clipboard browser-write off", 2)
    launch(guest, "browser")
    guest.click(left + 40, top + 8)
    guest.key("ctrl", "a")
    guest.key("ctrl", "x")
    browser["wait_color"](guest, "workplace-browser-denied-cut", left + 20, top + 40, (0, 0, 255))
    notes(guest, "Denied")
    guest.key("ctrl", "a")
    guest.key("ctrl", "c")
    terminal(guest)
    command(guest, "work clipboard browser-read off", 2)
    launch(guest, "browser")
    guest.click(left + 40, top + 8)
    guest.key("ctrl", "a")
    guest.key("ctrl", "v")
    browser["wait_color"](guest, "workplace-browser-denied-paste", left + 20, top + 40, (0, 0, 255))
    terminal(guest)
    command(guest, "work clipboard browser-read on", 2)
    command(guest, "work clipboard browser-write on", 2)
    result["installed_browser_clipboard_and_policy_pixels"] = True
    notes(guest, "Lock sentinel")
    guest.key("ctrl", "a")
    guest.key("ctrl", "c")
    scale = 2 if guest.width >= 2560 and guest.height >= 1440 else 1
    guest.click(36 * scale, 18 * scale)
    for _ in range(6):
        guest.key("down")
    guest.key("ret")
    guest.authenticate()
    terminal(guest)
    assert snapshot(guest)[4] == 0
    guest.key("ctrl", "v")
    guest.wait(lambda s: s[71] == 1, "lock cleared clipboard")
    result["safe_paste_policy_expiry_lock_clearing"] = True
    guest.screenshot("workplace-command")
    guest.stop()
    guest.boot(False)
    guest.authenticate()
    launch(guest, "command")
    digest(guest, source, original)
    digest(guest, backup, original)
    digest(guest, "/home/default/pictures/enterprise.txt", original)
    command(guest, "work checksum " + report, 5)
    result["detached_cold_boot_persistence"] = True
    return result
