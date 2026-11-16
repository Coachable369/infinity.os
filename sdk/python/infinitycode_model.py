"""Versioned InfinityCode projects and deterministic native Tk source export."""
import copy
import json
import keyword

KINDS = ("Label", "Button", "Entry", "Checkbutton", "Radiobutton", "Combobox")
WIDTH, HEIGHT = 640, 420
THEME = dict(background="#101B3A", foreground="#FFFFFF", font=("Inter", 11),
             fieldbackground="#090F26", bordercolor="#86C7FF", lightcolor="#1A2E63", darkcolor="#090F26")

# ------------------------=
# FUNC: form_size
# DESC: Resolves the shared design and runtime dimensions for the selected form.
# ------------------=
def form_size(project):
    return (360, 200) if project.get("form_type") == "widget" else (WIDTH, HEIGHT)


# ------------------------=
# FUNC: counter_widget
# DESC: Supplies an editable widget with state and two connected Python event methods.
# ------------------=
def counter_widget():
    elements = [
        dict(id="title", kind="Label", text="Session counter", x=20, y=16, width=300, height=32),
        dict(id="count", kind="Label", text="0", x=20, y=58, width=300, height=42),
        dict(id="add", kind="Button", text="Add one", handler="increment", x=20, y=126, width=148, height=40),
        dict(id="reset", kind="Button", text="Reset", handler="reset_count", x=184, y=126, width=148, height=40),
    ]
    return {"version": 1, "form_type": "widget", "elements": elements, "source": "",
            "methods": """total = 0

# ------------------------=
# FUNC: increment
# DESC: Adds one to the displayed count.
# ------------------=
def increment():
    global total
    total += 1
    controls['count'].configure(text=str(total))

# ------------------------=
# FUNC: reset_count
# DESC: Resets the counter and its label together.
# ------------------=
def reset_count():
    global total
    total = 0
    controls['count'].configure(text='0')
"""}


# ------------------------=
# FUNC: configure_workspace_bounds
# DESC: Fits initial and minimum client bounds to the native window manager's available work area.
# ------------------=
def configure_workspace_bounds(root):
    maximum_width, maximum_height = root.maxsize()
    width, height = min(1360,maximum_width,root.winfo_screenwidth()-32), min(840,maximum_height,root.winfo_screenheight()-108)
    root.minsize(min(800, width), min(560, height))
    root.geometry(f"{width}x{height}")

# ------------------------=
# FUNC: configure_theme
# DESC: Uses identical widget styling in the designer and exported native Tk application.
# ------------------=
def configure_theme(root):
    from tkinter import ttk
    style = ttk.Style(root)
    style.theme_use("clam")
    style.configure(".", **THEME)
    style.map("TButton", background=[("active", "#1A2E63"), ("pressed", "#3E6FFF")])
    return style

# ------------------------=
# FUNC: validate
# DESC: Validates bounded project data before it changes the designer or becomes Python.
# ------------------=
def validate(project):
    if not isinstance(project, dict) or project.get("version") != 1:
        raise ValueError("Unsupported project version")
    if project.get("form_type", "application") not in ("application", "widget"):
        raise ValueError("Unknown form type")
    width, height = form_size(project)
    items = project.get("elements")
    if not isinstance(items, list) or len(items) > 96:
        raise ValueError("A project supports at most 96 controls")
    identifiers = set()
    for item in items:
        if not isinstance(item, dict) or item.get("kind") not in KINDS:
            raise ValueError("Unknown control type")
        identifier = item.get("id")
        if not isinstance(identifier, str) or not identifier.isidentifier() or keyword.iskeyword(identifier) or identifier in identifiers:
            raise ValueError("Control identifiers must be unique Python identifiers")
        identifiers.add(identifier)
        for key in ("x", "y", "width", "height"):
            if type(item.get(key)) is not int:
                raise ValueError("Geometry must use integer pixels")
        if not (0 <= item["x"] <= width - 20 and 0 <= item["y"] <= height - 20
                and 20 <= item["width"] <= width - item["x"]
                and 20 <= item["height"] <= height - item["y"]):
            raise ValueError("Controls must fit inside the design surface")
        if not isinstance(item.get("text"), str) or len(item["text"]) > 256:
            raise ValueError("Control text is limited to 256 characters")
        handler = item.get("handler", "")
        if not isinstance(handler, str) or (handler and
                (not handler.isidentifier() or keyword.iskeyword(handler) or item["kind"] == "Label")):
            raise ValueError("Choose a valid method name for an interactive control")
    if not isinstance(project.get("source", ""), str) or len(project.get("source", "")) > 131072:
        raise ValueError("Python source is too large")
    methods = project.get("methods", "")
    if not isinstance(methods, str) or len(methods) > 131072:
        raise ValueError("Method source is too large")
    return copy.deepcopy(project)

# ------------------------=
# FUNC: make_widget
# DESC: Creates the same real native Tk control used in the designer and standalone export.
# ------------------=
def make_widget(parent, item):
    from tkinter import ttk
    cls = getattr(ttk, item["kind"])
    if item["kind"] == "Entry":
        widget = cls(parent)
        widget.insert(0, item["text"])
    elif item["kind"] == "Combobox":
        widget = cls(parent, values=item["text"].split("|"), state="readonly")
        if item["text"]:
            widget.current(0)
    else:
        widget = cls(parent, text=item["text"])
    widget.place(**{key: item[key] for key in ("x", "y", "width", "height")})
    return widget

# ------------------------=
# FUNC: export_source
# DESC: Produces ordinary Python with safely quoted properties and stable named controls for user event handlers.
# ------------------=
def export_source(project):
    project = validate(project)
    elements = project["elements"]
    methods = project.get("methods", "")
    import ast
    tree = ast.parse(methods)
    functions = {node.name: node for node in tree.body if isinstance(node, ast.FunctionDef)}
    for item in elements:
        handler = item.get("handler")
        if handler:
            if handler not in functions:
                raise ValueError("Define the connected method: " + handler)
            arguments = functions[handler].args
            if len(arguments.posonlyargs) + len(arguments.args) > len(arguments.defaults) or any(
                    default is None for default in arguments.kw_defaults):
                raise ValueError("Connected methods must be callable without arguments: " + handler)
    width, height = form_size(project)
    title = "Infinity widget" if project.get("form_type") == "widget" else "My Infinity app"
    lines = ["import tkinter as tk", "from tkinter import ttk", "",
             "root = tk.Tk()", f"root.title({title!r})",
             f"root.geometry('{width}x{height}')", "root.configure(background='#090F26')",
             "style = ttk.Style(root)", "style.theme_use('clam')",
             f"style.configure('.', **{THEME!r})",
             "style.map('TButton', background=[('active', '#1A2E63'), ('pressed', '#3E6FFF')])", "controls = {}"]
    if project.get("form_type") == "widget":
        lines[3] = "root = tk.Tk(className='InfinityWidget')"
        lines.extend(["from infinity_widget import attach", "widget_host = attach(root, __file__)"])
    for item in elements:
        name = repr(item["id"])
        kind = item["kind"]
        args = ("values=" + repr(item["text"].split("|")) + ", state='readonly'") if kind == "Combobox" else (
            "" if kind == "Entry" else "text=" + repr(item["text"]))
        lines.append(f"controls[{name}] = ttk.{kind}(root{', ' if args else ''}{args})")
        if kind == "Entry":
            lines.append(f"controls[{name}].insert(0, {item['text']!r})")
        elif kind == "Combobox" and item["text"]:
            lines.append(f"controls[{name}].current(0)")
        geometry = ", ".join(f"{key}={item[key]}" for key in ("x", "y", "width", "height"))
        if project.get("form_type") == "widget":
            geometry = f"relx={item['x']/width!r}, relwidth={item['width']/width!r}, y={item['y']}, height={item['height']}"
        lines.append(f"controls[{name}].place({geometry})")
    lines += ["", methods, ""]
    for item in elements:
        handler = item.get("handler")
        if not handler:
            continue
        name = repr(item["id"])
        if item["kind"] in ("Button", "Checkbutton", "Radiobutton"):
            lines.append(f"controls[{name}].configure(command={handler})")
        else:
            event = "<Return>" if item["kind"] == "Entry" else "<<ComboboxSelected>>"
            lines.append(f"controls[{name}].bind({event!r}, lambda event: {handler}())")
    lines += ["root.mainloop()", ""]
    return "\n".join(lines)

class Project:
    # ------------------------=
    # FUNC: __init__
    # DESC: Starts an empty bounded project and undo history.
    # ------------------=
    def __init__(self):
        self.data = {"version": 1, "elements": [], "source": "", "methods": ""}
        self.undo_stack, self.redo_stack = [], []

    # ------------------------=
    # FUNC: replace
    # DESC: Commits one validated transaction and preserves undo without accepting malformed loaded data.
    # ------------------=
    def replace(self, value):
        value = validate(value)
        if value == self.data:
            return
        self.undo_stack = (self.undo_stack + [copy.deepcopy(self.data)])[-50:]
        self.redo_stack.clear()
        self.data = value

    # ------------------------=
    # FUNC: add
    # DESC: Adds a uniquely named native control within the visible design surface.
    # ------------------=
    def add(self, kind):
        value = copy.deepcopy(self.data)
        names = {item["id"] for item in value["elements"]}
        number = 1
        while kind.lower() + str(number) in names:
            number += 1
        identifier = kind.lower() + str(number)
        value["elements"].append(dict(id=identifier, kind=kind, x=20, y=20,
            width=160, height=40, text="One|Two|Three" if kind == "Combobox" else kind))
        self.replace(value)
        return identifier

    # ------------------------=
    # FUNC: history
    # DESC: Restores an undo or redo snapshot without discarding the opposite history.
    # ------------------=
    def history(self, redo=False):
        origin, target = (self.redo_stack, self.undo_stack) if redo else (self.undo_stack, self.redo_stack)
        if origin:
            target.append(copy.deepcopy(self.data))
            self.data = origin.pop()
            return True
        return False

    # ------------------------=
    # FUNC: load
    # DESC: Loads bounded UTF-8 JSON and validates it before replacing the current project.
    # ------------------=
    def load(self, path):
        with open(path, encoding="utf-8") as stream:
            body = stream.read(262145)
        if len(body) > 262144:
            raise ValueError("Project file is too large")
        self.replace(json.loads(body))

    # ------------------------=
    # FUNC: save
    # DESC: Atomically saves validated project data in the application's granted namespace.
    # ------------------=
    def save(self, path):
        import os
        value = validate(self.data)
        with open(path + ".tmp", "w", encoding="utf-8") as stream:
            json.dump(value, stream, ensure_ascii=False, indent=2)
        os.replace(path + ".tmp", path)
