"""Discover the native Python payload and validate generated import requirements.

This is not a sandbox. Validation executes neither the candidate source nor
its dependencies; functional dependency probes are a separate explicit action.
The inventory distinguishes inclusion from successful import: frozen Python
wrappers alone do not establish native-library or operating-system support.
"""
import ast
import importlib
import importlib.util
from importlib.machinery import (BuiltinImporter, FrozenImporter, FileFinder,
    SourceFileLoader, ExtensionFileLoader, SOURCE_SUFFIXES, EXTENSION_SUFFIXES)
import sys
from _infinity_inventory import INVENTORY


class DependencyError(ImportError):
    # ------------------------=
    # FUNC: __init__
    # DESC: Retains structured source locations and module errors for editor diagnostics.
    # ------------------=
    def __init__(self, issues):
        self.issues = tuple(issues)
        super().__init__("; ".join(
            f"{item['filename']}:{item['line']}:{item['column']}: "
            f"{item['module']}: {item['reason']}" for item in issues))


class IncompleteApplicationError(ValueError):
    # ------------------------=
    # FUNC: __init__
    # DESC: Retains typed incomplete-source lint diagnostics without claiming application behavior was tested.
    # ------------------=
    def __init__(self, issues):
        self.issues = tuple(issues)
        super().__init__("; ".join(
            f"{item['filename']}:{item['line']}:{item['column']}: "
            f"{item['reason']}" for item in issues))


# ------------------------=
# FUNC: inventory
# DESC: Returns a detached versioned build inventory without claiming all included wrappers are functional.
# ------------------=
def inventory():
    return {
        "schema_version": INVENTORY["schema_version"],
        "python_version": INVENTORY["python_version"],
        "architecture": INVENTORY["architecture"],
        "modules": tuple(dict(item) for item in INVENTORY["modules"]),
        "limitations": dict(INVENTORY["limitations"]),
        "required_backends": dict(INVENTORY.get("required_backends", {})),
        "supported_scopes": dict(INVENTORY.get("supported_scopes", {})),
        "generation_libraries": {key: tuple(value) for key, value in INVENTORY.get("generation_libraries", {}).items()},
    }


# ------------------------=
# FUNC: generation_catalog
# DESC: Lists curated available defaults and sanitized mounted package roots without executing candidates, package initializers or arbitrary import hooks.
# ------------------=
def generation_catalog(package_path="/packages"):
    import os
    import keyword
    defaults = {}
    for purpose, names in INVENTORY.get("generation_libraries", {}).items():
        available = []
        for name in names:
            if unavailable_reason(name) or find_spec_without_import(name) is None:
                continue
            if any(find_spec_without_import(backend) is None
                   for backend in INVENTORY.get("required_backends", {}).get(name, ())):
                continue
            available.append(name)
        defaults[purpose] = tuple(available)
    roots, scanned, omitted = set(), 0, False
    try:
        with os.scandir(package_path) as entries:
            for entry in entries:
                scanned += 1
                if scanned > 2048:
                    omitted = True
                    break
                if entry.is_symlink():
                    continue
                name = entry.name if entry.is_dir(follow_symlinks=False) else \
                    entry.name.split(".", 1)[0] if entry.name.endswith((".py", ".so")) else ""
                if not name or len(name) > 63 or not name.isascii() or not name.isidentifier() \
                        or keyword.iskeyword(name) or name == "__pycache__":
                    continue
                if len(roots) == 48 and name not in roots:
                    omitted = True
                    continue
                roots.add(name)
        present = True
    except OSError:
        present = False
        roots.clear()
    return dict(schema_version=1, default_libraries=defaults,
                installed_import_roots=tuple(sorted(roots)), directory_available=present,
                omitted=omitted, package_apis_verified=False,
                supported_scopes=dict(INVENTORY.get("supported_scopes", {})),
                limitations=dict(INVENTORY["limitations"]))


# ------------------------=
# FUNC: installed_distributions
# DESC: Enumerates packages visible to this interpreter's authorized immutable package snapshot.
# ------------------=
def installed_distributions():
    from importlib.metadata import distributions
    return tuple(sorted(
        ((item.metadata.get("Name", ""), item.version) for item in distributions()),
        key=lambda item: item[0].casefold()))


# ------------------------=
# FUNC: requirement_issue
# DESC: Creates one machine-readable dependency diagnostic at its original source location.
# ------------------=
def requirement_issue(node, filename, module, reason):
    return dict(filename=filename, line=getattr(node, "lineno", 1),
                column=getattr(node, "col_offset", 0) + 1,
                module=module, reason=reason)


# ------------------------=
# FUNC: import_handle
# DESC: Recognizes only direct import-loader references, named aliases and literal getattr access without evaluating candidate code.
# ------------------=
def import_handle(node, modules, loaders):
    if isinstance(node, ast.Name):
        return "module" if node.id in modules else "loader" if node.id in loaders else None
    if isinstance(node, ast.Attribute) and node.attr == "import_module" \
            and import_handle(node.value, modules, loaders) == "module":
        return "loader"
    if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) \
            and node.func.id == "getattr" and node.args \
            and import_handle(node.args[0], modules, loaders) == "module":
        attribute = node.args[1] if len(node.args) > 1 else None
        if isinstance(attribute, ast.Constant) and isinstance(attribute.value, str):
            return "loader" if attribute.value == "import_module" else None
        return "unresolved"
    return None


# ------------------------=
# FUNC: requirements
# DESC: Extracts static imports and literal dynamic imports without executing application source.
# ------------------=
def requirements(source, filename="<generated>"):
    tree = ast.parse(source, filename=filename)
    # Parsing alone accepts some invalid module-level constructs such as return.
    compile(tree, filename, "exec")
    imports, issues = [], []
    importlib_names, importer_names = {"importlib"}, {"__import__"}
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                imports.append((alias.name, node, ()))
                if alias.name == "importlib":
                    importlib_names.add(alias.asname or alias.name)
        elif isinstance(node, ast.ImportFrom):
            if node.level:
                issues.append(requirement_issue(node, filename, node.module or ".",
                    "relative imports require an explicitly packaged application"))
            elif node.module:
                imports.append((node.module, node, tuple(alias.name for alias in node.names)))
                if node.module == "importlib":
                    importer_names.update(alias.asname or alias.name for alias in node.names
                                          if alias.name == "import_module")
                elif node.module == "builtins":
                    importer_names.update(alias.asname or alias.name for alias in node.names
                                          if alias.name == "__import__")
    # Track simple named aliases only, independent of source order. Keeping a
    # name tainted after reassignment is deliberately conservative; this is not
    # a general interpreter or speculative data-flow evaluator.
    assignments = [node for node in ast.walk(tree)
                   if isinstance(node, (ast.Assign, ast.AnnAssign, ast.NamedExpr))]
    for _ in range(len(assignments) + 1):
        changed = False
        for node in assignments:
            kind = import_handle(node.value, importlib_names, importer_names)
            targets = node.targets if isinstance(node, ast.Assign) else (node.target,)
            names = importlib_names if kind == "module" else importer_names if kind == "loader" else None
            if names is not None:
                for target in targets:
                    if isinstance(target, ast.Name) and target.id not in names:
                        names.add(target.id)
                        changed = True
        if not changed:
            break
    parents = {child: parent for parent in ast.walk(tree) for child in ast.iter_child_nodes(parent)}
    for node in ast.walk(tree):
        kind = import_handle(node, importlib_names, importer_names)
        if kind is None or isinstance(node, ast.Name) and not isinstance(node.ctx, ast.Load):
            continue
        parent = parents.get(node)
        # An importlib module is also a normal module. Restrict only its
        # loader-producing accesses and aliases; unrelated attribute use is safe.
        if kind == "module":
            continue
        named_alias = isinstance(parent, (ast.Assign, ast.AnnAssign, ast.NamedExpr)) \
            and parent.value is node and all(isinstance(target, ast.Name) for target in
                (parent.targets if isinstance(parent, ast.Assign) else (parent.target,)))
        direct_call = isinstance(parent, ast.Call) and parent.func is node
        if kind == "unresolved" or not (named_alias or direct_call):
            issues.append(requirement_issue(node, filename, "<dynamic import>",
                "import loaders must be called directly or assigned to a simple named alias"))
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call):
            continue
        func = node.func
        dynamic = import_handle(func, importlib_names, importer_names) == "loader"
        if not dynamic:
            continue
        argument = node.args[0] if node.args else next(
            (item.value for item in node.keywords if item.arg == "name"), None)
        if isinstance(argument, ast.Constant) and isinstance(argument.value, str) \
                and not argument.value.startswith("."):
            imports.append((argument.value, node, ()))
        else:
            issues.append(requirement_issue(node, filename, "<dynamic import>",
                "generated code must declare a literal, installed module name"))
    return imports, issues


# ------------------------=
# FUNC: unavailable_reason
# DESC: Finds the most specific declared platform limitation for a module rather than trusting an importable stub.
# ------------------=
def unavailable_reason(name):
    limitations = INVENTORY["limitations"]
    while name:
        if name in limitations:
            return limitations[name]
        name = name.rpartition(".")[0]
    return None


# ------------------------=
# FUNC: find_spec_without_import
# DESC: Resolves builtins, frozen modules and granted files without executing package initializers or arbitrary meta finders.
# ------------------=
def find_spec_without_import(name):
    path = list(sys.path)
    parts = name.split(".")
    for index in range(len(parts)):
        qualified = ".".join(parts[:index + 1])
        spec = BuiltinImporter.find_spec(qualified) or FrozenImporter.find_spec(qualified)
        if spec is None:
            for directory in path:
                if not isinstance(directory, str) or not directory:
                    continue
                finder = FileFinder(directory, (SourceFileLoader, SOURCE_SUFFIXES),
                                    (ExtensionFileLoader, EXTENSION_SUFFIXES))
                spec = finder.find_spec(qualified)
                if spec is not None:
                    break
        if spec is None:
            return None
        if index + 1 < len(parts):
            if spec.submodule_search_locations is None:
                return None
            path = list(spec.submodule_search_locations)
    return spec


# ------------------------=
# FUNC: validate_source
# DESC: Compiles and resolves generated import metadata using only authorized modules; neither source nor dependencies are run.
# ------------------=
def validate_source(source, filename="<generated>"):
    imports, issues = requirements(source, filename)
    declared = {item["name"] for item in INVENTORY.get("modules", ())}
    checked = set()
    for name, node, members in imports:
        key = name, members
        if key in checked:
            continue
        checked.add(key)
        reason = unavailable_reason(name)
        if reason:
            issues.append(requirement_issue(node, filename, name, reason))
            continue
        if find_spec_without_import(name) is None:
            issues.append(requirement_issue(node, filename, name,
                "module is not present in the installed interpreter or authorized packages"))
            continue
        ancestor = name
        while ancestor:
            for backend in INVENTORY.get("required_backends", {}).get(ancestor, ()):
                if find_spec_without_import(backend) is None:
                    issues.append(requirement_issue(node, filename, name,
                        "required native backend is not installed: " + backend))
            ancestor = ancestor.rpartition(".")[0]
        # A from-import may name an attribute or a submodule. Resolve only known
        # submodules without importing parent initializers. Attributes remain
        # Python's responsibility; they must not be mistaken for dependencies.
        for member in members:
            if member == "*":
                continue
            candidate = name + "." + member
            if candidate in declared or unavailable_reason(candidate) \
                    or find_spec_without_import(candidate) is not None:
                imports.append((candidate, node, ()))
    if issues:
        raise DependencyError(issues)
    return tuple(sorted({name for name, _, _ in imports}))


# ------------------------=
# FUNC: placeholder_statement
# DESC: Recognizes explicit unimplemented syntax only, not returns, callbacks, loops or application semantics.
# ------------------=
def placeholder_statement(node):
    if isinstance(node, ast.Pass):
        return True
    if isinstance(node, ast.Expr) and isinstance(node.value, ast.Constant):
        return node.value.value is Ellipsis or isinstance(node.value.value, str)
    if isinstance(node, ast.Raise) and node.cause is None:
        error = node.exc.func if isinstance(node.exc, ast.Call) else node.exc
        return isinstance(error, ast.Name) and error.id == "NotImplementedError"
    return False


# ------------------------=
# FUNC: literal_expression
# DESC: Checks that scaffold initializers and entrypoint arguments have no executable expression.
# ------------------=
def literal_expression(node):
    try:
        ast.literal_eval(node)
        return True
    except (ValueError, TypeError, SyntaxError, RecursionError, MemoryError):
        return False


# ------------------------=
# FUNC: scaffold_statement
# DESC: Conservatively recognizes declarations and direct local entrypoint wiring around wholly placeholder bodies.
# ------------------=
def scaffold_statement(node, local_names, abstract_names=frozenset(), class_names=frozenset()):
    if placeholder_statement(node) or isinstance(node, (ast.Import, ast.ImportFrom)):
        return True
    if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
        # Decorators and default/annotation calls can supply real behavior.
        signature = (*node.args.defaults, *node.args.kw_defaults,
                     *(arg.annotation for arg in (*node.args.posonlyargs,
                         *node.args.args, *node.args.kwonlyargs)),
                     node.args.vararg.annotation if node.args.vararg else None,
                     node.args.kwarg.annotation if node.args.kwarg else None,
                     node.returns)
        decorators_safe = all(qualified_name(value) in abstract_names
                              for value in node.decorator_list)
        return decorators_safe and not any(
            isinstance(child, (ast.Call, ast.Await, ast.Yield, ast.YieldFrom))
            for expression in signature if expression is not None
            for child in ast.walk(expression))
    if isinstance(node, ast.ClassDef):
        # Only a proven empty guarded entrypoint enables known abstract/local
        # skeleton inheritance. External bases or decorators may implement
        # behavior and remain outside this conservative lint.
        bases_safe = all(qualified_name(value) in abstract_names | class_names
                         for value in node.bases)
        return bases_safe and not (node.keywords or node.decorator_list) and all(
            scaffold_statement(child, local_names, abstract_names, class_names)
            for child in node.body)
    if isinstance(node, (ast.Assign, ast.AnnAssign)):
        if isinstance(node, ast.AnnAssign) and any(
                isinstance(child, (ast.Call, ast.Await, ast.Yield, ast.YieldFrom))
                for child in ast.walk(node.annotation)):
            return False
        targets = node.targets if isinstance(node, ast.Assign) else (node.target,)
        return all(isinstance(target, ast.Name) for target in targets) \
            and (node.value is None or literal_expression(node.value))
    if isinstance(node, ast.Expr) and isinstance(node.value, ast.Call):
        call = node.value
        return isinstance(call.func, ast.Name) and call.func.id in local_names \
            and all(literal_expression(value) for value in call.args) \
            and all(value.arg is not None and literal_expression(value.value)
                    for value in call.keywords)
    if isinstance(node, ast.If):
        return main_guard(node.test) and all(scaffold_statement(child, local_names,
            abstract_names, class_names) for child in (*node.body, *node.orelse))
    return False


# ------------------------=
# FUNC: qualified_name
# DESC: Reads a static Python name without evaluating attributes or following arbitrary aliases.
# ------------------=
def qualified_name(node):
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        parent = qualified_name(node.value)
        return parent + "." + node.attr if parent else None
    return None


# ------------------------=
# FUNC: main_guard
# DESC: Identifies only the conventional direct script entrypoint condition.
# ------------------=
def main_guard(node):
    return isinstance(node, ast.Compare) and len(node.ops) == 1 \
        and isinstance(node.ops[0], ast.Eq) and len(node.comparators) == 1 \
        and isinstance(node.left, ast.Name) and node.left.id == "__name__" \
        and isinstance(node.comparators[0], ast.Constant) \
        and node.comparators[0].value == "__main__"


# ------------------------=
# FUNC: empty_entrypoint_abstract_names
# DESC: Allows standard abstract skeleton declarations only beside a direct guarded call to an undecorated empty local entrypoint.
# ------------------=
def empty_entrypoint_abstract_names(tree, placeholders, local_names):
    local_empty = {node.name for node in placeholders
                   if node in tree.body and node.name in local_names and not node.decorator_list}
    guarded_empty = any(isinstance(node, ast.If) and main_guard(node.test)
        and any(isinstance(statement, ast.Expr) and isinstance(statement.value, ast.Call)
                and isinstance(statement.value.func, ast.Name)
                and statement.value.func.id in local_empty for statement in node.body)
        for node in tree.body)
    if not guarded_empty:
        return frozenset(), frozenset()
    abstract_names = set()
    for node in tree.body:
        if isinstance(node, ast.Import):
            for alias in node.names:
                if alias.name == "abc":
                    name = alias.asname or alias.name
                    abstract_names.update((name + ".ABC", name + ".abstractmethod"))
        elif isinstance(node, ast.ImportFrom) and node.module == "abc" and not node.level:
            abstract_names.update(alias.asname or alias.name for alias in node.names
                                  if alias.name in ("ABC", "abstractmethod"))
    return abstract_names, {node.name for node in tree.body
                           if isinstance(node, ast.ClassDef) and node.name in local_names}


# ------------------------=
# FUNC: module_rebindings
# DESC: Conservatively excludes imported or reassigned module names from local-stub reasoning without evaluating binding order or nested scopes.
# ------------------=
def module_rebindings(tree):
    names, wildcard = set(), False
    pending = list(tree.body)
    while pending:
        node = pending.pop()
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef, ast.Lambda)):
            continue
        if isinstance(node, ast.Import):
            names.update(alias.asname or alias.name.split(".", 1)[0] for alias in node.names)
        elif isinstance(node, ast.ImportFrom):
            wildcard = wildcard or any(alias.name == "*" for alias in node.names)
            names.update(alias.asname or alias.name for alias in node.names)
        elif isinstance(node, ast.Name) and isinstance(node.ctx, (ast.Store, ast.Del)):
            names.add(node.id)
        pending.extend(ast.iter_child_nodes(node))
    return names, wildcard


# ------------------------=
# FUNC: validate_generated_application
# DESC: Adds a conservative whole-placeholder lint only for AI-generated applications; this never executes source or proves functionality.
# ------------------=
def validate_generated_application(source, filename="<generated>"):
    imports = validate_source(source, filename)
    tree = ast.parse(source, filename=filename)
    functions = [node for node in ast.walk(tree)
                 if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))]
    placeholders = [node for node in functions
                    if all(placeholder_statement(statement) for statement in node.body)]
    local_names = {node.name for node in tree.body
                   if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef))}
    rebound, wildcard = module_rebindings(tree)
    local_names = set() if wildcard else local_names - rebound
    explicit_empty = next((node for node in tree.body if isinstance(node, ast.Pass)
        or isinstance(node, ast.Expr) and isinstance(node.value, ast.Constant)
        and node.value.value is Ellipsis), None)
    abstract_names, class_names = empty_entrypoint_abstract_names(tree, placeholders, local_names)
    if (placeholders and len(placeholders) == len(functions)
            or not functions and explicit_empty is not None) \
            and all(scaffold_statement(node, local_names, abstract_names, class_names)
                    for node in tree.body):
        target = placeholders[0] if placeholders else explicit_empty
        issue = requirement_issue(target, filename, "<implementation>",
            "generated application contains only unimplemented placeholders; implement the requested behavior")
        issue["kind"] = "whole_application_placeholder"
        raise IncompleteApplicationError((issue,))
    return imports


# ------------------------=
# FUNC: inspect_module
# DESC: Reports actual runtime import success separately from build inclusion for one explicitly requested dependency.
# ------------------=
def inspect_module(name):
    reason = unavailable_reason(name)
    if reason:
        return dict(module=name, importable=False, reason=reason)
    try:
        module = importlib.import_module(name)
        return dict(module=name, importable=True,
                    origin=getattr(getattr(module, "__spec__", None), "origin", None))
    except (Exception, SystemExit) as error:
        return dict(module=name, importable=False,
                    reason=type(error).__name__ + ": " + str(error))


# ------------------------=
# FUNC: main
# DESC: Provides a JSON module/package directory or explicit import probes through the native console.
# ------------------=
def main(arguments=None):
    import json
    arguments = sys.argv[1:] if arguments is None else arguments
    if arguments == ["--generation-catalog"]:
        print(json.dumps(generation_catalog(), sort_keys=True))
        return 0
    if arguments:
        values = [inspect_module(name) for name in arguments]
        print(json.dumps(values, sort_keys=True))
        return 0 if all(item["importable"] for item in values) else 1
    value = inventory()
    value["installed_distributions"] = installed_distributions()
    print(json.dumps(value, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
