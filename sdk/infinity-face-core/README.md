# Native facial setup policy

This dependency-free `no_std` crate implements the bounded policy **portion** of
enrollment and fresh-camera verification. It is not an inference engine and is
not wired into the installed configuration wizard yet.

Implemented:

- Named temporary enrollment; front/left/right observations.
- Single-face and minimum-size checks from a trusted detector's results.
- Exposure, clipping, contrast and Laplacian sharpness on aligned 112×112 crops.
- Finite 128-dimensional embedding validation and positive cosine comparison.
- Same-person consistency across all accepted views.
- Separate account/worker/model-bound capture lease for verification.
- Three spaced verification samples; bounded mismatch attempts and timeout.
- Volatile cleanup on cancellation, timeout, failure and drop.
- No account mutation, login token, template serialization or unlock API.

Thresholds must come from an evaluated model configuration. Numerical comparison
is not proof of identity or live presence. Pose/face-count/embedding inputs are
trusted-worker outputs, not values accepted from arbitrary user processes.
Frame sequence and timestamp checks catch transport reuse, not presentation
attacks. A new lease does not by itself establish that a face is alive.

The tests use synthetic pixels and vectors strictly for numerical/state behavior;
they do not test facial recognition or real camera capture.

Run through build authority:

```
./build-kit run cargo test --offline --manifest-path sdk/infinity-face-core/Cargo.toml --target-dir build/face-core
```

## Remaining end-to-end implementation

1. Complete the selected model/runtime transitive license audit and adopt a
   pinned native inference implementation. No model is currently linked here.
2. Decode actual camera MJPEG, detect/align faces and evaluate embeddings in an
   isolated worker. A model-specific memory budget is needed: the ordinary app
   loader currently caps an image at 256 KiB.
3. Add setup-authorized camera dispatch and trusted worker result delivery.
   Current camera grants require an active authenticated desktop session; the
   pre-session configuration wizard cannot simply call that path unchanged.
4. Connect the approved kit layout in
   `assets/ui-design-kit/default/face-enrollment-v1.md` to the real worker.
5. Implement protected named enrollment persistence separately. Temporary
   verification does not require pre-login key release, but saving reusable
   biometrics does require an explicit protected-key design.
6. Verify on installed ARM64, including camera loss, cancel and retakes. The
   x86_64 camera streaming backend remains unavailable.

Neither a verified workflow state nor a successful build may enable facial
login. Password authentication remains unchanged.

Validation status (October 23, 2026): the focused test command was attempted but
did not start because full build `20261024T024605482253Z-10205` held the ISO build
lock. The process was confirmed active and was not interrupted. This crate has
not yet been compiled or tested, and is not included in the installed payload.
