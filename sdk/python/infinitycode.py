"""InfinityCode: native Python source workspace and real-widget UI designer."""
import copy
import io
import keyword
import os
import sys
import tkinter as tk
from tkinter import ttk, messagebox
from infinitycode_model import counter_widget, form_size, Project, KINDS, WIDTH, HEIGHT, make_widget, export_source, configure_theme, configure_workspace_bounds

from infinity_uikit_artwork import COLORS
from infinity_uikit import MENU_HEIGHT, TOOLBAR_HEIGHT, workspace_tabs, workspace_controls, workspace_menu


class Studio:
    # ------------------------=
    # FUNC: __init__
    # DESC: Builds the first native Python workspace with shared designer/export geometry and persistent project controls.
    # ------------------=
    def __init__(self, root):
        self.root, self.project = root, Project()
        self.selected, self.drag = None, None
        self.widgets, self.generated = {}, ""
        self.zoom = 1.0
        self.base_scale = 2 if root.winfo_screenwidth()>=2560 and root.winfo_screenheight()>=1440 else 1
        self.scale = self.base_scale
        root.title("InfinityCode")
        configure_workspace_bounds(root)
        root.configure(background=COLORS["canvas"])
        style = configure_theme(root)
        self.style = style
        self.tab_images = workspace_tabs(root, style, self.scale)
        style.configure("TNotebook", background=COLORS["canvas"], borderwidth=0)
        style.configure("TNotebook.Tab", padding=(14, 8), font=("Inter",-15,"bold"))
        style.map("TNotebook.Tab", background=[("selected", "#1A2E63")], foreground=[("selected", COLORS["text"])])
        style.configure("TFrame", background=COLORS["panel"])
        style.configure("TLabel", background=COLORS["panel"], foreground=COLORS["text"], font=("Inter",-15))
        style.configure("Heading.TLabel", font=("Inter",-19,"bold"), foreground=COLORS["text"])
        style.configure("TEntry", fieldbackground=COLORS["canvas"], foreground=COLORS["text"], padding=4)
        style.configure("TButton", padding=(8,5), font=("Inter",-14))
        style.configure("Accent.TButton", background=COLORS["accent"], foreground="white")
        style.layout("Workspace.TMenubutton", [("Menubutton.padding", {"sticky":"nswe", "children":
            [("Menubutton.label", {"sticky":"w"})]})])
        style.configure("Workspace.TMenubutton", background=COLORS["panel"], foreground=COLORS["text"],
                        padding=(12,6), font=("Inter",-15), relief="flat")
        style.map("Workspace.TMenubutton", background=[("active",COLORS["accent"]),("focus",COLORS["accent"])])
        bar = tk.Frame(root, bg=COLORS["panel"], padx=8, height=MENU_HEIGHT)
        self.menu_bar = bar
        bar.pack_propagate(False)
        bar.pack(fill="x")
        for title, actions in (
            ("File", (("New form", self.new_form), ("New widget", self.new_widget),
                      ("Open...", self.open_project), ("Save", self.save), ("Save As...", self.save_as),
                      ("Export Python", self.export), ("Exit", self.close))),
            ("Edit", (("Undo", lambda:self.edit_command("Undo")), ("Redo", lambda:self.edit_command("Redo")),
                      ("Cut", lambda: self.edit_command("Cut")), ("Copy", lambda: self.edit_command("Copy")),
                      ("Paste", lambda: self.edit_command("Paste")), ("Delete control", self.delete),
                      ("View code behind", self.code_behind))),
            ("View", (("Zoom in   Cmd +", lambda:self.set_zoom(self.zoom+0.1)),
                      ("Zoom out   Cmd -", lambda:self.set_zoom(self.zoom-0.1)),
                      ("Actual size   Cmd 0", lambda:self.set_zoom(1.0)))),
            ("Help", (("Designer help", self.help),))):
            button = ttk.Menubutton(bar, text=title, style="Workspace.TMenubutton", width=0)
            menu = tk.Menu(button, tearoff=False, bg=COLORS["panel"], fg="white", activebackground=COLORS["accent"])
            for label, action in actions:
                menu.add_command(label=label, command=lambda fn=action:self.perform(fn))
            workspace_menu(button, menu)
            button.pack(side="right" if title=="Help" else "left")
        self.control_copy = None
        toolbar = ttk.Frame(root, padding=(12,6), height=TOOLBAR_HEIGHT)
        self.toolbar = toolbar
        toolbar.pack_propagate(False)
        toolbar.pack(fill="x")
        ttk.Label(toolbar,text="Design workspace",style="Heading.TLabel").pack(side="left",padx=(0,14))
        for title,action in (("Open",self.open_project),("Save",self.save),("Check Python",self.check_python),("Generate Python",self.generate)):
            ttk.Button(toolbar,text=title,style="Accent.TButton" if title=="Generate Python" else "TButton",
                       command=lambda fn=action:self.perform(fn)).pack(side="left",padx=3)
        footer = ttk.Frame(root, padding=(12,5))
        footer.pack(side="bottom",fill="x")
        workspace = ttk.Frame(root, padding=8)
        workspace.pack(fill="both", expand=True)
        left = ttk.Frame(workspace, padding=8, width=210)
        self.toolbox = left
        left.pack_propagate(False)
        left.pack(side="left", fill="y")
        toolbox_view = tk.Canvas(left,background=COLORS["panel"],highlightthickness=0)
        toolbox_scroll = ttk.Scrollbar(left,orient="vertical",command=toolbox_view.yview)
        toolbox_scroll.pack(side="right",fill="y")
        toolbox_view.configure(yscrollcommand=toolbox_scroll.set)
        toolbox_view.pack(fill="both",expand=True)
        left = ttk.Frame(toolbox_view)
        toolbox_item = toolbox_view.create_window(0,0,window=left,anchor="nw")
        left.bind("<Configure>",lambda event:toolbox_view.configure(scrollregion=toolbox_view.bbox("all")))
        toolbox_view.bind("<Configure>",lambda event:toolbox_view.itemconfigure(toolbox_item,width=event.width))
        ttk.Label(left, text="Toolbox", style="Heading.TLabel").pack(anchor="w", pady=(0,8))
        self.toolbox_filter = tk.StringVar()
        search = ttk.Entry(left,textvariable=self.toolbox_filter,width=16)
        ttk.Label(left,text="Filter controls").pack(anchor="w")
        search.pack(fill="x",pady=(2,6))
        search.bind("<ButtonPress-1>",lambda event:search.focus_force(),add="+")
        self.palette = ttk.Frame(left)
        self.palette.pack(fill="x")
        self.palette_items = []
        for kind in KINDS:
            button = ttk.Button(self.palette,text=kind,command=lambda value=kind:self.perform(lambda:self.add(value)))
            button.bind("<ButtonPress-1>",lambda event:event.widget.grab_set())
            button.bind("<ButtonRelease-1>", lambda event, k=kind: self.drop_control(event, k))
            self.palette_items.append((kind,button))
        self.toolbox_filter.trace_add("write",self.filter_controls)
        self.filter_controls()
        ttk.Label(left, text="Layers",style="Heading.TLabel").pack(anchor="w", pady=(15,5))
        self.layers = tk.Listbox(left, width=18, height=4, background=COLORS["canvas"], foreground="white",
                                 selectbackground=COLORS["accent"], exportselection=False, borderwidth=0)
        self.layers.pack(fill="both", expand=True)
        self.layers.bind("<<ListboxSelect>>", self.layer_selected)
        ttk.Button(left, text="Delete selected", command=lambda: self.perform(self.delete)).pack(fill="x", pady=8)
        right = ttk.Frame(workspace, padding=12, width=260)
        self.inspector = right
        right.pack_propagate(False)
        right.pack(side="right", fill="y")
        inspector_view = tk.Canvas(right,background=COLORS["panel"],highlightthickness=0)
        inspector_scroll = ttk.Scrollbar(right,orient="vertical",command=inspector_view.yview)
        inspector_scroll.pack(side="right",fill="y")
        inspector_view.configure(yscrollcommand=inspector_scroll.set)
        inspector_view.pack(fill="both",expand=True)
        right=ttk.Frame(inspector_view)
        inspector_item=inspector_view.create_window(0,0,window=right,anchor="nw")
        right.bind("<Configure>",lambda event:inspector_view.configure(scrollregion=inspector_view.bbox("all")))
        inspector_view.bind("<Configure>",lambda event:inspector_view.itemconfigure(inspector_item,width=event.width))
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
        ttk.Checkbutton(right, text="Snap to 10 px", variable=self.snap).pack(anchor="w")
        self.tabs = ttk.Notebook(workspace, style="Workspace.TNotebook")
        self.tabs.pack(fill="both", expand=True, padx=10)
        designer = ttk.Frame(self.tabs, padding=10)
        self.tabs.add(designer, text="Form Designer")
        self.form_label = tk.StringVar(root, "Application form   640 x 420")
        ttk.Label(designer, textvariable=self.form_label).pack(anchor="w", pady=8)
        viewport = tk.Canvas(designer,background=COLORS["canvas"],highlightthickness=0,
                             scrollregion=(0,0,WIDTH+24,HEIGHT+24))
        x_scroll = ttk.Scrollbar(designer,orient="horizontal",command=viewport.xview)
        y_scroll = ttk.Scrollbar(designer,orient="vertical",command=viewport.yview)
        x_scroll.pack(side="bottom",fill="x")
        y_scroll.pack(side="right",fill="y")
        viewport.pack(fill="both",expand=True)
        viewport.configure(xscrollcommand=x_scroll.set,yscrollcommand=y_scroll.set)
        self.canvas = tk.Frame(viewport, width=WIDTH, height=HEIGHT, background=COLORS["panel"],
                               highlightthickness=1,highlightbackground=COLORS["border"])
        self.viewport = viewport
        self.surface = viewport.create_window(12,12,anchor="nw",window=self.canvas,width=WIDTH,height=HEIGHT)
        self.canvas.pack_propagate(False)
        self.form_drag = None
        self.form_grips = []
        for axis in ("e","s","se"):
            handle = tk.Frame(viewport, background=COLORS["focus"], width=10, height=10)
            marker = viewport.create_window(0,0,window=handle)
            self.form_grips.append((axis, marker))
            handle.bind("<ButtonPress-1>", lambda event,a=axis:self.begin_form_resize(event,a))
            handle.bind("<B1-Motion>", self.size_form)
            handle.bind("<ButtonRelease-1>", self.finish_form_resize)
        viewport.bind("<Configure>", lambda event:self.layout_form())
        self.grip = tk.Frame(self.canvas, background=COLORS["focus"], width=10, height=10)
        self.grip.bind("<ButtonPress-1>", self.resize_start)
        self.grip.bind("<B1-Motion>", self.motion)
        self.grip.bind("<ButtonRelease-1>", self.release)
        source_frame = ttk.Frame(self.tabs)
        self.tabs.add(source_frame, text="Code")
        self.source = tk.Text(source_frame, undo=True, background=COLORS["canvas"], foreground=COLORS["text"],
                              insertbackground=COLORS["focus"], font=("IBM Plex Mono",-16), wrap="none", borderwidth=0)
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
        self.methods = tk.Text(methods_frame, undo=True, background=COLORS["canvas"], foreground=COLORS["text"],
                               insertbackground=COLORS["focus"], font=("IBM Plex Mono",-16), wrap="none", borderwidth=0)
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
            editor.tag_configure("diagnostic", background="#602D42", foreground=COLORS["text"])
            editor.bind("<Control-Return>",lambda event:self.check_python())
        ttk.Label(footer,text="Project").pack(side="left",padx=(0,6))
        self.path = ttk.Entry(footer, width=38)
        self.path.insert(0, "/data/project.yaml" if sys.platform == "infinityos" else os.path.abspath("project.yaml"))
        self.path.pack(side="left")
        self.status = tk.StringVar(root, "Ready")
        ttk.Label(footer, textvariable=self.status).pack(side="left", padx=12)
        self.popup = tk.Menu(root, tearoff=False, bg=COLORS["panel"], fg="white", activebackground=COLORS["accent"])
        self.popup.add_command(label="View / edit code behind",command=lambda:self.perform(self.code_behind))
        self.popup.add_command(label="Save project and code",command=lambda:self.perform(self.save))
        self.canvas.bind("<Button-3>",self.context)
        root.protocol("WM_DELETE_WINDOW", self.close)
        self.saved = copy.deepcopy(self.project.data)

        for modifier in ("Mod4", "Control") + (("Command",) if sys.platform == "darwin" else ()):
            root.bind(f"<{modifier}-plus>",lambda event:self.set_zoom(self.zoom+0.1))
            root.bind(f"<{modifier}-equal>",lambda event:self.set_zoom(self.zoom+0.1))
            root.bind(f"<{modifier}-minus>",lambda event:self.set_zoom(self.zoom-0.1))
            root.bind(f"<{modifier}-0>",lambda event:self.set_zoom(1.0))
        self.set_zoom(1.0)

    # ------------------------=
    # FUNC: set_zoom
    # DESC: Scale workspace controls and design preview without changing saved coordinates or shell chrome.
    # ------------------=
    def set_zoom(self, value):
        self.sync_source()
        self.zoom = min(1.75, max(0.75, round(value,2)))
        self.scale = self.zoom*self.base_scale
        size = round(18*self.scale)
        import tkinter.font as tkfont
        for name in ("TkDefaultFont","TkTextFont","TkMenuFont","TkHeadingFont"):
            tkfont.nametofont(name,root=self.root).configure(family="Inter",size=-size)
        self.tab_images=workspace_tabs(self.root,self.style,self.scale)
        self.control_images=workspace_controls(self.root,self.style,self.scale)
        self.style.configure(".", font=("Inter",-size))
        for name in ("TLabel", "TButton", "TEntry", "Workspace.TMenubutton", "Workspace.TNotebook.Tab"):
            self.style.configure(name,font=("Inter",-size))
        self.style.configure("Workspace.TNotebook.Tab",padding=(round(24*self.scale),round(9*self.scale)))
        self.style.configure("Heading.TLabel",font=("Inter",-round(20*self.scale),"bold"))
        self.toolbox.configure(width=round(210*self.scale))
        self.inspector.configure(width=round(260*self.scale))
        self.menu_bar.configure(height=round(MENU_HEIGHT*self.scale))
        self.toolbar.configure(height=round(TOOLBAR_HEIGHT*self.scale))
        self.layers.configure(font=("Inter",-size))
        for editor in (self.source,self.methods):
            editor.configure(font=("IBM Plex Mono",-round(18*self.scale)))
        self.render()
        self.status.set(f"Zoom {round(self.zoom*100)}%")
        return "break"

    # ------------------------=
    # FUNC: preview_geometry
    # DESC: Convert logical form coordinates to zoomed preview pixels without mutating the project.
    # ------------------=
    def preview_geometry(self,item):
        return {key:round(item[key]*self.scale) for key in ("x","y","width","height")}

    # ------------------------=
    # FUNC: place_grip
    # DESC: Keep the selected resize handle fixed in size at the zoomed control corner.
    # ------------------=
    def place_grip(self,item):
        self.grip.place(x=round((item["x"]+item["width"])*self.scale)-10,
                        y=round((item["y"]+item["height"])*self.scale)-10)


    # ------------------------=
    # FUNC: layout_form
    # DESC: Center the editable surface and its edge handles inside the available viewport.
    # ------------------=
    def layout_form(self, size=None):
        width,height = size or form_size(self.project.data)
        width,height = round(width*self.scale),round(height*self.scale)
        vw=max(self.viewport.winfo_width(),width+32)
        vh=max(self.viewport.winfo_height(),height+32)
        x,y=(vw-width)//2,(vh-height)//2
        self.viewport.coords(self.surface,x,y)
        self.viewport.itemconfigure(self.surface,width=width,height=height)
        self.viewport.configure(scrollregion=(0,0,vw,vh))
        for axis,marker in self.form_grips:
            self.viewport.coords(marker,x+(width if "e" in axis else width//2),y+(height if "s" in axis else height//2))

    # ------------------------=
    # FUNC: begin_form_resize
    # DESC: Capture the initial form dimensions for a single resize transaction.
    # ------------------=
    def begin_form_resize(self,event,axis):
        self.sync_source()
        event.widget.grab_set()
        self.form_drag=(axis,event.x_root,event.y_root,*form_size(self.project.data))

    # ------------------------=
    # FUNC: size_form
    # DESC: Preview form bounds while keeping each existing control inside the surface.
    # ------------------=
    def size_form(self,event):
        if self.form_drag is None:
            return
        axis,x,y,w,h=self.form_drag
        items=self.project.data["elements"]
        min_w=max([64]+[i["x"]+i["width"] for i in items])
        min_h=max([64]+[i["y"]+i["height"] for i in items])
        # A centered edge moves by half the change in the full dimension.
        width=min(4096,max(min_w,w+round(2*(event.x_root-x)/self.scale))) if "e" in axis else w
        height=min(4096,max(min_h,h+round(2*(event.y_root-y)/self.scale))) if "s" in axis else h
        self.layout_form((width,height))
        return width,height

    # ------------------------=
    # FUNC: finish_form_resize
    # DESC: Commit the new form dimensions as one undoable operation.
    # ------------------=
    def finish_form_resize(self,event):
        event.widget.grab_release()
        size=self.size_form(event)
        if size:
            self.form_drag=None
            self.project.replace(dict(self.project.data,width=size[0],height=size[1]))
            self.render()

    # ------------------------=
    # FUNC: drop_control
    # DESC: Place a toolbox item at its drop position or defer to the normal click binding.
    # ------------------=
    def drop_control(self,event,kind):
        event.widget.grab_release()
        x=round((event.x_root-self.canvas.winfo_rootx())/self.scale)
        y=round((event.y_root-self.canvas.winfo_rooty())/self.scale)
        width,height=form_size(self.project.data)
        if 0<=x<width and 0<=y<height:
            if self.snap.get():
                x,y=round(x/10)*10,round(y/10)*10
            event.widget.state(["!pressed"])
            self.perform(lambda:self.add(kind,x,y))
            return "break"

    # ------------------------=
    # FUNC: context
    # DESC: Open the design context menu for a form or control.
    # ------------------=
    def context(self,event,key=None):
        self.select(key)
        self.popup.tk_popup(event.x_root,event.y_root)
        return "break"

    # ------------------------=
    # FUNC: code_behind
    # DESC: Open the selected event method and create its binding on first edit.
    # ------------------=
    def code_behind(self):
        item=self.item()
        if item and item["kind"]!="Label":
            if not item.get("handler"):
                name="on_"+item["id"]
                self.fields["handler"].insert(0,name)
                self.apply()
            name=self.item()["handler"]
            import ast
            if not any(isinstance(n,ast.FunctionDef) and n.name==name for n in ast.parse(self.methods.get("1.0","end-1c")).body):
                self.methods.insert("end",f"\n\ndef {name}():\n    pass\n")
            self.goto_handler()
        else:
            self.tabs.select(2)
            self.methods.focus_set()

    # ------------------------=
    # FUNC: edit_command
    # DESC: Edit focused text or copy and paste selected design elements.
    # ------------------=
    def edit_command(self,action):
        target=self.root.focus_get()
        if isinstance(target,(tk.Text,tk.Entry,ttk.Entry)):
            target.event_generate("<<"+action+">>")
        elif action in ("Undo","Redo"):
            (self.undo if action=="Undo" else self.redo)()
        elif action in ("Copy","Cut") and self.item():
            self.control_copy=copy.deepcopy(self.item())
            if action=="Cut":
                self.delete()
        elif action=="Paste" and self.control_copy:
            self.sync_source()
            original=self.control_copy
            value=copy.deepcopy(self.project.data)
            number=1
            names={i["id"] for i in value["elements"]}
            while original["id"]+str(number) in names:
                number+=1
            self.selected=original["id"]+str(number)
            w,h=form_size(value)
            value["elements"].append(dict(original,id=self.selected,x=min(w-original["width"],original["x"]+10),y=min(h-original["height"],original["y"]+10)))
            self.project.replace(value)
            self.render()

    # ------------------------=
    # FUNC: new_form
    # DESC: Start an empty application form after protecting unsaved work.
    # ------------------=
    def new_form(self):
        if self.discard():
            self.project=Project()
            self.selected=None
            self.generated=""
            self.path.delete(0,"end")
            self.path.insert(0,"/data/project.yaml" if sys.platform=="infinityos" else os.path.abspath("project.yaml"))
            self.restore()
            self.saved=copy.deepcopy(self.project.data)
            self.tabs.select(0)

    # ------------------------=
    # FUNC: choose_path
    # DESC: Choose a project path in the application namespace.
    # ------------------=
    def choose_path(self,title):
        from tkinter import simpledialog
        return simpledialog.askstring(title,"Project path",initialvalue=self.path.get(),parent=self.root)

    # ------------------------=
    # FUNC: save_as
    # DESC: Persist the project to a new path and switch only after success.
    # ------------------=
    def save_as(self):
        path=self.choose_path("Save project as")
        if path:
            self.sync_source()
            self.project.save(path)
            self.path.delete(0,"end")
            self.path.insert(0,path)
            self.saved=copy.deepcopy(self.project.data)
            self.status.set("Saved "+path)

    # ------------------------=
    # FUNC: open_project
    # DESC: Open a selected project without replacing the workspace on failure.
    # ------------------=
    def open_project(self):
        path=self.choose_path("Open project")
        if path and self.discard():
            project=Project()
            project.load(path)
            self.project=project
            self.selected=None
            self.path.delete(0,"end")
            self.path.insert(0,path)
            self.restore()
            self.saved=copy.deepcopy(project.data)
            self.tabs.select(0)

    # ------------------------=
    # FUNC: help
    # DESC: Display the designer gestures and project save workflow.
    # ------------------=
    def help(self):
        messagebox.showinfo("InfinityCode","Drag a toolbox control onto the form. Drag a blue form handle to resize. Right-click a control to edit its event method. File → Save stores the layout and code together.",parent=self.root)

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
        import tokenize
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
        self.status.set("Python and event methods: no syntax errors | not executed")
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
    # FUNC: new_widget
    # DESC: Opens a writable copy of the SDK widget example with its connected methods.
    # ------------------=
    def new_widget(self):
        if not self.discard():
            return
        self.project = Project()
        self.project.replace(counter_widget())
        self.project.data["source"] = export_source(self.project.data)
        self.generated = self.project.data["source"]
        self.selected = None
        self.path.delete(0, "end")
        self.path.insert(0, "/data/widget.yaml" if sys.platform == "infinityos" else os.path.abspath("widget.yaml"))
        self.restore()
        self.tabs.select(0)
        self.status.set("Widget form  |  Select a control to edit its event method")

    # ------------------------=
    # FUNC: render
    # DESC: Recreates actual exported widget types and binds editor-only manipulation gestures.
    # ------------------=
    def render(self):
        width, height = form_size(self.project.data)
        kind = "Widget" if self.project.data.get("form_type") == "widget" else "Application"
        self.form_label.set(f"{kind} form   {width} x {height}")
        self.layout_form()
        for widget in self.widgets.values():
            widget.destroy()
        self.widgets.clear()
        self.layers.delete(0, "end")
        for item in self.project.data["elements"]:
            identifier = item["id"]
            widget = make_widget(self.canvas, item)
            widget.place(**self.preview_geometry(item))
            self.widgets[identifier] = widget
            widget.bind("<ButtonPress-1>", lambda event, key=identifier: self.start(event, key))
            widget.bind("<Button-3>", lambda event,key=identifier:self.context(event,key))
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
            self.place_grip(item)
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
    def add(self, kind, x=20, y=20):
        self.sync_source()
        self.selected = self.project.add(kind,x,y)
        self.render()
        self.status.set("Control added | Generate Python to update source")

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
        width,height=form_size(self.project.data)
        if not self.drag:
            return "break"
        x, y, original, resize = self.drag
        step = 10 if self.snap.get() else 1
        dx, dy = round((event.x_root - x) / (step*self.scale)) * step, round((event.y_root - y) / (step*self.scale)) * step
        item = copy.deepcopy(original)
        if resize:
            item["width"] = max(20, min(width - item["x"], original["width"] + dx))
            item["height"] = max(20, min(height - item["y"], original["height"] + dy))
        else:
            item["x"] = max(0, min(width - item["width"], original["x"] + dx))
            item["y"] = max(0, min(height - item["height"], original["y"] + dy))
        self.widgets[self.selected].place(**self.preview_geometry(item))
        for key in ("x", "y", "width", "height"):
            self.fields[key].delete(0, "end")
            self.fields[key].insert(0, str(item[key]))
        self.place_grip(item)
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
        self.status.set("Designer updated | Python changes only when Generate Python is selected")

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
        self.status.set("Python generated | connected event methods included")

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
        self.status.set("Exported " + path + " | syntax checked; not executed")

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
