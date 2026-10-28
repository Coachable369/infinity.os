# Native Python 3.13 library status

Evidence snapshot: 2026-10-20 UTC. Requirement 5 (default Python libraries,
including working Tkinter) is **not yet complete**. A compiled module, an import,
a native semantic probe, an installed application and a release ISO are separate
claims. Nothing below implies that an existing VM contains the new payload.

## Inventory and scope

The authority for packaged names is the builder-generated
`builds/toolchains/<arch>/python-modules.json`, also available through the native
`_infinity_modules.inventory()` API. It derives from pinned CPython 3.13.16
`Python/stdlib_module_names.h`, actual builtin units and frozen modules, with
explicit `limitations`, `required_backends` and `supported_scopes`.

The current ARM inventory has 76 builtin and 962 frozen entries, including
submodules and bundled packages—not 1,038 independently verified facilities.
It contains 290 top-level standard-library names. Its SHA-256 at this snapshot is
`4799180663a792614c80f200060a03436760cb51c7a83b824cc92a4c60b81b6e`.
The x86 payload must be rebuilt and verified separately; do not infer its status
from these ARM counts.

The requested [W3Schools list](https://www.w3schools.com/python/python_ref_modules.asp)
is a useful user-facing index. The pinned source and
[official Python 3.13 library reference](https://docs.python.org/3.13/library/index.html)
are canonical for module versions, optional dependencies and platform availability.
Removed Python 3.13 modules, such as `cgi`, `audioop`, `telnetlib`, `imghdr` and
`lib2to3`, are not missing InfinityOS ports and must not be silently restored as
standard modules. See the
[official removals](https://docs.python.org/3.13/whatsnew/3.13.html#removed-modules-and-apis).

## Implemented native behavior and evidence

| Facility | What is actually established | Evidence / remaining gate |
| --- | --- | --- |
| Core data/text libraries | Native arithmetic, Decimal/Fraction, JSON, SHA-256, XML, Unicode codecs, UUID, profiling and queue operations; broad core imports | ARM `stdlib-acceptance.py`, manifest `20261019T233804024960Z-2384`; not an exhaustive test of every imported API |
| Compression | Real zlib, bzip2 and liblzma; compression roundtrips, incremental decoding, malformed-input errors, bzip2 tar archive | Same native stdlib probe; installed archive workflows remain separate |
| `zoneinfo` | Genuine timezone backend and bundled tzdata 2026.5; Chicago winter/summer offsets, Tokyo and timezone directory | Same native stdlib probe |
| `sqlite3` | Genuine SQLite SQL, transactions, constraints, Python callbacks, JSON, FTS5, serialization and integrity check | Same probe proves in-memory behavior. Baseline installed file-open failed; corrected path handling and durable `/data` still require refreshed installed proof |
| `tkinter.Tcl()` | Upstream Tcl/Tk 8.6.18 and `_tkinter`; independent Tcl interpreters, evaluation, variables, Python callbacks and Tcl errors | Same probe plus detached installed Tcl import. This does **not** prove working widgets |
| `mmap` | Upstream module, private anonymous mappings, reads/writes, slices, buffer views, reuse and reclaim; 1 MiB committed budget | ARM manifest `20261020T012039970217Z-33976`; no file-backed, executable, resize or cross-process mappings; installed/x86 pending |
| `ctypes` | Upstream `_ctypes` and pinned libffi 3.4.8; LP64 types, private buffers, real target ELF calls, structs, variadics, pointers, errno and denied libraries | ARM exit 54, pages reclaimed, manifest `20261020T012058479315Z-34009`; no Python callbacks, ambient `CDLL(None)`, foreign ABI or early unloading; installed/x86 pending |
| Client networking | Native authorized IPv4 TCP/DNS, TLS 1.3 with hostname/system-trust validation, HTTP response parsing and `urllib.request` reads | Detached ARM baseline `20261020T003453766324Z-26516`; one active client, no listeners/UDP/IPv6/descriptor polling. New certifi-subset/Requests changes need refreshed installed proof |
| Online pip | Real downloads, install, imports/functions, metadata, reboot persistence, exact failed-install rollback and uninstall for `six`, `python-dateutil`, `packaging` | Same ARM installed manifest; overall run failed independent SQLite/source-build gates, so this is not a blanket pip compatibility claim |

Host-side focused tests are narrower evidence: manifest
`20261020T004827927328Z-28313` exercises actual loader/VFS/ObjectStore data
isolation, revision conflict, injected write failure, disk full and transaction
rollback. It does not substitute for native installed database durability.

## Pending installed gates

- **Tk widgets:** the first installed run (`20261020T004544574139Z-27852`)
  rendered native window chrome but failed a geometry/pack assertion. The
  ConfigureNotify target fields and stale-pixel damage fixes are authored.
  `tk-installed.py` must pass real Entry input, Ctrl-A selection, button/timer
  callbacks, Canvas/PhotoImage pixels, resize, move, close veto, cleanup pixels,
  repeated eight-window allocation and `python -m tkinter`. The inventory's
  conservative Tk restriction remains until this passes; its wording is a
  generation gate, not evidence that `_tkinter` is absent from the new binary.
- **Durable application files/SQLite:** direct saved scripts receive owner/source
  scoped `/data`; ordinary documents and packages remain read-only snapshots.
  Scratch is private and disposable. Pip, `-c`, validation and build children do
  not inherit `/data`. Actual sync/journal, abrupt-exit recovery, source isolation
  and detached-reboot checks are queued with `tk-installed.py --durable-data`.
  WAL shared memory and concurrent cross-process database access are unsupported.
- **PEP 517 source packages:** the first source-build gate failed. Regular-file
  short-write and resource-materialization fixes exist, but successful native
  build/install/import and rollback still need their installed receipt.
- **Architecture/release:** rerun the affected native and installed gates on
  x86_64. Publish only after the normal full-build/parity/checksum process; the
  preceding focused manifests do not establish a fresh installer release.

## Genuine remaining OS gaps

| Modules / scope | Missing contract; smallest appropriate existing-service integration |
| --- | --- |
| `threading`, worker `concurrent.futures`, blocking queue usage | Native `Task` owns one `Process` and timer deadline. Reuse `native_cpu::Context` with guarded stacks/PT_TLS and existing execution budgets, but add bounded thread groups, deferred syscall completion, join/condition wait queues, per-thread key values, errno/reentrancy and real Newlib locks. Current serial mutex/TLS adapters and `pthread_create`/condition-wait rejection cannot safely be relabeled threads |
| `select`, `selectors`, `asyncio` | Native network actor has asynchronous tickets, but no Python descriptor readiness/wakeup contract. Expose bounded readiness from that actor and native timers/events before adapting selectors; do not busy-loop or manufacture ready descriptors |
| General `subprocess`, `multiprocessing` | Existing process loader, scoped mappings and lifecycle are reusable. Only a bounded pip-authorized Python build-child adapter exists; general child launch, pipe ownership, concurrent children, cancellation and resource accounting are not implemented. Fork/POSIX shared-memory assumptions do not match current isolated execution |
| Listening servers and non-HTTP protocols | `http.server`/`socketserver` require native listen/accept plus polling. FTP/mail clients are not validated by the HTTP/TLS path; their frozen Python source is not an availability promise |
| POSIX terminal/account/process modules | `fcntl`, `grp`, `pwd`, `resource`, `syslog`, `termios`, `readline`, `_curses` and `_curses_panel` are not included. Native identities, Console input and scheduler accounting are not POSIX user databases, TTYs or process-control APIs; each needs an explicit authorized adapter |
| Platform-only modules | Windows modules (`nt`, `msvcrt`, `winreg`, `winsound`, `_winapi`, `_overlapped`, `_wmi`) and macOS `_scproxy` are not InfinityOS facilities |
| Environments / editor | `ensurepip` is omitted because pip is bundled; `venv`, IDLE and its multiprocessing assumptions are not supported. Tk availability does not automatically make IDLE or every turtle workflow complete |

Missing accelerators do not necessarily remove their public libraries:
`_decimal`, `_hashlib` and `_uuid` are absent, while `decimal`, `hashlib` and
`uuid` use actual bundled alternatives exercised above. `_dbm`/`_gdbm` and
cross-interpreter channel/queue extensions are also absent. Consult the exact
inventory rather than treating its `included` flag as a functional guarantee.

Generated code must use the conservative `sdk/python/generation-libraries.txt`
contract and non-executing `_infinity_modules.validate_source()` dependency
check. Installed package files are discovered without executing their imports.
No model assertion, successful import alone, or package download establishes that
an unavailable OS primitive works.
