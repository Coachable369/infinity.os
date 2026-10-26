# Native facial sign-in

Facial sign-in is enabled by default per account, but requires a verified,
encrypted enrollment. Password sign-in remains available. Privacy & Security's
Facial Recognition Sign-in row toggles the persisted preference; selecting the
row changes its On/Off value. Existing accounts default to On after migration.

On the login or lock screen, select the account and press Facial Recognition.
With microphone permission enabled, “Hello Infinity” or “Infinity Hello” starts
the same comparison. The pre-session speech path accepts only these greetings:
it never submits commands to the LLM, opens applications, or authenticates by
voice. Leaving the screen or selecting another account revokes its authority.

The fixed native face worker receives only the selected account's authenticated
vault payload and fixed model pair. It must compare three new, spaced, quality
checked frames against all enrolled views. Stale/out-of-order reports, early
success, camera loss, model mismatch, disabled preferences, process failure and
timeouts do not create sessions. Only a clean worker exit following completed
comparisons produces a non-cloneable, five-second, account-bound result. The
identity service consumes that result once and creates an ordinary session or
unlocks exactly the selected account's locked session. Attempts have a global
15-second admission interval. Private worker memory is scrubbed before reuse.

## Security limitation

This is experimental RGB face matching, **not liveness detection**. It is not
certified resistant to photographs, replay videos, masks or other presentation
attacks. Do not treat it as equivalent to depth/IR-backed biometric hardware.
Disable facial sign-in and use a password where that threat matters. The stored
biometrics use account/model-bound authenticated encryption and native node-key
custody; no TPM or secure-enclave protection is claimed.

## Verification boundaries

Behavioral tests cover stored-vector import, invalid vectors, three comparisons,
replay/freshness checks, expired/wrong-account proofs, protocol rejection,
greeting allowlisting and encrypted-vault tampering. Synthetic vectors are not
camera or recognition-accuracy evidence. Native ARM64/x86_64 compilation and ISO
parity do not establish installed camera, microphone or user-login acceptance.
Test each supported camera in a fresh installation without the ISO attached.
