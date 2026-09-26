# Native runtime primitives — not yet connected to a scheduler

Allocation-free TLS key bookkeeping for an eventual native Servo runtime. This
crate does not create threads or implement the std ABI. The scheduler must own
one `Values` instance per execution thread and one synchronized `Keys` registry
per process. Never share `Values` between execution threads.

On exit, use `Teardown::next` under the registry lock, then release that lock
before invoking the returned destructor. Repeat until `None`. Values are cleared
before callbacks, callbacks may repopulate TLS, and at most four complete passes
run. After the last pass every remaining value is discarded. Do not reuse a
completed teardown object for a different thread. Pointer validity and destructor
code lifetime remain the runtime owner's responsibility.

Verification:
`./build-kit run cargo test --manifest-path sdk/servo-runtime-primitives/Cargo.toml`

Three behavioral host tests pass. No installed-system execution is claimed.
