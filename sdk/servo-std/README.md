# Infinity Rust std port: initial adapter sources

Compile-verified work in progress, **not an operational runtime**. These files replace
the missing std platform selections in a repository-local copy of Rust 1.98.0.
They intentionally require unresolved `infinity_std_*` native symbols until a
real runtime provider exists. No host fallback or simulated service is provided.

Run only via the build kit:

```
./build-kit run python3 tools/servo-platform-probe/prepare-std.py
./build-kit run python3 tools/servo-platform-probe/run.py --native-overlay
```

The staging step copies Rust library sources into `build/servo-rust-src/library`;
it does not modify the host sysroot. The pinned toolchain check prevents silently
applying selectors to a different Rust layout. The overlay is selected only by
`infinity_native` on `target_os = "none"`. It does not affect production builds.

## Required native ABI contracts

- Allocation returns disjoint, correctly aligned writable private memory or
  null. Deallocation accepts the original pointer/size/alignment; operations must
  be thread safe. Rust's default realloc/zeroing logic preserves those semantics.
- Last-error state is per execution thread, not a process-global variable.
  Numeric codes are an adapter protocol, not ambient POSIX authority.
- Entropy returns zero only after filling the entire requested range using the
  native entropy service. Failure must not seed hashes from predictable bytes.
- TLS key creation returns a nonzero process-wide key or a nonzero error.
  Get/set resolve values on the calling native thread. Invalid set fails.
  Destroy releases the key without invoking destructors. Thread exit must clear
  values before invoking registered destructors and support bounded destructor
  iterations. Key reuse must not expose values left by a previous key generation.

The native symbol implementations, general threads/waits/clocks and guest tests
are still required. Compilation alone is not a runtime proof. The compiler's
serial C shim is not an implementation of this contract.
