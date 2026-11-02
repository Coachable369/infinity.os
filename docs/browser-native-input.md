# Native Browser Input

The shared keyboard decoder feeds both USB HID and PS/2 into the browser's
authorized input queue. Command and Control application shortcuts normalize to
Control at the engine boundary so clipboard capability checks remain shared.

## Commands

- Command/Control A, C, X, V: edit the focused native field or selected web content.
- Command/Control Z, Y, Shift-Z: native field undo/redo; page keys are forwarded.
- Command/Control plus (including Shift-=), minus, 0: page zoom in/out/reset.
- Command/Control L, R, T, W: location, reload, new tab, close tab.
- F5: browser reload, using a dedicated code rather than Command reverse-history.
- Control Tab/Shift-Tab and Command/Control PageDown/PageUp: next/previous tab.
- Command/Control 1-9: select tab, with 9 selecting the last tab.
- Command/Control F: focus find and select the query; navigation/tab commands
  remain available while find is open. Enter/Down and Up traverse results.
- Command/Control brackets or Alt-Left/Right: Back/Forward.
- Tab/Shift-Tab: forward/reverse content focus. Command-Tab remains the desktop
  window switcher. Reserved shifted desktop shortcuts retain their OS actions,
  including Control/Command-Shift-V clipboard history.
- PageUp/PageDown, Home/End: native named keys delivered to the focused page.
- Escape: dismiss the native field/menu or deliver Escape to the page. The pinned
  Servo API has no exposed stop-loading operation; network-stop is not claimed.

Native find and location fields have bounded, atomic single-line paste and one
step of undo/redo. Native field commands never edit background web content.
The existing workplace session boundary clears native drafts, selection, and
both history directions on lock, logout or an authenticated session change.
The pinned Servo editing action table does not implement native page undo/redo;
forwarding these keys supports page handlers, not an engine editing guarantee.

## Generic Mouse And Transport Boundaries

Inside web content, ordinary wheel scrolls vertically; Shift-wheel scrolls
horizontally; held Control or either Command/Super key plus wheel zooms. Zoom
takes precedence over Shift. Physical wheel-up zooms in and wheel-down zooms out.
One nonzero wheel packet requests one bounded zoom step and emits no scroll.
The canonical pointer delta is down-positive, including normalized USB reports.

Modifier state comes from physical USB boot reports, PS/2 make/break bytes, or
valid current SimpleTextInputEx state,
including modifier-only packets. Left/right keys and separate USB keyboards are
tracked independently. Interrupt timeouts preserve held state; source removal,
invalid packets, transport faults and reinitialization clear that source. The
existing pointer location, session owner and queue-capacity guards remain in use.

On ARM, the existing EDK path retains firmware ownership of the USB keyboard.
The native SimpleTextInputEx adapter samples valid current shift state on both
successful reads and `EFI_NOT_READY`, including modifier-only press/release.
When a valid toggle state is available, `SetState` requests `EFI_KEY_STATE_EXPOSED`
while preserving every existing toggle bit, including Caps/Num/Scroll Lock.
An unsupported exposure request does not prevent sampling valid current state.
Invalid/missing state, protocol errors, legacy SimpleTextInput and source changes
clear the firmware modifier contribution; character events never latch it.
This follows the [EDK USB keyboard implementation](https://github.com/tianocore/edk2/blob/master/MdeModulePkg/Bus/Usb/UsbKbDxe/EfiKey.c)
and [console splitter state aggregation](https://github.com/tianocore/edk2/blob/master/MdeModulePkg/Universal/Console/ConSplitterDxe/ConSplitter.c).
The current VirtualBox installation still needs live interaction verification;
firmware that omits valid current state cannot support modifier-wheel admission.

There is no native pinch event in the current guest pointer contract. A host
trackpad gesture is supported only if the VM translates it into the already
supported guest events. Genuine pinch and separate horizontal HID-wheel routing
are not claimed by this change.

## Focused Checks

`./build-kit browser-core-tests` exercises typed shortcut resolution, wheel-axis
selection, tab wrapping and native field editing/history. The existing
`tools/firmware-key-test.rs` main harness also exercises the production HID and
firmware decoders plus physical modifier lifecycle and reset behavior, and mocks
the actual SimpleTextInputEx function-pointer ABI to verify idle state, partial
keys, lock preservation and unsupported/invalid-state behavior. Compile
and execute that harness through `./build-kit run`, with output under `build/`.
These checks are not installed-VM or release/installer evidence.
