# Authentication

Status: **TESTED** in the Milestone 7 host acceptance suite.

Passwords are accepted only through bounded private input and are immediately
derived into a salted PBKDF2-HMAC-SHA256 verifier. Plaintext is never retained
or serialized. Credential projections expose only stable ID, owner, type,
state, creation time, and last-use time; salt and verifier remain private.

Verification uses constant-time comparison. Three failed attempts activate
bounded exponential backoff. The human-facing error deliberately does not
distinguish a missing account from a bad secret. Revocation zeroes verifier
material and takes effect without deleting the user.

The present PBKDF iteration count is a bring-up compromise chosen for a
freestanding kernel. A future hardware-backed credential provider can replace
the derivation implementation behind the same typed authentication contract.
