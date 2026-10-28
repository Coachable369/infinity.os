"""Real native Tk interactive acceptance; the companion driver uses pixels and input.

The six status cells visualize actual timer, Entry, button, resize, close-veto,
and Text operations. They are not host-written state or simulated service output.
Successful exit requires the real user-input path to complete all operations.
"""
import sys
import tkinter as tk
from tkinter import ttk, font

root = tk.Tk()
root.title("InfinityOS native Tk acceptance")
root.geometry("620x420+120+160")
root.minsize(560, 380)
root.maxsize(900, 650)
root.configure(background="#0b1220")
canvas = tk.Canvas(root, background="#0b1220", highlightthickness=0, borderwidth=0)
canvas.pack(fill="both", expand=True)
state = dict(flags=[False] * 6, failed=False, completed=False, close_count=0,
             control_seen=False)
require_control = "--control-selection" in sys.argv[1:]
cells = [canvas.create_rectangle(20 + i * 40, 20, 44 + i * 40, 44,
                                fill="#162838", outline="") for i in range(6)]
anchor = canvas.create_rectangle(0, 0, 16, 16, fill="#f43f5e", outline="")


# ------------------------=
# FUNC: mark
# DESC: Displays one verified application state transition in actual Canvas pixels.
# ------------------=
def mark(index):
    state["flags"][index] = True
    canvas.itemconfigure(cells[index], fill="#22c55e")


# ------------------------=
# FUNC: failed_callback
# DESC: Preserves callback failures as a nonzero process result rather than a deceptively live test window.
# ------------------=
def failed_callback(kind, error, traceback):
    state["failed"] = True
    sys.__excepthook__(kind, error, traceback)
    root.destroy()


root.report_callback_exception = failed_callback
style = ttk.Style(root)
assert style.layout("TButton")
assert font.nametofont("TkDefaultFont").metrics("linespace") > 0
value = tk.StringVar(root)
entry = ttk.Entry(root, textvariable=value)
entry.place(x=20, y=80, width=280, height=38)


# ------------------------=
# FUNC: edited
# DESC: Verifies real Entry key editing via the Tcl variable trace before marking input accepted.
# ------------------=
def edited(*arguments):
    if value.get() == "native42" and (not require_control or state["control_seen"]):
        mark(1)


# ------------------------=
# FUNC: clicked
# DESC: Requires prior real keyboard input before the ttk button callback can satisfy its stage.
# ------------------=
def clicked():
    assert entry.get() == "native42"
    mark(2)


value.trace_add("write", edited)


# ------------------------=
# FUNC: select_all
# DESC: Requires a real normalized Control event before replacing selected Entry text; no host state injection is used.
# ------------------=
def select_all(event):
    assert event.state & 4
    state["control_seen"] = True
    entry.selection_range(0, "end")
    entry.icursor("end")
    return "break"


if require_control:
    entry.bind("<Control-a>", select_all)
button = ttk.Button(root, text="Verify native input", command=clicked)
button.place(x=340, y=80, width=220, height=38)
labels = ttk.Frame(root)
labels.place(x=20, y=128, width=540, height=28)
labels.columnconfigure(0, weight=1)
labels.columnconfigure(1, weight=1)
ttk.Label(labels, text="Upstream Tcl/Tk").grid(row=0, column=0, sticky="w")
ttk.Label(labels, text="Native InfinityOS").grid(row=0, column=1, sticky="e")
photo = tk.PhotoImage(master=root, width=32, height=32)
for row in range(4):
    for column in range(4):
        photo.put("#38bdf8" if (row + column) % 2 else "#e2e8f0",
                  to=(column * 8, row * 8, column * 8 + 8, row * 8 + 8))
assert photo.get(1, 1) == (226, 232, 240)
assert photo.get(9, 1) == (56, 189, 248)
canvas.create_image(20, 170, image=photo, anchor="nw")
canvas.create_oval(100, 170, 160, 230, fill="#38bdf8", outline="#e2e8f0", width=2)
text = tk.Text(root, undo=True, autoseparators=False, background="#142438", foreground="#e2e8f0",
               insertbackground="#38bdf8", borderwidth=0, highlightthickness=0)
text.place(x=20, y=250, width=540, height=100)
text.insert("1.0", "first\nsecond")
assert text.get("1.0", "end-1c") == "first\nsecond"
text.edit_separator()
text.delete("2.0", "2.end")
text.insert("2.0", "edited")
assert text.get("2.0", "2.end") == "edited"
text.edit_undo()
assert text.get("1.0", "end-1c") == "first\nsecond"
mark(5)


# ------------------------=
# FUNC: configured
# DESC: Verifies native resize notifications and the real pack-managed drawable's new dimensions.
# ------------------=
def configured(event):
    if event.widget is not canvas:
        return
    canvas.coords(anchor, event.width - 24, event.height - 24,
                  event.width - 8, event.height - 8)
    if event.width >= 700 and event.height >= 480:
        assert root.winfo_width() == event.width and root.winfo_height() == event.height
        mark(3)


# ------------------------=
# FUNC: close_requested
# DESC: Vetoes the first native chrome request and permits the next only after every observable stage has passed.
# ------------------=
def close_requested():
    state["close_count"] += 1
    if state["close_count"] == 1:
        mark(4)
        return
    assert all(state["flags"]) and not state["failed"]
    state["completed"] = True
    root.destroy()


# ------------------------=
# FUNC: expired
# DESC: Ensures a stalled installed test eventually exits with failure without indefinite event-loop activity.
# ------------------=
def expired():
    state["failed"] = True
    root.destroy()


canvas.bind("<Configure>", configured)
root.protocol("WM_DELETE_WINDOW", close_requested)
root.after(500, lambda: mark(0))
root.after(180000, expired)
root.mainloop()
assert state["completed"] and not state["failed"] and all(state["flags"]), state
