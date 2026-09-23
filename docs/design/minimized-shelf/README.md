# Minimized app shelf

The right edge now reserves a narrow vertical glass shelf for minimized native
windows, separate from the bottom launcher dock. Default overview/chat widgets
move left to leave that lane clear. The generated IDesign Kit guided the glass
edges, icon tiles, gutters, and left-opening menu. No generated control image is
used in the OS: icons come from the selected installed theme; chrome, labels,
hover hints, and menus are real native UI.

## Interaction

- Minimize adds the existing window; closing does not add it.
- Click restores that exact window, retaining its document/location/state.
- Right-click offers Restore, Maximize, and Close. Editor Close retains the
  existing unsaved-document confirmation flow.
- Separate File Navigator instances have numbered badges.
- Hover identifies the app without crowding the narrow rail with truncated labels.
- Overflow is reachable using the mouse wheel or the rail's up/down controls.
- Context menus support Up, Down, Enter, Escape, and outside-click dismissal.
- Minimized state is session-local, not a new cross-reboot application snapshot.
  Lock/unlock retains it; a new authenticated session clears singleton entries.

## Implementation

`ui/minimized_shelf.rs` holds bounded state, exact identities, shared hit geometry,
and dirty bounds. The console controller delegates to existing app lifecycles;
it does not launch replacement tasks when restoring. `FileNavigatorWorkspace::restore`
adds exact-slot restoration. The painter reads a UI-thread snapshot, not services.

The existing glass recipe moved unchanged to `bootstrap/glass.rs` so the production
renderer and pixel harness use one implementation. It remains internal. Menu and
shelf changes recompose their old/new bounds including shadow margins. Ordinary
unchanged movement produces no shelf scene damage. Shelf chrome is painted after
windows so window movement cannot erase it, and before launcher/spatial overlays.

No new installed artwork dependency is introduced. The shelf is compiled into
the same installed kernel carried in the ISO's System Generation, and uses its
existing icon/font resources. Installer-payload/kernel byte parity must pass.

## Verification evidence

- Typed shelf tests: identity lifecycle, invalid identity rejection, all menu
  hit rows, overflow limits, six resolutions, bounded damage and unchanged state.
- File Navigator workspace test: restoring one of two minimized windows preserves
  its task, selection and namespace while the other stays minimized.
- Production pixel harness: compares partial redraws with complete composition
  at 800x600, 1440x900 and 2560x1440, including tooltip/menu dismissal and overflow.
- `native-render.png` is output of the actual native shelf/glass/font/icon code
  in a host framebuffer fixture. It is **not** an installed-VM screenshot.
- Installed interactive VM QA remains pending. The user's running VM is not
  interrupted. An isolated QEMU attempt could not boot the VirtualBox-linked
  installed kernel because its fixed load address is outside QEMU's RAM map.

## Generated reference

Release verification (2026-09-23): `make aarch64 x86_64`,
`make input-regression-test`, `make file-navigator-workspace-test`,
`sh tools/infinity-ui-test.sh`, `sh tools/build-hermes.sh`,
`python3 tools/installed-kernel-parity-test.py`, and the four-image
`tools/ui-install-parity-test.sh` invocation completed successfully. The three
shelf state tests and all 11 native painter tests pass. Rustfmt checks pass for
new modules; existing unrelated dirty files were not reformatted or committed.

Published local artifacts:

- `builds/InfinityOS-aarch64.iso`: SHA256
  `ab12747b6b0132de16fec2418513306128357dd577a9481a0f65b463304d8781`
- `builds/InfinityOS-x86_64.iso`: SHA256
  `06cc5bf7c30178f9aa91160b03053637b82880a08f8dddd97eeaf8e7c53eb952`

These results establish build/artifact and host-render behavior, not installed
interactive VM acceptance. The running user's VM has not been upgraded.

### Reference provenance

Built-in image generation, saved as `idesign-kit.png`. Exact prompt:

> Use case: ui-mockup. Create an InfinityOS IDesign Kit reference for a native minimized-running-app shelf. High fidelity dark navy translucent blue glass, restrained cyan edges, mixed-material polished desktop icons (folder with papers, terminal screen, text document, settings gears). Main composition: a tall NARROW vertical glass rail at the far right of a starfield desktop, 92px wide and 480px tall at 1440x900 scale; a wide low bottom dock provides scale reference. Existing system overview/chat widgets sit just LEFT of the rail, never behind it. Four minimized app tiles stacked with generous vertical gutters, recognizable icons and subtle count/status indicators. A contextual menu opens LEFT of a selected tile, glass surface with crisp legible rows exactly: Restore, Maximize, Close. Include a small inset showing default, hover and selected tile states, and empty shelf state. Professional native OS finish, no ice crystals, no excessive neon, no arbitrary extra panels. This is design documentation only; implementation will render genuine interactive controls.
