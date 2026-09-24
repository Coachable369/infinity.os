"""Keep persistent installed acceptance disks outside disposable build output."""
from pathlib import Path


# ------------------------=
# FUNC: validate_workspace
# DESC: Rejects paths erased by normal build cleanup, including symlink aliases, before any guest or artifact is created.
# ------------------=
def validate_workspace(output, root=None):
    root = (Path(root) if root is not None else Path(__file__).parents[1]).resolve()
    work = Path(output).resolve()
    for disposable in (root / "build", root / "target"):
        if work == disposable or disposable in work.parents:
            raise ValueError("Installed acceptance requires persistent output outside build/ and target/; use builds/ms10-acceptance/<run>.")
    return work
