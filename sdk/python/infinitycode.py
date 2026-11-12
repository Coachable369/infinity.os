"""InfinityCode: native Python source workspace and real-widget UI designer."""
import copy
import io
import keyword
import tokenize
import os
import sys
import tkinter as tk
from tkinter import ttk, messagebox
from infinitycode_model import Project, KINDS, WIDTH, HEIGHT, make_widget, export_source, configure_theme, configure_workspace_bounds


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
        configure_workspace_bounds(root)
        root.configure(background="#090F26")
        style = configure_theme(root)
        style.configure("TNotebook", background="#090F26", borderwidth=0)
        style.configure("TNotebook.Tab", padding=(14, 8), font=("Inter",-15,"bold"))
        style.map("TNotebook.Tab", background=[("selected", "#1A2E63")], foreground=[("selected", "#FFFFFF")])
        style.configure("TFrame", background="#101B3A")
        style.configure("TLabel", background="#101B3A", foreground="#FFFFFF", font=("Inter",-15))
        style.configure("Heading.TLabel", font=("Inter",-19,"bold"), foreground="#DAE9FF")
        style.configure("TEntry", fieldbackground="#090F26", foreground="#FFFFFF", padding=4)
        style.configure("TButton", padding=(8,5), font=("Inter",-14))
        style.configure("Accent.TButton", background="#315ED2", foreground="white")
        toolbar = ttk.Frame(root, padding=(12,8))
        toolbar.pack(fill="x")
        ttk.Label(toolbar, text="InfinityCode", font=("Inter",-24,"bold")).pack(side="left", padx=(2,14))
        for group in ((("Open", self.load), ("Save", self.save)), (("Undo", self.undo), ("Redo", self.redo)),
                      (("Check Python", self.check_python), ("Generate Python", self.generate), ("Export", self.export))):
            ttk.Separator(toolbar, orient="vertical").pack(side="left",fill="y",padx=5)
            for title, command in group:
                ttk.Button(toolbar,text=title,width=0,style="Accent.TButton" if title=="Generate Python" else "TButton",
                           command=lambda action=command:self.perform(action)).pack(side="left",padx=2)
        footer = ttk.Frame(root, padding=(12,5))
        footer.pack(side="bottom",fill="x")
        workspace = ttk.Frame(root, padding=8)
        workspace.pack(fill="both", expand=True)
        left = ttk.Frame(workspace, padding=8)
        left.pack(side="left", fill="y")
        ttk.Label(left, text="Toolbox", style="Heading.TLabel").pack(anchor="w", pady=(0,8))
        self.toolbox_filter = tk.StringVar()
        search = ttk.Entry(left,textvariable=self.toolbox_filter,width=16)
        ttk.Label(left,text="Find a control").pack(anchor="w")
        search.pack(fill="x",pady=(2,6))
        search.bind("<ButtonPress-1>",lambda event:search.focus_force(),add="+")
        self.palette = ttk.Frame(left)
        self.palette.pack(fill="x")
        self.palette_items = []
        for kind in KINDS:
            button = ttk.Button(self.palette,text=kind,command=lambda value=kind:self.perform(lambda:self.add(value)))
            self.palette_items.append((kind,button))
        self.toolbox_filter.trace_add("write",self.filter_controls)
        self.filter_controls()
        ttk.Label(left, text="Layers",style="Heading.TLabel").pack(anchor="w", pady=(15,5))
        self.layers = tk.Listbox(left, width=18, background="#090F26", foreground="white",
                                 selectbackground="#3E6FFF", exportselection=False, borderwidth=0)
        self.layers.pack(fill="both", expand=True)
        self.layers.bind("<<ListboxSelect>>", self.layer_selected)
        ttk.Button(left, text="Delete selected", command=lambda: self.perform(self.delete)).pack(fill="x", pady=8)
        right = ttk.Frame(workspace, padding=12, width=190)
        right.pack(side="right", fill="y")
        ttk.Label(right, text="Properties",style="Heading.TLabel").pack(anchor="w",pady=(0,6))
        self.fields = {}
        for label, keys in (("Identity",("id","text")),("Layout",("x","y","width","height")),("Event",("handler",))):
            group = ttk.LabelFrame(right,text=label,padding=6)
            group.pack(fill="x",pady=(0,6))
            for index,key in enumerate(keys):
                row,col = divmod(index,2) if label=="Layout" else (index,0)
                cell = ttk.Frame(group)
                cell.grid(row=row,column=col,sticky="ew",padx=2,pady=2)
                group.columnconfigure(col,weight=1)
                ttk.Label(cell,text="Name" if key=="id" else key.capitalize()).pack(anchor="w")
                field = ttk.Entry(cell,width=7 if label=="Layout" else 17)
                field.pack(fill="x")
                field.bind("<ButtonPress-1>",lambda event,entry=field:entry.focus_force(),add="+")
                field.bind("<Return>",lambda event:self.perform(self.apply))
                self.fields[key] = field
        ttk.Button(right, text="Apply properties",style="Accent.TButton", command=lambda:self.perform(self.apply)).pack(fill="x",pady=(2,4))
        ttk.Button(right, text="Go to handler", command=lambda:self.perform(self.goto_handler)).pack(fill="x",pady=(0,6))
        self.snap = tk.BooleanVar(root, True)
        ttk.Checkbutton(right, text="Snap · 10 px", variable=self.snap).pack(anchor="w")
        ttk.Label(right, text="Drag to move.\nCorner grip resizes.\nCanvas: 640 × 420").pack(anchor="w", pady=16)
        self.tabs = ttk.Notebook(workspace)
        self.tabs.pack(fill="both", expand=True, padx=10)
        designer = ttk.Frame(self.tabs, padding=10)
        self.tabs.add(designer, text="Form Designer")
        ttk.Label(designer, text="Form    /    640 × 420").pack(anchor="w", pady=8)
        viewport = tk.Canvas(designer,background="#070D20",highlightthickness=0,
                             scrollregion=(0,0,WIDTH+24,HEIGHT+24))
        x_scroll = ttk.Scrollbar(designer,orient="horizontal",command=viewport.xview)
        y_scroll = ttk.Scrollbar(designer,orient="vertical",command=viewport.yview)
        x_scroll.pack(side="bottom",fill="x")
        y_scroll.pack(side="right",fill="y")
        viewport.pack(fill="both",expand=True)
        viewport.configure(xscrollcommand=x_scroll.set,yscrollcommand=y_scroll.set)
        self.canvas = tk.Frame(viewport, width=WIDTH, height=HEIGHT, background="#101B3A",
                               highlightthickness=1,highlightbackground="#456B94")
        viewport.create_window(12,12,anchor="nw",window=self.canvas,width=WIDTH,height=HEIGHT)
        self.canvas.pack_propagate(False)
        self.grip = tk.Frame(self.canvas, background="#86C7FF", width=10, height=10)
        self.grip.bind("<ButtonPress-1>", self.resize_start)
        self.grip.bind("<B1-Motion>", self.motion)
        self.grip.bind("<ButtonRelease-1>", self.release)
        source_frame = ttk.Frame(self.tabs)
        self.tabs.add(source_frame, text="Python")
        self.source = tk.Text(source_frame, undo=True, background="#090F26", foreground="#FFFFFF",
                              insertbackground="#86C7FF", font=("IBM Plex Mono",-16), wrap="none", borderwidth=0)
        vertical = ttk.Scrollbar(source_frame, orient="vertical", command=self.source.yview)
        horizontal = ttk.Scrollbar(source_frame, orient="horizontal", command=self.source.xview)
        vertical.pack(side="right", fill="y")
        horizontal.pack(side="bottom", fill="x")
        self.source.pack(fill="both", expand=True)
        self.source.configure(yscrollcommand=vertical.set, xscrollcommand=horizontal.set)
        methods_frame = ttk.Frame(self.tabs)
        self.tabs.add(methods_frame, text="Event Methods")
        ttk.Label(methods_frame, text="Define methods here; enter a method name in the control's Handler property.\n"
                  "Methods take no arguments and access native widgets through controls['id'].").pack(anchor="w", padx=10, pady=10)
        self.methods = tk.Text(methods_frame, undo=True, background="#090F26", foreground="#FFFFFF",
                               insertbackground="#86C7FF", font=("IBM Plex Mono",-16), wrap="none", borderwidth=0)
        methods_scroll = ttk.Scrollbar(methods_frame, orient="vertical", command=self.methods.yview)
        methods_scroll.pack(side="right", fill="y")
        self.methods.pack(fill="both", expand=True)
        self.methods.configure(yscrollcommand=methods_scroll.set)
        self.syntax_jobs = {}
        for editor in (self.source,self.methods):
            for role,color in (("keyword","#91BDFF"),("string","#9CE1C0"),("comment","#8A9BB4"),("number","#EBC293")):
                editor.tag_configure(role,foreground=color)
            editor.bind("<<Modified>>",lambda event,e=editor:self.schedule_syntax(e))
            editor.edit_modified(False)
            editor.tag_configure("diagnostic", background="#602D42", foreground="#FFFFFF")
            editor.bind("<Control-Return>",lambda event:self.check_python())
        ttk.Label(footer,text="Project").pack(side="left",padx=(0,6))
        self.path = ttk.Entry(footer, width=38)
        self.path.insert(0, "/data/project.json" if sys.platform == "infinityos" else os.path.abspath("project.json"))
        self.path.pack(side="left")
        self.status = tk.StringVar(root, "Ready")
        ttk.Label(footer, textvariable=self.status).pack(side="left", padx=12)
        root.protocol("WM_DELETE_WINDOW", self.close)
        self.saved = copy.deepcopy(self.project.data)

    # ------------------------=
    # FUNC: schedule_syntax
    # DESC: Coalesces edits before colorizing a bounded native Python document.
    # ------------------=
    def schedule_syntax(self, editor):
        if not editor.edit_modified():
            return
        editor.edit_modified(False)
        job = self.syntax_jobs.pop(editor,None)
        if job is not None:
            self.root.after_cancel(job)
        self.syntax_jobs[editor] = self.root.after(180,lambda:self.colorize(editor))

    # ------------------------=
    # FUNC: colorize
    # DESC: Applies real Python token ranges without altering source, history, selection or diagnostics.
    # ------------------=
    def colorize(self, editor):
        self.syntax_jobs.pop(editor,None)
        ranges = {role:[] for role in ("keyword","string","comment","number")}
        roles = {tokenize.STRING:"string",tokenize.COMMENT:"comment",tokenize.NUMBER:"number"}
        source = editor.get("1.0","end-1c")
        try:
            for token in tokenize.generate_tokens(io.StringIO(source).readline):
                role = "keyword" if token.type==tokenize.NAME and keyword.iskeyword(token.string) else roles.get(token.type)
                if role:
                    ranges[role].extend((f"{token.start[0]}.{token.start[1]}",f"{token.end[0]}.{token.end[1]}"))
        except (tokenize.TokenError,IndentationError,SyntaxError):
            pass
        for role,indices in ranges.items():
            editor.tag_remove(role,"1.0","end")
            if indices:
                editor.tag_add(role,*indices)
        editor.tag_raise("diagnostic")

    # ------------------------=
    # FUNC: filter_controls
    # DESC: Filters the existing toolbox without changing the current form or selection.
    # ------------------=
    def filter_controls(self, *args):
        query = self.toolbox_filter.get().strip().casefold()
        for kind, button in self.palette_items:
            button.pack_forget()
            if query in kind.casefold():
                button.pack(fill="x", pady=2)

    # ------------------------=
    # FUNC: check_python
    # DESC: Checks source and event methods without execution and navigates to the first precise syntax diagnostic.
    # ------------------=
    def check_python(self):
        for editor,tab,label in ((self.source,1,"Python"),(self.methods,2,"Event Methods")):
            editor.tag_remove("diagnostic","1.0","end")
            try:
                compile(editor.get("1.0","end-1c"),label,"exec")
            except SyntaxError as error:
                line=max(1,error.lineno or 1)
                index=f"{line}.{max(0,(error.offset or 1)-1)}"
                self.tabs.select(tab);editor.mark_set("insert",index)
                editor.tag_add("diagnostic",f"{line}.0",f"{line}.end")
                editor.see(index);editor.focus_set()
                self.status.set(f"{label}:{line}:{error.offset or 1} — {error.msg}")
                return "break"
        self.status.set("Python and event methods: no syntax errors · not executed")
        return "break"

    # ------------------------=
    # FUNC: goto_handler
    # DESC: Navigates to an existing connected method using Python syntax rather than ambiguous text matching.
    # ------------------=
    def goto_handler(self):
        import ast
        name=self.fields["handler"].get().strip()
        if not name:
            self.status.set("Select a control with a connected handler first")
            return
        tree=ast.parse(self.methods.get("1.0","end-1c"))
        for node in tree.body:
            if isinstance(node,(ast.FunctionDef,ast.AsyncFunctionDef)) and node.name==name:
                index=f"{node.lineno}.0"
                self.tabs.select(2);self.methods.mark_set("insert",index)
                self.methods.see(index);self.methods.focus_set()
                self.status.set("Event method: "+name)
                return
        self.status.set("No event method defined: "+name)

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
        self.project.data["methods"] = self.methods.get("1.0", "end-1c")

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
                field.insert(0, str(item.get(key, "")))
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
        self.methods.delete("1.0", "end")
        self.methods.insert("1.0", self.project.data.get("methods", ""))
        self.render()

    # ------------------------=
    # FUNC: generate
    # DESC: Explicitly regenerates standalone Python while confirming replacement of manual source edits.
    # ------------------=
    def generate(self):
        self.sync_source()
        current = self.source.get("1.0", "end-1c")
        if current and current != self.generated and not messagebox.askyesno(
                "Replace Python?", "Replace manually edited Python with the current designer layout? Save or export your edits first.", parent=self.root):
            return
        self.generated = export_source(self.project.data)
        self.source.delete("1.0", "end")
        self.source.insert("1.0", self.generated)
        self.sync_source()
        self.tabs.select(1)
        self.status.set("Python generated · connected event methods included")

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
