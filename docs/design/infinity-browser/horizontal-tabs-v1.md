# Horizontal adornment tabs

Reference: the user's blue/violet window-edge AI adornment, rotated into a
horizontal browser tab. Generated reference: `idesign-kit-horizontal-tabs-v1.png`.

## Native recipe

- Raised trapezoid, narrow top and angled shoulders joining the bottom rail.
- Midnight glass interior; cyan left rim blending to violet on the right.
- Active rim is luminous, inactive rim subdued, hover brightens the rim.
- 36 logical-pixel height, 8-pixel inter-tab gutter, 16-pixel title inset.
- 24 × 24 close target, inset clear of the sloped shoulder; a quiet cross becomes
  a violet-backed control on hover. Title truncation never enters this target.
- Authored type sizes: 18px window title, 14px tab/URL text, 12px status copy;
  centered 16px Go label without the unrelated decorative arrow. The opaque
  midnight chrome tint keeps underlying desktop text from competing with tabs.
- Painter and hit tester share the bevel geometry. Four-sample edge coverage
  avoids nearest-neighbor scaling. No new raster icons or per-frame blur.
- Hover invalidates chrome only; unchanged hover targets do not change revision.

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

Installed ARM screenshot: `installed-horizontal-tabs-v1.png`, real HTTPS Example
Domain rendered by Servo. Review caught and corrected oversized inherited text
and the Go-arrow collision. Interaction batch passed create/select/close,
background scroll preservation, JS input, CSS/image, scroll and Ctrl+L replace.
Final typography render: `20260927T182700531051Z-64758.json`. Both architectures
compile; this pass does not establish new x86 installed acceptance or update
release ISOs. Existing x86 scroll/second-tab failures remain in the milestone log.
