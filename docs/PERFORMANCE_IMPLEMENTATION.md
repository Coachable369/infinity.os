# Performance milestone acceptance ledger

The user-supplied September 2026 rendering and Performance Monitor specification is authoritative.

Implementation order and acceptance checklist:

1. Capture measured baselines and regression fixtures for retained composition, cursor damage, dragging, invalid commits, and opaque/translucent workloads.
2. Add bounded versioned telemetry with monotonic duration measurements, availability flags, fallback reasons, and rolling aggregates.
3. Instrument the actual framebuffer presenter and input dispatch, alongside the retained compositor. Distinguish these backends in diagnostics.
4. Fix measured rendering bottlenecks and pixel correctness failures; verify baseline comparisons.
5. Expose the same capability-authorized typed operations to a native Performance Monitor, Console commands, and Settings; add bounded native charts and sampling.
6. Exercise effects policy, protected scheduling, queue pressure, adversarial cases, and recovery.
7. Package all runtime components and assets in the installed System Generation; verify a clean install with media detached.

Visual recipe: existing InfinityUI glass window chrome, selected theme icons, eight sidebar sections, aligned numeric columns, cyan frame-budget line, explicit unavailable values, bounded history graphs, and the existing close/minimize/maximize controls. Reuse existing themed assets where suitable.

Status is tracked by evidence, not by the existence of declarations. Full milestone acceptance requires live VM measurements and clean-install proof in addition to host behavior tests.
