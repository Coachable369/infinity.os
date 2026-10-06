# Enterprise Workplace

## Acceptance Status: Incomplete

As of 2026-10-06, the workplace behavior passes the installed ARM acceptance
suite, but the images are not yet release-accepted.

- All 11 workplace core tests pass, as do the full object-store and input
  regression suites. Format-7 migration passed interrupted-write and remount tests.
- The navigator harness now waits for a fresh post-release window snapshot.
  The reused ARM disk and a fresh ARM QEMU installation both passed all workplace
  checks: text/file transfer, checksum/comparison/backup/restore, report readback,
  history/private mode, browser clipboard policy and pixels, safe paste,
  expiry/lock clearing, and detached cold-boot persistence.
- Subsequent screenshot review found the eight-action context menu still used
  five-row paint and hit bounds. Both bounds now include all eight rows, with
  layout assertions at compact and high-density resolutions. A focused installed
  test passed Copy/Cut/Paste clicks and unchanged object identity on collision.
  Screenshot review confirms all eight rows are enclosed. This focused run used
  an updated disposable clone; the earlier fresh-install full-suite receipt
  predates this extra acceptance check. Other existing desktop text overlaps
  are visible in the screenshot; this is not whole-desktop visual acceptance.
- The fresh ARM disk used the QEMU media with byte-verified installed kernel
  parity, not an offline replacement kernel. Its initial cold boot faulted at
  address zero during native speech initialization. Unchanged-media retries
  booted successfully; this intermittent startup failure remains unresolved.
  The ARM VirtualBox-targeted final ISO is not covered by that runtime proof.
- All 46 browser core tests and all eight native engine interaction stages pass.
  Object-store, input, network, transport, resource-policy, and installed UI asset
  parity checks pass after correcting a namespace-capacity test fixture.
- Both architecture ISO profiles rebuilt successfully after the menu fix. These
  were focused builds, not a clean `full` build. The initial x86 image required
  a contiguous allocation crossing the PC machine's 3 GiB RAM boundary. The
  allocation correction below now reaches the final-ISO installer, but fresh
  installation exceeded its 30-minute provisioning limit. Installed x86
  workplace behavior remains unverified.

Fresh ARM workplace evidence:
`build/browser-installed-1791273370904977000/result.json`, with successful run
manifest `builds/manifests/20261006T080234621774Z-21192.json`.
The original startup failure is retained in
`builds/manifests/20261006T075610816696Z-21014.json` and that disk's installed log.
Reused-disk evidence: `build/browser-installed-1791194800249204000/result.json`;
its receipt explicitly distinguishes checks from preceding resumed runs.
Failed final x86 ISO run:
`builds/manifests/20261006T081312907031Z-21487.json`, with firmware allocation
diagnostics in `build/browser-installed-1791274392995346000/node-1/installer.log`.
The ARM fault remains a release blocker. The x86 allocation correction below
has final-ISO live-startup proof but still requires installed acceptance.
Focused installed menu evidence:
`build/browser-installed-menu-20261006T0841/result.json`, with manifest
`builds/manifests/20261006T084155757763Z-32278.json` and screenshot
`build/browser-installed-menu-20261006T0841/node-1/workplace-context-eight-rows.png`.
Post-fix input regression manifest:
`builds/manifests/20261006T084425665850Z-32966.json`.
Both updated full-bundle ISO profiles completed successfully after the menu fix:
ARM `builds/manifests/20261006T082041262424Z-22071.json`, and x86
`builds/manifests/20261006T084506607296Z-33444.json`. These package the current
workspace, including pre-existing local changes left outside this commit.

### Boot Allocation Follow-Up

The shared loader now reserves and clears merged ELF load segments independently,
leaving physical gaps untouched. It validates segment extents and the executable
entry before reserving memory, and rolls back earlier reservations on failure.
The x86 linker places its 1.5 GiB speech arena in a separate zero-fill segment at
4 GiB. Only the arena's private libc adapter uses large-model addressing; normal
kernel data remains below the PC aperture. The UEFI loader's preferred image base
is 32 MiB, avoiding its former 5 GiB overlap with that arena.

Host behavioral loader checks, including exact copied/zeroed/untouched bytes and
multi-range rollback, passed in
`builds/manifests/20261006T100442485846Z-60621.json`. Binary loader-parity tests now
reject matching but conflicting PE image extents; four tests passed in
`builds/manifests/20261006T103906165304Z-68265.json`. Both live and installed ELF
packaging checks require the independent high arena and reject PCI-gap crossings.

A disposable default-PC/12-GiB QEMU gate reached structured live startup using
the corrected loader and kernel:
`build/browser-installed-boot-gate-1791282163277862000/result.json`, manifest
`builds/manifests/20261006T102243202130Z-66895.json`. This is not final-ISO or
installed proof. The earlier intermediate ISO still collided with its own
loader at 5 GiB (`20261006T101749362883Z-66741.json`); it is not an accepted image.

The corrected x86 full-bundle ISO passed the focused build and binary packaging
checks in `builds/manifests/20261006T103914782900Z-68280.json`. Its first runtime
attempt exceeded the 120-second firmware startup allowance. A bounded
300-second retry reached live startup and installer steps 0 through 6, including
the default-Cancel destructive confirmation. Provisioning then exceeded its
unchanged 1800-second limit after allocating approximately 4.85 GB on the target
disk. No completion, readback, detached boot, or installed workplace success is
claimed. Evidence is retained in
`build/browser-installed-1791284158956976000/` and failed manifest
`builds/manifests/20261006T105558853893Z-73180.json`.
That diagnostic retry temporarily increased generic default waits to 300
seconds; the production harness change below confines the allowance to startup.

The installed harness now supports a per-guest startup allowance: 120 seconds
by default and 300 seconds for the full-bundle x86 probe. UI predicates and
interaction deadlines are unchanged. Two behavioral routing/default tests passed
in `builds/manifests/20261006T113123796104Z-73794.json`.

The ARM full-bundle ISO was rebuilt with the same shared loader correction;
its focused build and binary packaging checks passed in
`builds/manifests/20261006T113128208915Z-73802.json`. Updated candidates are
`builds/InfinityOS-x86_64.iso` and `builds/InfinityOS-aarch64.iso`, with combined
checksums in `builds/SHA256SUMS`. These are focused workspace builds, not a clean
release, and include the pre-existing local changes excluded from this fix's
commit. The rebuilt ARM image has not passed fresh detached-install acceptance.

Bounded ARM debugger runs observed valid speech-worker stack reuse but did not
isolate the intermittent bad write. No speculative ARM runtime change was made.
Google search-result rendering also remains unverified.

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
