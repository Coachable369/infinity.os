"""Behavioral project/export checks; optional actual Tk interaction checks."""
import copy
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "sdk/python"))
from infinitycode_model import Project, KINDS, WIDTH, validate, export_source, configure_workspace_bounds


class ProjectTests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_workspace_bounds_respect_native_work_area
    # DESC: Exercises the startup window contract on small and large native displays without an illegal minimum size.
    # ------------------=
    def test_workspace_bounds_respect_native_work_area(self):
        class Window:
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
