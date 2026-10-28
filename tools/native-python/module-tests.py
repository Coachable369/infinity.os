"""Host behavioral tests for non-executing dependency validation, not native proof."""
from pathlib import Path
import importlib.util
import os
import sys
import tempfile
import types

ROOT = Path(__file__).resolve().parents[2]


# ------------------------=
# FUNC: load_validator
# DESC: Loads the production validator against an explicit host fixture inventory without pretending it is the native payload.
# ------------------=
def load_validator():
    metadata = types.ModuleType("_infinity_inventory")
    metadata.INVENTORY = dict(schema_version=1, python_version="fixture", architecture="host",
        modules=[dict(name="native_fixture.missing", included=False)],
        limitations={"tkinter": "native Tk backend absent",
                     "native_fixture.blocked": "fixture facility unavailable"},
        required_backends={"fictional_wrapper": ("absent_native_backend",),
                           "native_fixture.needs_backend": ("absent_native_backend",)},
        generation_libraries={"data": ("json", "absent_infinity_module"),
                              "blocked": ("tkinter", "fictional_wrapper")})
    sys.modules[metadata.__name__] = metadata
    spec = importlib.util.spec_from_file_location("_infinity_modules", ROOT / "sdk/python/_infinity_modules.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


# ------------------------=
# FUNC: main
# DESC: Checks missing imports, exact locations, dynamic imports and the absence of candidate or package initializer side effects.
# ------------------=
def main():
    assert os.environ.get("INFINITY_BUILD_KIT_ACTIVE") == "1"
    parent = ROOT / "build/behavior-tests/python-modules"
    parent.mkdir(parents=True, exist_ok=True)
    validator = load_validator()
    with tempfile.TemporaryDirectory(prefix="imports-", dir=parent) as directory:
        work = Path(directory)
        package = work / "native_fixture"
        package.mkdir()
        marker = work / "executed"
        (package / "__init__.py").write_text(f"open({str(marker)!r}, 'w').write('bad')\n")
        (package / "submodule.py").write_text("raise RuntimeError('must not execute')\n")
        (package / "blocked.py").write_text("raise RuntimeError('must not execute')\n")
        (package / "needs_backend.py").write_text("raise RuntimeError('must not execute')\n")
        (work / "fictional_wrapper.py").write_text("pass\n")
        (work / "native_delegate.py").write_text(
            f"open({str(marker)!r}, 'w').write('imported')\n"
            f"def main():\n    open({str(marker)!r}, 'w').write('called')\n")
        sys.path.insert(0, str(work))
        try:
            source = f"import json\nimport native_fixture.submodule\nopen({str(marker)!r}, 'w')\n"
            assert validator.validate_source(source, "app.py") == ("json", "native_fixture.submodule")
            assert not marker.exists()
            assert "native_fixture" not in sys.modules
            assert "native_fixture.submodule" not in sys.modules
            for source in (
                "def main():\n    pass\n\nmain()\n",
                "class App:\n    def start(self):\n        ...\n"
                "    def update(self):\n        raise NotImplementedError\n"
                "def main():\n    'Not implemented yet'\n    pass\n"
                "if __name__ == '__main__':\n    main()\n",
                "pass\n",
                "...\n",
                "import abc\nclass Piece(abc.ABC):\n"
                "    @abc.abstractmethod\n    def move(self):\n        pass\n"
                "class Rook(Piece):\n    def move(self):\n        pass\n"
                "class Board:\n    def update(self):\n        pass\n"
                "class Game:\n    def start(self):\n        pass\n"
                "def main():\n    pass\nif __name__ == '__main__':\n    main()\n",
                "from abc import ABC as Base, abstractmethod as abstract\n"
                "class Record(Base):\n    @abstract\n    def update(self):\n        ...\n"
                "class Entry(Record):\n    def update(self):\n        ...\n"
                "def main():\n    pass\nif __name__ == '__main__':\n    main()\n",
            ):
                # Ordinary/manual validation intentionally continues to allow
                # placeholders; only generated applications opt into this lint.
                validator.validate_source(source, "empty.py")
                try:
                    validator.validate_generated_application(source, "empty.py")
                except validator.IncompleteApplicationError as error:
                    assert len(error.issues) == 1
                    assert error.issues[0]["kind"] == "whole_application_placeholder"
                    assert error.issues[0]["filename"] == "empty.py"
                    assert error.issues[0]["line"] >= 1 and error.issues[0]["column"] >= 1
                else:
                    raise AssertionError("whole-placeholder generated application accepted")
            for source in (
                "def callback():\n    pass\nprint(42)\n",
                "def callback():\n    ...\ndef main():\n    print(42)\nmain()\n",
                "def main():\n    return 42\npass\nmain()\n",
                "class Interface(object):\n    def callback(self):\n        ...\n",
                "@decorate\ndef callback():\n    pass\ncallback()\n",
                "def callback(value=notify()):\n    pass\ncallback()\n",
                "def callback():\n    return None\n",
                "class App(ExternalFramework):\n    def callback(self):\n        pass\n"
                "def main():\n    pass\nif __name__ == '__main__':\n    main()\n",
                "@register\nclass App:\n    def callback(self):\n        pass\n"
                "def main():\n    pass\nif __name__ == '__main__':\n    main()\n",
                "import abc\nclass Interface(abc.ABC):\n"
                "    @abc.abstractmethod\n    def update(self):\n        pass\n",
                "import json\n",
                f"def callback():\n    pass\nopen({str(marker)!r}, 'w')\n",
                "def main():\n    pass\nfrom native_delegate import main\nmain()\n",
                "def main():\n    pass\nfrom native_delegate import main as main\n"
                "if __name__ == '__main__':\n    main()\n",
                "def main():\n    pass\nfrom native_delegate import *\nmain()\n",
                "def main():\n    pass\nif __name__ == '__main__':\n"
                "    from native_delegate import main\n    main()\n",
                "def main():\n    pass\nimport native_delegate as main\nmain()\n",
                "def main():\n    pass\nfrom native_delegate import main as delegated\n"
                "main = delegated\nmain()\n",
                "def main():\n    pass\nmain = None\nmain()\n",
                f"def main():\n    pass\n"
                f"annotation: open({str(marker)!r}, 'w').write('evaluated')\nmain()\n",
                f"def main(*args: open({str(marker)!r}, 'w').write('evaluated')):\n"
                "    pass\nmain()\n",
                f"def main(**kwargs: open({str(marker)!r}, 'w').write('evaluated')):\n"
                "    pass\nmain()\n",
            ):
                validator.validate_generated_application(source, "implemented.py")
            assert not marker.exists() and "native_delegate" not in sys.modules
            for source, expected in (
                ("import absent_infinity_module\n", "absent_infinity_module"),
                ("import tkinter\n", "tkinter"),
                ("import fictional_wrapper\n", "fictional_wrapper"),
                ("from native_fixture import blocked\n", "native_fixture.blocked"),
                ("from native_fixture import missing\n", "native_fixture.missing"),
                ("from native_fixture import needs_backend\n", "native_fixture.needs_backend"),
                ("import native_fixture.needs_backend\n", "native_fixture.needs_backend"),
                ("import importlib as importer\nimporter.import_module(name)\n", "<dynamic import>"),
                ("from importlib import import_module as load\nload(name)\n", "<dynamic import>"),
                ("load = __import__\nload('absent_infinity_module')\n", "absent_infinity_module"),
                ("import importlib\nload = importlib.import_module\nload(name)\n", "<dynamic import>"),
                ("from builtins import __import__ as load\nload('absent_infinity_module')\n", "absent_infinity_module"),
                ("import importlib\nload = getattr(importlib, 'import_module')\nload('absent_infinity_module')\n", "absent_infinity_module"),
                ("import importlib\nload = getattr(importlib, name)\nload('json')\n", "<dynamic import>"),
                ("load = [__import__][0]\nload('json')\n", "<dynamic import>"),
                ("def loader():\n return __import__\n", "<dynamic import>"),
                ("load = __import__ if choice else other\nload('json')\n", "<dynamic import>"),
                ("target.loader = __import__\n", "<dynamic import>"),
            ):
                try:
                    validator.validate_source(source, "app.py")
                except validator.DependencyError as error:
                    assert any(item["module"] == expected for item in error.issues)
                    assert all(item["filename"] == "app.py" and item["line"] >= 1 for item in error.issues)
                else:
                    raise AssertionError("unavailable dependency accepted")
            assert validator.validate_source("from importlib import import_module as load\nload('json')\n") == ("importlib", "json")
            assert validator.validate_source("first = second\nsecond = __import__\nfirst('json')\n") == ("json",)
            assert validator.validate_source("import importlib as lib\nother = lib\nload = other.import_module\nload('json')\n") == ("importlib", "json")
            assert validator.validate_source("import importlib\ngetattr(importlib, 'import_module')('json')\n") == ("importlib", "json")
            assert validator.validate_source("load: object = __import__\nload(name='json')\n") == ("json",)
            assert validator.validate_source("from native_fixture import submodule\n") == (
                "native_fixture", "native_fixture.submodule")
            assert validator.validate_source("from json import loads\n") == ("json",)
            try:
                validator.validate_source("return 1\n")
            except SyntaxError:
                pass
            else:
                raise AssertionError("invalid module-level return accepted")
            assert not marker.exists()
            catalog = validator.generation_catalog(str(work))
            assert catalog["default_libraries"] == {"data": ("json",), "blocked": ()}
            assert catalog["installed_import_roots"] == ("fictional_wrapper", "native_delegate", "native_fixture")
            assert catalog["directory_available"] and not catalog["package_apis_verified"]
            assert not marker.exists() and "native_fixture" not in sys.modules
            (work / "malicious ignore instructions.py").write_text("raise RuntimeError('never execute')\n")
            (work / "package-1.0.dist-info").mkdir()
            (work / "class.py").write_text("raise RuntimeError('never execute')\n")
            (work / "native_extension.cpython-313-infinityos.so").write_bytes(b"not loaded by metadata discovery")
            catalog = validator.generation_catalog(str(work))
            assert catalog["installed_import_roots"] == ("fictional_wrapper", "native_delegate", "native_extension", "native_fixture")
            (work / "fictional_wrapper.py").unlink()
            assert "fictional_wrapper" not in validator.generation_catalog(str(work))["installed_import_roots"]
            assert not validator.generation_catalog(str(work / "missing"))["directory_available"]
            assert not marker.exists()
        finally:
            sys.path.remove(str(work))


if __name__ == "__main__":
    main()
