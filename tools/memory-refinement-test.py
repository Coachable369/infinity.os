"""Exercise the production Memory controls with real Tk and an isolated record."""
import runpy
import sys
import tkinter as tk
import types
from unittest.mock import patch
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"sdk/python"))

# ------------------------=
# FUNC: request
# DESC: Supplies a single isolated AIKit record for host-side interaction checks.
# ------------------=
def request(op, *args):
    if op == 3:
        return b"42\t2\t0\t0\t0\0local\0user\0My cat is Shade"
    return b"42\t2\tMy cat is Shade\ncounts\t1\t1\t1\t15"

sys.modules["_infinity_app"] = types.SimpleNamespace(memory_request=request)
# Production UIKit raster data supplies button fixtures; only AIKit records are simulated.
import base64, zlib
from infinity_uikit_artwork import SURFACES
artwork = zlib.decompress(base64.b64decode(SURFACES))
# ------------------------=
# FUNC: surface
# DESC: Supply shared production button pixels to the isolated native-service fixture.
# ------------------=
def surface(op, data, key):
    index = (key % 2)*4 + key//2
    return b"P6\n80 40\n255\n" + artwork[index*9600:(index+1)*9600]
sys.modules["_infinity_media"] = types.SimpleNamespace(io=surface)
root = tk.Tk()
root.withdraw()
try:
    browser = runpy.run_path("runtime/install/System/infinity-memory.py")["Browser"](root)
    root.deiconify()
    root.update()
    menu_bar = root.winfo_children()[0]
    fixed_height = menu_bar.winfo_height()
    assert fixed_height >= 36
    for width,height in ((900,650),(1250,850)):
        root.geometry(f"{width}x{height}")
        root.update()
        assert menu_bar.winfo_height() == fixed_height
    button = menu_bar.winfo_children()[0]
    with patch.object(browser.menus["File"],"tk_popup") as post:
        button.event_generate("<ButtonPress-1>",x=8,y=8)
        root.update()
        post.assert_not_called()
        button.event_generate("<ButtonRelease-1>",x=8,y=8)
        root.update()
        post.assert_called_once_with(button.winfo_rootx(),button.winfo_rooty()+button.winfo_height())
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
    if "--capture" in sys.argv:
        from PIL import ImageGrab
        browser.query.set("")
        browser.show(42)
        root.update()
        root.lift()
        root.update()
        ImageGrab.grab(bbox=(root.winfo_rootx(),root.winfo_rooty(),
            root.winfo_rootx()+root.winfo_width(),root.winfo_rooty()+root.winfo_height())).save(
                "builds/validation/sapphire-coverage/memory-host-uikit.png")
finally:
    root.destroy()
