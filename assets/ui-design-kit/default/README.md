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

## Horizontal browser tab

Kit gap: the vertical assistant master cannot directly serve as a horizontal,
text-bearing browser tab. `browser-horizontal-tab-v1.png` is its generated sibling:
the same smoked navy glass, rounded shoulders and cyan-to-violet illuminated rim,
without a baked-in sparkle, title, favicon or close button.

- Body: existing variable-width browser tab, 224 × 36 logical px at maximum.
- Exterior glow: at most 6 logical px, clipped above the content attachment rail.
- Runtime derivative: 944 × 192 RGBA8, 4× authored footprint including padding.
- Active uses original pixels; inactive uses 100/256 brightness, hover 208/256,
  active-hover 272/256 (clamped). Alpha is preserved in every state.
- Runtime area sampling is alpha weighted; no paint-time decode, allocation or blur.
- Titles, document glyphs, 24px close targets and new-tab control stay native.

Generate the deterministic runtime derivative with
`./build-kit run python3 tools/build-browser-tab-artwork.py`.
`sdk/infinity-browser-core/tab_style.rs` is shared by both CPU targets.
The existing design-kit package list includes both the PNG and RGBA in installed
ESPs; the kernel embeds the exact RGBA bytes. The vertical AI sprite is unchanged.

Built-in image-generation prompt (reference: `ai-window-tab-v1.png`): Create one
production horizontal browser tab background, orthographic, transparent, straight
bottom attachment baseline, symmetric smoothly curved short diagonal shoulders
and long flat rounded top. Match the reference's midnight navy smoked glass,
subtle upper reflection, electric cyan-blue rim transitioning to violet, restrained
tight halo. Wide quiet center for live text. No text, symbols, sparkle, close/plus
icon, window, checkerboard, floor or other objects. Smooth polished bevels, not
angular crystalline metal. Wide landscape, tight transparent padding.

Production-pixel review: `tools/browser-tab-artwork-proof.rs` shows inactive,
hover, active and active-hover left to right, plus crowded/full-width 2× examples.

## Welcome hero and charcoal page

Kit gap: no matching welcome-page illustration existed. `browser-welcome-hero-v1.png`
is the built-in imagegen extraction of the approved browser mockup's blue/cyan
glass infinity loop and reflection, with all surrounding UI removed and alpha preserved.
`tools/build-browser-welcome-artwork.py` derives a bounded 960px-wide 32-bit BMP.
Both native kernels embed that same BMP; both installed ESPs carry master and derivative.
Use aspect-fit, never crop the loop or reflection. Charcoal page #202124, cards
#292B2F, borders #44474D, accent #22D3EE. Reuse authored Inter atlases and native
button states. `infinity-browser-core/welcome.rs` owns responsive spacing and hit targets.
The tab-stack and search glyphs extend the existing native browser stroke family.

## AI readiness veil

Kit gap: the assistant previously had only a small header status, which did not
prevent users from speaking while native models were still loading. The reusable
readiness veil occupies the assistant timeline and composer as one smoked-navy
glass surface, retains the model card above it, and uses a large eight-segment
cyan orbit with a two-line wait message. The orbit advances through the existing
desktop animation frame; no timer, bitmap, blur, or paint-time allocation is added.

- State: visible until the language model, microphone route, output route, and
  resident speech recognizer all report usable.
- Primary copy: `PREPARING CONVERSATION`.
- Secondary copy explains that local language and speech models are loading.
- Input and send controls are fully covered; voice capture remains closed while visible.
- Surface, border, typography, cyan focus, and 8px rhythm reuse the assistant kit.

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
