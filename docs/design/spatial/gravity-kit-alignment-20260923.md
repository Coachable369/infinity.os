# Gravity Wall kit alignment

Target: `spatial-desktop-idesign-kit-v2.png`, left-hand application view (not
the component specimen sheet at right).

This pass replaces the two-row header with segmented single-row navigation,
centers the empty-state message in a glass card, replaces rectangular collection
buttons with circular cyan/violet/amber/ice controls, and removes detached help
text. Empty collection slots offer Ideas, Projects, Inspiration and Archive;
selection materializes that category without creating example documents.
Existing named categories and ideas remain intact. Pointer/drop geometry moves
with the controls. The spatial workspace keeps its luminous wallpaper;
the independent carousel retains its edge-to-edge blur and motion.

The cached transition clip includes the new header. Animated orb and carousel
timing are unchanged. Collection gradients render only during scene composition,
not on ordinary pointer movement.

Verification: production collection-control pixel tests and spatial metadata /
interaction tests. This is not yet a claim of exact whole-screen visual parity:
installed-VM review is still required. The running VM initially showed the old
installed kernel. Do not confuse an ISO rebuild with an installed-system update.
