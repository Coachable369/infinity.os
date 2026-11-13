"""Exercise the production Memory controls with real Tk and an isolated record."""
import runpy
import sys
import tkinter as tk
import types

# ------------------------=
# FUNC: request
# DESC: Supplies a single isolated AIKit record for host-side interaction checks.
# ------------------=
def request(op, *args):
    if op == 3:
        return b"42\t2\t0\t0\t0\0local\0user\0My cat is Shade"
    return b"42\t2\tMy cat is Shade\ncounts\t1\t1\t1\t15"

sys.modules["_infinity_app"] = types.SimpleNamespace(memory_request=request)
sys.modules["_infinity_media"] = types.SimpleNamespace(io=lambda *args: b"P6\n80 28\n255\n" + bytes((32,61,93))*2240)
root = tk.Tk()
root.withdraw()
try:
    browser = runpy.run_path("runtime/install/System/infinity-memory.py")["Browser"](root)
    browser.show(42)
    browser.copy_fact()
    assert root.clipboard_get() == browser.original
    browser.duplicate()
    assert not browser.record and browser.editor.get("1.0","end-1c") == "My cat is Shade"
    browser.editor.insert("end-1c","!")
    browser.history(False)
    assert browser.editor.get("1.0","end-1c") == "My cat is Shade"
    browser.history(True)
    assert browser.editor.get("1.0","end-1c").endswith("!")
    for i in range(8):
        browser.query.set(str(i))
        browser.remember_search()
    browser.recent_menu.invoke(3)
    assert len(browser.recent_queries)==6 and browser.query.get()=="4"
finally:
    root.destroy()
