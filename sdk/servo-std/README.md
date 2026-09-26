# Infinity Rust std port: initial adapter sources

Native port work in progress, **not an installed browser runtime**. These files replace
the missing std platform selections in a repository-local copy of Rust 1.98.0.
The opt-in `servo-runtime-primitives/native-abi` provider now implements the
thread/wait/TLS/allocation ABI and delegates clocks/entropy to native callbacks.
Real std thread and synchronization programs execute in disposable guests on
both architectures. Production worker/service integration is still required.
No host fallback or simulated service is provided.

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
- Thread creation either transfers ownership of the entry closure to a real
  independently scheduled thread, or fails without consuming it. Join waits for
  completion; detach releases the handle without stopping the thread. Thread exit
  runs TLS destructors outside registry locks. A serial callback is not a thread.
- Futex wait atomically compares and parks, preventing a wake between comparison
  and registration from being lost. Timeouts use monotonic time. Wake reports the
  number actually unparked. Upstream std mutex/condvar/once/rwlock algorithms use
  this primitive; busy polling on the desktop thread is not an implementation.
- Monotonic time cannot move backward. UTC is independently supplied by the native
  wall clock. Both return normalized nanoseconds, and report unavailable clocks
  instead of inventing a timestamp.

The overlay opts the staged std crate out of its unsupported-target stability
gate only under `infinity_native`. This avoids injecting std-only feature names
into third-party no_std crates. It does not implement any missing service and is
not a declaration of platform support.

## Executable provider and limits

`sdk/servo-runtime-primitives/executor.rs` owns independent native stacks,
round-robin cooperative dispatch, sleep/wait, join/detach, stale-handle rejection,
and bounded TLS destruction. `native.rs` binds the std ABI to that executor and
an explicitly granted reclaiming arena. It permits 16 live handles, reclaims
joined/detached stacks only after execution leaves them, and rejects calls from
a CPU other than its bound owner. It reports **one** CPU of parallelism.

The provider is cooperative, not preemptive. It must run on a separately granted
worker, never on the desktop event path. Kernel worker reservation, event pumping,
capability context inheritance and cross-CPU message ingress remain integration
work. A non-yielding script can still monopolize this worker. Guard pages and
installed-system validation remain absent. Do not enable it as production Servo
support based only on the fixture.

The guest tests use actual architectural clocks (ARM counter or x86 HPET).
The fixture explicitly denies unavailable UTC and entropy; it does not fabricate
them. A production installation must supply those services. Root error state is
separate from each thread; upcoming IO adapters still need to set error values.

`mio-poll.rs` bridges native service readiness into pinned Mio's Poll/Waker/Source
API, sharing the existing native selector implementation. Control notifications,
read/write edges, rearming, deregistration and bounded exhaustion are tested.
This is not a TCP/UDP socket backend, and does not grant network authority.

Execute the staged std and Mio API tests via:
`./build-kit run python3 tools/servo-platform-probe/run-memory-guest.py --mio-probe
--arch aarch64` (or `x86_64`). The compiler's serial C shim is not used as a
thread implementation. These are freestanding fixtures, not installed OS proof.
