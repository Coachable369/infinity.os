# Eclipse Access — setup and sign-in

Reference: `eclipse-access-v1-reference.png` (built-in image generation, visual
specification only). Reuse the authored header BMP, existing native Inter faces,
the generated Eclipse glyph atlas and eight-pixel spacing. The approved
[Eclipse Precision surface contract](eclipse-precision-v1.md) owns panel,
button, field and outer-window artwork; it supersedes the earlier rounded
surface recipe without changing authored content or interaction bounds.
The reference's example checkbox is illustrative, not a new authentication feature.
The original user wordmark is retained byte-for-byte; the generated night-water
wallpaper is `eclipse-access-wallpaper-v1.png` and its native BMP derivative.

The kit lacked a shared pre-session palette. Both first-run configuration and
login now use midnight indigo #090F26, glass #101B3A, elevated #1A2E63,
electric blue #3E6FFF, icy border #86C7FF, white focus #FFFFFF, white text
#FFFFFF and neutral-grey secondary #BEBEBE. Error colors retain semantic red contrast.
The signed-in default skin uses this same white/blue/grey palette. Saved custom
primary/accent choices and alternate skins remain user-owned. The original
reference board's violet highlights are superseded by these white tokens;
reuse its existing geometry, artwork, typography and interaction states.
All surfaces and focus/selection states use the same palette; desktop theme
settings cannot silently recolor first-run or login controls.

Native application integration: the default editor's interior, selection, and
syntax palette use navy/blue/silver, not violet selection or teal accents.
Explicit alternate editor schemes remain selectable. Application menus reuse
the existing compact Inter rows with #101B3A panels, #1A2E63 selection and
#86C7FF edges. The minimized-app rail reuses the glass-panel recipe with a
silver #C2DBF5 edge. Command Window reuses the shared image-backed field/card and clipped
Inter label components; partial repaints must use the same #090F26 body color
as initial paint. These components consume the shared Precision atlas.

Typography is Inter regular (400) and semibold (600), not the host system face.
Native configuration and authentication bind FontKit family 1 explicitly for
glyphs, advances and kerning; Studio registers the same existing
`assets/fonts/Inter-Variable.ttf`. Authored sizes remain editable. Labels fit only
inside their own bounds and fields use the authored size for both text and caret.
Legacy installer, settings and signed-in desktop font selections are unchanged.

Keep the current forms, actions and hit rectangles. Login branding is top-right
below the identity bar. Configuration branding is center-right, an editable
template image layer. Its crop removes only transparent margins of the original
header; the full lettering and its glow remain visible. Existing configuration
illustrations sit below the wordmark without overlap.

Native surface edges use subpixel coverage. Reduced wordmark images use
premultiplied-alpha filtering; transparent RGB must not create dark fringes.
Rendering and hit geometry do not change on focus. Mask passwords, retain all
existing security/capability checks and preserve the success animation.

Primary buttons interpolate from #31AEFF at the top to #163EDC at the bottom;
the palette's #3E6FFF remains their semantic base. Other surfaces mix their base
with #86C7FF by 20/255 at the top, tapering to zero at the bottom. Focus uses an
inward five-pixel rim and #FFFFFF edge, eased out over 140ms at a bounded 16ms
paint cadence. Pressed primaries darken toward the elevated token. Reduced
motion settles immediately. Controls have no idle shimmer or geometry animation.
Input-panel entrance and selected-panel edge breathing are specified separately
in [Access panel motion](access-panel-motion.md); they do not alter control geometry.
The sign-in background alone reuses `infinity-auth-success-ripple-v1.bmp` for a
quiet 3.2-second expanding water ring beneath the stationary orb (maximum opacity
64/255). Its elapsed-time loop restores only the water rectangle, subtracting the
card and utility tray; it never touches the header or top-right wordmark. Reduced
motion disables this ambient effect. Successful-login choreography/audio remain
independent and unchanged; the old upper-screen infinity particle path is removed.

`eclipse-access-glyphs-v1.png` is the generated monochrome mask master, packed by
`tools/build-access-artwork.py` into sixteen 64px RGBA cells. The black/white
mask maps directly to alpha; shapes are not redrawn. Silver-white glyphs use
area-filtered coverage when reduced. Rows: wifi/home/up/right,
user/nodes/microphone/settings, shield/power/disk/info, search/eye/lock/check.
The preview sheet is a QA artifact; native controls consume the RGBA bytes.

Generation prompt: a high-fidelity InfinityOS Eclipse Access design board,
configuration with center-right wordmark, login with top-right wordmark, shared
indigo glass/silver/electric-blue/violet palette; button, field, choice, step,
menu, icon and typography states; restrained glow and antialiased edges. The
original header was supplied as the branding reference. Runtime uses the original
BMP and native controls, not a screenshot of this board.

Wallpaper prompt: expansive midnight-blue mountain lake under a crescent planet
and star field, dark left for forms, illuminated water and mountains at right,
matching the approved Eclipse Access board, no embedded text or controls.
Glyph prompt: clean monochrome 4x4 outline UI atlas in the documented order,
uniform thin rounded white strokes, black alpha-mask canvas, no texture or text.
All three masters used the built-in image generator; rejected transparency
variants are not packaged.
