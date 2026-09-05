# Renderer

The first Milestone 7C renderer is intentionally software based. It supports
XRGB8888 and ARGB8888 retained surfaces, integer alpha blending, clipping,
damage-only composition, and coherent front-buffer presentation. Large content
is referenced as surface memory rather than embedded in IOP or IEF messages.

`FrameClock` provides monotonic pacing and missed-deadline accounting.
`AdaptiveQualityController` uses bounded hysteresis: three pressure frames lower
optional visual quality; 120 stable frames restore one level. Safe mode removes
blur and shadows and reduces animation frequency, never windows, input, or
trusted UI.

## Status

- Software blend and damage presenter: **TESTED**.
- Pressure degradation and recovery policy: **TESTED**.
- SIMD, GPU, color management, and complex effects: **PLANNED**.
