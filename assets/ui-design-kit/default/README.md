# Infinity UI Design Kit — Default

First reusable component: `ai-window-tab-v1.png`, a transparent AI-generated PNG master.
Use for the attached assistant on every application window. Navy smoked glass,
smooth shoulders, tight blue/violet light rim, one embedded four-point sparkle.

## Geometry and states

- Logical footprint: **28 × 104 px**, down from 48 × 104.
- Straight left attachment; mirror the same pixels for left-side windows.
- Preserve alpha; never add an opaque image background or a second sparkle.
- Hover gently increases brightness, without changing layout or hit geometry.
- Runtime sprite: 112 × 416 RGBA8 (186,368 bytes), four-times logical resolution.
- Renderer uses alpha-weighted area sampling, with no decode/allocation in paint.

`tools/build-assistant-artwork.py` derives the sprite from the unmodified master.
Run with `./build-kit run python3 tools/build-assistant-artwork.py`.
Both installed ESPs carry this kit under `EFI/InfinityOS/InfinityUI/DesignKit/Default`.
The shared kernel embeds the sprite, so live and installed rendering use identical pixels.

## Generation

Created with the built-in image-generation tool, transparent background enabled.
Prompt: One production UI sprite for InfinityOS: a slim vertical AI side tab
attached flush to the right edge of a window; orthographic, transparent; straight
left edge, smooth curved diagonal shoulders, rounded outer edge. Dark navy smoked
glass, subtle blue rim transitioning to violet, polished bevel, restrained tight
glow, no ice/crystalline texture. One centered luminous white-blue four-point
sparkle. No text, window, mockup, extra objects, or painted checkerboard.

## Verification scope

`tools/assistant-tab-proof.rs` exercises actual production pixels and mirror states.
`tools/design-kit-parity.py` compares installed payload bytes against this kit.
Rendered proof is host-side; a new ISO and installed desktop screenshot are needed
to confirm the appearance on the target machine.
