# Disk explorer — ANSI usage wheel

Extends `command-terminal.md`; reuse its monospace raster fonts, sixteen ANSI
colors, charcoal surface and compositor-owned chrome. No bitmap icon or new
window recipe is needed for this data-driven text-cell visualization.

- Keep the physical GPT `#`/`=` partition strip at the top on every level.
- One compact title, one partition strip, one measured pool-usage line.
- Below: paths on the left, right-aligned human-readable sizes and percentages;
  a circular ANSI donut on the right. Matching colors associate rows and sectors.
- Correct for the actual glyph advance/line-height ratio so the disk is circular.
  Use `#` for selected sectors, `:` for others and `.` for unknown/unlisted space.
- At disk level the circle represents partition capacity, including real gaps.
  Inside a partition it represents visible logical file sizes, not allocation or
  free space. Never imply that inaccessible namespaces are empty or free.
- Enter opens partitions/directories; Left returns to parent; selected paths remain
  visible in the footer. Scrolling the list does not alter chart proportions.
- Narrow terminals keep navigation and sizes; omit the chart if it would crowd
  paths. Tiny viewports must not overlap headers, rows or footer.
- Aggregate only accessible paths. Directory totals include descendants; no
  hidden user's private data is exposed. Read-only FAT volumes remain read-only.
- Empty, unsupported and bounded/truncated scans show an explicit status.

Acceptance: typed directory aggregation/navigation and sector boundary tests;
production terminal renderer captures at 86x15 and larger/smaller grids with
pixel-bound checks and no intersecting text cells. Live and installed kernels
embed the same renderer.
