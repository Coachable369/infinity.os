"""Behavioral profile application and installed-source checks; no source-string oracles."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('app_design', ROOT/'runtime/install/System/infinity_app_design.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class Widget:
    # ------------------------=
    # FUNC: __init__
    # DESC: Exercises or records application design state for behavioral verification.
    # ------------------=
    def __init__(self, children=()):
        self.children, self.options, self.packing = list(children), {}, {}
    # ------------------------=
    # FUNC: title
    # DESC: Exercises or records application design state for behavioral verification.
    # ------------------=
    def title(self, value): self.title_value = value
    # ------------------------=
    # FUNC: pack_slaves
    # DESC: Exercises or records application design state for behavioral verification.
    # ------------------=
    def pack_slaves(self): return self.children
    # ------------------------=
    # FUNC: pack_info
    # DESC: Exercises or records application design state for behavioral verification.
    # ------------------=
    def pack_info(self): return dict(self.packing)
    # ------------------------=
    # FUNC: pack_forget
    # DESC: Exercises or records application design state for behavioral verification.
    # ------------------=
    def pack_forget(self): self.packing = {}
    # ------------------------=
    # FUNC: pack
    # DESC: Exercises or records application design state for behavioral verification.
    # ------------------=
    def pack(self, **values): self.packing.update(values)
    # ------------------------=
    # FUNC: pack_configure
    # DESC: Exercises or records application design state for behavioral verification.
    # ------------------=
    def pack_configure(self, **values): self.packing.update(values)
    # ------------------------=
    # FUNC: configure
    # DESC: Exercises or records application design state for behavioral verification.
    # ------------------=
    def configure(self, **values): self.options.update(values)

class Tests(unittest.TestCase):
    # ------------------------=
    # FUNC: test_real_geometry_application
    # DESC: Exercises or records application design state for behavioral verification.
    # ------------------=
    def test_real_geometry_application(self):
        with tempfile.TemporaryDirectory(dir=ROOT/'build/tmp') as directory:
            path = Path(directory)/'design.json'
            data = json.loads((ROOT/'runtime/install/System/app-design.json').read_text())
            app = next(item for item in data['apps'] if item['id']=='code')
            app.update(enabled=True,title='My Studio',contentInset=24,inspectorSide='left',inspectorWidth=320,menuAlignment='trailing')
            path.write_text(json.dumps(data))
            root, content, inspector = Widget(), Widget(), Widget()
            menu = Widget([Widget(),Widget(),Widget()])
            menu.children[0].pack(padx=7, fill="y")
            menu.children[2].pack(side="right", padx=8)
            self.assertTrue(module.apply(root,'code',menu,content,inspector,str(path)))
            self.assertEqual(root.title_value,'My Studio')
            self.assertEqual(menu.children[2].packing,dict(side='right',padx=8))
            self.assertEqual(menu.children[0].packing['padx'],7)
            self.assertEqual(content.packing,dict(padx=24,pady=24))
            self.assertEqual(inspector.options['width'],320)
            self.assertEqual(inspector.packing['side'],'left')
            self.assertTrue(all(child.packing['side']=='right' for child in menu.children))
    # ------------------------=
    # FUNC: test_invalid_file_preserves_factory
    # DESC: Exercises or records application design state for behavioral verification.
    # ------------------=
    def test_invalid_file_preserves_factory(self):
        root = Widget()
        self.assertFalse(module.apply(root,'code',path='/no/such/app-design.json'))
        self.assertFalse(hasattr(root,'title_value'))
    # ------------------------=
    # FUNC: test_native_payload_matches_installed_source
    # DESC: Exercises or records application design state for behavioral verification.
    # ------------------=
    def test_native_payload_matches_installed_source(self):
        source=json.loads((ROOT/'runtime/install/System/app-design.json').read_text())
        binary=(ROOT/'assets/ui-design-kit/default/app-design.bin').read_bytes()
        self.assertEqual(len(binary),len(source['apps'])*128)
        for index,app in enumerate(source['apps']):
            record=binary[index*128:(index+1)*128]
            self.assertEqual(record[16:16+record[0]],app['title'].encode())
            self.assertEqual(record[1],app['contentInset'])

if __name__=='__main__': unittest.main()
