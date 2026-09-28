# New-tab correction

- Plus and Ctrl/Cmd+T retain bounded add-tab requests while the native worker
  starts or its mailbox is busy. Closing the window or changing sessions clears
  pending requests.
- Servo and the native tab strip share the same insertion rule: immediately
  after the active tab, selecting the new tab without rearranging existing tabs.
- Engine creation failures now emit a native error event instead of silently
  ignoring the request. The existing eight-tab engine limit remains.

Verification: 29 browser-core tests passed, including insertion at first,
middle, last and empty positions. Native ARM64 and x86_64 kernel checks passed.
The ARM64 Servo component also rebuilt and linked successfully (not executed).
These checks do not establish installed pointer interaction. A fresh ISO and
installed-system plus-button verification remain outstanding.
