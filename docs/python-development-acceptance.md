# Native Python development acceptance

Request: 2026-10-19. All eight requirements are release gates, not optional
follow-ups. A checked item requires observable runtime evidence; compiling an
adapter, listing a module, or generating a screenshot does not establish it.

## Implementation sequence

1. Derive a module/capability inventory from the pinned CPython build, distinguish
   importable modules from unavailable native backends, and provide that inventory
   to generation and validation. Reject missing imports before running generated
   code; do not silently download dependencies for a generation request.
2. Connect upstream pip to authorized native HTTPS with chain, hostname and date
   verification, native entropy, timeouts and cancellation. Retain isolated,
   transactional per-user publication. Add index resolution, wheel downloads,
   dependency resolution, update/uninstall and reproducible failure handling.
3. Complete standard-library native dependencies and genuine Tcl/Tk integration
   with InfinityOS windows/input/rendering. Frozen Python files alone are not a
   working tkinter, ssl, sqlite3, threading or subprocess implementation.
4. Retrieve the exact source version for edits; preserve unrelated content and
   dirty user work. Apply completed updates atomically to the main editor with
   one undo record. Save the same source identity and only relaunch on request.
5. Add the reusable editor terminal dock, persistent visibility/height, resize
   capture, scrollback, input, separate output channels and diagnostic navigation.
   Run/Stop/Restart reuse native C/Python and existing process lifecycle services.
6. Increase first-run wizard typography in the authored Eclipse Access template;
   retain its colors, generated assets, antialiasing and animation. Compile the
   same template used by Studio, live media and installed boot.
7. Persist generated application identity, source/artifact references, language,
   launch name/aliases and declared interaction contract. Resolve voice launch
   through this registry. Live hooks remain scoped to the actual process/session;
   registration must not grant AI control or expose nonexistent actions.
8. Execute the matrix below, clean-build both installers via `./build-kit full`,
   verify payload/checksum parity, and repeat installed tests with ISO detached.

## Hard acceptance matrix

| Requirement | Working-system evidence required |
| --- | --- |
| 1. Available libraries only | Generated scripts import only the actual system/user environment; unavailable imports are caught without execution or silent installation; inventory updates after package publication. |
| 2. Python/pip SSL | Successful native verified HTTPS; expired, untrusted and wrong-host certificates rejected; revocation of network capability cancels pending work; no verification bypass. Python `ssl` itself must have a genuine backend. |
| 3. Install libraries | Download/install/import representative utility, HTTP, templating, parsing and native-extension packages; real dependency resolution, upgrade, uninstall, cancellation rollback and reboot persistence. Source builds use authorized native tools where compatible. An incompatibility report identifies the exact unavailable ABI/system dependency. |
| 4. Surgical AI edits | Create then edit both C/Python applications; correct document is opened, untouched portions retained, changes visible, undo restores original, concurrent edits and stale versions rejected, explicit relaunch uses changed source. |
| 5. Standard library/tkinter | Pinned CPython 3.13 module matrix, actual functional probes for native-backed modules; `python -m tkinter` opens an interactive native window; widgets, callbacks, layout, text input, timers, resize and close work. No stub imports or lookalike module as proof. |
| 6. Terminal | Dock collapse/resize/scroll and reboot-persisted state; interactive C and Python stdin/stdout/stderr; stop a blocked program, restart without old output/input leaking; click diagnostics to exact file/line; close/session-loss cleanup. |
| 7. Wizard readability | Production-rendered screenshots at supported sizes, larger legible Inter text, no cropped labels/overlap, consistent template and input caret metrics. |
| 8. Registered apps | Two differently named AI-created apps survive reboot, appear in the OS catalog, launch by typed/voice name, expose actual scoped action schemas and reject stale/foreign/sensitive actions appropriately. |

No requirement is marked passed at plan creation. Existing offline-pip receipts
are baseline evidence only, not proof of online pip or this complete workflow.

## Current verification ledger

- Python generation follow-up (2026-10-20 UTC): focused editor, module,
  generated-validation, source-snapshot and atomic-repair checks passed in
  `20261020T042024227845Z-76553`. Prompts now select small relevant library
  hints, preserve repair-source boundaries and consult the authenticated
  installed-package directory. Output-budget stops retain an exact bounded
  continuation checkpoint; failed/cancelled jobs release the model.
- Actual pinned Hermes, using the shared native inference/editor path, completed
  `generate a chess game in python` in `20261020T042347989403Z-77015`.
  The 1,029-byte source reached the editor through 305 paced commits; first
  editor text was at 29.692 seconds and completion at 110.120 seconds. Syntax
  and installed-dependency validation passed; the next request was admitted.
  These are host-harness measurements, not installed desktop timing.
- ARM native CPython validation passed in `20261020T042842596480Z-79714`.
  The same placeholder fixture passes ordinary script validation but fails
  generated-only validation with trusted exit 67. Exact generated source passes
  syntax/dependency/placeholder checks. Candidates are not executed by
  validation; native page reclamation passed.
- **Chess application acceptance remains failed.** Native guest execution of
  the exact generated bytes, with controlled `e2e4` then `quit` input, raised
  `ValueError` after one input line. The smoke harness encoded that observation
  as exit 145 in `20261020T043141300710Z-80852`. This is a verified failure, not
  a passing game, installed keyboard test, or complete game-rules test.
  Exact source, source bindings, native binary receipts and model observations
  are retained in `builds/validation/python-generation-20261020/continued/`.
- A same-content upstream system-role experiment changed dependency behavior
  but produced incorrect chess rules. It was diagnostic only and was not adopted
  as a production prompt-format change. No model was replaced or package
  silently downloaded. A clean release attempt remains blocked by another
  active repository build; this follow-up has not published replacement ISOs.
- ARM QEMU test media packaged successfully in manifest
  `20261019T234224251799Z-2830`. This is test media, not a refreshed published ISO.
- That media completed a fresh installation and detached-media first boot.
  All eight setup screens and login were captured under
  `builds/validation/eclipse-access/editor-terminal-20261019/`. Visual review
  found one remaining small-font area: network choices. Its shared native/Studio
  correction is queued for the next payload and requires a new screenshot.
- Installed terminal acceptance exposed missing native C `fprintf`/`stderr`
  support. Compilation errors appeared in the terminal; application launch did
  not pass. Failed-run manifest: `20261020T000807901466Z-24895`.
- Non-executing import validation, including `from package import submodule`,
  passed in `20261020T002028351828Z-25524`. Native mapping ownership/range tests
  passed in `20261020T002036648006Z-25537`. These are focused fixtures, not proof
  of installed Python mmap semantics or end-to-end AI generation.
- Measured configuration typography and bounds at five resolutions passed in
  `20261020T002058166121Z-25589`. Screenshots remain the visual acceptance gate.
- Installed ARM manifest `20261020T003453766324Z-26516` successfully exercised
  ungranted-network denial, verified TLS 1.3, multi-record `http.client`/`urllib`
  responses, and wrong-host rejection. Public `six`, `python-dateutil` and
  `packaging` installation, real functions/metadata, cold-boot imports, failed
  install archive/version rollback, and uninstall passed. Its PEP 517 source
  build failed and remains open; the complete pip matrix has not passed.
- The installed SQLite file test exposed an existing `xFullPathname` defect:
  resolving a nonexistent database leaf failed before authorized creation. The
  correction resolves its existing parent without relaxing namespace authority.
  New-media SQLite and durable-data acceptance remain required.
- The actual password-focused login capture has been visually reviewed: branded
  artwork, antialiased controls, readable hierarchy and contained text are intact.
  The next-media network-configuration capture remains outstanding.
- Current-target source updates now preserve unsaved editor changes by retaining
  a separate exact saved checkpoint. The model receives the live editor buffer;
  stale persisted versions and changes made during generation remain fenced.
  Unrelated dirty documents still cannot be silently replaced. Focused and
  installed acceptance for this refinement are queued.
- Native C stdio behavior and editor source-update regressions passed in
  `20261020T005017839050Z-28576` and `20261020T005047435505Z-28700`.
  The actual ARM Clang/LLD/child execution probe passed in
  `20261020T005345024854Z-29170`, including separately measured stdout/stderr,
  a nonzero application exit code, compile rejection, and page reclamation.
  The installed dock workflow must still be repeated with the corrected SDK.
- Native C compile/link/execution now also passes on x86 in
  `20261020T010801584939Z-31851`. Explicit GOT placement before TLS in the shared
  linker script preserves the actual TLS segment extent on both architectures.
- PEP 517 helper materialization was truncated at 4096 bytes by a one-chunk
  regular-file write. The blocking file adapter now completes bounded chunks;
  `20261020T010948098357Z-32152` verifies exact multi-chunk bytes, partial failure,
  zero-progress rejection, revocation and stale descriptor behavior. Native
  resource materialization and full source-package installation still need the
  refreshed interpreter payload.
- Installed genuine Tk created widgets but failed exact child-frame resize
  acceptance in `20261020T004544574139Z-27852`. The production StructureNotify
  event constructor now sets both receiver and affected-window identity; its
  typed event regression passed in `20261020T005233353346Z-29062`.
  Desktop reconstruction now tracks old/new native window footprints separately
  from console content so destruction and movement can clear exposed pixels.
  Installed Tk resize and cleanup are not yet counted as passed.
- Durable-data storage and SQLite path contracts passed in
  `20261020T004827927328Z-28313`: source/owner isolation, version advancement,
  CAS conflict, disk failure rollback, child denial and journal flush barriers.
  These are focused contracts; cold-boot file/database tests remain required.
- The rebuilt native ARM interpreter passed anonymous mapping semantics in
  `20261020T012039970217Z-33976` and genuine outbound `ctypes`/libffi calls in
  `20261020T012058479315Z-34009`. Both ran in isolated guest user mode and
  reclaimed their resources. FFI covered integer/floating-point arguments,
  structures, variadic and stack arguments, private buffers, and denied library
  paths/callback allocation. This does not establish installed or x86 parity.
- Production ARM kernel integration type-checks in
  `20261020T012149920828Z-34060`, including per-connection restricted certificate
  trust and native GUI damage handling. The restricted trust policy and frozen
  pip child entry helper have separate behavioral fixtures; installed HTTPS
  packages and source builds remain the next gate.

SSL/pip, interactive Tk, generated application workflows, durable application
data, and final release/installed parity remain open until their actual runtime
checks pass. No complete hard requirement is inferred from these partial results.

## Constraints and sources

- One shared ARM64/x86_64 implementation, native OS services only; no host bridge,
  second intent engine, or parallel app executor.
- All build/probe artifacts use the exclusive repository build kit. Preserve
  unrelated worktree changes. Changes are committed on mainline only.
- Existing default UI kit and Eclipse Access/editor assets are the visual authority.
- Compatibility baseline: [Python 3.13 standard library](https://docs.python.org/3.13/library/).
  The requested [module reference](https://www.w3schools.com/python/python_ref_modules.asp)
  includes modules removed by Python 3.13; these are not silently reintroduced.
- [Tkinter architecture](https://docs.python.org/3.13/library/tkinter.html) requires
  Tcl/Tk and an actual platform window/event backend.
- [pip certificate policy](https://pip.pypa.io/en/stable/topics/https-certificates/)
  requires verified transport; installing certifi alone is not SSL support.
