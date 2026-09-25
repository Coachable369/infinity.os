"""Bounded FAT staging: size actual contents and consume only private build copies."""
import argparse
from pathlib import Path
import shutil
import subprocess

MIB = 1024 * 1024

# ------------------------=
# FUNC: image_size
# DESC: Allows FAT metadata, cluster slack and growth above regular-file contents.
# ------------------=
def image_size(tree):
    paths = list(tree.rglob('*'))
    if any(p.is_symlink() for p in paths):
        raise ValueError('staging must not contain symlinks')
    size = sum(p.stat().st_size for p in paths if p.is_file())
    reserve = max(64 * MIB, size // 20) + len(paths) * 32768
    return max(128 * MIB, ((size + reserve + 32 * MIB - 1) // (32 * MIB)) * 32 * MIB)

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
# DESC: Exposes image sizing, phase-specific free-space checks and consumptive FAT copies.
# ------------------=
def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('action', choices=['size', 'consume', 'check'])
    parser.add_argument('path', type=Path)
    parser.add_argument('value', nargs='?')
    args = parser.parse_args()
    if args.action == 'size':
        print(image_size(args.path))
    elif args.action == 'consume':
        consume(args.path, Path(args.value))
    else:
        require_space(args.path, int(args.value))

if __name__ == '__main__':
    main()
