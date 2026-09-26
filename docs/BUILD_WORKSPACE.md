# InfinityOS workspace layout

All project inputs, temporary files and outputs live beneath
`/opt/codebase/infinity.os`.

- `build/` — disposable products and repository-local temporary files. A full
  `build.sh` run removes this tree before validation or compilation.
- `builds/` — durable release ISOs, checksums, benchmarks and explicitly saved
  evidence. A failed build must not remove the last published release.
- `model-cache/` — durable downloaded model inputs. Builds reuse these files.
- `third_party/patches/` — versioned modifications needed to reproduce work
  performed against external source trees.
- `target/` and nested `target/`/`.build/` directories — disposable language
  and toolchain caches removed by the managed cleanup.

`build.sh` exports `TMPDIR`, `TMP` and `TEMP` as `build/tmp` before invoking
subordinate tools. Build tooling must not write project state to system
temporary directories.

Run `python3 tools/build-workspace.py clean` or `make clean` for the same bounded
cleanup used automatically at the beginning of `build.sh`. The cleaner accepts
no arbitrary path, rejects symlinked workspaces and preserves `builds/`,
`model-cache/`, source assets and versioned third-party patches.
