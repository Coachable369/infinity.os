# Installed runtime tree

`runtime/install/` is the staging root for files shipped with InfinityOS.
Its contents map to the guest root `/`; `install` is not a guest directory.

| Repository source | Installed path |
| --- | --- |
| `runtime/install/System/awk.py` | `/System/awk.py` |
| `runtime/install/System/Tools/example.py` | `/System/Tools/example.py` |
| `runtime/install/Applications/example/config.json` | `/Applications/example/config.json` |

All regular files and directories are discovered recursively on every package
build. Additions, edits and removals update the generated native catalog. Empty
directories are retained. Symlinks, special files, paths longer than the native
95-byte limit and collisions with generated SDK or user-template entries fail
packaging explicitly. Do not put secrets or host-only files in this tree.

Files are immutable system-generation resources available through the same
object service in live and freshly installed systems. Rebuilding an ISO does
not update an existing VM. Python execution permissions and shell command/help
registration remain separate from deployment; adding a script does not grant
it execution capabilities automatically.

Use `./build-kit sdk-tests` for catalog/object-store checks, and `./build-kit full`
for ARM-first and x86_64 release packaging. Release parity checks include every
staged file in both live and installed kernels.
