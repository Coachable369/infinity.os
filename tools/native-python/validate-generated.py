#!/usr/bin/env python3
"""Host generated-source validation against an exact native payload inventory.

Neither generated code nor its dependencies run here. This is not installed
execution, GUI behavior, game-rule validation, or native-interpreter proof.
"""
from pathlib import Path
import argparse
import hashlib
import importlib.util
import json
import os
import sys
import types

ROOT = Path(__file__).resolve().parents[2]


# ------------------------=
# FUNC: validate
# DESC: Applies production source rules to exact native inventory facts without admitting host-only libraries or executing application code.
# ------------------=
def validate(source, filename, inventory):
    metadata = types.ModuleType('_infinity_inventory')
    metadata.INVENTORY = inventory
    previous = sys.modules.get(metadata.__name__)
    sys.modules[metadata.__name__] = metadata
    try:
        spec = importlib.util.spec_from_file_location('native_generation_validator',
            ROOT / 'sdk/python/_infinity_modules.py')
        validator = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(validator)
    finally:
        if previous is None:
            del sys.modules[metadata.__name__]
        else:
            sys.modules[metadata.__name__] = previous
    included = {item['name'] for item in inventory['modules'] if item['included']}
    # The production resolver reads installed native files. This host-only
    # probe substitutes the exact build inventory, never host module discovery.
    validator.find_spec_without_import = lambda name: name if name in included else None
    try:
        imports = validator.validate_generated_application(source, filename)
    except SyntaxError as error:
        return dict(syntax_valid=False, dependencies_valid=False, implementation_lint_valid=None, imports=[], issues=[dict(
            filename=filename, line=error.lineno or 1, column=error.offset or 1,
            module='', reason=error.msg)])
    except validator.DependencyError as error:
        return dict(syntax_valid=True, dependencies_valid=False, implementation_lint_valid=None, imports=[], issues=list(error.issues))
    except validator.IncompleteApplicationError as error:
        return dict(syntax_valid=True, dependencies_valid=True, implementation_lint_valid=False,
                    imports=[], issues=list(error.issues))
    return dict(syntax_valid=True, dependencies_valid=True, implementation_lint_valid=True,
                imports=list(imports), issues=[])


# ------------------------=
# FUNC: main
# DESC: Writes structured host-only validation and source-located diagnostics for real generated artifacts, failing closed on inventory/version mismatch.
# ------------------=
def main():
    if os.environ.get('INFINITY_BUILD_KIT_ACTIVE') != '1':
        raise SystemExit('Use ./build-kit run python3 tools/native-python/validate-generated.py')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', required=True, type=Path)
    parser.add_argument('--inventory', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--target', choices=('aarch64', 'x86_64'), required=True)
    args = parser.parse_args()
    source, inventory_path, output = (path.resolve() for path in (args.source, args.inventory, args.output))
    assert all(path.is_relative_to(ROOT) for path in (source, inventory_path, output))
    raw = source.read_bytes()
    inventory_raw = inventory_path.read_bytes()
    inventory = json.loads(inventory_raw)
    assert inventory['schema_version'] == 1 and inventory['architecture'] == args.target
    native_version = tuple(int(part) for part in inventory['python_version'].split('.')[:2])
    if sys.version_info[:2] != native_version:
        raise SystemExit('Host parser minor version must match the native Python inventory')
    report = validate(raw, str(source), inventory)
    report.update(schema_version=1, target=args.target, source_bytes=len(raw),
        source_sha256=hashlib.sha256(raw).hexdigest(),
        inventory_sha256=hashlib.sha256(inventory_raw).hexdigest(),
        native_python_version=inventory['python_version'], host_python_version=sys.version.split()[0],
        host_validation=True, source_executed=False, dependencies_executed=False,
        installed_acceptance=False, domain_behavior_verified=False)
    output.write_text(json.dumps(report, indent=2) + '\n')
    for issue in report['issues']:
        print(f"{issue['filename']}:{issue['line']}:{issue['column']}: "
              f"{issue['module']}: {issue['reason']}", file=sys.stderr)
    return 0 if report['syntax_valid'] and report['dependencies_valid'] \
        and report['implementation_lint_valid'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
