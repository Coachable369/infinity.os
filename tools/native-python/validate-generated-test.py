#!/usr/bin/env python3
"""Behavioral host-only validation tests; no generated application is executed."""
from pathlib import Path
import importlib.util
import os
import tempfile

ROOT = Path(__file__).resolve().parents[2]


# ------------------------=
# FUNC: main
# DESC: Verifies parser errors, exact inventory admission, missing backends, dynamic-import rejection and absence of application side effects.
# ------------------=
def main():
    assert os.environ.get('INFINITY_BUILD_KIT_ACTIVE') == '1'
    spec = importlib.util.spec_from_file_location('generated_validator', Path(__file__).with_name('validate-generated.py'))
    validator = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(validator)
    inventory = dict(modules=[dict(name='infinity', included=True), dict(name='math', included=True),
        dict(name='importlib', included=True), dict(name='json', included=False)],
        limitations={'threading': 'worker threads unavailable'}, required_backends={'infinity': ('_infinity_app',)})
    assert not validator.validate(b'import infinity\n', 'probe.py', inventory)['dependencies_valid']
    inventory['modules'].append(dict(name='_infinity_app', included=True))
    assert validator.validate(b'import infinity\nimport math\n', 'probe.py', inventory)['dependencies_valid']
    for source in (b'import json\n', b'import absent_native_library\n', b'import threading\n',
                   b'import importlib\nimportlib.import_module(name)\n'):
        result = validator.validate(source, 'probe.py', inventory)
        assert result['syntax_valid'] and not result['dependencies_valid'] and result['issues']
    for source in (b'def missing():\n', b'return 1\n', b'x = "unterminated\n'):
        result = validator.validate(source, 'probe.py', inventory)
        assert not result['syntax_valid'] and result['issues'][0]['line'] >= 1
    stub = validator.validate(b'def main():\n    pass\nmain()\n', 'stub.py', inventory)
    assert stub['syntax_valid'] and stub['dependencies_valid']
    assert stub['implementation_lint_valid'] is False
    assert stub['issues'][0]['kind'] == 'whole_application_placeholder'
    assert stub['issues'][0]['line'] == 1 and stub['issues'][0]['column'] == 1
    inventory['modules'].append(dict(name='abc', included=True))
    inherited_stub = validator.validate(
        b'import abc\nclass Piece(abc.ABC):\n'
        b'    @abc.abstractmethod\n    def move(self):\n        pass\n'
        b'class Concrete(Piece):\n    def move(self):\n        pass\n'
        b'def main():\n    pass\nif __name__ == "__main__":\n    main()\n',
        'inherited.py', inventory)
    assert inherited_stub['syntax_valid'] and inherited_stub['dependencies_valid']
    assert inherited_stub['implementation_lint_valid'] is False
    assert inherited_stub['issues'][0]['kind'] == 'whole_application_placeholder'
    working = validator.validate(b'def ignored_callback():\n    pass\nprint(42)\n', 'working.py', inventory)
    assert working['implementation_lint_valid'] is True and not working['issues']
    inventory['modules'].append(dict(name='installed_app', included=True))
    for source in (
        b'def main():\n    pass\nfrom installed_app import main\nmain()\n',
        b'def main():\n    pass\nfrom installed_app import main as main\n'
        b'if __name__ == "__main__":\n    main()\n',
        b'def main():\n    pass\nfrom installed_app import *\nmain()\n',
        b'def main():\n    pass\nfrom installed_app import main as delegated\n'
        b'main = delegated\nmain()\n',
    ):
        delegated = validator.validate(source, 'delegated.py', inventory)
        assert delegated['syntax_valid'] and delegated['dependencies_valid']
        assert delegated['implementation_lint_valid'] is True and not delegated['issues']
    parent = ROOT / 'build/behavior-tests/generated-python'
    parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=parent) as directory:
        marker = Path(directory) / 'must-not-execute'
        result = validator.validate(f'open({str(marker)!r}, "w").write("bad")\n', 'probe.py', inventory)
        assert result['syntax_valid'] and result['dependencies_valid'] \
            and result['implementation_lint_valid'] and not marker.exists()
        for source in (
            f'def main():\n    pass\nvalue: open({str(marker)!r}, "w").write("bad")\nmain()\n',
            f'def main(*args: open({str(marker)!r}, "w").write("bad")):\n    pass\nmain()\n',
            f'def main(**kwargs: open({str(marker)!r}, "w").write("bad")):\n    pass\nmain()\n',
        ):
            result = validator.validate(source, 'annotations.py', inventory)
            assert result['syntax_valid'] and result['dependencies_valid']
            assert result['implementation_lint_valid'] is True and not result['issues']
            assert not marker.exists()


if __name__ == '__main__':
    main()
