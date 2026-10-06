# Enterprise Workplace

## Acceptance Status: Incomplete

As of 2026-10-06, this implementation is not release-accepted. The repository's
six-attempt correction limit stopped the installed acceptance run.

- All 11 workplace core tests pass, as do the full object-store and input
  regression suites. Format-7 migration passed interrupted-write and remount tests.
- An updated ARM installed kernel built successfully. A reused, detached ARM
  installation passed text clipboard transfer, checksums, comparison, verified
  backup, bad-digest rejection, restoration, and support-report readback checks
  across separate runs.
- The current installed harness incorrectly treats diagnostic slot 8 (settings
  focus) as window focus in `documents()`. Its preceding run used stale window
  coordinates after opening another File Navigator. Neither failure establishes
  that file clipboard behavior passes or fails.
- Installed file transfer, history/private mode, browser clipboard policy,
  expiry/lock clearing, and final cold-boot persistence remain unverified.
- Native browser clipboard fixtures passed earlier; the latest broker changes
  still need that fixture rerun. Fresh ARM/x86 installations and updated ISO
  acceptance are outstanding. Existing ISO files are not the completed release.

Latest failed run: `builds/manifests/20261006T062605662027Z-92628.json`.
Installed evidence: `build/browser-installed-1791194800249204000/result.json`.
That receipt explicitly distinguishes checks from preceding resumed runs.

The workplace implementation is shared by ARM and x86. It adds these fifteen
capabilities to native applications and the installed System Generation:

1. Session-wide text copy, cut, and paste across Notes, Command, AI composers,
   browser page editing, browser chrome, and editable File Navigator fields.
2. File Navigator Copy, Cut, and Paste, available from its context menu and
   standard keyboard shortcuts. Cut stages a move until paste succeeds.
3. Clipboard erasure on lock, logout, and session replacement.
4. Configurable clipboard expiry.
5. Independent browser clipboard read and write restrictions.
6. Atomic terminal paste that rejects multiline and control-character input.
7. Command history with Up/Down recall and restoration of unfinished drafts.
8. Reverse history search and explicit history queries.
9. Tab completion from the native command registry and workplace commands.
10. Private command mode with history erasure and suppressed command echo.
11. SHA-256 file checksums.
12. Byte-exact file comparison with first differing offset.
13. Independent, readback-verified file backups.
14. Digest-gated restoration with native previous-version retention.
15. Redacted, readback-verified JSON support reports.

## Commands

Run `work help` in Command. Paths containing spaces accept single or double
quotes. Destinations must be new names under existing Personal Space folders.

```text
work clipboard clear
work clipboard status
work clipboard ttl 300
work clipboard browser-read off
work clipboard browser-write off
work history find "system"
work history clear
work private on
work private off
work checksum /home/default/documents/example.txt
work compare /home/default/documents/example.txt /home/default/documents/backup.txt
work backup /home/default/documents/example.txt /home/default/documents/backup.txt
work restore /home/default/documents/backup.txt /home/default/documents/example.txt SHA256
work report /home/default/documents/support.json
```

Replace `SHA256` with the full digest returned by backup. Restore rejects a
mismatched digest before writing. Existing backup/report names are never
overwritten. A readback failure is reported as a failure, not a successful backup.

## Interaction And Safety

Command/Ctrl+C, X, V use the current application's selection; Command copies its
whole editable draft. Up/Down recalls commands, Ctrl/Command+R searches history,
and Tab completes the draft without executing it. History-control commands and
recognized credential-bearing commands are excluded from history. Private mode
does not suppress an invoked command's own output: it is not an output-redaction
or forensic-erasure mechanism.

Clipboard policy and history are session-local, not machine-wide administrator
policy. Expiry is disabled by default; `ttl 0` disables it and values through
86400 seconds are accepted. Browser read/write are initially enabled, but each
operation still requires a native keyboard gesture and a focused view. Grants
are one-shot, expire after two seconds, and are revoked by lock, navigation,
tab changes, and closure. Password-field copying is refused. A rejected browser
cut does not remove selected text.

The clipboard holds one typed payload, at most 16 KiB of UTF-8 text or a staged
file reference. Copying text replaces a file reference and vice versa. Text is
never coerced into a file operation. Native ASCII-only fields reject unsupported
characters atomically; Notes also accepts its existing newline/tab controls.
Empty, expired, unsupported, or oversized paste never deletes the current
selection. Terminal paste never submits a command.

File paste validates the complete source object identity again, refuses existing
destination names, and clears a cut payload only after a successful native move.
Backups and restoration use the native bounded object-content capacity; oversized
files fail explicitly. Previous target versions remain in object history.

The existing bootstrap store still has bounded indexes: 52 objects, 64 content
versions, and now 44 namespace references. Format 7 uses previously reserved
metadata sectors for the extra references and reads formats 4..6 for upgrades.
This is not an unbounded enterprise-scale filesystem. Capacity failures leave
existing files intact and are reported explicitly.

Support reports contain only format/architecture, installed status, display
dimensions, ready-device count, storage block counts, task count, and browser
state/error numbers. Unavailable measurements are JSON null. No clipboard,
command, user name, serial number, address, credential, path, or prompt is included.

## Verification

Behavioral core tests run with:

```sh
./build-kit run cargo test --manifest-path sdk/infinity-enterprise-core/Cargo.toml
```

The native engine interaction fixture additionally exercises real browser DOM
editing, cross-boundary clipboard content, denied/oversized/password cuts, and
locked-session paste. Installed acceptance runs through actual QEMU input and
read-only structured diagnostics, including a detached cold boot:

```sh
./build-kit run python3 tools/servo-platform-probe/run-desktop-installed.py --arch aarch64 --accel hvf --iso-parity --workplace --automatic-network
```

Successful host tests or native engine fixtures alone do not establish installed
desktop acceptance. Build manifests and installed-run receipts are the evidence
for the exact images tested.
