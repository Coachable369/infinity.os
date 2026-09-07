# Node Discovery

Discovery accepts bounded, signed, leased advertisements containing stable identity, public key, protocol range, service bits, sequence, and expiry. Validation runs before state changes. Discovery creates `Untrusted` state only; it grants no trust, membership, session, or authority.

Status: signed fixture discovery, tamper rejection, expiry, bounds, and untrusted-by-default behavior are **TESTED**. Native network advertisement and two-machine discovery are **SCAFFOLDED**.
