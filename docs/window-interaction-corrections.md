# Window interaction corrections

## Changes

- Keep click dispatch on the topmost window selected by the shared stack; remove the second inactive-window search that could select a covered window.
- Do not dispatch desktop-chat controls underneath the active application rectangle.
- Include the File Navigator device row in shared painted-row hit testing, navigating it to the namespace root.
- Route attached assistant maximize/restore through saved window geometry and persist minimize state. Add explicitly approved `open folder /absolute/path` navigation for the owning File Navigator, validating the destination kind.
- Give Spatial Desktop tabs a 12-unit gutter and replace the World Shift hero's abrupt scrim boundary with a continuous gradient and shared glass action styling.

## Verification boundary

Navigator layout and editor/assistant executable harnesses pass. Window stack/layout harness passes. Spatial state suite: 22 tests pass; the existing tab-copy assertion is not behavioral evidence. Production painter suite: 19 tests pass, including partial-frame/full-frame pixel equivalence fixtures.

These checks do not establish that every intermittent artifact reported by the user is resolved. Installed VM single-click and visual acceptance remain pending. The attached assistant supports bounded explicit actions, not unrestricted natural-language app automation. No claim of general model-driven application control is made.
