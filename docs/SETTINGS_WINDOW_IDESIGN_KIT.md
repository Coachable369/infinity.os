# Settings Window IDesign Kit

## Measurable target

- Preserve the screenshot's centered navy window, compact title chrome, left navigation rail, and generous content column.
- Use frosted translucent navy for window, header, navigation, dock, and widget fills. The default accent is clean Infinity Blue (`#4DA3FF`), mixed toward white for borders and focus lines; never wash a complete panel with the accent color.
- Replace detached configuration controls with one accordion: each 46-unit summary row owns a 12-unit disclosure twiddle and, when open, a rounded detail well directly beneath it.
- Rotate the twiddle from right-facing to down-facing. Use the active accent only for the open row edge, focus ring, selected option, and scrollbar thumb.
- Keep a 12-unit gap between rows. Only one row may be open, so hierarchy remains calm and scannable.

## Overflow and resize behavior

- The content viewport begins below the section heading and ends 18 units above the window edge.
- Show a 6-unit scrollbar track only when the accordion is taller than the viewport. The thumb reflects the visible fraction and supports track paging; pointer-wheel input scrolls while the pointer is over the viewport.
- Restored Settings, Home, Text Editor, and Command windows expose four 18-unit corner resize targets. A three-line diagonal grip marks the lower-right corner without competing with window controls.
- Resizing is live, bounded to the desktop work area, and constrained by each window's usable minimum content size. Maximized windows do not expose resize handles.

## States

- Summary: quiet deep-navy surface, label left, current value and twiddle right.
- Hover/focus: brighter outline and twiddle.
- Expanded: down twiddle, accent edge, contained detail surface.
- Scrolling: subdued track, accent thumb, no content may paint outside the viewport.
- Resizing: geometry follows the pointer continuously and all content reflows from shared layout calculations.
