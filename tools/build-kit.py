#!/usr/bin/env python3
"""InfinityOS build authority: isolation, cleanup, locking and run evidence."""
import argparse
from datetime import datetime, timezone
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tomllib

PROJECT_ROOT = Path(__file__).resolve().parents[1]
CONFIG_PATH = PROJECT_ROOT / "build-kit.toml"


# ------------------------=
# FUNC: load_workspace_module
# DESC: Loads the bounded workspace manager without creating a second cleanup implementation.
# ------------------=
def load_workspace_module(root=PROJECT_ROOT):
    source = root / "tools/build-workspace.py"
    spec = importlib.util.spec_from_file_location("infinity_build_workspace", source)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


# ------------------------=
# FUNC: load_log_retention_module
# DESC: Loads the bounded log pruner used by clean and build operations.
# ------------------=
def load_log_retention_module(root=PROJECT_ROOT):
    source = root / "tools/log-retention.py"
    spec = importlib.util.spec_from_file_location("infinity_log_retention", source)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


# ------------------------=
# FUNC: load_config
# DESC: Reads and validates the versioned build-kit manifest from the project root.
# ------------------=
def load_config(path=CONFIG_PATH):
    with path.open("rb") as stream:
        config = tomllib.load(stream)
    if config.get("version") != 1 or config.get("project") != "InfinityOS":
        raise ValueError("Unsupported or foreign build-kit manifest")
    if not config.get("profiles"):
        raise ValueError("Build kit has no profiles")
    return config


# ------------------------=
# FUNC: resolve_layout
# DESC: Resolves declared storage classes and rejects paths outside the repository.
# ------------------=
def resolve_layout(root, config):
    root = root.resolve()
    layout = {}
    for role, relative in config["paths"].items():
        path = root / relative
        if path == root or root not in path.resolve(strict=False).parents:
            raise ValueError(f"Build-kit path escaped project root: {role}")
        layout[role] = path
    if len(set(layout.values())) != len(layout):
        raise ValueError("Build-kit storage roles must use distinct paths")
    return layout


# ------------------------=
# FUNC: process_exists
# DESC: Determines whether the process recorded by a build lock is still alive.
# ------------------=
def process_exists(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


# ------------------------=
# FUNC: acquire_lock
# DESC: Serializes builds and replaces only a demonstrably stale lock.
# ------------------=
def acquire_lock(root, layout, build_id):
    lock = layout["releases"] / ".build-kit.lock"
    lock.parent.mkdir(parents=True, exist_ok=True)
    if lock.exists():
        try:
            owner = json.loads(lock.read_text())
            owner_pid = int(owner["pid"])
        except (KeyError, ValueError, json.JSONDecodeError):
            raise RuntimeError(f"Invalid build lock requires inspection: {lock}")
        if process_exists(owner_pid):
            raise RuntimeError(f"Build {owner.get('build_id', 'unknown')} is active as PID {owner_pid}")
        lock.unlink()
    payload = {"build_id": build_id, "pid": os.getpid(), "root": str(root)}
    descriptor = os.open(lock, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
    with os.fdopen(descriptor, "w") as stream:
        json.dump(payload, stream, sort_keys=True)
        stream.write("\n")
    return lock


# ------------------------=
# FUNC: build_environment
# DESC: Creates a contained environment for every subordinate compiler and test process.
# ------------------=
def build_environment(root, layout, build_id):
    temporary = layout["temporary"]
    cache = layout["scratch"] / "cache"
    cargo = layout["scratch"] / "cargo"
    python_cache = cache / "python"
    swift_cache = cache / "swift"
    logs = layout["logs"]
    for path in (temporary, cache, cargo, python_cache, swift_cache, logs):
        path.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    environment.update({
        "TMPDIR": str(temporary),
        "TMP": str(temporary),
        "TEMP": str(temporary),
        "XDG_CACHE_HOME": str(cache),
        "CLANG_MODULE_CACHE_PATH": str(cache / "clang"),
        "CARGO_TARGET_DIR": str(cargo),
        "PYTHONPYCACHEPREFIX": str(python_cache),
        "SWIFTPM_MODULECACHE_OVERRIDE": str(swift_cache),
        "INFINITY_BUILD_KIT_ACTIVE": "1",
        "INFINITY_BUILD_ID": build_id,
        "INFINITY_PROJECT_ROOT": str(root),
        "INFINITY_BUILD_LOG_DIR": str(layout["logs"]),
    })
    return environment


# ------------------------=
# FUNC: git_revision
# DESC: Records the exact local revision without requiring a configured remote.
# ------------------=
def git_revision(root):
    result = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=root, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=False)
    return result.stdout.strip() if result.returncode == 0 else None


# ------------------------=
# FUNC: write_manifest
# DESC: Atomically publishes structured evidence for a completed or failed build run.
# ------------------=
def write_manifest(layout, record):
    destination = layout["manifests"]
    destination.mkdir(parents=True, exist_ok=True)
    final = destination / f"{record['build_id']}.json"
    partial = destination / f".{record['build_id']}.json.partial"
    partial.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n")
    partial.replace(final)
    return final


# ------------------------=
# FUNC: command_for
# DESC: Resolves a named profile or explicit command without shell interpolation.
# ------------------=
def command_for(config, selection, arguments):
    if selection == "run":
        if not arguments:
            raise ValueError("Custom run requires an executable and arguments")
        return arguments
    if selection not in config["profiles"]:
        raise ValueError(f"Unknown build profile: {selection}")
    return [*config["profiles"][selection]["command"], *arguments]


# ------------------------=
# FUNC: execute
# DESC: Cleans once, locks the project, runs one contained build, and records its outcome.
# ------------------=
def execute(selection, arguments, root=PROJECT_ROOT, config_path=CONFIG_PATH):
    root = root.resolve()
    config = load_config(config_path)
    layout = resolve_layout(root, config)
    command = command_for(config, selection, arguments)
    build_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + f"-{os.getpid()}"
    lock = acquire_lock(root, layout, build_id)
    started = datetime.now(timezone.utc)
    removed = []
    pruned_logs = []
    returncode = 1
    manifest = None
    try:
        removed = load_workspace_module(root).clean(root)
        pruned_logs = load_log_retention_module(root).prune(root, config.get("log_retention", {}))
        environment = build_environment(root, layout, build_id)
        returncode = subprocess.run(command, cwd=root, env=environment, check=False).returncode
    finally:
        finished = datetime.now(timezone.utc)
        record = {
            "build_id": build_id,
            "build_kit_version": config["version"],
            "command": command,
            "duration_seconds": round((finished - started).total_seconds(), 3),
            "finished_at": finished.isoformat(),
            "git_revision": git_revision(root),
            "profile": selection,
            "pruned_logs": len(pruned_logs),
            "removed_bytes": sum(entry["bytes"] for entry in removed),
            "result": "passed" if returncode == 0 else "failed",
            "root": str(root),
            "started_at": started.isoformat(),
        }
        try:
            manifest = write_manifest(layout, record)
        finally:
            lock.unlink(missing_ok=True)
    print(f"Build manifest: {manifest}")
    return returncode


# ------------------------=
# FUNC: audit
# DESC: Reports resolved storage ownership and verifies every managed path remains contained.
# ------------------=
def audit(root=PROJECT_ROOT, config_path=CONFIG_PATH):
    config = load_config(config_path)
    layout = resolve_layout(root, config)
    report = {
        "project": config["project"],
        "root": str(root.resolve()),
        "version": config["version"],
        "paths": {role: str(path) for role, path in layout.items()},
        "profiles": sorted(config["profiles"]),
    }
    print(json.dumps(report, indent=2, sort_keys=True))
    return report


# ------------------------=
# FUNC: main
# DESC: Provides the sole supported CLI for full, focused and custom InfinityOS builds.
# ------------------=
def main():
    config = load_config()
    choices = [*sorted(config["profiles"]), "run", "clean", "prune-logs", "audit"]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=choices)
    parser.add_argument("arguments", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.action == "clean":
        removed = load_workspace_module().clean()
        pruned = load_log_retention_module().prune(PROJECT_ROOT, config.get("log_retention", {}))
        print(json.dumps({
            "pruned_logs": len(pruned),
            "removed_bytes": sum(entry["bytes"] for entry in removed),
        }, sort_keys=True))
        return 0
    if args.action == "prune-logs":
        pruned = load_log_retention_module().prune(PROJECT_ROOT, config.get("log_retention", {}))
        print(json.dumps({"pruned": pruned}, indent=2, sort_keys=True))
        return 0
    if args.action == "audit":
        audit()
        return 0
    return execute(args.action, args.arguments)


if __name__ == "__main__":
    sys.exit(main())
