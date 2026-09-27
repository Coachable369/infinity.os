# Infinity Browser — Sapphire / Titanium v0.1

Status: generated visual target and shell recipes; **not an installed browser**.
The reference must not be shown as a fake page or treated as runtime proof.

## Authoritative assets

- `assets/design/infinity-browser-idesign-kit-v1.png`: browser hierarchy,
  chrome, fields, controls, states and spacing.
- `assets/apps/infinity-browser-icon-v1-source.png`: final app identity,
  1254 × 1254 RGBA. Blue globe, brushed **grey** infinity orbit and grey
  latitude/longitude lines. The board also uses this revised identity.
- `assets/apps/infinity-browser-navigation-v1-source.png`: 1774 × 887 RGBA,
  four columns by two rows. Back, Forward, Reload, Stop; Download,
  Site controls, Menu, Go. Silver-grey titanium with restrained sapphire insets.

Created with the built-in image generation tool. The source alpha is preserved;
no background color has been keyed out. Both icons were visually inspected.
No icon or screenshot is evidence of functional browser installation.

## Native skin contract

Use `sdk/infinity-browser-core/skin.rs` for opaque RGB recipes and interaction
states, and `layout.rs` for window-local rectangles and hit testing. Do not render
labels or controls by stretching the design-board bitmap. Use the existing
native Fira Sans font pipeline, matching the updated board's typography choice;
exact rendered typography parity remains to be reviewed.

- 16-pixel outer gutter, 8-pixel gaps, 44-pixel navigation hit targets.
- 40-pixel title strip; 32-pixel minimize/maximize/close targets, vertically centered.
- 60-pixel toolbar: back, forward, reload/stop, address, Go, downloads, menu.
- 24-pixel status strip. Content fills the remaining viewport.
- Minimum 760 × 240 logical pixels; integer scales 1–4.
- Neutral page content: never apply the chrome's glass tint to Servo pixels.
- Cached sapphire glass chrome; small hover/focus changes must not blur the
  whole desktop again. No animated glow while idle.
- Address focus uses a thin cyan ring and native caret. Never show verified
  connection status unless the active document's TLS verification succeeded.
- Back/forward disabled when history is unavailable. Reload becomes Stop only
  during cancellable navigation. Do not expose inert menu actions.
- A browser failure stays in the content/status area; window controls and
  address entry remain usable. Keep diagnostic engine details out of the UI.

## UX and installed acceptance still required

Enter submits a URL. Ctrl+L focuses/selects the address. Escape cancels loading
or dismisses a menu. Tab traverses visible controls before web content. Native
window actions must close/cancel, minimize, and resize the actual engine surface.
The download control reports durable object commits, not just received bytes.

Register `app.infinity.browser` and HTTP/HTTPS default handling only when the
real engine shell is wired. Package skin, icon, resources and runtime together
in the fresh-installed System Generation. Cold reboot with ISO detached, open
the app, follow a real HTTPS link, and capture a screenshot against this kit.
Review geometry, font quality, contrast, alpha edges and control states at 1×/2×.
These installation/runtime/screenshot gates are **not yet passed**.

## Current verification

Nine browser-core behavior tests pass, including control hit targets, resize,
atlas partitioning, state recipes and lifecycle bounds:
`builds/manifests/20260927T013651247052Z-99622.json`.
Byte-identical RGBA artwork in both installed ESP templates passes:
`builds/manifests/20260927T013704641318Z-99656.json`.
The regular build runs this artwork parity check after model packaging.
Neither result proves a running browser or pixel-perfect native rendering.

## Generation briefs

Design prompt: production-quality Infinity Browser IDesign Kit; familiar
Chrome-like navigation but original midnight/sapphire InfinityOS glass,
restrained cyan edges, crisp typography; one browsing context, title/window
controls, back/forward/reload, recessed URL field, Go/download/menu, neutral
Example Domain content, slim status footer; state samples, error card, menu,
typography and 8/16/24 spacing; no tabs, AI sidebar, extensions or Chrome branding.
Final board edit: replace the plain infinity identity and flat navigation marks
with the approved globe and titanium atlas; change the typography sample to
Fira Sans, remove the incidental slogan, and retain the original layout.

Final app-icon edit prompt: preserve the blue globe, continents, composition,
perspective and transparent silhouette; change only the infinity orbital ribbon
to neutral silver-grey brushed titanium, and all latitude/longitude lines to
thin grey; soft white highlights and graphite shading, no extra marks or tile.

Navigation prompt: eight isolated glyphs in a 4×2 transparent atlas ordered as
above; straight-on, equal cells, generous gutters, substantial readable strokes,
lightly beveled grey brushed titanium with machined texture and restrained blue
recessed edges; no labels, separators, containers, broad glow or background.
