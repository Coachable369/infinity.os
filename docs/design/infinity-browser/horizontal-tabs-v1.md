# Horizontal adornment tabs

Reference: the user's blue/violet window-edge AI adornment, rotated into a
horizontal browser tab. Generated reference: `idesign-kit-horizontal-tabs-v1.png`.

## Native recipe

### Corrected kit geometry (27 September review)

The supplied board is authoritative. The earlier separate application-title row
was incorrect. Chrome now uses a 48px shared window/tab row and 48px navigation
row; tabs remain 36px high, with 8px gutters, a 224px maximum body, and broader
24px shoulders. Text is 14px, with a 16px document glyph, 8px glyph/text gap,
and independent 24px close target. The add-tab control follows the last tab.
Compact crowded tabs hide text rather than
paint over close controls. The address field is 32px tall with 40px leading text
inset and vertically aligned text, selection and caret. Navigation uses quiet
unboxed strokes. Active/hover tabs receive a bounded 6px exterior blue-violet
halo; unchanged chrome remains retained. Both left traffic lights and right
window controls operate the window. The existing Go and download actions remain
available as quiet glyphs; no nonfunctional bookmark star is substituted.

Typography now uses Inter rendered into native 14/28/42/56px atlases rather than
resampling the installer's Roboto atlas. Matching advance/kerning tables drive
both paint and caret/selection geometry. These resources are compiled directly
into both native kernels, so installed-kernel byte parity covers their delivery.
Generation uses `tools/generate-installer-font-atlas.py`, the existing packaged
`Inter-Variable.ttf`, Regular variation, via the build kit.

Installed comparison: `installed-kit-chrome-v2.png`, real HTTPS Example Domain.
The separate title row, boxed navigation, incorrect font family, URL baseline,
vertical whitespace and layered frame were corrected. The sample webpage and
surrounding explanatory labels are not browser chrome. This is not a claim of
pixel identity: Go/download remain real controls instead of the board's
illustrative bookmark star; site favicon fetching is not added by this pass.

Painter and hit tester share the bevel geometry. Four-sample edge coverage
avoids nearest-neighbor scaling. The glow is analytic and bounded, not a
per-frame blur. Hover invalidates chrome only; unchanged hover targets do not
change revision. Close hover receives a violet backing and title truncation
never enters its target.

The generated board is a material/shape reference, not an implementation claim
for its illustrative page content or unused toolbar icons. Native layout remains
the existing Infinity Browser shell; functional controls are preserved.

## Generation provenance

Tool: built-in image generation, transparent background disabled. The supplied
image was a style reference. Prompt: Create a polished UI design specification
board for Infinity Browser native horizontal tabs. Rotate the protruding
blue-violet glass AI adornment silhouette to horizontal browser tabs attached
along the bottom edge. Midnight navy glass, short sloped shoulders, flat top,
soft edges, cyan-to-violet luminous rim, restrained specular light; not ice or
capsules. Show active/inactive/hover/close-hover states, readable titles,
24-pixel close targets and 8-pixel spacing. Preserve existing icons and minimal
sapphire/titanium browser visual language.

## Verification

Core behavior tests cover transparent shoulders, opaque centers, symmetry,
distinct interaction recipes, and close-target containment at 1–4× scale and
all 1–8 tab counts. Installed screenshots and receipt are recorded in the
daily-driver milestone document; generated imagery is not runtime evidence.

Earlier ARM screenshot: `installed-horizontal-tabs-v1.png`, real HTTPS Example
Domain rendered by Servo. Review caught and corrected oversized inherited text
and the Go-arrow collision. Interaction batch passed create/select/close,
background scroll preservation, JS input, CSS/image, scroll and Ctrl+L replace.
Final typography render: `20260927T182700531051Z-64758.json`. Both architectures
compile; this pass does not establish new x86 installed acceptance or update
release ISOs. The later PS/2 correction passes x86 scrolling and tab state
preservation in `20260927T185647729669Z-66589.json`; that predates this kit revision.

Revised-kit installed interaction batch `20260927T191738314582Z-68105.json`
passes all page, scroll, tab and address checks on ARM. Final paint-only changes
pass 27 core tests, both architecture links, and installed HTTPS rendering in
`20260927T192510242805Z-69676.json`. Final x86 installed UI and fresh release ISO
parity are not claimed. No release ISO was rebuilt for this visual correction.
