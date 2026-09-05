# InfinityOS Installation Progress IDesign Kit

The installation-progress screen uses the same generated background, masthead,
and activation artwork as the preceding guided-installation screens. The
supplied 2006 x 784 reference defines only the lower progress-console treatment;
it is not used as a replacement full-screen background.

The centered setup console retains the same outer card, title rail, masthead,
and placement used by the immediately preceding installer step. Only its inner
content changes for installation progress.

## Live layers

- The status label and percentage are framebuffer text driven by verified installer progress.
- The progress track, fill, quarter markers, and active flares are rendered for every reported percentage.
- A moving star glint follows the original infinity path over the illuminated
  activation emblem independently of progress.
- The progress console is native framebuffer geometry and contains no baked
  progress values.

## Reference geometry

- Progress console: approximately 11–89% horizontal and 67–86% vertical.
- Status baseline: approximately 71% vertical.
- Progress track: approximately 14–86% horizontal and 77% vertical.
- Palette: near-black navy glass, restrained cyan outlines, ice-blue fill, and white-blue highlights.

The panel is installer-only native composition. No progress-specific
full-screen plate is embedded in either the live ISO or installed desktop.
