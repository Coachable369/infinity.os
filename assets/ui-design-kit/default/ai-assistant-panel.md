# Attached AI panel recipe

Reuse the default kit's existing narrow AI tab/material, rounded app cards,
buttons, Inter metrics and cyan focus state. This recipe fills the missing
scrollable attached-conversation layout; it does not add an icon family.

- Keep the existing 28 × 104 logical-pixel tab and viewport-side fallback.
- Header: existing AI mark and title; 28 × 28 close hit target, 12px inset.
- Transcript: begins 72px below panel top; 12px gutters; fixed footer clearance.
- Cards: measured with the painter's actual word wrapping, 12px internal gutter,
  24px line pitch. Clip all text/cards to the transcript viewport.
- Scrollbar: 12px track; proportional thumb, minimum twice the track width;
  wheel, track click and grab-offset-preserving drag. Hide when content fits.
- Footer: existing Apply/Dismiss controls, 8px gap; 40px composer/send row.
- Focus: cyan field border, caret, I-beam. Hover does not change geometry.
- Closing the panel hides only the panel; `close window` invokes native app close.
- Launcher/spatial use the same component, not a separately styled assistant.

Behavioral geometry tests are in `tools/editor-assistant-test.rs`. Installed
render/capture comparison remains required before claiming visual acceptance.
