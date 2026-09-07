# Native window controls

Scope: File Navigator, Settings, Command Window, Text Editor, and Task Manager.

- Shared glass button painter supplies centered minimize, maximize/restore, and close glyphs. Existing hit geometry is preserved.
- Task Manager process-row picking excludes title controls and summary space. Clicking a control no longer selects process zero.
- Native-app minimize preserves content and geometry instead of running close/reset logic. Launcher entries reopen the hidden apps.
- Navigator minimize retains its workspace/task; the Files dock entry restores the most recently minimized navigator. Close still removes its workspace slot.
- Loading a registry-selected navigator no longer overwrites it with the previous window's state.

Behavioral verification:

- `make active-painter-test`: actual kernel control painter, all three glyphs, restore variant, scales 1–3, opaque visible glyph pixels in retained RGBA surfaces. Produces `build/window-controls-proof.ppm`.
- `tools/infinity-ui-test.rs`: normal/maximized native control targets, editor/non-editor dispatch, 1024×768 through 3840×2160; Task Manager row picker rejects every control.
- `tools/file-navigator-workspace-test.rs`: minimize retains count, task handle and selection; restore recovers the same slot; existing close-isolation coverage remains.

Live guest QA is pending: at inspection, `infinityos-4` was powered off and both VirtioSCSI slots were empty. No disk was attached or modified. An installed-disk choice is required before testing controls in that VM.

ARM64 and x86-64 live/installed kernel builds and ISO packaging pass. `tools/installed-kernel-parity-test.py` confirms byte-identical installed kernels in each installer payload. The native control render was captured as `builds/InfinityOS-window-controls-proof.png` and visually reviewed; this is a painter fixture, not a guest screenshot.
