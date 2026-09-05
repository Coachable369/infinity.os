# Native Resolver

The Resolver owns a bounded typed cache with positive and negative entries,
explicit expiry, family-neutral address results, source identity, and deadline
failure. Local system names and deterministic test fixtures are supported.

Deadline, positive cache, expiry, resolver-disabled, and bounded-cache behavior
are **TESTED**. DNS packet transport, response parser hardening, configurable
upstream resolvers, and canonical-name validation are **UNSUPPORTED** until a
wire adapter exists. Calls fail with typed `ResolutionTimeout`,
`ResolverUnavailable`, or `NameResolutionFailed`; they never wait forever.
