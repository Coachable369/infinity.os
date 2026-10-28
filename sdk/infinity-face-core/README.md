# Native facial setup policy

This dependency-free `no_std` crate contains bounded enrollment policy plus a
fixed-model inference implementation. The current source implementation is
uncompiled and has no installed-camera acceptance evidence yet.

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
- Fixed float32 YuNet/SFace operators, landmark alignment and real RGB inference.
- Worker-only C JPEG decoding with bounded private scratch and no host runtime.
- Isolated worker commands, real thumbnail delivery, cancellation and memory wiping.
- Optional configuration-page source wiring, with separate password-authorized
  setup camera authority; successful comparison does not enable facial login.

Thresholds must come from an evaluated model configuration. Numerical comparison
is not proof of identity or live presence. Pose/face-count/embedding inputs are
trusted-worker outputs, not values accepted from arbitrary user processes.
Frame sequence and timestamp checks catch transport reuse, not presentation
attacks. A new lease does not by itself establish that a face is alive.

The fixed worker calls `admit_frame` immediately after camera delivery, before
JPEG decoding and inference. This checks capture age, binding, sequence and
spacing. `observe_admitted` consumes that capture-bound receipt after inference,
with a separate 60-second processing bound and the existing session deadline.
Slow computation does not turn a fresh capture into a stale one. Recognition,
quality, pose and all three verification comparisons are still required; a
receipt is not proof of identity and does not bypass replay checks.

The tests use synthetic pixels and vectors strictly for numerical/state behavior;
they do not test facial recognition or real camera capture.

Run through build authority:

```
./build-kit run cargo test --offline --manifest-path sdk/infinity-face-core/Cargo.toml --target-dir build/face-core
```

## Remaining end-to-end implementation

1. Compile the complete first implementation pass, then correct surfaced errors.
2. Validate numerical behavior on actual model data and native camera captures;
   synthetic policy tests are not recognition evidence.
3. Inspect the production setup page against the existing kit, including error,
   keyboard focus, cancellation and captured-thumbnail states.
4. Implement protected named enrollment persistence separately. Temporary
   verification does not require pre-login key release, but saving reusable
   biometrics does require an explicit protected-key design.
5. Verify on installed ARM64, including camera loss, cancel and retakes. The
   x86_64 camera streaming backend remains unavailable.

Neither a verified workflow state nor a successful build may enable facial
login. Password authentication remains unchanged.

Validation status: compilation and test execution are deferred at the user's
request until the implementation pass is finished. The new build recipes and
live/installed kernel dependencies are source changes, not evidence of updated
ISOs, installed operation, recognition accuracy, or security qualification.
