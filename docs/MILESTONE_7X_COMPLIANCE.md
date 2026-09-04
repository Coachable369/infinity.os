# Milestone 7.x Compliance

## Implemented and tested

- Native retained semantic UI primitives and fixed-point layout.
- Versioned `INFSKIN1` packages, safe SVG compiler boundary, semantic icon pack.
- Official dark skin, distinct diagnostic skin, built-in SafeSkin.
- Transactional activation, Previous, LastKnownGood, rollback, and safe mode.
- Typed skin/appearance/UI/window/clipboard IOP identifiers and Console schemas.
- Dependency-ordered Skin Registry, Window Server, InfinityUI, and Clipboard services.
- Identity-backed gold-standard login composition and eleven keyboard/mouse targets.
- Bounded damage, adaptive pointer acceleration, focus traversal, window isolation.
- ISO/live and freshly installed ESP packaging for x86_64 and AArch64; legacy x86
  ISO carries the same architecture-neutral skin sources/packages.

## Explicit limits

Full GPU composition, general third-party applications, arbitrary skin code,
complex-script shaping, native IME providers, screen-reader speech, multi-seat,
and cryptographic third-party package signing are PLANNED. The window server
provides logical/capability isolation; existing kernel-wide MMU limitations still
apply and are not hidden.
