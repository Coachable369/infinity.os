# Explicit durable Pool authority

`node capability-grant node:<peer> name=pool-metadata durable=true confirm=true`
approves one exact peer/operation/scope until explicitly revoked. Omit `seconds`:
finite and durable lifetimes cannot be combined. The allowlist is resource
advertisement, replica inspect/delete/transfer begin/chunk/commit, object read,
and Pool metadata. General node administration cannot receive this authority.

Approval IDs have bit 63 set and occupy a separate 32-entry table. They are not
live grants or sessions. Canonical node-state version 4 stores the table inside
the existing 12288-byte atomic journal; older formats load without approvals.
Commit failure does not publish authority. Revocation retains a tombstone;
trust revocation/blocking also invalidate approvals. Fresh authenticated
transport, current trust, exact operation/scope/rights, and current policy are
still required on every remote request. No traffic keys are persisted.

Requester `pool participate`, `pool metadata-authority`, and
`pool retire-authority` additionally require `durable=true confirm=true` with
no `lease`; `pool advertise` requires the same explicit choice. Only tagged
peer approvals can back these configurations. Finite local uptime leases are
discarded on cold configuration load, never silently renewed across reboot.

`pool share ... durable=true confirm=true` explicitly authorizes durable reader
and repair delegation, requiring durable metadata authority for every configured
peer. Persistent finite sharing is UNSUPPORTED until a restart-stable trusted
clock exists; historical finite bundles fail closed at native/runtime admission.
Finite nonpersistent cryptographic primitives remain available for stable-clock
callers. Successor publication preserves the existing signed delegation deadline
rather than extending it during mutations.

Grant issuance acceptance must use the exact local typed node completion,
not a maximum grant ID or a remote-operation completion. The dedicated local
completion diagnostic is separate from the existing remote-operation snapshot.
Diagnostic 87 remains exclusively the legacy ephemeral grant projection.

Host tests exercise cold restore with zero live grants/sessions, exact scope
denial, permanent revocation, atomic save failure and unsupported operation
denial. Installed cold-boot distribution acceptance requires a freshly installed
generation and explicit durable configuration; old ephemeral approvals are
never automatically upgraded.
