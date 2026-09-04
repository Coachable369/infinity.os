# InfinityOS System Generations

**Status: TESTED on x86_64 UEFI/QEMU; architecture-neutral format IMPLEMENTED for AArch64 but installed AArch64 boot is not yet implemented.**

An installation constructs a declared System Generation in System Space; it does not clone the recovery environment. Generation 1 follows `INSTALLING (1) -> READY (2) -> ACTIVE (3)`. Values `FAILED (4)` and `ROLLBACK (5)` are reserved. The installer writes all payload data, verifies it, writes READY and verifies again, then writes ACTIVE and finally publishes the active-generation Boot Catalog and complete container marker. An interrupted installation therefore has no selectable ACTIVE generation.

The checksummed Boot Catalog (`INFBOOT1`, container-relative sector 3) contains the active generation ID and its manifest reference. A zero active ID means no generation is selected. It also reserves previous-generation and Recovery Space references so activation can later retain a known-good generation without changing the loader contract.

Generation activation is installer-private in this milestone. Read-only machine operations are `System.GenerationList` (`0x8001`), `System.GenerationInspect` (`0x8002`), and `System.BootStatus` (`0x8004`). `System.GenerationActivate` (`0x8003`) is reserved and is not exposed as a console mutation.

Every completed architecture milestone must also produce an equivalently updated installable InfinityOS system. A feature is not integrated until it is registered as a system component, survives clean installation, and works after independent target-disk boot.
# Identity compatibility

System generation manifests now include the identity/session service suite.
Machine identity and Personal Space ownership are persistent data and are not
recreated when a new System Generation boots. Sessions remain transient and a
new authenticated session is required after reboot.
