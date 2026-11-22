# Drop Compass

Reference: Drop Compass cell of `desktop-refinement-reference-v1.png` and the
user's supplied crop. Reuse existing themed folder/document/trash icons and
Precision panel states. New reusable gap: circular translucent glass substrate.
`drop-compass-ring.png` is the original built-in imagegen output;
`drop-compass-ring.bmp` is the 480px RGBA runtime export, composited at 64/255
opacity so the desktop remains visible. Do not bake text or destination icons
into the ring.

Native layout: circular 440px substrate; four 244x76 cards in cardinal positions;
20px heading, 14px supporting text, 40px glyph, 65px label inset; center 92px
item and a compact sapphire caption. Cards use shared UIKit Panel normal/focus
states. Geometry reduces within small viewports without offscreen hit targets.

Hold bare desktop for 550ms with at most 8px movement to open. Quick clicks do
not open it. A stationary file drag offers actual local move destinations.
Release over a card or click it to act. Release in the center keeps the desktop
navigation compass open; releasing a dragged file outside cards cancels it.
Escape/outside click cancels without a file mutation. Arrow keys select cardinal
cards and Enter activates. No idle animation or audio changes.

Supported destinations: Projects, Documents, Downloads, Trash. Desktop mode
opens folders; file mode moves only owned paths within the signed-in home and
never overwrites an existing destination. Mail and Cloud in the illustration
remain unavailable integrations rather than misleading enabled controls.

Artwork prompt (built-in imagegen): Production front-facing circular translucent
sapphire glass disc on transparency, continuous soft pale sapphire rim, subtle
lower crescent, no text/icons/cards/backdrop/perspective/thick bevel/neon bloom.

Verification: `tools/drop-compass-test.rs` checks hold/release/slop and screen-edge
hit regions. `tools/desktop-drag-render-test.rs` captures the production painter,
checks bounded changes and dismissal. Installed kit and kernel payload parity
includes the ring. Build success alone does not prove installed interaction.
