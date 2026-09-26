# InfinityOS workspace layout

All project inputs, temporary files and outputs live beneath
`/opt/codebase/infinity.os`.

`./build-kit` is the sole build entrypoint. Its versioned policy is
`build-kit.toml`; every full, focused, test, Studio, or custom build runs under
the same cleanup, locking, environment, and evidence rules.

- `build/` — disposable products and repository-local temporary files. A full
  `build.sh` run removes this tree before validation or compilation.
- `build/logs/` — disposable logs for the active build only.
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

Use these commands:

- `./build-kit full`
- `./build-kit x86_64`
- `./build-kit aarch64`
- `./build-kit tests`
- `./build-kit studio`
- `./build-kit run <executable> [arguments...]`
- `./build-kit audit`
- `./build-kit prune-logs`

Each run is serialized by `builds/.build-kit.lock`, cleans prior disposable
state exactly once, and writes a JSON outcome to `builds/manifests/`. Failed
runs retain the last published release images.

Log retention is intentionally conservative. Cleanup removes disposable
`build/logs/`, empty `.log` remnants below `builds/`, and root-level compiler
crash dumps matching the configured patterns. Non-empty logs in `builds/` are
durable evidence and are never removed automatically.

Run `./build-kit clean` for the same bounded cleanup used automatically at the
beginning of every build. The cleaner accepts
no arbitrary path, rejects symlinked workspaces and preserves `builds/`,
`model-cache/`, source assets and versioned third-party patches.
