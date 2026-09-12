# InfinityOS Repository Rules

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
