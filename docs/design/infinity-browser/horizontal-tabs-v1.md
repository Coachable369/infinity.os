# Horizontal adornment tabs

Reference: the user's blue/violet window-edge AI adornment, rotated into a
horizontal browser tab. Generated reference: `idesign-kit-horizontal-tabs-v1.png`.

## Native recipe

### Generated artwork revision — 28 September

The renderer now consumes the reusable kit's `browser-horizontal-tab-v1.rgba`
rather than approximating the reference with analytic straight slopes. The
unmodified generated PNG master and derivation recipe live in
`assets/ui-design-kit/default/`. This uses the actual current AI adornment as
the material reference: rounded shoulders, smoked glass, reflected upper bevel,
and cyan-to-violet rim. A second synthetic sparkle is no longer painted.
Existing dimensions, tab commands, title layout and close targets are retained.
Inactive/hover/active states share the exact silhouette and vary brightness;
the image contains no static page title or controls.

Production-pixel state sheet: `generated-tab-states-v1.png` (not a generated
mockup). Top row: inactive, hover, active, active-hover; bottom: crowded and
full-width tabs at 2×. Review confirms curved shoulders, quiet title space,
preserved glass highlights and a bounded glow. Core run
`20260928T045625154485Z-61668.json` passes all 31 tests, including tab hit targets
at 1–4× and 1–8 tabs. Pixel proof run: `20260928T045702489098Z-61718.json`.

ARM build and exact design-kit ESP/kernel-byte parity pass in
`20260928T045810185241Z-61776.json`. Fresh installation from that unmodified test
ISO, detached-media boot, live HTTPS/CSS/image/JavaScript, scrolling, tab
create/select/close, per-tab scroll retention and address entry pass in
`20260928T050751358884Z-68241.json`. Installed kernel bytes: 877,951,144;
SHA-256 `222d4a30031872566a722e236f04a358c15bfef4d8affe851bab6bef6e12c291`.
Reviewed actual desktop capture: `installed-generated-tabs-v1.png`. Both active
and inactive tabs have curved shoulders, aligned native titles and independent
close controls; the halo does not overlap navigation. This is ARM acceptance,
not new x86 runtime verification.

Model-inclusive ARM ISO rebuilt successfully by
`20260928T051428992693Z-68428.json`: `builds/InfinityOS-aarch64.iso`, including
the same 877,951,144-byte installed kernel and packaged kit. This was a focused
incremental build, not a clean-release run.

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

The earlier procedural painter used four-sample edge coverage and an analytic
bounded glow. The generated-artwork revision above replaces that painter with
alpha-weighted sprite sampling and smooth symmetric shoulder hit geometry.
Hover invalidates chrome only; unchanged hover targets do not
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
