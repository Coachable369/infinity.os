# Native Layout Stack Guard

The native engine uses debug code generation. Google's results page exhausted
Stylo's upstream 512 KiB worker stack: the captured saved SP was below its lease
and inside the adjacent worker's stack. The shared build now sets Stylo's
supported `SERVO_STYLE_THREAD_STACK_SIZE_KB` option to 4096 for both architectures.
Parallel layout remains enabled.

The shared native layout overlay checks the actual executing worker's stack lease
through `infinity_std_stack_bounds`, and samples SP using a small ARM64/x86_64
adapter. It neither allocates nor yields. Bounds are not cached across Rayon jobs.
Unavailable, malformed, out-of-range, and insufficient leases fail closed.

The reserve is 512 KiB. The captured ARM `google-stack.elf` has these entry frames:
`BlockContainer::construct` 896 bytes, `handle_inline_level_element` 1136 bytes,
`IntermediateBlockContainer::finish` 256 bytes, and
`traverse_children_of<BlockContainerBuilder>` 400 bytes. The reserve is a
conservative margin, not a proven upper bound for all intervening engine calls.
New ARM/x86 codegen still needs frame and guard-order inspection.

Checks run at three recursive call edges, before the next callee's frame:

- `display: contents` descent into child traversal.
- Inline box descent into non-replaced contents traversal.
- Deferred block construction into `BlockContainer::construct`.

The guarded callees have native-only `inline(never)` annotations so optimization
cannot fold their frames above those checks. The failure is a static-payload
panic: production's existing panic hook terminates in the browser quarantine
callback; the guest probe hook reports failure and terminates. There is no child
skipping, partial-tree continuation, or production unwinding. The check itself
does not format a diagnostic or allocate; panic-hook internals remain those of
the existing runtime.

This does not protect every recursion path, layout phase, unusually large frame,
foreign call, or arbitrary memory write. It is not a replacement for inaccessible
guard pages and stack probing. Allocation safepoints alone are not sufficient.
The supported Stylo 4 MiB stack configuration is a separate capacity change.

## Behavioral Verification

Run through build-kit only:

```sh
./build-kit run rustc --edition=2021 --test tools/servo-platform-probe/native-layout-stack.rs -o build/native-layout-stack-test
./build-kit run build/native-layout-stack-test
./build-kit run python3 tools/servo-platform-probe/run-memory-guest.py --std-probe --arch aarch64
./build-kit run python3 tools/servo-platform-probe/run-memory-guest.py --std-probe --arch x86_64
```

Host tests exercise typed boundaries, invalid/revoked leases, per-thread lease
changes, the real SP adapter, and rejection before a callee runs. The existing
native std harness now checks real 64 KiB and 4 MiB executor stacks without
deliberately overflowing them. Native rejection tests use the typed check;
production gate rejection is tested with a host-only caught panic. These tests
do not establish complete layout-path coverage or installed-VM acceptance.
The four host tests and both native architecture probes passed for this change.

`prepare-engine.py` stages the guard during the next normal native engine build;
do not run preparation against an engine currently being compiled.
