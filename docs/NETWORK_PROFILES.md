# Network Profiles

Profiles are typed operational connectivity postures, not shell scripts or
text configuration. Built-ins are Standard, Restricted, Offline/Air-Gapped,
Operations, and Developer. Custom profiles use a bounded persistent registry.

Activation executes `validate -> stage -> apply -> verify -> commit`. Failure
restores the last-known-good selection. The committed profile and generation
encode into the versioned `INFNET01` native object. The IEF activation record
is published only after persistence succeeds on the target OS.

Profile transaction, invalid-profile rejection, Offline local operation,
restore, and binary state round-trip are **TESTED**. Settings activation is
**IMPLEMENTED BUT UNTESTED IN A VM**. Administratively protected mode changes
require the Trusted UI capability path; the full prompt is **SCAFFOLDED**.
