# Desktop productivity workflows

These five global workflows use Ctrl+Shift plus the listed letter. They work
from the unlocked desktop, Settings, system menus, and the app launcher.
They are keyboard access to existing native applications, not five new apps.

| Chord | Workflow | State preservation |
| --- | --- | --- |
| Ctrl+Shift+N | Quick notes: bring Text Editor forward | Keeps the current document and unsaved edits |
| Ctrl+Shift+E | Open Home in a new File Navigator window | Keeps other navigator windows |
| Ctrl+Shift+T | Open or resume Command Window | Keeps the existing command session |
| Ctrl+Shift+P | Open Task Manager | Uses the live native task service |
| Ctrl+Shift+L | Securely lock the desktop | Uses identity session locking and retains window layout |

The chords are routed before application-local text input, so they do not type
letters into a focused editor or chat box. Login, lock, installer, and onboarding
screens cannot dispatch them. Ordinary Ctrl shortcuts and Ctrl+Shift+Z redo keep
their existing behavior. Both firmware and native HID inputs share one decoder.

No visual assets are added: existing native app surfaces, typography, selected
icon pack, spacing, and window chrome are reused unchanged.

## Verification

The input regression target exercises the shared chord mapping, all five typed
actions, authentication/surface guards, ordinary Ctrl non-collisions, and redo.
The firmware test exercises actual modified UEFI key packets. This is decoder
and routing-policy coverage, not installed GUI acceptance.

The code is compiled into the common live and installed kernels; no live-only
asset or service payload is introduced. Full installed, ISO-detached interaction
verification and the rebuilt ISO parity check remain required before completion.
