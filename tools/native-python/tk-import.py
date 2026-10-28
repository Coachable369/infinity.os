"""Tests genuine upstream interpreter and widget bindings without opening a window."""
import tkinter
import _tkinter
from tkinter import ttk, font

tcl = tkinter.Tcl()
assert _tkinter.TCL_VERSION == "8.6" and _tkinter.TK_VERSION == "8.6"
assert tuple(map(int, tcl.call("info", "patchlevel").split("."))) == (8, 6, 18)
assert tcl.call("expr", "3 * 7") == 21
assert tcl.call("string", "length", "\u03bb\u754c") == 2
assert tcl.call("package", "require", "msgcat")
