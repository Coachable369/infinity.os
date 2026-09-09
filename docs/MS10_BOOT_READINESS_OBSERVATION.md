# Installed boot readiness observation

The single preserved node1 of `/tmp/ms10-order-installed-bab4753` reached a coherent installed login projection after **211.897 seconds**. Its original120-second readiness limit failed. No authentication input, installation, configuration write, or guest-memory injection was performed; the VM was stopped after login became observable.

Evidence: `node1-boot-diagnostic-1788987834745785000.json` in that installed directory; `/tmp/ms10-focused-boot-diagnostic.log`; node1 screenshots `boot-diagnostic-125s` and `boot-diagnostic-211s`. Numeric samples recorded changing instruction addresses and increasing serial byte counts, then installed=1, mode=9, schema=1, matching even generation guards. This establishes late boot progress, not a diagnosis of its cause or a successful120-second acceptance result.

The owner-offline harness now permits explicit `--serial-boot --boot-readiness-timeout 300`. Default remains120. Each serial node records the selected budget, actual elapsed readiness, and whether it met the original120-second limit. Authentication, network/session, and transfer deadlines are unchanged. This bounded environment-specific startup allowance is not an OS performance fix. Installed verification using the new option remains pending.
