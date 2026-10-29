"""InfinityCode: native Python source workspace and real-widget UI designer."""
import copy
import os
import sys
import tkinter as tk
from tkinter import ttk, messagebox
from infinitycode_model import Project, KINDS, WIDTH, HEIGHT, make_widget, export_source, configure_theme


class Studio:
    # ------------------------=
    # FUNC: __init__
    # DESC: Builds the first native Python workspace with shared designer/export geometry and persistent project controls.
    # ------------------=
    def __init__(self, root):
        self.root, self.project = root, Project()
        self.selected, self.drag = None, None
        self.widgets, self.generated = {}, ""
        root.title("InfinityCode — Python & UI Designer")
        root.geometry("1180x760")
        root.minsize(1180, 650)
        root.configure(background="#090F26")
        style = configure_theme(root)
        style.map("TNotebook.Tab", background=[("selected", "#1A2E63")])
        style.configure("TFrame", background="#101B3A")
        style.configure("TLabel", background="#101B3A", foreground="#FFFFFF", font=("Inter", 11))
        toolbar = ttk.Frame(root, padding=12)
        toolbar.pack(fill="x")
        ttk.Label(toolbar, text="InfinityCode   /   PYTHON", font=("Inter", 15, "bold")).pack(side="left", padx=(0, 20))
        for title, command in (("Open", self.load), ("Save", self.save), ("Undo", self.undo),
                               ("Redo", self.redo), ("Generate Python", self.generate), ("Export .py", self.export)):
            ttk.Button(toolbar, text=title, command=lambda action=command: self.perform(action)).pack(side="left", padx=3)
        workspace = ttk.Frame(root, padding=10)
        workspace.pack(fill="both", expand=True)
        left = ttk.Frame(workspace, padding=8)
        left.pack(side="left", fill="y")
        ttk.Label(left, text="TOOLBOX").pack(anchor="w", pady=8)
        for kind in KINDS:
            ttk.Button(left, text=kind, command=lambda value=kind: self.perform(lambda: self.add(value))).pack(fill="x", pady=3)
        ttk.Label(left, text="LAYERS").pack(anchor="w", pady=(20, 8))
        self.layers = tk.Listbox(left, width=18, background="#090F26", foreground="white",
                                 selectbackground="#3E6FFF", exportselection=False, borderwidth=0)
        self.layers.pack(fill="both", expand=True)
        self.layers.bind("<<ListboxSelect>>", self.layer_selected)
        ttk.Button(left, text="Delete selected", command=lambda: self.perform(self.delete)).pack(fill="x", pady=8)
        right = ttk.Frame(workspace, padding=12, width=190)
        right.pack(side="right", fill="y")
        ttk.Label(right, text="PROPERTIES").pack(anchor="w", pady=8)
        self.fields = {}
        for key in ("id", "text", "x", "y", "width", "height"):
            ttk.Label(right, text=key.capitalize()).pack(anchor="w", pady=(10, 3))
            field = ttk.Entry(right, width=20)
            field.pack(fill="x")
            self.fields[key] = field
        ttk.Button(right, text="Apply properties", command=lambda: self.perform(self.apply)).pack(fill="x", pady=16)
        self.snap = tk.BooleanVar(root, True)
        ttk.Checkbutton(right, text="Snap · 10 px", variable=self.snap).pack(anchor="w")
        ttk.Label(right, text="Drag to move.\nCorner grip resizes.\nCanvas: 640 × 420").pack(anchor="w", pady=16)
        self.tabs = ttk.Notebook(workspace)
        self.tabs.pack(fill="both", expand=True, padx=10)
        designer = ttk.Frame(self.tabs, padding=10)
        self.tabs.add(designer, text="Designer")
        ttk.Label(designer, text="Actual native controls · fixed export dimensions").pack(anchor="w", pady=8)
        self.canvas = tk.Frame(designer, width=WIDTH, height=HEIGHT, background="#090F26")
        self.canvas.pack(anchor="nw")
        self.canvas.pack_propagate(False)
        self.grip = tk.Frame(self.canvas, background="#86C7FF", width=10, height=10)
        self.grip.bind("<ButtonPress-1>", self.resize_start)
        self.grip.bind("<B1-Motion>", self.motion)
        self.grip.bind("<ButtonRelease-1>", self.release)
        source_frame = ttk.Frame(self.tabs)
        self.tabs.add(source_frame, text="Python")
        self.source = tk.Text(source_frame, undo=True, background="#090F26", foreground="#FFFFFF",
                              insertbackground="#86C7FF", font=("Menlo", 12), wrap="none", borderwidth=0)
        vertical = ttk.Scrollbar(source_frame, orient="vertical", command=self.source.yview)
        horizontal = ttk.Scrollbar(source_frame, orient="horizontal", command=self.source.xview)
        vertical.pack(side="right", fill="y")
        horizontal.pack(side="bottom", fill="x")
        self.source.pack(fill="both", expand=True)
        self.source.configure(yscrollcommand=vertical.set, xscrollcommand=horizontal.set)
        footer = ttk.Frame(root, padding=10)
        footer.pack(fill="x")
        self.path = ttk.Entry(footer, width=55)
        self.path.insert(0, "/data/project.json" if sys.platform == "infinityos" else os.path.abspath("project.json"))
        self.path.pack(side="left")
        self.status = tk.StringVar(root, "Ready · Save a project or export Python for Text Editor / Run")
        ttk.Label(footer, textvariable=self.status).pack(side="left", padx=12)
        root.protocol("WM_DELETE_WINDOW", self.close)
        self.saved = copy.deepcopy(self.project.data)

    # ------------------------=
    # FUNC: perform
    # DESC: Reports failed editor actions without losing the project or swallowing diagnostics.
    # ------------------=
    def perform(self, action):
        try:
            action()
        except (ValueError, OSError, tk.TclError, SyntaxError) as error:
            self.status.set(str(error))
            messagebox.showerror("InfinityCode", str(error), parent=self.root)

    # ------------------------=
    # FUNC: item
    # DESC: Resolves the selected element by stable identifier.
    # ------------------=
    def item(self):
        return next((item for item in self.project.data["elements"] if item["id"] == self.selected), None)

    # ------------------------=
    # FUNC: sync_source
    # DESC: Retains manual Python edits when designer transactions or saves occur.
    # ------------------=
    def sync_source(self):
        self.project.data["source"] = self.source.get("1.0", "end-1c")

    # ------------------------=
    # FUNC: render
    # DESC: Recreates actual exported widget types and binds editor-only manipulation gestures.
    # ------------------=
    def render(self):
        for widget in self.widgets.values():
            widget.destroy()
        self.widgets.clear()
        self.layers.delete(0, "end")
        for item in self.project.data["elements"]:
            identifier = item["id"]
            widget = make_widget(self.canvas, item)
            self.widgets[identifier] = widget
            widget.bind("<ButtonPress-1>", lambda event, key=identifier: self.start(event, key))
            widget.bind("<B1-Motion>", self.motion)
            widget.bind("<ButtonRelease-1>", self.release)
            self.layers.insert("end", identifier)
        self.select(self.selected)

    # ------------------------=
    # FUNC: select
    # DESC: Synchronizes layer selection, resize grip and property fields.
    # ------------------=
    def select(self, identifier):
        self.selected = identifier
        item = self.item()
        self.layers.selection_clear(0, "end")
        for key, field in self.fields.items():
            field.delete(0, "end")
            if item:
                field.insert(0, str(item[key]))
        self.grip.place_forget()
        if item:
            index = self.project.data["elements"].index(item)
            self.layers.selection_set(index)
            self.grip.place(x=item["x"] + item["width"] - 10, y=item["y"] + item["height"] - 10)
            self.grip.lift()

    # ------------------------=
    # FUNC: layer_selected
    # DESC: Selects the same stable element from the layer list as from the canvas.
    # ------------------=
    def layer_selected(self, event):
        choice = self.layers.curselection()
        if choice:
            self.select(self.layers.get(choice[0]))

    # ------------------------=
    # FUNC: add
    # DESC: Inserts a control and selects it for immediate placement.
    # ------------------=
    def add(self, kind):
        self.sync_source()
        self.selected = self.project.add(kind)
        self.render()
        self.status.set("Control added · Generate Python to update source")

    # ------------------------=
    # FUNC: start
    # DESC: Starts a single undoable drag without activating the design-time widget.
    # ------------------=
    def start(self, event, identifier):
        self.sync_source()
        self.select(identifier)
        self.drag = (event.x_root, event.y_root, copy.deepcopy(self.item()), False)
        return "break"

    # ------------------------=
    # FUNC: resize_start
    # DESC: Starts a snapped resize transaction from the selected control's grip.
    # ------------------=
    def resize_start(self, event):
        if self.item():
            self.start(event, self.selected)
            self.drag = (*self.drag[:3], True)
        return "break"

    # ------------------------=
    # FUNC: motion
    # DESC: Shows bounded snapped geometry while dragging without growing undo history per pixel.
    # ------------------=
    def motion(self, event):
        if not self.drag:
            return "break"
        x, y, original, resize = self.drag
        step = 10 if self.snap.get() else 1
        dx, dy = round((event.x_root - x) / step) * step, round((event.y_root - y) / step) * step
        item = copy.deepcopy(original)
        if resize:
            item["width"] = max(20, min(WIDTH - item["x"], original["width"] + dx))
            item["height"] = max(20, min(HEIGHT - item["y"], original["height"] + dy))
        else:
            item["x"] = max(0, min(WIDTH - item["width"], original["x"] + dx))
            item["y"] = max(0, min(HEIGHT - item["height"], original["y"] + dy))
        self.widgets[self.selected].place(**{key: item[key] for key in ("x", "y", "width", "height")})
        for key in ("x", "y", "width", "height"):
            self.fields[key].delete(0, "end")
            self.fields[key].insert(0, str(item[key]))
        self.grip.place(x=item["x"] + item["width"] - 10, y=item["y"] + item["height"] - 10)
        return "break"

    # ------------------------=
    # FUNC: release
    # DESC: Commits a completed drag as one validated operation.
    # ------------------=
    def release(self, event):
        if self.drag:
            self.drag = None
            self.perform(self.apply)
        return "break"

    # ------------------------=
    # FUNC: apply
    # DESC: Applies validated properties atomically, keeping invalid edits out of the model.
    # ------------------=
    def apply(self):
        if not self.item():
            return
        self.sync_source()
        value = copy.deepcopy(self.project.data)
        item = next(item for item in value["elements"] if item["id"] == self.selected)
        for key, field in self.fields.items():
            item[key] = int(field.get()) if key in ("x", "y", "width", "height") else field.get()
        self.project.replace(value)
        self.selected = item["id"]
        self.render()
        self.status.set("Designer updated · Python changes only when Generate Python is selected")

    # ------------------------=
    # FUNC: delete
    # DESC: Removes the selected control with undo support.
    # ------------------=
    def delete(self):
        self.sync_source()
        value = copy.deepcopy(self.project.data)
        value["elements"] = [item for item in value["elements"] if item["id"] != self.selected]
        self.project.replace(value)
        self.selected = None
        self.render()

    # ------------------------=
    # FUNC: undo
    # DESC: Restores the previous designer transaction.
    # ------------------=
    def undo(self):
        self.sync_source()
        if self.project.history():
            self.restore()

    # ------------------------=
    # FUNC: redo
    # DESC: Reapplies an undone designer transaction.
    # ------------------=
    def redo(self):
        if self.project.history(True):
            self.restore()

    # ------------------------=
    # FUNC: restore
    # DESC: Restores source and designer after history or project loading.
    # ------------------=
    def restore(self):
        self.source.delete("1.0", "end")
        self.source.insert("1.0", self.project.data["source"])
        self.render()

    # ------------------------=
    # FUNC: generate
    # DESC: Explicitly regenerates standalone Python while confirming replacement of manual source edits.
    # ------------------=
    def generate(self):
        current = self.source.get("1.0", "end-1c")
        if current and current != self.generated and not messagebox.askyesno(
                "Replace Python?", "Replace manually edited Python with the current designer layout? Save or export your edits first.", parent=self.root):
            return
        self.generated = export_source(self.project.data)
        self.source.delete("1.0", "end")
        self.source.insert("1.0", self.generated)
        self.sync_source()
        self.tabs.select(1)
        self.status.set("Python generated · attach handlers using the controls dictionary")

    # ------------------------=
    # FUNC: save
    # DESC: Saves both manual Python and layout to the user-selected project path.
    # ------------------=
    def save(self):
        self.sync_source()
        self.project.save(self.path.get())
        self.saved = copy.deepcopy(self.project.data)
        self.status.set("Saved " + self.path.get())

    # ------------------------=
    # FUNC: discard
    # DESC: Confirms leaving unsaved project work rather than silently discarding it.
    # ------------------=
    def discard(self):
        self.sync_source()
        return self.project.data == self.saved or messagebox.askyesno("Unsaved project", "Discard unsaved changes?", parent=self.root)

    # ------------------------=
    # FUNC: load
    # DESC: Opens a validated project without losing current work on read failure.
    # ------------------=
    def load(self):
        if self.discard():
            self.project.load(self.path.get())
            self.restore()
            self.saved = copy.deepcopy(self.project.data)
            self.status.set("Opened " + self.path.get())

    # ------------------------=
    # FUNC: export
    # DESC: Syntax-checks and exports edited Python without executing it inside the designer.
    # ------------------=
    def export(self):
        source = self.source.get("1.0", "end-1c")
        if not source.strip():
            self.generate()
            source = self.source.get("1.0", "end-1c")
        path = os.path.splitext(self.path.get())[0] + ".py"
        compile(source, path, "exec")
        with open(path + ".tmp", "w", encoding="utf-8") as stream:
            stream.write(source)
        os.replace(path + ".tmp", path)
        self.status.set("Exported " + path + " · syntax checked; not executed")

    # ------------------------=
    # FUNC: close
    # DESC: Keeps unsaved projects open unless the user explicitly discards them.
    # ------------------=
    def close(self):
        if self.discard():
            self.root.destroy()


# ------------------------=
# FUNC: main
# DESC: Enters the existing native Tk application lifecycle, without an alternate execution framework.
# ------------------=
def main():
    root = tk.Tk()
    Studio(root)
    root.mainloop()


if __name__ == "__main__":
    main()
