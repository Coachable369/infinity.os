# InfinityOS Repository Rules

## Fresh-install parity is mandatory

Every implementation added to InfinityOS must also be included in the System Generation produced by a fresh installation from the ISO. This includes runtime components, services, manifests, policies, schemas, registries, fonts, artwork, other UI assets, and user-visible behavior.

- A feature is not complete when it works only in the live ISO.
- Update the installer payload and installed-boot path in the same change.
- Add or update an automated parity assertion for every new packaged component.
- Verify the installed system without the ISO attached whenever the affected component can be exercised at boot or runtime.
- Installer-only components may remain live-only only when explicitly identified as installer-only.

## Function documentation

Every function must have this comment header immediately above it:

```text
// ------------------------=
// FUNC: <name>
// DESC: <description>
// ------------------=
```
