# Compact workspace tabs

These SVGs are the reusable source of truth for normal, selected, hover and
keyboard-focus states. They replace the Python-generated approximation of the
soft-shoulder reference. The compact strip is 32 logical pixels high; only its
flat center stretches horizontally. Display density and explicit zoom scale it,
window resizing does not. Labels use Inter 18 px, centered, with 4 px vertical
padding. Never add the image height to the label's requested height.

`states.png` shows the exact compiled source pixels at 2x. Compile source changes
with `./build-kit run python3 tools/native-python/uikit-tabs.py` and commit the
resulting `sdk/python/infinity_uikit_tabs.py`. Native Python and the installed
SDK consume that same module. All applications use `workspace_tabs` rather than
recreating the silhouette. Existing window chrome, controls and typography
remain governed by `../application-workspace.md`.

The earlier full reference board illustrates hierarchy and states; this compact
geometry supersedes its 40 px strip measurement. Acceptance requires source-to-
native pixel comparison plus selected, hover, focus, normal and resize checks.
