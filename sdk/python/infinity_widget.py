"""Native desktop widget gestures and durable, per-project placement."""
import json
import os
import sys
import tkinter as tk


class WidgetHost:
    # ------------------------=
    # FUNC: __init__
    # DESC: Restores saved placement and binds background gestures without intercepting interactive controls.
    # ------------------=
    def __init__(self, root, project):
        self.root = root
        self.grab = None
        base = os.path.basename(project)
        folder = "/data" if sys.platform == "infinityos" else os.path.dirname(os.path.abspath(project))
        self.path = os.path.join(folder, base + ".placement.json")
        try:
            with open(self.path, encoding="utf-8") as stream:
                x, y = json.load(stream)
            self.place(int(x), int(y))
        except (OSError, TypeError, ValueError):
            pass
        self.menu = tk.Menu(root, tearoff=False)
        self.menu.add_command(label="Reset position", command=self.reset)
        self.menu.add_command(label="Close widget", command=root.destroy)
        root.bind("<ButtonPress-1>", self.press, add="+")
        root.bind("<B1-Motion>", self.move, add="+")
        root.bind("<ButtonRelease-1>", self.release, add="+")
        root.bind("<Button-3>", self.configure, add="+")

    # ------------------------=
    # FUNC: configure
    # DESC: Presents widget configuration at the pointer.
    # ------------------=
    def configure(self, event):
        self.menu.tk_popup(event.x_root, event.y_root)

    # ------------------------=
    # FUNC: press
    # DESC: Captures the initial pointer offset on exposed client background.
    # ------------------=
    def press(self, event):
        if event.widget == self.root:
            self.grab = (event.x_root - self.root.winfo_x(), event.y_root - self.root.winfo_y())

    # ------------------------=
    # FUNC: place
    # DESC: Keeps the widget reachable after resolution changes.
    # ------------------=
    def place(self, x, y):
        width, height = self.root.winfo_screenwidth(), self.root.winfo_screenheight()
        x = min(max(0, x), max(0, width - 80))
        y = min(max(0, y), max(0, height - 48))
        self.root.geometry(f"+{x}+{y}")

    # ------------------------=
    # FUNC: move
    # DESC: Follows captured mouse motion using native window coordinates.
    # ------------------=
    def move(self, event):
        if self.grab is not None:
            self.place(event.x_root - self.grab[0], event.y_root - self.grab[1])

    # ------------------------=
    # FUNC: release
    # DESC: Ends a background drag and atomically saves the last placement.
    # ------------------=
    def release(self, event):
        if self.grab is not None:
            self.grab = None
            self.save()

    # ------------------------=
    # FUNC: save
    # DESC: Commits the coordinates after native geometry events settle.
    # ------------------=
    def save(self):
        self.root.update_idletasks()
        with open(self.path + ".tmp", "w", encoding="utf-8") as stream:
            json.dump([self.root.winfo_x(), self.root.winfo_y()], stream)
        os.replace(self.path + ".tmp", self.path)

    # ------------------------=
    # FUNC: reset
    # DESC: Returns the widget to the initial desktop corner.
    # ------------------=
    def reset(self):
        self.place(64, 96)
        self.save()


# ------------------------=
# FUNC: attach
# DESC: Adds standard widget behavior and returns a host whose menu can be extended by app code.
# ------------------=
def attach(root, project):
    return WidgetHost(root, project)
