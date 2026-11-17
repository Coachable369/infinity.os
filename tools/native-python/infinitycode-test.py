"""Behavioral project/export checks; optional actual Tk interaction checks."""
import copy
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch, MagicMock

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "sdk/python"))
from infinitycode_model import Project, KINDS, WIDTH, validate, export_source, configure_workspace_bounds


class ProjectTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_widget_example_in_tk
    # DESC: Runs the packaged counter in Tk, dispatches its real buttons and restores the released drag on a second launch.
    # ------------------=
    @unittest.skipUnless(os.environ.get("INFINITYCODE_GUI_TEST") == "1", "explicit GUI session required")
    def test_widget_example_in_tk(self):
        import tkinter as tk
        from types import SimpleNamespace
        template = ROOT / "sdk/widgets/examples/counter.py"
        with tempfile.TemporaryDirectory() as folder, patch.object(tk.Misc, "mainloop", lambda self: None):
            filename = str(Path(folder) / template.name)
            code = compile(template.read_text(), filename, "exec")
            env = {"__file__": filename}
            exec(code, env)
            root = env["root"]
            try:
                root.update()
                controls = env["controls"]
                controls["add"].invoke()
                self.assertEqual(controls["count"].cget("text"), "1")
                controls["reset"].invoke()
                self.assertEqual(controls["count"].cget("text"), "0")
                host = env["widget_host"]
                event = SimpleNamespace(widget=root, x_root=root.winfo_x()+5, y_root=root.winfo_y()+5)
                host.press(event)
                event.x_root, event.y_root = 245, 205
                host.move(event)
                root.update()
                host.release(event)
                saved = json.loads(Path(host.path).read_text())
                self.assertEqual(saved, [root.winfo_x(), root.winfo_y()])
            finally:
                root.destroy()
            exec(code, env)
            root = env["root"]
            try:
                root.update()
                self.assertEqual([root.winfo_x(), root.winfo_y()], saved)
            finally:
                root.destroy()

    # ------------------------=
    # FUNC: test_packaged_widget_backend
    # DESC: Executes the installed template callbacks and compares its exported code with the editable form.
    # ------------------=
    def test_packaged_widget_backend(self):
        from infinitycode_model import counter_widget
        folder = ROOT / "sdk/widgets/examples"
        project = json.loads((folder / "counter.json").read_text())
        source = export_source(counter_widget())
        self.assertEqual(export_source(project), source)
        self.assertEqual((folder / "counter.py").read_text(), source)
        import types
        tk = MagicMock()
        ttk = types.SimpleNamespace(Style=MagicMock(), Label=lambda *a, **k: MagicMock(), Button=lambda *a, **k: MagicMock())
        tk.ttk = ttk
        with patch.dict(sys.modules, {"tkinter":tk,"tkinter.ttk":ttk,"infinity_widget":MagicMock()}):
            namespace = {"__file__":"counter.py"}
            exec(compile(source,"counter.py","exec"),namespace)
        controls = namespace["controls"]
        controls["add"].configure.call_args.kwargs["command"]()
        controls["count"].configure.assert_called_with(text="1")
        controls["reset"].configure.call_args.kwargs["command"]()
        controls["count"].configure.assert_called_with(text="0")

    # ------------------------=
    # FUNC: test_editor_diagnostics_and_handler_navigation
    # DESC: Exercises source validation and method navigation with widget doubles, including exact error selection and no execution.
    # ------------------=
    def test_editor_diagnostics_and_handler_navigation(self):
        import importlib.util
        from types import SimpleNamespace
        tk=SimpleNamespace(ttk=SimpleNamespace(),messagebox=SimpleNamespace())
        spec=importlib.util.spec_from_file_location("infinitycode_diagnostic_test",ROOT/"sdk/python/infinitycode.py")
        module=importlib.util.module_from_spec(spec)
        with patch.dict(sys.modules,{"tkinter":tk,"tkinter.ttk":tk.ttk,"tkinter.messagebox":tk.messagebox}):
            spec.loader.exec_module(module)
        editor=module.Studio.__new__(module.Studio)
        editor.source=MagicMock();editor.methods=MagicMock()
        editor.tabs=MagicMock();editor.status=MagicMock()
        editor.source.get.return_value="x = 1\ndef broken(:\n    pass\n"
        editor.methods.get.return_value="def activate():\n    pass\n"
        editor.check_python()
        editor.tabs.select.assert_called_with(1)
        editor.source.tag_add.assert_called_with("diagnostic","2.0","2.end")
        editor.source.get.return_value="raise RuntimeError('must not execute')\n"
        editor.check_python()
        editor.fields={"handler":MagicMock()}
        editor.fields["handler"].get.return_value="activate"
        editor.goto_handler()
        editor.tabs.select.assert_called_with(2)
        editor.methods.mark_set.assert_called_with("insert","1.0")
        editor.methods.focus_set.assert_called_once()

    # ------------------------=
    # FUNC: test_connected_methods_survive_regeneration
    # DESC: Executes exported callbacks against a widget adapter and verifies control events mutate application state.
    # ------------------=
    def test_connected_methods_survive_regeneration(self):
        from types import SimpleNamespace
        class Widget:
            # ------------------------=
            # FUNC: __init__
            # DESC: Records native widget configuration and event bindings without a display server.
            # ------------------=
            def __init__(self, *args, **kwargs):
                self.options, self.events = kwargs, {}
            # ------------------------=
            # FUNC: configure
            # DESC: Applies runtime property updates from a generated handler.
            # ------------------=
            def configure(self, *args, **kwargs):
                self.options.update(kwargs)
            # ------------------------=
            # FUNC: bind
            # DESC: Captures an event callback for dispatch by the behavioral harness.
            # ------------------=
            def bind(self, event, callback):
                self.events[event] = callback
            # ------------------------=
            # FUNC: noop
            # DESC: Accepts presentation calls irrelevant to event wiring.
            # ------------------=
            def noop(self, *args, **kwargs):
                pass
            title = geometry = theme_use = map = place = insert = current = mainloop = noop
        ttk = SimpleNamespace(Style=Widget, **{kind: Widget for kind in KINDS})
        tk = SimpleNamespace(Tk=Widget, ttk=ttk)
        project = Project()
        for kind in KINDS:
            project.add(kind)
            if kind != "Label":
                project.data["elements"][-1]["handler"] = "activate"
        methods = "def activate():\n    controls['label1'].configure(text='Activated')\n"
        project.data["methods"] = methods
        with patch.dict(sys.modules, {"tkinter": tk, "tkinter.ttk": ttk}):
            namespace = {}
            exec(compile(export_source(project.data), "export.py", "exec"), namespace)
            controls = namespace["controls"]
            for item in project.data["elements"][1:]:
                widget = controls[item["id"]]
                controls["label1"].configure(text="Before")
                if widget.events:
                    next(iter(widget.events.values()))(None)
                else:
                    widget.options["command"]()
                self.assertEqual(controls["label1"].options["text"], "Activated")
        self.assertEqual(project.data["methods"], methods)
        project.data["methods"] = "def activate(required):\n    pass\n"
        with self.assertRaises(ValueError):
            export_source(project.data)
        project.data["methods"] = methods
        project.data["elements"][1]["handler"] = "missing_method"
        with self.assertRaises(ValueError):
            export_source(project.data)

    # ------------------------=
    # FUNC: test_workspace_bounds_respect_native_work_area
    # DESC: Exercises the startup window contract on small and large native displays without an illegal minimum size.
    # ------------------=
    def test_workspace_bounds_respect_native_work_area(self):
        class Window:
            # ------------------------=
            # FUNC: winfo_screenwidth
            # DESC: Supplies the complete display width.
            # ------------------=
            def winfo_screenwidth(self):
                return self.maximum[0]+32
            # ------------------------=
            # FUNC: winfo_screenheight
            # DESC: Supplies the complete simulated display including its reserved work-area margins.
            # ------------------=
            def winfo_screenheight(self):
                return self.maximum[1]+108
            # ------------------------=
            # FUNC: maxsize
            # DESC: Returns the native work-area constraint for this test case.
            # ------------------=
            def maxsize(self):
                return self.maximum
            # ------------------------=
            # FUNC: minsize
            # DESC: Records the applied minimum dimensions.
            # ------------------=
            def minsize(self, width, height):
                self.minimum = (width, height)
            # ------------------------=
            # FUNC: geometry
            # DESC: Records requested client dimensions as structured values.
            # ------------------=
            def geometry(self, value):
                self.size = tuple(map(int, value.split("x")))
        for maximum in ((640, 480), (1000, 650), (1920, 1080)):
            root = Window()
            root.maximum = maximum
            configure_workspace_bounds(root)
            minimum = root.minimum
            self.assertTrue(all(0 < size <= limit for size, limit in zip(minimum, maximum)))
            geometry = root.size
            self.assertTrue(all(low <= size <= limit for low, size, limit in zip(minimum, geometry, maximum)))

    # ------------------------=
    # FUNC: test_designer_interactions
    # DESC: Exercises actual selection, snapped move and resize, properties, undo, save, reload and Python export.
    # ------------------=
    @unittest.skipUnless(os.environ.get("INFINITYCODE_GUI_TEST") == "1", "explicit GUI session required")
    def test_designer_interactions(self):
        import tkinter as tk
        from types import SimpleNamespace
        from infinitycode import Studio
        root = tk.Tk()
        studio = Studio(root)
        try:
            studio.add("Button")
            root.update()
            studio.start(SimpleNamespace(x_root=100, y_root=100), studio.selected)
            studio.motion(SimpleNamespace(x_root=127, y_root=116))
            studio.release(None)
            self.assertEqual((studio.item()["x"], studio.item()["y"]), (50, 40))
            studio.resize_start(SimpleNamespace(x_root=100, y_root=100))
            studio.motion(SimpleNamespace(x_root=137, y_root=111))
            studio.release(None)
            self.assertEqual((studio.item()["width"], studio.item()["height"]), (200, 50))
            studio.undo()
            self.assertEqual(studio.item()["width"], 160)
            studio.redo()
            self.assertEqual(studio.item()["width"], 200)
            studio.fields["text"].delete(0, "end")
            studio.fields["text"].insert(0, "Calculate")
            studio.apply()
            self.assertEqual(studio.widgets[studio.selected].cget("text"), "Calculate")
            studio.generate()
            with tempfile.TemporaryDirectory(dir=ROOT / "build/tmp") as folder:
                path = str(Path(folder) / "design.json")
                studio.path.delete(0, "end")
                studio.path.insert(0, path)
                studio.save()
                studio.export()
                exported = Path(folder, "design.py").read_text()
                self.assertEqual(exported, studio.source.get("1.0", "end-1c"))
                compile(exported, "design.py", "exec")
                studio.load()
                self.assertEqual(studio.project.data, studio.saved)
            root.update()
            self.assertGreaterEqual(studio.canvas.winfo_width(), WIDTH)
        finally:
            root.destroy()

    # ------------------------=
    # FUNC: test_form_workspace_roundtrip
    # DESC: Exercises viewport anchoring, toolbox drops, form sizing and code behind through a YAML round trip.
    # ------------------=
    @unittest.skipUnless(os.environ.get("INFINITYCODE_GUI_TEST") == "1", "explicit GUI session required")
    def test_form_workspace_roundtrip(self):
        import tkinter as tk
        import yaml
        from types import SimpleNamespace
        from infinitycode import Studio
        root=tk.Tk()
        try:
            studio=Studio(root)
            for width,height in ((1200,780),(960,650),(1380,820)):
                root.geometry(f"{width}x{height}")
                root.update()
                vw=studio.viewport.winfo_width()
                x,y=studio.viewport.coords(studio.surface)
                self.assertEqual(x,max(16,(vw-WIDTH)//2))
                inspector=studio.fields["id"].winfo_rootx()-root.winfo_rootx()
                self.assertLess(width-inspector,280)
            self.assertEqual(studio.menu_bar.winfo_height(),36)
            self.assertEqual(studio.toolbar.winfo_height(),48)
            before=copy.deepcopy(studio.project.data)
            studio.set_zoom(1.5)
            root.update()
            self.assertEqual(studio.canvas.winfo_width(),960)
            self.assertEqual(studio.project.data,before)
            fixed=(studio.menu_bar.winfo_height(),studio.toolbar.winfo_height())
            root.geometry("1500x940")
            root.update()
            self.assertEqual((studio.menu_bar.winfo_height(),studio.toolbar.winfo_height()),fixed)
            studio.set_zoom(1.0)
            root.update()
            root.focus_force()
            root.event_generate("<Control-KeyPress-equal>")
            root.update()
            self.assertEqual(studio.zoom,1.1)
            root.event_generate("<Control-KeyPress-0>")
            root.update()
            self.assertEqual(studio.zoom,1.0)
            studio.base_scale=2
            studio.set_zoom(1.0)
            root.update()
            self.assertEqual(studio.menu_bar.winfo_height(),72)
            self.assertEqual(studio.toolbar.winfo_height(),96)
            page=root.nametowidget(studio.tabs.select())
            self.assertLessEqual(page.winfo_y(),90)
            self.assertGreaterEqual(page.winfo_y(),78)
            self.assertEqual(studio.tab_images[0].height(),80)
            self.assertEqual(studio.project.data,before)
            studio.base_scale=1
            studio.set_zoom(1.0)
            root.update()
            tool=dict(studio.palette_items)["Button"]
            drop=SimpleNamespace(widget=tool,x_root=studio.canvas.winfo_rootx()+104,
                                 y_root=studio.canvas.winfo_rooty()+63)
            tool.grab_set()
            studio.drop_control(drop,"Button")
            self.assertEqual((studio.item()["x"],studio.item()["y"]),(100,60))
            grip=root.nametowidget(studio.viewport.itemcget(studio.form_grips[-1][1],"window"))
            drag=SimpleNamespace(widget=grip,x_root=300,y_root=200)
            studio.begin_form_resize(drag,"se")
            drag.x_root+=30
            studio.finish_form_resize(drag)
            self.assertEqual(studio.project.data["width"],700)
            studio.code_behind()
            self.assertEqual(studio.item()["handler"],"on_button1")
            studio.methods.insert("end","\nresult = 42\n")
            with tempfile.TemporaryDirectory(dir=ROOT/"build/tmp") as folder:
                path=str(Path(folder)/"form.yaml")
                with patch.object(studio,"choose_path",return_value=path):
                    studio.save_as()
                    studio.new_form()
                    studio.open_project()
                saved=yaml.safe_load(Path(path).read_text())
                self.assertEqual(studio.project.data,saved)
                self.assertEqual(saved["elements"][0]["handler"],"on_button1")
                self.assertEqual(saved["elements"][0]["x"],100)
                self.assertEqual(saved["methods"],studio.methods.get("1.0","end-1c"))
                self.assertEqual(compile(export_source(saved),path,"exec").co_name,"<module>")
        finally:
            root.destroy()

    # ------------------------=
    # FUNC: test_transactions
    # DESC: Verifies unique additions, undo/redo and invalid transactions leaving prior state intact.
    # ------------------=
    def test_transactions(self):
        project = Project()
        first = project.add("Button")
        second = project.add("Button")
        self.assertNotEqual(first, second)
        saved = copy.deepcopy(project.data)
        self.assertTrue(project.history())
        self.assertEqual(len(project.data["elements"]), 1)
        self.assertTrue(project.history(True))
        self.assertEqual(project.data, saved)
        bad = copy.deepcopy(saved)
        bad["elements"][0]["width"] = 9999
        with self.assertRaises(ValueError):
            project.replace(bad)
        self.assertEqual(project.data, saved)

    # ------------------------=
    # FUNC: test_save_load_and_source
    # DESC: Roundtrips real JSON bytes and editable source, rejecting malformed loads without losing state.
    # ------------------=
    def test_save_load_and_source(self):
        project = Project()
        for kind in KINDS:
            project.add(kind)
        project.data["elements"][0]["text"] = "Quoted ' \\\nvalue"
        project.data["source"] = export_source(project.data)
        compile(project.data["source"], "export.py", "exec")
        with tempfile.TemporaryDirectory(dir=ROOT / "build/tmp") as folder:
            path = str(Path(folder) / "project.json")
            project.save(path)
            loaded = Project()
            loaded.load(path)
            self.assertEqual(loaded.data, project.data)
            Path(path).write_text(json.dumps({"version": 999}))
            with self.assertRaises(ValueError):
                loaded.load(path)
            self.assertEqual(loaded.data, project.data)

    # ------------------------=
    # FUNC: test_export_actual_widgets
    # DESC: Runs exported Python against actual Tk and compares control geometry, values and kinds to the designer factory.
    # ------------------=
    @unittest.skipUnless(os.environ.get("INFINITYCODE_GUI_TEST") == "1", "explicit GUI session required")
    def test_export_actual_widgets(self):
        import tkinter as tk
        from infinitycode_model import make_widget
        project = Project()
        for index, kind in enumerate(KINDS):
            project.add(kind)
            project.data["elements"][-1]["y"] = index * 55
        original = tk.Misc.mainloop
        tk.Misc.mainloop = lambda *args, **kwargs: None
        namespace = {}
        try:
            exec(compile(export_source(project.data), "export.py", "exec"), namespace)
        finally:
            tk.Misc.mainloop = original
        root = namespace["root"]
        root.update()
        designer = tk.Toplevel(root)
        designer.geometry("640x420")
        try:
            for item in project.data["elements"]:
                expected = make_widget(designer, item)
                designer.update()
                actual = namespace["controls"][item["id"]]
                self.assertEqual(actual.winfo_class(), expected.winfo_class())
                self.assertEqual(actual.place_info(), expected.place_info() | {"in": root})
                self.assertEqual((actual.winfo_x(), actual.winfo_y(), actual.winfo_width(), actual.winfo_height()),
                                 tuple(item[key] for key in ("x", "y", "width", "height")))
        finally:
            root.destroy()


if __name__ == "__main__":
    unittest.main()
