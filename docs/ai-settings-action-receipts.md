# AI settings action receipts

## Root causes found

- The typed control registry could open World Shift, but could not select a
  named world. Requests to change it could fall through to conversational text.
- The model-tool action veto omitted `set`, `change`, `rename` and `turn`, so
  non-alias settings wording could be classified as conversation or rejected.
- Computer-name and other edits relied on narrow installed aliases. Expanded
  deterministic parsing now also handles polite prefixes, articles, punctuation
  and preserved-case values, even with an older installed alias file.
- Every setting edit reapplied all appearance settings. Name-only edits now
  leave the current world palette alone.

## Implementation

`setting_command::Request::spoken` routes supported settings directly to typed
execution without waiting for Hermes generation. World selection uses the normal
workspace controller, session settings capability, native checkpoint and visible
world publication. It verifies both the resulting desktop world and the stored
checkpoint before acknowledging success. User-renamed worlds are supported;
`reserch` is accepted as the explicit requested spelling variant of Research.

Identity settings require an active, matching, authorized session, successful
identity persistence and read-back of the requested value. Invalid values and
unknown world names cannot receive success. Failed execution remains recorded as
failure by ExecutionBroker, but its truthful response is completed for speech
playback rather than being discarded as a failed inference.

Supported direct settings: computer name, primary/accent color, theme, glass
opacity, background blur, inactivity timeout, speech output, named World Shift.
This is not universal settings coverage. Unsupported settings still require their
native UI; conversational prose is not evidence of execution.

Native setting mutations also set a one-shot presentation invalidation. This
refreshes the System Overview and retained settings surfaces, including machine
name changes that otherwise only repainted the chat receipt. Ordinary pointer
movement does not trigger this invalidation.

## Verification

Focused parser tests cover the reported World Shift requests, polite computer
rename, case preservation, invalid values and negated/explanatory input. AI harness
coverage routes voice envelopes to typed controls and checks identity round-trip
values. Installed microphone-to-visible-setting proof remains a separate manual
acceptance step; no latency improvement percentage is claimed from source alone.

### Release evidence (2026-10-24)

- Parser: 2 behavioral tests passed; AI harness passed, including typed voice
  dispatch and identity round-trip checks.
- Clean release: `./build-kit full --skip-tests` exited successfully. Broad
  regression tests were deferred; packaging/integrity gates remained enabled.
- Manifest: `builds/manifests/20261024T051225712413Z-38432.json`.
- Both ISOs passed current-kernel, model, live/installed artwork and design-kit
  payload parity, followed by checksum verification.
- ARM64 SHA-256: `d16a99b0642587c04b7c48a33197bd3389f522472baf6daec8f1f4a18c9a68ea`.
- x86_64 SHA-256: `2f111110b35ed1fd3a170e2a2f8d2fe018c93368d53b489a054653ba1a2247d8`.
- No installed VM was updated or microphone-driven acceptance claimed.
