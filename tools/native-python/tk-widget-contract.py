"""Checks upstream geometry managers and widgets in the actual native interpreter."""
import tkinter as tk
from tkinter import ttk

root = tk.Tk()
root.geometry("420x300+80+120")
root.rowconfigure(0, weight=1)
root.columnconfigure(0, weight=1)
frame = ttk.Frame(root)
frame.grid(sticky="nsew")
frame.columnconfigure(0, weight=1)
frame.rowconfigure(1, weight=1)
value = tk.IntVar(root, 2)
spin = ttk.Spinbox(frame, from_=0, to=10, textvariable=value)
spin.grid(row=0, column=0, sticky="ew")
tree = ttk.Treeview(frame, columns=("number",))
tree.grid(row=1, column=0, sticky="nsew")
row = tree.insert("", "end", text="native", values=(17,))
assert tree.item(row, "values") == ("17",)
tree.selection_set(row)
assert tree.selection() == (row,)
root.update()
assert frame.winfo_width() == root.winfo_width(), (frame.winfo_width(), root.winfo_width())
assert tree.winfo_height() > 100 and spin.winfo_width() == frame.winfo_width()
root.geometry("520x400")
root.update()
assert frame.winfo_width() == 520 and frame.winfo_height() == 400, (
    frame.winfo_width(), frame.winfo_height(), root.winfo_width(), root.winfo_height())
root.destroy()
