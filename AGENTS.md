# InfinityOS Repository Rules

## Shared architecture ports

Maintain one shared implementation for each feature across aarch64 and x86_64.
Select target-specific code through the build target/compiler configuration.
Keep CPU instructions, timers, worker startup, and device interfaces behind small
native adapters; do not fork application logic or introduce host-OS services.
Apply this rule to future ports as well as the speech stack. Verify both live and
freshly installed paths for each target before claiming that target complete.

## Build authority

Every build, test build, focused architecture build, compiler probe, Studio
build, and agent-performed verification build must run through the repository
build kit. Do not invoke `make`, `cargo build`, `swift build`, or subordinate
build scripts directly.

- Normal dependency-aware build: `./build-kit incremental` or `./build.sh`
- Clean release build: `./build-kit full`
- Focused profiles: `./build-kit x86_64`, `./build-kit aarch64`,
  `./build-kit tests`, or `./build-kit studio`
- Other bounded commands: `./build-kit run <executable> [arguments...]`
- Workspace inspection: `./build-kit audit`

The build kit owns cleanup policy, serialization, repository-local
temporary/cache paths, and run manifests. Incremental, focused, and custom runs
retain valid build products; full builds and explicit `clean` remove them. A
release or fresh-build claim requires `./build-kit full`. Adding a new build
entrypoint requires adding a named profile to `build-kit.toml` or invoking it
through `build-kit run`. No build may write project data outside
`/opt/codebase/infinity.os`.

## Deprecation removal

A file marked or declared deprecated must be removed in the same change that
migrates its consumers. Do not retain deprecated source, assets, templates,
scripts, generated payloads, compatibility copies, or renamed backups in the
repository or build packages.

- Verify that runtime, build, installer, fresh-install parity, and test consumers
  have migrated before deletion.
- Delete the deprecated file and remove its packaging, manifest, dependency, and
  test references.
- If compatibility requires retention, the file is not yet deprecated; document
  the active compatibility contract and its removal gate instead.
- Deprecation warnings must not be silenced to preserve dead files.

## Correction-loop limit

Stop and report remaining acceptance failures after six unsuccessful correction
loops, rather than two. Avoid speculative rewrites between verification passes.

## Fresh-install parity is mandatory

Every implementation added to InfinityOS must also be included in the System Generation produced by a fresh installation from the ISO. This includes runtime components, services, manifests, policies, schemas, registries, fonts, artwork, other UI assets, and user-visible behavior.

- A feature is not complete when it works only in the live ISO.
- Update the installer payload and installed-boot path in the same change.
- Add or update an automated parity assertion for every new packaged component.
- Verify the installed system without the ISO attached whenever the affected component can be exercised at boot or runtime.
- Installer-only components may remain live-only only when explicitly identified as installer-only.

## Automated verification must be behavioral

Automated tests must verify observable behavior and state transitions. They must not use source text, rendered copy, log messages, symbol names, or command output strings as acceptance evidence.

- Do not use `grep`, `rg`, substring matching, snapshots of prose, or equivalent text searches as a test oracle.
- Exercise the implementation through a typed API, executable harness, VM interaction, artifact extraction, or state inspection.
- Assert structured values, binary/artifact contents, state changes, pixels/damage regions, protocol results, exit status, or other observable outcomes.
- Human-readable text may be emitted for diagnostics, but changing that text must not make a behavioral test pass or fail.
- Static lint checks may enforce code style, but they must be explicitly labeled as lint and must never be reported as behavioral verification.
- Existing text-oracle tests must be converted before they can be cited as evidence that a feature works.

## Function documentation

Every function must have this comment header immediately above it:

```text
// ------------------------=
// FUNC: <name>
// DESC: <description>
// ------------------=
```
