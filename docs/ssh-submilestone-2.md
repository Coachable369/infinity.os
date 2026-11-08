# SSH Submilestone 2 — headless native Console

## Implementation

The graphical Console and SSH share the existing native parser, operation schemas
and extracted handlers through `runtime::console_execution`. Results contain bounded
stdout/stderr, numeric status, operation ID and typed diagnostics. Success follows
actual handler completion. There is no Unix shell or process-launch compatibility.

After native password or signed public-key authentication, IdentitySystem provisions an explicitly
SSH-origin session with exactly `SESSION_REMOTE_QUERY`. It inherits no desktop,
microphone, camera, personal-space or AI-context authority. Every dispatch rechecks
live identity/credential/session state, validates a nondelegable exact-operation capability
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

SSH defaults to enabled on port 22 with two connection slots when no saved
configuration exists. A saved disabled setting remains disabled. Native account
passwords and registered public keys are accepted. New accounts automatically receive an Ed25519
key pair and public-key registration during first-run setup, Settings creation,
and Console creation. Existing accounts and their keys are not silently modified.
An authorized local Console can use:

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
`ssh status` reports the actual listening IPv4 address, port, a connection example,
and typed startup/connection failures. An enabled listener waits for an enabled
network profile, live link and valid IPv4 address. For a NAT VM, use the host
address and forwarded SSH port from outside the VM.

## Verification and release

Native implementation commit: `d98403e8` (mainline).
The final clean release was stopped at the user's request to avoid repeated
full byte-for-byte comparisons. Final ISO packaging is unfinished; older ISOs
are not claimed as current. The subsequent default-on change passed the focused
SSH suite in `builds/manifests/20261104T141255435598Z-71923.json`; it has not yet
been verified on an installed guest. Automatic account-creation enrollment has since been implemented; current
release and installed acceptance results are recorded below.

### Behavioral host checks

The focused SSH suite passed in
`builds/manifests/20261104T105747072725Z-94219.json`: seven encrypted transport
tests, three listener tests, and native identity, storage, executor and terminal
harnesses. Coverage includes signed authentication, AEAD policy, deadlines,
stdout/stderr separation, numeric exit status, explicit EOF/close, PTY input and
resize, independent histories, revocation and capability cleanup. Both native
no_std architecture checks passed. Account migration and durable checkpoint
regression passed in `20261104T085510914200Z-70007.json`.

### ARM64 installed acceptance

macOS OpenSSH authenticated against an unchanged fresh installation with installer
media detached. Durable evidence is under
`builds/validation/ssh-submilestone-2/aarch64/ssh-arm-installed-03/`:

- `transport-progress.json`: public-key success and rejection, non-AEAD rejection,
  bounded connections, malformed clients, silent-client timeout, clean disconnect
  and persistent host identity across reboot.
- `console-result.json`: interactive Console; exec of `help`, `system status`,
  `node list`, `ai status`; stdout and stderr separation; successful status 0;
  unauthorized operation status 126; two independent sessions and histories;
  resize; Ctrl-C status 130 and reuse; Ctrl-D termination; zero remote sessions.
- `regressions-result.json`: outbound HTTPS status/body hash; graphical AI input,
  generation admission, Escape cancellation and recovery; unchanged editor text;
  native speech with 338940 nonzero PCM samples during concurrent SSH requests;
  credential revocation and listener shutdown with zero active connections/grants.

All 31 concurrent exec requests succeeded (54–192 ms observed); AI cancellation
was observed at 0.213 seconds. These are observations, not comparative performance
benchmarks. Microphone recognition was not tested. The final successful regression
manifest is `builds/manifests/20261104T113351270727Z-98856.json`.

The original failed receipt is retained: an initial broad widget helper hit an
occluded pointer target, then expected an obsolete menu action. Focused regression
resumption verified media hashes and installed kernel bytes before testing the
unchanged disk. No guest memory, credentials or installed executables were patched.
The earlier native exec failure was real: Sunset released the channel handle
without protocol EOF/close. The fixed encrypted regression requires both events,
and the installed Console run passed after the correction.

### x86_64 installed blocker

Compilation, packaging and the live interactive installer boot passed. Installed
SSH acceptance did **not** pass: after the full one-hour installation deadline,
the installer remained at stage 7, 67%, verifying boot assets. Read-only disk
inspection confirmed the generation was not activated. No media-detached installed
boot or SSH success is claimed for this architecture.

Evidence is preserved under
`builds/validation/ssh-submilestone-2/x86_64/ssh-x86-installed-03/`, including the
failed receipt, disk, exact installer media and executables. The failing manifest
is `builds/manifests/20261104T122855244124Z-7020.json`. The captured instruction
address maps to the installer's ATA `read_blocks` path; this identifies the observed
location, not a proven storage root cause. Screenshot:
`builds/validation/ssh-x86-installer-timeout.png`.

Two narrow prerequisites were corrected during verification: the loader's existing
firmware watchdog-disable call now runs before payload preparation, and the shared
installer harness honors its configured completion deadline instead of always
stopping at 300 seconds. A preserved pre-SSH ISO reproduced the five-minute firmware
reset; the updated loader passed live boot. No unrelated storage redesign or
offline generation activation was attempted.

## Reproduce

Run the installed harness only through the build kit:

```
./build-kit run python3 tools/ssh-installed-test.py build/UNIQUE-DIRECTORY \
  --arch ARCH --iso PATH-TO-MATCHING-MEDIA --console --regressions
```

ARM QEMU requires media produced by `./build-kit package-aarch64-qemu`, located at
`build/test-media/InfinityOS-aarch64-qemu-models.bootmedia`. The canonical ARM ISO
uses the VirtualBox layout. The harness creates a disposable installation, attests
media-detached boot, uses macOS OpenSSH and asserts typed guest state and protocol
outcomes. All guest provisioning goes through the local graphical Console.

## Readiness

Submilestone 2 implementation is available with ARM installed evidence. Full
cross-architecture acceptance remains blocked by x86_64 installation. No
Submilestone 3 work has been performed or authorized.

## Automatic account-key provisioning

New accounts receive `~/.ssh/id_ed25519` and `~/.ssh/id_ed25519.pub`. Home labels
follow the existing native home resolver; immutable user IDs enforce ownership.
The private file uses the standard [OpenSSH key format](https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.key).
It is unencrypted file content protected by native owner access, not an offline
disk-encryption guarantee. Temporary generation buffers are zeroized. The key is
independent of the account password and SSH host key.

Identity, key registration, home directories and both key files commit in one
native storage transaction. Failure restores identity state; ambiguous commits
invalidate authentication until reboot. Missing entropy prevents creation rather
than substituting a fixed key. Generic object reads, versions, copies, aliases,
writes and removal cannot access these key objects. The owner's authenticated
local editor can read the current files; remote Console authority cannot.

To connect from another machine, the owner must deliberately copy the private
key to that SSH client and restrict its local permissions (for macOS OpenSSH,
`chmod 600 PATH_TO_KEY`), then use `ssh -i PATH_TO_KEY USER@HOST`. Merely generating
a key on InfinityOS does not install it on the remote client. Password login also
works without exporting a key; see [password authentication and client](ssh-password-client.md).
Native SSH retains the Submilestone 2 capability ceiling.

Identity format 6 preserves earlier offsets and appends twelve credential slots,
allowing all eight identity slots to hold passwords and SSH credentials. Formats
1–5 remain readable. Object-store capacity continues to be bounded; insufficient
space fails the entire provisioning transaction. Renaming or signing in does not
regenerate keys. Key revocation does not delete the owner's retained key files. Password reset
retains the existing policy of revoking previous credentials, including SSH
registrations; it does not silently replace the saved key pair.

Focused SSH tests passed in `20261104T142513349151Z-79012.json`, including real
OpenSSH private-key parsing, eight-account credential capacity, version-5 migration,
owner isolation, disk-write rollback and cold-remount registration. The existing
accounts suite passed in `20261104T142605885459Z-79110.json`, including legacy
migration and 200 durable checkpoints. ARM incremental packaging passed in `20261104T173641203499Z-27640.json`.
The corrected ARM test image installed and completed onboarding on detached-media
boots. The subsequent Settings account automation stopped on clipped form controls;
Settings/Console creation and SSH authentication with generated account files have
not completed installed acceptance. The user elected to do SSH runtime testing.

Per the user's instruction, this handoff uses build-kit's incremental packaging
with `--skip-byte-comparisons`. This explicit option omits full kernel/image copy
comparisons and the duplicate full-kernel freshness scan. Structural packaging
checks, targeted behavioral tests and release checksum publication remain. It is
not a full clean release/parity claim. Default builds retain their comparisons.

The first account-enrollment installed run (`build/ssh-account-installed-01`)
stopped before onboarding with corrupted stack data and a firmware assertion.
Inspection of its actual ARM executable showed 380032 bytes in kernel entry plus
716848 in Console initialization: 1096880 bytes before nested work, exceeding the
loader's 1048576-byte reservation. The shared loader now reserves 2097152 bytes.
`builds/validation/ssh-account-stack-overflow.json` records the addresses and
measurements. Corrected installed boot and initial onboarding succeeded; the full multi-account
acceptance remains incomplete as described above.
