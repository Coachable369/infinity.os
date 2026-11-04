# Native Console reference and registration

The authoritative in-OS catalog is `kernel/runtime/console_help.rs`, backed by
`console_language::DOMAINS` and `OPERATIONS`. `help`, `help commands`,
`help DOMAIN`, `help DOMAIN ACTION`, `help COMMAND`, and `help search WORD` all
use this source. Local results page with Enter/Space; Escape ends paging. SSH
uses the same documentation renderer without acquiring the documented operation's
authority. A documented operation may require local consent or administrator
capabilities; documentation is not an execution grant.

Every new typed operation must supply its domain, action, description, typed
arguments, capability, effect, and example in the existing OperationSchema.
Every native front end or alias outside that parser must register an Entry with
its exact name, usage and description in COMMANDS in the same change. Add its
behavioral acceptance to the owning service harness. Do not add a second help
string table in a dispatcher. Existing schema-backed operations appear without
manual duplication; front-end commands share this supplemental registry.

## Added front ends

- `ipconfig [--all]`: all native interfaces, MAC, link state, MTU, addresses,
  prefixes/netmask, routes/gateways, DNS and packet/drop counters.
- `list`, `ls`, `dir`: identical namespace listing; `-l`/`--long` includes owner,
  size, kind, version, creation and modification times; `-a` includes dot paths;
  `-R`/`--tree` includes descendants. Short switches can be combined. Paths may
  be absolute, relative or quoted. `--` ends switch parsing.
- `edit FILE`: opens a new or existing local text file in the command window.
  Native ASCII documents are bounded to 16 KiB; unsupported files are rejected
  without modification. Ctrl+S/O saves, Ctrl+X exits, Ctrl+W/F searches, Ctrl+G
  goes to a line, Ctrl+K cuts a line, Ctrl+U pastes, Ctrl+Z/Y undo/redo, Ctrl+A
  selects all. Arrows, Home/End, Delete, Backspace, Tab and Enter edit normally.
  Unsaved exits ask Y (save), N (discard), or Escape (cancel). Save rechecks
  authority and the original object/version to reject external conflicts.

Object timestamps now record trusted UTC with a tagged encoding in the existing
fields. Old objects store transaction generations, not calendar dates; listings
explicitly identify those as not recorded. No fabricated migration dates.
These new file utilities are local Console operations; the SSH remote capability
ceiling is unchanged.

## Facial enrollment storage repair

An installed account with generated SSH files filled all 44 v7 namespace slots,
preventing the encrypted enrollment record from being attached. Format v8 adds
8 slots using four sectors already reserved by allocation bitmap rounding. Both
metadata banks keep independent extension sectors. Existing object extents and
v4-v7 namespace locations remain unchanged. Compact stores without the scalable
allocation-map reservation retain their original 44-slot limit; old formats are readable and the
next atomic checkpoint publishes v8. The object and version limits remain
bounded. Capacity and uncertain-commit errors now have distinct enrollment
messages; successful capture is no longer described as a camera failure when
persistence fails. Encryption and authenticated readback remain mandatory.

## Handoff

Requested validation: compilation and incremental ARM packaging; no tests or
installed-VM acceptance run in this cycle. Runtime interactions and facial
capture/save need user verification on the updated installed system.
