# InfinityOS workspace layout

All project inputs, temporary files and outputs live beneath
`/opt/codebase/infinity.os`.

`./build-kit` is the sole build entrypoint. Its versioned policy is
`build-kit.toml`; every full, incremental, focused, test, Studio, or custom
build runs under the same locking, environment, and evidence rules.

- `build/` — reusable compiler products and repository-local temporary files.
  A full build removes this tree before validation or compilation; incremental
  builds retain it so Make, Cargo, Swift, and Clang can rebuild changed inputs.
- `build/logs/` — disposable logs for the active build only.
- `builds/` — durable release ISOs, checksums, benchmarks and explicitly saved
  evidence. A failed build must not remove the last published release.
- `model-cache/` — durable downloaded model inputs. Builds reuse these files.
- `third_party/patches/` — versioned modifications needed to reproduce work
  performed against external source trees.
- `target/` and nested `target/`/`.build/` directories — reusable language and
  toolchain caches retained incrementally and removed by a full build or
  explicit managed cleanup.

`build.sh` exports `TMPDIR`, `TMP` and `TEMP` as `build/tmp` before invoking
subordinate tools. Build tooling must not write project state to system
temporary directories.

Use these commands:

- `./build.sh` or `./build-kit incremental` — normal dependency-aware build
- `./build-kit full`
- `./build-kit x86_64`
- `./build-kit aarch64`
- `./build-kit package-aarch64-qemu` — full-model ARM QEMU test installer;
  refreshes the installed kernel and reuses valid native dependencies without
  the broad regression suite. Publishes only
  `build/test-media/InfinityOS-aarch64-qemu-models.bootmedia` and its SHA-256,
  leaving the canonical release ISOs and checksum index unchanged. It uses the
  shared native implementation linked for QEMU RAM, with the same Hermes and
  Ministral payload, loader, audio, kernel-byte and full-bundle parity gates.
- `./build-kit tests`
- `./build-kit studio`
- `./build-kit run <executable> [arguments...]`
- `./build-kit audit`
- `./build-kit prune-logs`

Each run is serialized by `builds/.build-kit.lock` and writes a JSON outcome to
`builds/manifests/`, including whether cleanup occurred. Incremental and focused
runs retain compiler products while still rebuilding source and asset updates
through the Make dependency graph. Full builds clean prior disposable state
exactly once. Failed runs retain the last published release images.

Log retention is intentionally conservative. Cleanup removes disposable
`build/logs/`, empty `.log` remnants below `builds/`, and root-level compiler
crash dumps matching the configured patterns. Non-empty logs in `builds/` are
durable evidence and are never removed automatically.

Run `./build-kit clean` for the same bounded cleanup used automatically at the
beginning of `./build-kit full`. The cleaner accepts
no arbitrary path, rejects symlinked workspaces and preserves `builds/`,
`model-cache/`, source assets and versioned third-party patches.

For isolated generated-application acceptance after that profile succeeds:

```sh
./build-kit run python3 tools/native-editor-installed-test.py \
  --arch aarch64 --accel hvf \
  --iso build/test-media/InfinityOS-aarch64-qemu-models.bootmedia \
  --generate-request 'Create a graphical tic tac toe game and play against me.' \
  --generation-timeout 900 \
  build/acceptance/generated-arm-01
```

The harness creates a private sparse 32 GiB disk and a 12 GiB guest, installs the
tested media, then cold-boots without its ISO. It never changes a VirtualBox VM.
Supply `--application-case <repository-local JSON>` for independent domain
assertions; generation, compilation and launch alone are not game/app correctness
evidence. Test media is not a full release claim. Keep generation probes and this
VM serialized to avoid competing inference workloads.

To establish the reusable installed-disk baseline without starting inference,
use `--boot-only` instead of `--generate-request` and omit generation/case options.
This verifies account setup, media-detached cold boot, preserved node identity
and the authenticated desktop; `installed_acceptance` remains false because no
generated application has been tested. Its private disk is stopped on return.
Subsequent runs may use the same work directory with `--reuse-installed` and
`--update-kernel build/aarch64/installed-kernel-qemu.elf`. That explicit clone
update is iteration evidence, not fresh-install parity or release evidence.

For buffered-delivery and narration verification on that stopped private disk:

```sh
./build-kit run python3 tools/native-editor-installed-test.py \
  --arch aarch64 --accel hvf --reuse-installed \
  --update-kernel build/aarch64/installed-kernel-qemu.elf \
  --streaming-only --audio-checkpoints \
  build/acceptance/generated-arm-01
```

This requires increasing editor deliveries while the real model is still running,
edit cancellation without compilation, and subsequent inference admission without
changing the preserved document. The optional audio mode adds emulated HDA and
records repository-local WAV output; acceptance requires a completed narration
checkpoint and non-silent PCM during the generation interval. It does not prove
microphone capture, physical speaker output, or generated-application correctness.
