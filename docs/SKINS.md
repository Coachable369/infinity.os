# Pluggable Skins

`INFSKIN1` is an explicit little-endian, versioned package format. A package
declares a stable Skin ID, minimum InfinityUI ABI, integrity hash, tokens,
wallpaper, semantic icons, typography, metrics, and motion. Rust memory layout is
not an on-disk ABI.

The build-time compiler accepts a narrow source projection and safe SVG subset.
It rejects scripts, foreign objects, external references, filters, entities,
unbounded input, missing IDs, and icon sets smaller than the required semantic
pack. Runtime packages contain XML-free `IVEC1` vector records.

Activation is transactional:

1. Validate schema, ABI, identity, inheritance, and integrity.
2. Preserve `Previous`.
3. Activate and invalidate the appearance generation.
4. Mark the result `LastKnownGood` only after activation succeeds.
5. Roll back on failure; activate dependency-free `infinity.safe` for recovery.

Machine, user, and session scopes are explicit. `infinity.default.dark` is the
official visual system matching the supplied gold-standard login screen.
`infinity.diagnostic.light` is deliberately distinct and proves that skin
replacement is real. `infinity.safe` remains compiled into the runtime.

Parser, corruption rejection, alternate activation, rollback, and SafeSkin are
TESTED. Cryptographic signatures and third-party distribution are PLANNED.
