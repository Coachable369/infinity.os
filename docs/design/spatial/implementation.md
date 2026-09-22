# Spatial desktop implementation and verification

## Native workflows

Open with **Ctrl+Shift+K**, or top-bar **Search → Spatial desktop**.

- **Holographic Workspace:** independent retained surfaces for six File Navigator
  instances plus Command, Editor, Task Manager and Settings. Selection raises the
  exact instance. Wheel or +/- zooms real content with aspect-preserving bilinear
  filtering. Background state changes refresh the retained scene at a bounded
  rate; scanout waits until the overlay is recomposed. Invalidated surfaces retain
  their last committed pixels until refreshed, rather than showing blank previews.
- **Worldshift:** four named environments with durable layouts, navigator location
  and saved editor path. Editor buffers, cursor/selection/undo history and complete
  navigator workspaces are retained per world in-session. Switching has a
  cancellable 280 ms departure and a 180 ms retained-frame arrival crossfade.
  Input immediately ends arrival. Failed checkpoint writes roll back the switch.
- **Gravity Well:** explicit references, pointer placement, collection-drop preview,
  confirmation/cancellation and 180 ms settling. Originals never move or disappear
  because their reference changes collection.
- **Matter Shelf:** compact bottom ribbon, four slots per page, keyboard/wheel
  paging and pointer-following drag previews. Text drops into the active editor;
  Enter is the keyboard alternative. File drops into the active navigator propose
  a collision-safe copy to its folder. Text-file drops into the editor require
  confirmation and cannot replace an unsaved document. Confirmation rechecks
  source identity. Capacity/copy errors leave sources intact.
- **Constellations:** positioned nodes, symmetric user-authored links, explicit
  unlinking and opening. Removing a reference clears links, never the source.
  Moved/replaced paths are reported as stale instead of silently opening another file.

## Rendering and privacy

The generated `idesign-kit.png` supplies the visual recipe: midnight glass, cyan
edges, installed InfinityOS fonts, selected icon pack, native controls, 8-unit
spacing, 16-unit padding and 24-unit gutters. Its incidental Apple/Finder marks,
invented font name and traffic lights are not runtime assets.

Application painters write desktop-owned retained surfaces, not scanout. Ordinary
pointer motion remains cursor-only. Drag damage covers old/new preview bounds;
linked-node motion covers the bounded graph. Animations use elapsed time, skip
delayed frames and stop at their endpoint. Arrival reuses one destination frame
without rerunning application painters. Reduced motion settles immediately.

There are twelve bounded 2560×1600 retained slots and one 3840×2160 arrival buffer.
Their zero-initialized BSS does not inflate embedded image data or allocate per
frame. Additional static reservation is about 126 MiB over the earlier six-slot
cache; this is not a measured resident-memory or performance result.

Metadata is bounded to sixteen references, four worlds, 96-byte paths, 32-byte
labels and 192-byte printable-ASCII clippings. The 8 KiB checkpoint is authenticated
to its owner. Generic reads/copies/writes cannot expose it through aliases.
Replacement revalidates stored type, ownership and checksum.

## Defined limitations

- Unsaved buffers and complete navigator histories survive in-session switching,
  not reboot. Durable worlds restore the primary navigator location and saved
  editor file, not every navigator instance/history.
- Drop targets are native Editor and File Navigator, not arbitrary applications.
  Unsupported content and storage copy errors are reported; recursive folder
  copying is not added.
- No GPU renderer, distributed workspace or ambient clipboard capture is introduced.

## Behavioral verification

- Seven state tests: ownership, checksums, persistence, capacity, relationships,
  shelf paging, ten-window geometry/zoom and non-mutating drop proposals.
- Native retained-cache test: independent pixels, no painter call on translation,
  last-committed previews during invalidation, selective refresh, clipped damage.
- Native backdrop pixel test: fade endpoints, intermediate colors, clipping,
  persistent capture and invalidation.
- Three motion tests: elapsed-time endpoints, reversal, reduced motion and
  cancellable/exactly-once deferred activation.
- Existing object-store tests cover private checkpoint remount and alias denial.

These spatial/motion tests are part of `make input-regression-test`, run by
`build.sh`. Full build validation also includes TCP/HTTP, both UEFI architectures,
model packaging and byte-identical installed-kernel payload parity.

## Release and installed acceptance

Build command: **`sh build.sh`**. ARM release: `builds/InfinityOS-aarch64.iso`.
Provisioning model image: `build/hermes/InfinityOS-Hermes-Qwen-aarch64.iso`
(legacy filename: Hermes + Ministral, not Qwen). The build checks equality of those
ARM copies.

The existing VM disk `infinityos-4-spatial-20260922.vdi` boots with no ISO, but
contains the earlier integration and remains at sign-in. A new ISO does not update
that disk. Updated installed interaction, screenshot comparison and frame-time
measurements have **not** passed; host tests/builds are not substitutes.
Full installed visual/performance acceptance remains open.
