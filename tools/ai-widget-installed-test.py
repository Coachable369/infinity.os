"""Installed desktop composer and status-menu checks using real keyboard/pointer input."""
import time


# ------------------------=
# FUNC: snapshot
# DESC: Reads only versioned non-secret widget state through the existing privileged debugger harness.
# ------------------=
def snapshot(guest, native):
    state = native.read(guest, "INFINITY_AI_WIDGET_DIAGNOSTIC_SNAPSHOT", 48)
    assert state[:2] == (0x494e464149574431, 1)
    return state


# ------------------------=
# FUNC: wait
# DESC: Requires a bounded structured widget transition without relying on displayed status prose.
# ------------------=
def wait(guest, native, predicate, timeout=30):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        state = snapshot(guest, native)
        if predicate(state):
            return state
        time.sleep(.1)
    guest.screenshot("ai-widget-failure")
    raise AssertionError({"widget_state": list(state)})


# ------------------------=
# FUNC: open_menu
# DESC: Opens the real status menu at its published hit rectangle and checks viewport containment.
# ------------------=
def open_menu(guest, native):
    state = snapshot(guest, native)
    x, y, width, height = state[38:42]
    assert width > 0 and height > 0
    native.ui.click(guest, x + width // 2, y + height // 2)
    state = wait(guest, native, lambda s: s[36] == 1)
    x, y, width, height = state[32:36]
    display = guest.state()
    assert width > 0 and height > 0 and x + width <= display[11] and y + height <= display[12]
    return state


# ------------------------=
# FUNC: verify
# DESC: Verifies focused editing, scroll-aware pointer placement, menu cancellation and keyboard actions on a detached installed boot.
# ------------------=
def verify(guest, native):
    guest.launch("text editor", 5)
    editor_before = native.features(guest)[7]
    guest.key("ctrl", "j")
    wait(guest, native, lambda s: s[3] == 1 and s[4] == 2 and s[10] == 1, timeout=600)
    guest.key("ctrl", "x")
    wait(guest, native, lambda s: s[5:7] == (0, 0))
    text = "Wide WWW narrow iii mixed calculator instructions " * 5
    native.ui.text(guest, text)
    state = wait(guest, native, lambda s: s[5:8] == (len(text), len(text), 1))
    guest.screenshot("ai-widget-composer-overflow-end")
    x, y, width, height = state[20:24]
    native.ui.click(guest, x + 14, y + height // 2)
    state = wait(guest, native, lambda s: 0 < s[6] < len(text) and s[7] == 1)
    clicked_caret = state[6]
    guest.key("home")
    wait(guest, native, lambda s: s[6] == 0 and s[7] == 1)
    guest.screenshot("ai-widget-composer-overflow-home")
    guest.key("end")
    wait(guest, native, lambda s: s[6] == len(text))
    guest.key("left")
    guest.key("backspace")
    wait(guest, native, lambda s: s[5:7] == (len(text) - 1, len(text) - 2))
    guest.key("delete")
    wait(guest, native, lambda s: s[5:7] == (len(text) - 2, len(text) - 2))
    native.ui.text(guest, "Z")
    state = wait(guest, native, lambda s: s[5:7] == (len(text) - 1, len(text) - 1))
    assert native.features(guest)[7] == editor_before
    original_voice = state[11]
    open_menu(guest, native)
    guest.screenshot("ai-widget-status-menu")
    guest.key("esc")
    wait(guest, native, lambda s: s[36] == 0 and s[5] == len(text) - 1)
    open_menu(guest, native)
    guest.key("ret")
    wait(guest, native, lambda s: s[36] == 0 and s[11] != original_voice)
    open_menu(guest, native)
    guest.key("ret")
    wait(guest, native, lambda s: s[36] == 0 and s[11] == original_voice)
    guest.key("ctrl", "j")
    guest.key("ctrl", "x")
    native.ui.text(guest, "Explain how rain forms in one sentence.")
    guest.key("ret")
    wait(guest, native, lambda s: s[9] == 1 and s[5] == 0)
    open_menu(guest, native)
    guest.key("down")
    wait(guest, native, lambda s: s[37] == 1)
    guest.key("ret")
    wait(guest, native, lambda s: s[36] == 0 and s[9] != 1 and s[15] == 0)
    guest.key("ctrl", "j")
    native.ui.text(guest, "Ready again")
    wait(guest, native, lambda s: s[5:8] == (11, 11, 1))
    guest.screenshot("ai-widget-recovered-after-stop")
    guest.key("ctrl", "x")
    open_menu(guest, native)
    guest.key("down")
    guest.key("down")
    wait(guest, native, lambda s: s[37] == 2)
    guest.key("ret")
    guest.wait(lambda s: s[4] == 8 and s[8] == 3, "AI and Voice settings opened")
    guest.key("ctrl", "j")
    native.ui.text(guest, "Settings composer")
    wait(guest, native, lambda s: s[5:8] == (17, 17, 1))
    guest.screenshot("ai-widget-focused-over-settings")
    guest.key("ctrl", "x")
    return {"detached_installed_boot": True, "typed_bytes": len(text),
            "scroll_aware_pointer_caret": clicked_caret, "editing_keys": True,
            "editor_unchanged": True, "voice_toggle_restored": True,
            "cancel_and_readmit": True, "menu_settings_action": True,
            "microphone_recognition_verified": False}
