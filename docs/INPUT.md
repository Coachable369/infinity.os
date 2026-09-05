# Window Input Routing

Device drivers normalize keyboard and pointer reports before InfinityUI policy.
`InputRouter` selects exactly one destination:

1. an active secure-input context;
2. the current pointer-capture window;
3. the highest eligible modal-aware hit-tested window; or
4. the focused window for keyboard input.

Pointer capture is owned and cannot be released by another execution context.
Focus ignores hidden or non-interactive windows. Modal windows trap hit testing
and focus. Window geometry and the pointer remain in one global coordinate
space; applications receive only routed local interactions in the future IOP
adapter.

## Status

- Secure input precedence, focus, capture ownership, and modal hit testing:
  **TESTED**.
- Current firmware/USB pointer normalization and desktop integration:
  **IMPLEMENTED, VM-TESTED by existing input regression suite**.
- Per-window local-coordinate IOP delivery: **SCAFFOLDED**.
