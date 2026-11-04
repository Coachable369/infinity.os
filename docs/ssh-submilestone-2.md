# SSH Submilestone 2 — headless native Console

## Implementation

The graphical Console and SSH share the existing native parser, operation schemas
and extracted handlers through `runtime::console_execution`. Results contain bounded
stdout/stderr, numeric status, operation ID and typed diagnostics. Success follows
actual handler completion. There is no Unix shell or process-launch compatibility.

After signed public-key authentication, IdentitySystem provisions an explicitly
SSH-origin session with exactly `SESSION_REMOTE_QUERY`. It inherits no desktop,
microphone, camera, personal-space or AI-context authority. Every dispatch rechecks
live identity/key/session state, validates a nondelegable exact-operation capability
through IOP, and retires that capability after completion.

The ceiling permits `help`, `system status`, `node list` and `ai status` (metrics
only). Other operations, plans and command pipelines are rejected explicitly.
Graphical Console retains its existing broader native operations. No AI inference,
Pool administration, file transfer, forwarding or privileged remote execution exists.

One shell or exec channel is allowed per connection; up to four connections have
independent line editing, history, buffers, terminal geometry and cancellation.
OpenSSH connection multiplexing for additional channels is intentionally unsupported.
PTY sizes, CR/LF, arrows, Home/End, Ctrl-A/E/U, backspace/delete, Ctrl-C, Ctrl-D,
window changes and SSH interrupt signals are handled. Ctrl-D on an empty line closes
the session. `exit` is not a separate SSH-specific command. Terminal editing is byte
oriented; a full Unicode terminal emulator is outside this implementation.

Each command is limited to 512 input bytes, 8192 stdout bytes and 1024 stderr bytes.
Transport windows apply backpressure; a ten-second command/output deadline bounds
stalled clients. Accepted operations are bounded local queries. No long-running
inference or network job is executed on the SSH reactor. Exit status and close are
queued before native authority is retired, and the transport remains alive to flush.

## Local provisioning

SSH remains disabled by default. An authorized local Console can use:

```
ssh key add USER ssh-ed25519 BASE64_PUBLIC_KEY
ssh enable 22 2
ssh status
ssh key revoke USER ssh-ed25519 BASE64_PUBLIC_KEY
ssh disable
```

Use a standard client with the registered native handle. `ssh USER@HOST` opens
an interactive native Console; `ssh USER@HOST "system status"` requests native exec.
Verify the host fingerprint through a trusted local path before accepting it.

## Acceptance checklist

- [x] Host transport: signed authentication, rejection, AEAD policy and deadlines.
- [x] Host encrypted exec: distinct stdout/stderr and numeric exit status 37.
- [x] Host encrypted shell: PTY allocation, resize and terminal input delivery.
- [x] Host native executor: shared operations, authorization, revocation, IOP cleanup.
- [x] Host terminal: independent history, editing, CRLF, interruption, bounded output.
- [x] Native ARM64 and x86_64 no_std compilation checks.
- [ ] Final corrected release compilation, fresh installer parity and checksums.
- [x] Installed ISO-detached ARM64: real OpenSSH shell/exec/resize/isolation/cleanup.
- [ ] Installed ISO-detached x86_64: same acceptance.
- [x] ARM64 installed host-key persistence, malformed traffic and connection-limit checks.
- [x] ARM64 installed graphical Console/network/audio regression evidence.

Host SSH suite: `builds/manifests/20261104T085031730369Z-69470.json`.
Seven encrypted transport tests: `20261104T085725726235Z-70342.json`.
Account regression suite: `20261104T085510914200Z-70007.json`.
Clean full release: `20261104T085744555933Z-70456.json` (4970.192 seconds).
Pre-correction ISOs and SHA256 (do not contain the channel-completion fix):

- `builds/InfinityOS-aarch64.iso`: `b07126369c564afc85a7f47b8b662505846764156ca759680198e5becdf8a372`
- `builds/InfinityOS-x86_64.iso`: `2209ce3967306006564eee41d8cb4b0760f636aadd5195a394c59396fad39536`

Installed ARM run `build/ssh-arm-installed-02/result.json` verified four
media-detached boots and transport acceptance, then timed out on the first exec
request. The application had released its channel handle without sending protocol
EOF and close. A strengthened encrypted regression test reproduced this failure;
the corrected implementation and focused SSH suite passed in
`builds/manifests/20261104T105747072725Z-94219.json`.

Corrected ARM test media passed packaging and parity in
`builds/manifests/20261104T105830989622Z-94542.json`. The fresh installed run
`build/ssh-arm-installed-03/console-result.json` passed all four exec commands,
status/stream separation, denial with status 126, independent interactive sessions,
history, resize, interruption and clean disconnect. No remote sessions remained.
`network-result.json` also verifies concurrent outbound HTTPS response bytes.
The broader AI-widget helper initially failed because Console occluded its pointer
target, then expected an obsolete status-menu action. No widget code changed.
The focused SSH regression instead exercises composer input, generation admission,
Escape cancellation and recovery, editor preservation, concurrent HTTPS and actual
native speech PCM. Regression-only resumption validates both
media hashes and installed kernel bytes before boot; it never updates the disk
offline or patches guest memory. The original failed receipt remains preserved.
`build/ssh-arm-installed-03/regressions-result.json` passed, with manifest
`builds/manifests/20261104T113351270727Z-98856.json`. All 31 concurrent exec requests
succeeded (54–192 ms observed), cancellation took 0.213 s, speech produced 338940
nonzero captured samples, and revocation/disable left zero connections and grants.
These are observed timings, not a comparative performance benchmark. Corrected
release packaging and x86_64 installed acceptance remain pending.

Durable ARM evidence is archived under
`builds/validation/ssh-submilestone-2/aarch64/ssh-arm-installed-03/`, including
`console-result.json`, `regressions-result.json`, `transport-progress.json`,
interactive terminal captures and native audio. Earlier failed receipts are
retained alongside their diagnostic images. Disposable disk and media copies
were removed only after evidence archival.

The first x86_64 attempt reset before the installer became ready. The preserved
earlier ISO reproduced a guest-initiated reset at 302 seconds under the identical
TCG configuration (`20261104T115604068065Z-3266.json`). The shared loader's existing
watchdog-disable call has been moved to entry, before full payload preparation,
as a minimal prerequisite for installed verification. This follows UEFI's
[image watchdog contract](https://uefi.org/specs/UEFI/2.10/07_Services_Boot_Services.html#efi-boot-services-setwatchdogtimer).
The corrected image passed the live installer boot gate. Its first installation
attempt exposed a shared harness bug: the final completion wait ignored the
configured installation deadline and always stopped at 300 seconds. The helper
now honors the configured bound (one hour for x86 TCG disk installation).
Fresh installed x86_64 SSH acceptance and final release generation remain pending.

Installed harness: run `./build-kit run python3 tools/ssh-installed-test.py
build/UNIQUE-DIRECTORY --arch ARCH --iso builds/MATCHING.iso --console --regressions`.
It creates a disposable installation, checks media-detached boots, uses macOS
OpenSSH, and asserts typed guest snapshots and client protocol outcomes. It does
not patch installed code, identity state or guest memory. Regression mode compares
outbound HTTPS response bytes against a host reference, exercises graphical AI
input/cancellation during concurrent SSH queries, and verifies actual native speech
playback using emulated HDA PCM. These checks do not establish microphone recognition
or a performance comparison with an earlier release.

Status: implementation under verification; not accepted or ready for Submilestone 3.
