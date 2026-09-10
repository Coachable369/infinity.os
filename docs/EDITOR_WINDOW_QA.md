# Editor, Network icon and window activation verification

## Implemented

- Save As has independent filename and Pool location inputs, folder browsing,
  parent navigation and paging. Duplicate destinations are rejected without
  overwriting an existing object. Save retains the chosen object reference.
- Open browses real namespace entries and loads persisted text objects. Objects
  larger than the existing editor capacity are rejected rather than truncated.
- Settings Network uses the installed icon family's network artwork.
- Native app painting and click activation share a bounded stacking order.
  Activating an exposed window preserves other window bounds and editor state.
  Settings remains visible behind another active app.

## TESTED

- `make editor-window-test`: native ObjectStore create, edit, independent paths,
  duplicate rejection, remount and byte-for-byte recovery; picker navigation and
  validation; stable stacking, topmost hit testing and hidden-window exclusion.
- Compiled and executed `tools/infinity-ui-test.rs`: native UI behavioral gates,
  including shared picker render/hit geometry.
- `make active-painter-test video-driver-test`: renderer/video behavioral gates.
- `make aarch64` and `make x86_64`: successful final builds.
- `tools/installed-kernel-parity-test.py`: both architectures' installed payloads
  and bootloaders match their live installer artifacts byte for byte.
- Isolated ARM64 VirtualBox installed-disk boot with installer media detached.
- Installed ARM64 QEMU clone, using the same installed kernel archive relinked
  at the repository's QEMU load address: desktop mouse/keyboard interaction and
  screenshot review at 1024x768. Save As created `/personal/projects/qa.txt`;
  New cleared the editor; Open navigated to that location and restored content.
- Overlapping Settings/Text Editor were raised in both directions by clicking
  exposed window regions. Typing followed the active editor. Settings retained
  its own machine-name value rather than displaying the editor buffer.
- Final cold boot without installer media: Open found the saved object and
  restored its saved content; the deliberately unsaved edit was not persisted.

These are behavioral/artifact checks, not source or diagnostic-text oracles.
VM screenshots and private test disks are retained under
`/tmp/infinity-editor-qa.f4HKl2/` for this run.

## Packaging and boundaries

Versioned media preserves the user's currently mounted default ISO:

- `builds/InfinityOS-aarch64-editor-window-fixes.iso`
  SHA-256 `7fb6db8552be4aeaaca682e24445a82048f9f5e2be3786df8c45beb9ef5828f1`
- `builds/InfinityOS-x86_64-editor-window-fixes.iso`
  SHA-256 `bca854a092f49abf1dd8497f40e68fb69060b490b74db98e9f77dc78c97160cf`

The user's running VirtualBox VM was not updated or reprovisioned. x86_64 runtime
interaction remains IMPLEMENTED BUT UNTESTED. The latest full UI sequence used
an updated installed clone, not another fresh VirtualBox installation. Binary
parity verifies that the final fixes are in fresh-install payloads.

Existing limitations remain: the editor is bounded to 2048 ASCII bytes; picker
locations are bounded to 95 bytes and listing state to 64 projected children;
File Navigator instances retain their existing internal ordering as one app
group. Arbitrary interleaving of multiple Navigator instances with other apps
and extremely small resized layouts were not verified. Pre-existing small-screen
clipping elsewhere in the desktop was outside this change.
