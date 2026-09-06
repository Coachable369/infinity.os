# Settings Window IDesign Kit

## Measurable target

- Preserve the screenshot's centered navy window, compact title chrome, left navigation rail, and generous content column.
- Use frosted translucent navy for window, header, navigation, dock, and widget fills. The default accent is clean Infinity Blue (`#4DA3FF`), mixed toward white for borders and focus lines; never wash a complete panel with the accent color.
- Expose Primary and Secondary as adjacent color controls. Primary starts at Frosted Navy (`#0D2238`) and tints window, header, navigation, dock, and widget glass; Secondary starts at Infinity Blue (`#4DA3FF`) and is reserved for borders, focus, selection, twiddles, and scrollbar details.
- Replace detached configuration controls with one accordion: each 46-unit summary row owns a 12-unit disclosure twiddle and, when open, a rounded detail well directly beneath it.
- Rotate the twiddle from right-facing to down-facing. Use the active accent only for the open row edge, focus ring, selected option, and scrollbar thumb.
- Keep a 12-unit gap between rows. Only one row may be open, so hierarchy remains calm and scannable.
- Place Opacity and Blur directly after the two color rows. Each opens an inline horizontal slider with a quiet frosted track, a full-strength blue/white thumb and outline, and a right-aligned numeric value.
- Opacity spans 40–100% and changes only semantic window/component background fills. Blur spans 0–8 logical pixels and softens only the framebuffer region beneath those fills; neither value dims outlines, controls, text, focus rings, resize grips, or scrollbar details.

## Overflow and resize behavior

- The content viewport begins below the section heading and ends 18 units above the window edge.
- Show a 6-unit scrollbar track only when the accordion is taller than the viewport. The thumb reflects the visible fraction and supports track paging; pointer-wheel input scrolls while the pointer is over the viewport.
- Restored Settings, Home, Text Editor, and Command windows expose four 18-unit corner resize targets. A three-line diagonal grip marks the lower-right corner without competing with window controls.
- Privacy & Security includes a user-scoped `No Activity Timeout` row. Its expanded well reuses the polished inline slider recipe: a full-width luminous track, high-contrast circular thumb, and live minute value in the summary. The range is 1–120 minutes, defaults to 5 minutes, and commits once when pointer capture ends.
- Resizing is live, bounded to the desktop work area, and constrained by each window's usable minimum content size. Maximized windows do not expose resize handles.
- The scrollbar thumb is a captured drag target: press anywhere on the thumb, drag proportionally through the track, and release without losing the current accordion state.
- Desktop applications remain simultaneously open as independent entities. Clicking an exposed inactive window raises it; moving, resizing, maximizing, minimizing, or closing it does not alter sibling window geometry.
- Text Editor chrome exposes New, Open, Save, and Delete as equal toolbar actions backed by the native Personal-space document object.

## States

- Summary: quiet deep-navy surface, label left, current value and twiddle right.
- Hover/focus: brighter outline and twiddle.
- Expanded: down twiddle, accent edge, contained detail surface.
- Primary and Secondary pickers: matching inline HSV wells with independent live previews and one durable commit on pointer release.
- Opacity and Blur sliders: captured thumb/track drag, live preview while held, keyboard stepping through the same typed values, and one durable machine-wide commit on release.
- Scrolling: subdued track, accent thumb, no content may paint outside the viewport.
- Resizing: geometry follows the pointer continuously and all content reflows from shared layout calculations.
