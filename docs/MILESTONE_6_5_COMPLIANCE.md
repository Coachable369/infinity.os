# Milestone 6.5 Compliance

Milestone 6.5 introduces two independent `no_std`, fixed-capacity modules:

- `kernel/storage/organization.rs`: taxonomy, metadata schemas, relationships,
  Project, Collection, View, Object.Query, ObjectSet, and location-neutral refs.
- `kernel/runtime/console_language.rs`: Domain/Operation registries, parsing,
  schema introspection, typed refs/variables, planning, side effects, and graph
  type checking.

Neither module imports filesystem, process, Unix socket, or JSON facilities.
Console rendering cannot feed another operation stage.

## Acceptance commands

```sh
make milestone-6-5-test
make object-test runtime-test ai-test
make test-console
make install-boot-test
make installed-object-test
./build.sh
```

The host suite verifies multidimensional organization without duplication,
reverse lookup, schema corruption, dynamic Collections, Views, mesh-compatible
refs, quoting, bounded dates, discovery/help, side-effect plans, session state,
valid ObjectSet composition, invalid DeviceSet composition, and service
discovery. VM tests exercise the Console registry on all architectures. The
installed-disk suite validates schema persistence without the ISO.

| Capability | x86 | x86_64 | AArch64 |
|---|---|---|---|
| Console grammar/discovery | TESTED | TESTED | TESTED |
| Typed graph preflight | TESTED | TESTED | TESTED |
| Organization contracts | TESTED on host | TESTED on host | TESTED on host |
| Installed schema persistence | N/A | TESTED | IMPLEMENTED BUT UNTESTED |

## Known limits

- Bring-up catalogs, relationship tables, and ObjectSets are bounded.
- Multi-stage graphs preflight correctly but execution is scaffolded; no stage
  falls back to rendered-text parsing.
- Calendar dates await a trusted time service for wall-clock conversion.
- `@selected` and `@last` are planned; numbered session references work.
- Automation, vector search, and mesh storage remain outside this milestone.

Next: replace the bounded relationship catalog with a copy-on-write native
index object and complete transactional multi-stage execution through IOP,
without changing the public Object.Query or OperationGraph contracts.
