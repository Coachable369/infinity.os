# Futuristic glass button extension

Reference: futuristic-glass-v1-reference.png, AI generated with built-in imagegen from the user's three supplied glass references. Prompt: four distinct families, normal/hover/pressed columns, sapphire backdrop, localized optical glints, capsule reflection, faceted sapphire chamfers, restrained violet prism refraction; stable geometry in all states.

Gap: the existing collection lacks localized caustic clear glass, optical capsule, faceted glass and prism edge treatments. Add these as reusable UIKit materials 25–28, preserving existing material IDs including frames 15–24.

All states keep the same bounds and caption placement. Hover increases reflected light; pressed darkens the interior and shifts light inward. Use the shared native/Studio renderer, not separate dropdown illustrations. Dropdown choices show three labeled live-rendered samples. Corners: luminous 10px, capsule 32px, faceted 24px chamfer, prism 10px at 1x. The runtime clips each silhouette with antialiasing. Artwork is 256×64 RGBA per state. Existing per-element overrides remain authoritative; new options do not replace saved global styles.

Acceptance: decode and persist all new material IDs, distinct state pixels with identical coverage, identical Swift/Rust sampling, visually inspect the dropdown and production-rendered state board. Reference is design direction; do not claim pixel equality to the AI-generated board.
