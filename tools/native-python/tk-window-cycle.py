"""Exercises genuine top-level withdrawal, multiple windows and resource recycling."""
import tkinter as tk

root = tk.Tk()
root.geometry("300x180+80+120")


# ------------------------=
# FUNC: observed_interval
# DESC: Keeps real lifecycle states observable through native frames while yielding via actual Tcl timers.
# ------------------=
def observed_interval():
    resumed = tk.BooleanVar(root, False)
    root.after(750, resumed.set, True)
    root.wait_variable(resumed)


root.update()
assert root.state() == "normal" and root.winfo_ismapped()
root.withdraw()
root.update()
assert root.state() == "withdrawn" and not root.winfo_ismapped()
observed_interval()
root.deiconify()
root.update()
assert root.state() == "normal" and root.winfo_ismapped()
for cycle in range(3):
    windows = [tk.Toplevel(root) for _ in range(7)]
    for index, window in enumerate(windows):
        window.geometry("160x100+%d+%d" % (100 + index * 30, 180 + index * 20))
    root.update()
    assert all(window.winfo_ismapped() for window in windows)
    observed_interval()
    for window in windows:
        window.destroy()
    root.update()
    observed_interval()
root.destroy()
