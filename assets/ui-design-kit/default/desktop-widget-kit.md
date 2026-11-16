# Desktop widget design kit

[AI-generated reference board](desktop-widget-reference.png)

The board specifies the three stock widgets at rest. Use a six-dot drag handle, inset heading, midnight glass surface, fine cool edge and consistent corner radius. All three panels use the same Studio-defined column width, a shared downward shadow and a 12-unit stacking gap. The smoke on Overview, AI and Settings uses a neutral greyscale lightmap. Weather retains its blue artwork. Preserve native text, focus, hit targets and right-click configuration.

- **System Overview** reserves its first content row for two equal circular gauges. Use a cyan CPU sweep, lavender memory sweep, rounded ends and centered percentages. Place captions below the rings. Separate the five detail rows with fine dividers and align values along their trailing edge. Use the same live metric sampler and 320 ms keyframe easing for both gauges. Reduced motion displays each sampled value immediately.
- **Weather** places the location and condition on the left, weather artwork on the right, and the drag handle in its header. First use opens location entry. Reference fixture uses Chicago, clear and 12 °C.
- **Infinity AI** layers a smoke lightmap behind its microphone, model, conversation and compose controls. Text remains legible at rest and during audio activity. Preserve all existing keyboard, pointer and voice actions.

For acceptance, capture fixed-data native renders of each state at the same size, scale, backdrop and animation frame as its reference. Compare all pixels before approving. Verify drag capture, configuration, focus and persistence separately.

Use `python3 tools/widget-design-compare.py REFERENCE.png CAPTURE.png --diff DIFF.png` for the exact comparison. Supply independent, widget-sized fixtures. A difference in dimensions or in any RGBA channel fails the check.
