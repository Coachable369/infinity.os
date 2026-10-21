"""Bounded FAT staging: size actual contents and consume only private build copies."""
import argparse
from pathlib import Path
import shutil
import subprocess

MIB = 1024 * 1024
ROOT = Path(__file__).resolve().parents[1]

# ------------------------=
# FUNC: reset_ui_stage
# DESC: Removes only validated managed UI subtrees from either disposable installed-ESP staging tree before repopulation.
# ------------------=
def reset_ui_stage(tree):
    tree = Path(tree)
    if not tree.is_absolute():
        tree = ROOT / tree
    allowed = (ROOT / 'build/installed-fat', ROOT / 'build/installed-fat-aarch64')
    if tree not in allowed:
        raise ValueError('UI reset requires an exact managed installed-ESP staging path')
    targets = [tree / 'EFI/InfinityOS/InfinityUI' / name for name in ('Installer', 'DesignKit')]
    # Validate every target before removing any. Resolving a symlink first would
    # lose the evidence that a managed path was redirected outside its ownership.
    for target in targets:
        current = ROOT
        for component in target.relative_to(ROOT).parts:
            current = current / component
            if current.is_symlink() or (current.exists() and not current.is_dir()):
                raise ValueError('UI staging ancestors must be real directories')
        if target.exists():
            for path in target.rglob('*'):
                if path.is_symlink() or not (path.is_dir() or path.is_file()):
                    raise ValueError('UI staging must not contain symlinks or special files')
    for target in targets:
        if target.exists():
            shutil.rmtree(target)

# ------------------------=
# FUNC: image_size
# DESC: Allows FAT metadata, cluster slack and growth above regular-file contents.
# ------------------=
def image_size(tree):
    paths = list(tree.rglob('*'))
    if any(p.is_symlink() for p in paths):
        raise ValueError('staging must not contain symlinks')
    size = sum(p.stat().st_size for p in paths if p.is_file())
    # FAT allocation grows beyond the byte total through cluster slack, directory
    # entries and mtools bookkeeping. Keep enough headroom for large kernels and
    # asset families instead of allowing a nearly-full image to fail in mcopy.
    reserve = max(128 * MIB, size // 5) + len(paths) * 32768
    return max(128 * MIB, ((size + reserve + 32 * MIB - 1) // (32 * MIB)) * 32 * MIB)

# ------------------------=
# FUNC: allocate
# DESC: Creates a right-sized sparse image after checking durable workspace capacity.
# ------------------=
def allocate(tree, image):
    size = image_size(tree)
    image.parent.mkdir(parents=True, exist_ok=True)
    require_space(image.parent, size)
    with image.open('wb') as stream:
        stream.truncate(size)
    return size

# ------------------------=
# FUNC: require_space
# DESC: Fails before allocation while preserving existing release artifacts.
# ------------------=
def require_space(destination, needed):
    free = shutil.disk_usage(destination).free
    if free < needed:
        raise RuntimeError(f'Build needs {needed / MIB:.0f} MiB free at {destination}; '
                           f'only {free / MIB:.0f} MiB available. Existing ISO preserved.')

# ------------------------=
# FUNC: consume
# DESC: Copies private staging files into FAT and unlinks each only after successful copy.
# ------------------=
def consume(tree, image):
    paths = sorted(tree.rglob('*'))
    if any(p.is_symlink() for p in paths):
        raise ValueError('staging must not contain symlinks')
    for path in paths:
        target = '::/' + path.relative_to(tree).as_posix()
        if path.is_dir():
            subprocess.run(['mmd', '-i', str(image), target], check=True)
        else:
            subprocess.run(['mcopy', '-i', str(image), str(path), target], check=True)
            path.unlink()

# ------------------------=
# FUNC: main
# DESC: Exposes image sizing, bounded UI staging reset, free-space checks and consumptive FAT copies.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('action', choices=['size', 'allocate', 'consume', 'check', 'reset-ui'])
    parser.add_argument('path', type=Path)
    parser.add_argument('value', nargs='?')
    args = parser.parse_args()
    if args.action == 'size':
        print(image_size(args.path))
    elif args.action == 'allocate':
        print(allocate(args.path, Path(args.value)))
    elif args.action == 'consume':
        consume(args.path, Path(args.value))
    elif args.action == 'reset-ui':
        reset_ui_stage(args.path)
    else:
        require_space(args.path, int(args.value))

if __name__ == '__main__':
    main()
