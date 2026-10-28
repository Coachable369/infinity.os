"""Real installed editor terminal and generated-catalog acceptance.

Invoked by native-editor-installed-test.py --editor-terminal-only through the
build kit. Every mutation is a real key/pointer event on its disposable guest.
Read-only numeric snapshots corroborate rendering, execution and storage state.
"""
import json
import time

C_SOURCE = """#include <stdio.h>
#include <stdlib.h>
#include <infinity/console.h>
int main(void){char b[32];puts("ready");if(infinity_read_line(b,32)<0)return 99;fprintf(stderr,"received\\n");return atoi(b)+5;}"""
PYTHON_SOURCE = """import sys
n=int(input("Number: "))
print(n+5)
print(n,file=sys.stderr)
sys.exit(n+5)"""
GUI_SOURCE = """import sys
import infinity as a
a.window("Python Native Actions",400,280)
a.display(2,20,20,360,32,"0")
a.button(1,20,80,160,44,"Increment")
a.state(True,False,"0")
a.choice(1,"counter.increment","Increment")
a.publish()
while True:
 e=a.event()
 if e and e["id"]==1:
  a.display(2,20,20,360,32,"1")
  a.state(False,True,"1")
  a.publish()
  if e["kind"]==a.AI: a.done(e["token"],True)
  break
 a.yield_native()
while True:
 a.yield_native()"""


# ------------------------=
# FUNC: terminal
# DESC: Reads bounded numeric dock geometry and state without inspecting rendered prose or source implementation.
# ------------------=
def terminal(guest, h):
    raw = h.read(guest, "INFINITY_EDITOR_TERMINAL_SNAPSHOT", 72)
    assert raw[:2] == (0x494e464554455231, 1)
    names = ("open", "height", "focused", "attached", "running", "rows", "input_bytes", "input_cursor", "scroll",
             "document_cursor", "document_bytes", "diagnostics", "diagnostic_line", "diagnostic_column",
             "diagnostic_x", "diagnostic_y", "cursor_line", "cursor_column", "streamed_bytes")
    result = dict(zip(names, raw[3:22]))
    result["sequence"] = raw[2]
    for i, name in enumerate(("panel", "splitter", "output", "input", "toggle", "run", "stop", "restart", "track", "thumb")):
        result[name] = raw[24+i*4:28+i*4]
    result.update(visible_rows=raw[64], code_height=raw[65], scale=raw[66], editor_active=bool(raw[67]))
    return result


# ------------------------=
# FUNC: wait
# DESC: Polls typed state with a fixed deadline while retaining the final state in any failure.
# ------------------=
def wait(read, predicate, timeout=30):
    deadline = time.monotonic() + timeout
    while True:
        value = read()
        if predicate(value):
            return value
        assert time.monotonic() < deadline, value
        time.sleep(.1)


# ------------------------=
# FUNC: type_text
# DESC: Delivers full printable ASCII through real keyboard events, including source-code punctuation.
# ------------------=
def type_text(guest, value):
    plain = {" ": "spc", "-": "minus", "=": "equal", ".": "dot", ",": "comma", "/": "slash", ";": "semicolon",
             "'": "apostrophe", "[": "bracket_left", "]": "bracket_right", "\\": "backslash", "`": "grave_accent"}
    shifted = dict(zip('!@#$%^&*()_+{}|:"<>?~',
                       ("1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "minus", "equal", "bracket_left",
                        "bracket_right", "backslash", "semicolon", "apostrophe", "comma", "dot", "slash", "grave_accent")))
    for c in value:
        assert c.isascii() and c.isprintable(), repr(c)
        guest.key(*(("shift", shifted[c]) if c in shifted else ("shift", c.lower()) if c.isupper() else (plain.get(c, c),)))


# ------------------------=
# FUNC: type_source
# DESC: Respects the real editor's automatic indentation while entering exact fixture bytes with normal keys.
# ------------------=
def type_source(guest, source):
    previous_indent = 0
    for index, line in enumerate(source.split("\n")):
        if index:
            guest.key("ret")
            for _ in range(previous_indent):
                guest.key("backspace")
        type_text(guest, line)
        previous_indent = len(line) - len(line.lstrip(" "))


# ------------------------=
# FUNC: fingerprint
# DESC: Computes the editor's documented byte-identity digest for typed source integrity checks.
# ------------------=
def fingerprint(source):
    result = 1469598103934665603
    for byte in source.encode("ascii"):
        result = ((result ^ byte) * 1099511628211) & ((1 << 64) - 1)
    return result


# ------------------------=
# FUNC: click
# DESC: Activates the center of a measured native dock control using actual pointer input.
# ------------------=
def click(guest, h, control):
    x, y, w, height = terminal(guest, h)[control]
    assert w > 0 and height > 0
    h.ui.click(guest, x + w//2, y + height//2)


# ------------------------=
# FUNC: new_document
# DESC: Starts from saved work, types and saves an exact fixture through the native editor and Save As dialog.
# ------------------=
def new_document(guest, h, source, filename):
    guest.launch("text", 5)
    before = h.ui.wait_feature(guest, lambda v: v[12] == 2)
    assert before[11], "Existing unsaved work was preserved; fixture requires a saved editor"
    if before[13]:
        h.ui.panel_click(guest, "toggle")
    before = h.features(guest)
    h.ui.click(guest, before[30] + 20, before[29] + 12)
    guest.key("ctrl", "n")
    h.ui.wait_feature(guest, lambda v: v[7] == 0 and v[11])
    type_source(guest, source)
    typed = h.ui.wait_feature(guest, lambda v: v[7] == len(source))
    assert typed[16] == fingerprint(source) and not typed[11]
    guest.key("ctrl", "s")
    h.ui.wait_editor_dialog(guest, 1)
    type_text(guest, filename)
    guest.key("ret")
    h.ui.wait_editor_dialog(guest, 0)
    saved = h.ui.wait_feature(guest, lambda v: v[11] and v[16] == fingerprint(source))
    return {"bytes": saved[7], "fingerprint": saved[16], "filename": filename}


# ------------------------=
# FUNC: stdin
# DESC: Sends one line through the dock's real focused command input without changing source bytes.
# ------------------=
def stdin(guest, h, text):
    source = h.features(guest)[16]
    click(guest, h, "input")
    type_text(guest, text)
    state = wait(lambda: terminal(guest, h), lambda v: v["input_bytes"] == len(text))
    assert state["input_cursor"] == len(text) and state["focused"]
    guest.key("ret")
    wait(lambda: terminal(guest, h), lambda v: not v["input_bytes"])
    assert h.features(guest)[16] == source


# ------------------------=
# FUNC: c_waiting
# DESC: Requires a real runnable C process waiting on native stdin, not merely an admitted compile request.
# ------------------=
def c_waiting(guest, h):
    # ------------------------=
    # FUNC: current
    # DESC: Stops promptly on a compiler failure and retains its typed diagnostic receipt rather than waiting for a nonexistent process.
    # ------------------=
    def current():
        build = h.read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)
        assert build[3] != 4, {"build": build, "terminal": terminal(guest, h)}
        return h.read(guest, "INFINITY_NATIVE_APP_SNAPSHOT", 8)
    return wait(current, lambda v: v[3] == 2 and v[6] == 1, 180)


# ------------------------=
# FUNC: c_finished
# DESC: Verifies the program's arithmetic-derived exit status and observed native output byte count.
# ------------------=
def c_finished(guest, h):
    state = wait(lambda: h.read(guest, "INFINITY_NATIVE_APP_SNAPSHOT", 8), lambda v: v[3] >= 3)
    assert state[3] == 3 and state[6] == 42 and state[7] >= 15, state
    channels = h.read(guest, "INFINITY_NATIVE_APP_IO_SNAPSHOT", 8)
    assert channels[:2] == (0x494e464341494f31, 1) and channels[3:7] == (3, 6, 9, 0), channels
    return {"process": state, "streams": channels}


# ------------------------=
# FUNC: python_finished
# DESC: Verifies the actual CPython process exit, separate stdout and stderr capture, and cleanup state.
# ------------------=
def python_finished(guest, h, previous, exit_code=42):
    state = wait(lambda: h.read(guest, "INFINITY_PYTHON_SNAPSHOT", 10), lambda v: v[2] > previous and v[3] >= 2, 60)
    assert state[3] == 2 and state[4] == exit_code, state
    return state


# ------------------------=
# FUNC: resize
# DESC: Drags the actual terminal divider and verifies both bounded source geometry and committed height.
# ------------------=
def resize(guest, h):
    before = terminal(guest, h)
    x, y, w, height = before["splitter"]
    guest.width, guest.height = guest.state()[11:13]
    guest.click(x + w//2, y + height//2, press=False)
    guest.qmp("input-send-event", {"events": [{"type": "btn", "data": {"button": "left", "down": True}}]})
    guest.wait(lambda v: v[15] & 1, "terminal splitter pressed")
    guest.click(x + w//2, y + height//2 + 44 * before["scale"], press=False)
    guest.qmp("input-send-event", {"events": [{"type": "btn", "data": {"button": "left", "down": False}}]})
    guest.wait(lambda v: not v[15] & 1, "terminal splitter released")
    after = wait(lambda: terminal(guest, h), lambda v: v["height"] != before["height"])
    assert after["code_height"] >= 72 * after["scale"] and after["height"] >= 112
    assert after["panel"][3] < before["panel"][3], (before, after)
    return after


# ------------------------=
# FUNC: scroll
# DESC: Verifies wheel and grabbed-thumb scrolling move only the real dock transcript, then returns to the live tail.
# ------------------=
def scroll(guest, h):
    before = terminal(guest, h)
    assert before["rows"] > before["visible_rows"]
    source = h.features(guest)[16]
    x, y, w, height = before["output"]
    guest.click(x + w//2, y + height//2, press=False)
    for down in (True, False):
        guest.qmp("input-send-event", {"events": [{"type": "btn", "data": {"button": "wheel-up", "down": down}}]})
    wheeled = wait(lambda: terminal(guest, h), lambda v: v["scroll"] > 0)
    x, y, w, height = wheeled["thumb"]
    guest.click(x + w//2, y + height//2, press=False)
    guest.qmp("input-send-event", {"events": [{"type": "btn", "data": {"button": "left", "down": True}}]})
    guest.wait(lambda v: v[15] & 1, "terminal thumb pressed")
    track = wheeled["track"]
    guest.click(x + w//2, track[1] + track[3] - 1, press=False)
    guest.qmp("input-send-event", {"events": [{"type": "btn", "data": {"button": "left", "down": False}}]})
    guest.wait(lambda v: not v[15] & 1, "terminal thumb released")
    tail = wait(lambda: terminal(guest, h), lambda v: v["scroll"] == 0)
    assert h.features(guest)[16] == source
    return {"wheel": wheeled, "tail": tail}


# ------------------------=
# FUNC: verify
# DESC: Verifies installed terminal execution, controls, diagnostic navigation, persisted layout, registry launch and real Python hooks.
# ------------------=
def verify(guest, h):
    result = {"passed": False, "scope": "Installed native editor terminal and registered application services; no LLM generation or physical microphone claim", "stages": []}
    guest.editor_terminal_evidence = result
    directory = guest.work.parent / ("editor-terminal-" + str(time.time_ns()))
    directory.mkdir()
    result["evidence_directory"] = str(directory)
    nonce = format(time.time_ns() & 0xffffffff, "08x")
    app_name = "terminalproof" + nonce

    # ------------------------=
    # FUNC: record
    # DESC: Keeps completed typed observations and screenshots durable if a later independent acceptance item fails.
    # ------------------=
    def record(stage, receipt):
        result["stages"].append({"stage": stage, "receipt": receipt, "terminal": terminal(guest, h)})
        (directory / "evidence.json").write_text(json.dumps(result, indent=2) + "\n")
        guest.screenshot("editor-terminal-" + stage)

    source = new_document(guest, h, C_SOURCE, app_name + ".c")
    # Maximize through native chrome to leave measurable space for both panes.
    editor = h.features(guest)
    h.ui.click(guest, editor[21] + editor[23] - 59, editor[22] + 24)
    h.ui.wait_feature(guest, lambda v: v[24] > editor[24])
    click(guest, h, "run")
    waiting = c_waiting(guest, h)
    dock = wait(lambda: terminal(guest, h), lambda v: v["open"] and v["running"])
    assert h.features(guest)[12] == 2 and dock["attached"]
    stdin(guest, h, "37")
    record("c-run-stdin-stdout-stderr", {"source": source, "waiting": waiting, "finished": c_finished(guest, h)})
    wait(lambda: terminal(guest, h), lambda v: not v["running"])
    click(guest, h, "run")
    c_waiting(guest, h)
    click(guest, h, "input")
    type_text(guest, "discarded")
    compile_before = h.read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2]
    click(guest, h, "restart")
    wait(lambda: h.read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10), lambda v: v[2] > compile_before and v[3] == 3, 180)
    c_waiting(guest, h)
    assert terminal(guest, h)["input_bytes"] == 0
    click(guest, h, "stop")
    stopped = wait(lambda: h.read(guest, "INFINITY_NATIVE_APP_SNAPSHOT", 8), lambda v: v[3] == 4)
    wait(lambda: terminal(guest, h), lambda v: not v["running"])
    record("c-restart-stop", stopped)

    resized = resize(guest, h)
    record("transcript-wheel-and-thumb", scroll(guest, h))
    click(guest, h, "toggle")
    collapsed = wait(lambda: terminal(guest, h), lambda v: not v["open"])
    assert collapsed["panel"][3] == 32 * collapsed["scale"] and collapsed["code_height"] > resized["code_height"]
    click(guest, h, "toggle")
    restored = wait(lambda: terminal(guest, h), lambda v: v["open"])
    assert restored["height"] == resized["height"]
    record("collapse-resize", {"resized": resized, "collapsed": collapsed})

    # Searchable launcher and conversational instruction must reach one catalog,
    # then the same native process; neither may infer launch from reply prose.
    catalog_before = h.read(guest, "INFINITY_GENERATED_APPS_SNAPSHOT", 16)
    guest.launch(app_name, 5)
    c_waiting(guest, h)
    catalog = h.read(guest, "INFINITY_GENERATED_APPS_SNAPSHOT", 16)
    assert catalog[3] > 0 and catalog[4] == catalog[3] and catalog[12] > catalog_before[12]
    # Launcher intentionally presents the native console, independent of editor focus.
    type_text(guest, "37")
    guest.key("ret")
    c_finished(guest, h)
    record("catalog-launcher", catalog)
    guest.key("ctrl", "j")
    type_text(guest, "launch " + app_name)
    guest.key("ret")
    c_waiting(guest, h)
    after = h.read(guest, "INFINITY_GENERATED_APPS_SNAPSHOT", 16)
    assert after[12] > catalog[12] and after[6:8] == catalog[6:8]
    type_text(guest, "37")
    guest.key("ret")
    c_finished(guest, h)
    record("typed-ai-named-launch", after)

    # Closing the saved editor must transfer its live dock task, not revoke the
    # authenticated process merely because its original presentation vanished.
    guest.launch("text", 5)
    h.ui.wait_feature(guest, lambda v: v[12] == 2)
    click(guest, h, "run")
    c_waiting(guest, h)
    h.editor_window_control(guest, "close")
    h.ui.wait_feature(guest, lambda v: v[12] != 2)
    transferred = wait(lambda: terminal(guest, h), lambda v: not v["attached"])
    guest.launch("command", 5)
    c_waiting(guest, h)
    type_text(guest, "37")
    guest.key("ret")
    record("closing-editor-preserves-live-process", {"transferred": transferred, "finished": c_finished(guest, h)})

    guest.stop()
    guest.boot(False)
    guest.authenticate()
    guest.launch("text", 5)
    persisted = wait(lambda: terminal(guest, h), lambda v: v["editor_active"])
    assert persisted["open"] and persisted["height"] == resized["height"]
    catalog = h.read(guest, "INFINITY_GENERATED_APPS_SNAPSHOT", 16)
    assert catalog[3] > 0 and catalog[4] == catalog[3]
    record("cold-boot-layout-and-catalog", {"dock": persisted, "catalog": catalog})

    source = new_document(guest, h, PYTHON_SOURCE, "terminalpython" + nonce + ".py")
    previous = h.read(guest, "INFINITY_PYTHON_SNAPSHOT", 10)[2]
    before_bytes = terminal(guest, h)["streamed_bytes"]
    click(guest, h, "run")
    wait(lambda: h.read(guest, "INFINITY_PYTHON_SNAPSHOT", 10), lambda v: v[2] > previous and v[3] == 1)
    wait(lambda: terminal(guest, h), lambda v: v["running"] and v["streamed_bytes"] > before_bytes)
    stdin(guest, h, "37")
    finished = python_finished(guest, h, previous)
    assert finished[5] >= 11 and finished[6] >= 3
    record("python-run-stdin-stdout-stderr", {"source": source, "finished": finished})
    wait(lambda: terminal(guest, h), lambda v: not v["running"])
    click(guest, h, "run")
    running = wait(lambda: h.read(guest, "INFINITY_PYTHON_SNAPSHOT", 10), lambda v: v[2] > finished[2] and v[3] == 1)
    click(guest, h, "input")
    type_text(guest, "discarded")
    click(guest, h, "restart")
    restarted = wait(lambda: h.read(guest, "INFINITY_PYTHON_SNAPSHOT", 10), lambda v: v[2] > running[2] and v[3] == 1)
    assert terminal(guest, h)["input_bytes"] == 0
    click(guest, h, "stop")
    cancelled = wait(lambda: h.read(guest, "INFINITY_PYTHON_SNAPSHOT", 10), lambda v: v[2] > restarted[2] and v[3] == 3)
    wait(lambda: terminal(guest, h), lambda v: not v["running"])
    record("python-restart-stop", cancelled)

    # Syntax failure must leave the source intact and navigate through an actual
    # compiler-emitted source location, rather than an injected output string.
    source = new_document(guest, h, "int main(void){\nreturn does_not_exist;\n}", "terminalinvalid" + nonce + ".c")
    before = h.read(guest, "INFINITY_NATIVE_BUILD_SNAPSHOT", 10)[2]
    click(guest, h, "run")
    h.wait_build(guest, before, 4)
    error = wait(lambda: terminal(guest, h), lambda v: v["diagnostics"] > 0 and not v["running"])
    assert error["diagnostic_line"] == 2
    h.ui.click(guest, error["diagnostic_x"], error["diagnostic_y"])
    navigated = wait(lambda: terminal(guest, h), lambda v: not v["focused"] and v["cursor_line"] == error["diagnostic_line"] and v["cursor_column"] == error["diagnostic_column"])
    assert h.features(guest)[16] == source["fingerprint"]
    record("compiler-clickable-diagnostic", navigated)

    source = new_document(guest, h, GUI_SOURCE, "terminalhooks" + nonce + ".py")
    previous = h.read(guest, "INFINITY_PYTHON_SNAPSHOT", 10)[2]
    click(guest, h, "run")
    contract = wait(lambda: h.interaction_state(guest), lambda v: v["choice_count"] == 1 and v["commits"] > 0)
    assert contract["choices"][0]["id"] == 1 and contract["choices"][0]["arguments"] == (0, 0, 0, 0)
    native = wait(lambda: h.native_ui_state(guest), lambda v: v["active"] and len(v["elements"]) == 2)
    assert any(v["id"] == 1 and v["kind"] == 3 for v in native["elements"])
    h.ui.wait_feature(guest, lambda v: v[12] == 1)
    h.click_native_element(guest, 1)
    changed = wait(lambda: h.interaction_state(guest), lambda v: v["instance"] == contract["instance"] and v["commits"] > contract["commits"] and v["terminal"])
    rendered = h.native_ui_state(guest)
    assert int(next(v["value"] for v in rendered["elements"] if v["id"] == 2)) == 1
    guest.key("ctrl", "c")
    cancelled = wait(lambda: h.read(guest, "INFINITY_PYTHON_SNAPSHOT", 10), lambda v: v[2] > previous and v[3] == 3)
    record("python-real-uikit-and-control-hooks", {"source": source, "contract": contract, "native": native, "changed": changed, "rendered": rendered, "cancelled": cancelled})
    result["passed"] = True
    (directory / "evidence.json").write_text(json.dumps(result, indent=2) + "\n")
    return result
