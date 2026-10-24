# Camera and AI lifecycle recovery

## Changes

- Explicit camera-check startup captures and displays one real JPEG thumbnail
  before model loading. It uses the existing authenticated isolated worker and
  bounded native camera operations. The preview does not count as enrollment.
- Model delivery uses checked 64 KiB chunks rather than a syscall plus yield for
  every 4 KiB. Cancellation and the existing setup deadline remain enforced.
- Desktop submissions no longer tear down a generating application's lease
  before admission. Independent explicit controls use the authenticated execution
  broker while application generation retains its model. A second inference
  request is rejected without cancelling that work; this is not parallel model
  inference. Context/code-generation requests still use the normal goal planner.
- Rejected generated tool records receive a terminal failure receipt instead of
  leaving a Running chat with no pending action. Orphaned desktop generation is
  terminated, and desktop inference has a two-minute deadline. Application
  generation retains its existing separate watchdog.
- Computer-name phrasing includes “change the name of the computer to …”. A
  successful explicit rename dismisses the obsolete General-settings edit draft
  and uses the existing one-shot redraw invalidation.
- The reusable mask mesh remains visible while idle: dimmer, four times slower,
  15 Hz capped. Active scanning remains 30 Hz; reduced motion is static. These
  connectors are decorative, not measured facial landmarks.

## Verification boundary

Focused setup protocol and parser tests passed. The AI harness passed after
correcting the fast path to preserve complex software-goal routing. Its added
ownership test confirms foreground control admission preserves a background app
owner and rejected conversation does not clear that owner.

These checks are not camera-hardware, installed voice, render-capture, or
security-qualified facial-authentication proof. Manual installed checks must
exercise camera attachment/disconnection, preview and capture, natural computer
rename followed by Settings/reboot, and window commands during code generation.

## Installer artifacts

The clean dual-target release completed through `./build-kit full --skip-tests`:
`builds/manifests/20261024T071007621162Z-33271.json`.
ARM64 was then refreshed through `./build-kit package-aarch64 --skip-tests`
to include the final worker startup-failure report:
`builds/manifests/20261024T075722900136Z-19196.json`.
Both runs passed live/installed kernel currentness, model/artwork payload parity,
and release checksum verification. Broad regression testing was deferred.

| Installer | SHA-256 |
| --- | --- |
| `builds/InfinityOS-aarch64.iso` | `2ddfb7d36d8d545f83925f4654bf0bc90be915b2115f4048a4adc9a383e0e77c` |
| `builds/InfinityOS-x86_64.iso` | `28874ef80a58f3c5f07078539ff93cbf1bc7d40f83c77fcc76618581aaecd581` |

Camera startup and idle mesh changes are committed on mainline as `0fc5d883`.
The AI corrections are present in the built working tree, but remain uncommitted
because those files contain substantial interleaved pre-existing work.

Final focused verification passed with exit status zero:
`builds/manifests/20261024T080757954954Z-22593.json`.
This reran setup protocol, rename parsing, and the AI harness, including the
orphaned-generation terminal-state and background-owner preservation tests.
Diagnostic output is retained in `builds/face-ai-recovery-final-tests.log`.
