# Studio App Design integration status

Studio's left sidebar contains **App Design**. App identities are fixed; authored
appearance is stored in `runtime/install/System/app-design.json` and a native
128-byte-per-app title projection in `assets/ui-design-kit/default/app-design.bin`.
Both files are saved in one transaction. App-specific designs are opt-in, preserving
factory layouts until enabled. Shared materials remain controlled by UIKit.

## Implemented adapters

- InfinityCode: title, menu alignment, content inset, inspector side/width.
- Infinity Memory: title, menu alignment, content inset.
- Infinity Fabric: title, menu alignment, content inset.
- Media Player: title and content inset (no menu or inspector options).
- File Navigator, Text Editor, Command Window, Task Manager, System Settings:
  native title projection. Menu/content placement adapters remain outstanding.

## Open acceptance

This is an incomplete implementation of the all-app editor request.

- Browser, World Shift and Holographic Desktop title adapters.
- Native application menu and content layout adapters, using shared paint/input
  geometry rather than paint-only offsets.
- Full region editing/preview for app-specific controls, beyond workspace settings.
- Python app rendering in the installed OS, resize/zoom and action-dispatch proof.
- Production screenshots compared with the application workspace kit.
- Updated installer publication and installed disk-only parity.

No audio pipeline changes are part of this work. x86 publication remains on hold
under the user's Fabric acceptance instruction.
