# Media, monitoring and resize interaction

Use `accounts-media-reference.png` for Media Player hierarchy, Eclipse Precision
for shared outer chrome, and the existing image-backed primary/secondary buttons.
Transport is reserved at the bottom before sizing the artwork viewport: 24px
side gutters, 8px rhythm, 480x500 minimum, 640x600 initial logical window.
Artwork fits a 288x240 box; video fits the remaining viewport. Status and errors
remain visible above the seek rail; Play/Pause remains keyboard accessible.

The existing cursor kit had no grab hand. `grab-cursor-v1.png` is its generated
master; `tools/build-grab-cursor.py` produces the 56px straight-alpha sprite.
The palm center at (28,32) is the resize hotspot, aligned to the pointer coordinate.
Logical footprint is 28px, scaled with accessibility cursor preferences. The same
hotspot must be used for paint, damage restoration and capture. Corner targets
retain the shared 30px square including 6px outside halo. Edge targets remain
native, with the hand replacing diagonal arrow cursors at all four corners.

Monitor cards and history use existing Eclipse card surfaces and typography.
Never derive CPU load from runnable task count or RAM utilization from app budgets.

Accounts reuse the exact portrait from `accounts-media-reference.png`, packaged
by `tools/build-account-avatar.py`. The two-pane workspace owns a joined 36px
segment strip, 64px identity rows, an 80px detail portrait and right-aligned
52x26 toggles. The selected authenticated account is visible on first entry.
Role selection uses the real Administrators membership transaction, including
last-administrator protections. The Settings navigation and compositor-owned
outer frame remain shared. Membership rows reflect actual groups, not fictional
accounts or unenforced permissions from the concept board.
