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

## Browser tabs

Use [Tron Lightrail tabs](browser-tabs-tron-v3.md) and its generated reference
board. The shared native recipe uses clipped navy surfaces, cyan active light
rails, Inter labels and separate close targets. The vertical AI adornment is
unchanged. Rendering stays inside tab bounds with no exterior glow or idle work.

`sdk/infinity-browser-core/tab_style.rs` owns the shared pixel and hit recipe.
`tools/browser-tab-artwork-proof.rs` renders production pixels and native labels
for all four states and crowded/full-width examples. Both installed design-kit
payloads include the reference board and specification through the common list.

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

- State: visible only while the selected language model is unavailable.
- Primary copy: `PREPARING CONVERSATION`.
- Secondary copy explains language model loading. Microphone preparation stays
  in the voice header; it must not cover an otherwise usable text conversation.
- Input and send controls are covered during preparation. Capture opens only after
  model readiness, then measures 500 ms of ambient PCM before admitting speech.
  Active or temporarily unavailable playback holds calibration so login sound
  cannot become the microphone noise floor. No calibration audio is retained.
- Surface, border, typography, cyan focus, and 8px rhythm reuse the assistant kit.

Readiness integration: model unavailability blocks composer keyboard/paste,
send, and timeline clicks; model selection, minimize, close, and voice cancellation
remain available. Both veil visibility edges invalidate the entire widget body;
after removal, subsequent animation damage returns to the header only.
Terminal failures must show their existing error state rather than a perpetual
loading veil. Reuse this element; no additional loading artwork is needed.

The controller now accepts a cold start as pending, opens capture only after all
dependencies are ready, and retries a transiently occupied composer at 250 ms
intervals. It retains the existing boot-time speech prewarm and removes the fixed
one-second desktop-entry delay. This removes a scheduled delay, not one second
of recognition compute. Whisper model/context, endpoint silence, decoding and
worker count are unchanged to preserve recognition quality.

Behavioral evidence: `voice-toggle-test` exercises cold model startup, delayed
recognizer readiness, model readiness loss during startup, veil-to-capture
transition, cancellation and terminal failures. ARM64 and x86_64 kernel checks
pass. Installed microphone latency, rendered veil review, and fresh-installed
runtime verification remain pending; no conversational speedup is claimed from
host tests. These shared kernel changes require rebuilt ISOs to reach installs.

Readiness repaint correction (2026-10-11): entering `Starting` explicitly
invalidates the widget. Its animation damages the full widget while the orbit
is in the body, then returns to header-only damage for normal listening.
Do not impose an artificial loading delay when boot prewarm already completed.
Rejected overlong microphone input uses the header state `PAUSE TO RETRY` until
the detector observes a real pause; this must not be presented as ready for a
new wake command. Changing that state invalidates the widget without waiting
for an unrelated pointer event. Typed chat remains usable during that pause.

## Clipboard and loading interactions

Reuse the native context-menu surface, compact 28px rows, Inter labels, cyan focus,
and the File Navigator location field for Paste To. No new panel recipe.
All magnifying glasses reuse generated theme role 27 (`27-search.png`),
including browser chrome; do not substitute procedural strokes.

The replacement busy cursor is `wait-orbit-v2.png`, generated with built-in
imagegen and genuine transparency. Prompt: one face-on circular satin silver/cyan
comet ribbon, tapered violet tail, luminous white tip, restrained glow, no text,
background, arrow, infinity symbol or ice shards. The previous infinity sheet
has been removed. `tools/build-wait-artwork.py` derives sixty equally spaced
64×64 RGBA frames using premultiplied-alpha filtering. Display at the pointer
hotspot in a 32px footprint (respecting larger user sizes), one revolution/sec,
time-based 60fps phase. No catch-up loop, decoding, rotation or allocation in
paint. Alpha-weighted sampling preserves clean small-size edges. The sprite is
983,040 bytes shared by both target kernels. `wait-orbit-v2-preview.png` shows
six production frames at their actual 32px size, not enlarged concept art.
Only cursor damage is presented, never a full-window repaint for animation.
Loading cancellation, completion, error, lock and window dismissal restore the
normal cursor. Master and packed sprite use the existing design-kit install list.

## Compact application menus

Application menu bars reuse one smoked-navy glass dropdown recipe: a soft cyan
outline, 6px interior padding, compact 28px command rows, native 14px Inter labels,
and a restrained cyan focus surface. Menus longer than the available viewport use
24px arrow-only scroll rows at the top and bottom. They scroll continuously by one
command with pointer, wheel, or keyboard input; never expose page-number, Next Page,
or Previous Page copy. `ui::app_menu` owns the shared density and viewport policy.

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
