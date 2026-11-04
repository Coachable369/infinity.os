# InfinityOS transport fork

Source: published `sunset` 0.6.0, upstream commit
`cb7fbe2c7ba0258a2e9c853668495cc6fc93d502`, 0BSD license retained.
Published archive SHA256:
`3b52312b804ac95f10d3963f6ef5889117ad678bd6ce977be4aa4cf491bad78f`.

Local changes:
- Explicit application completion queues EOF and close before releasing the channel handle; releasing a handle alone does not complete the SSH wire protocol.
- Keep pending protocol events ahead of subsequent input so event cleanup cannot discard the first channel request after authentication.
- Expose existing decoded PTY, resize, interrupt and exit-status messages to the native Console adapter; no host terminal or process dependencies.
- Retain `deny(unsafe_code)` globally, with a single native-only exception for the checked entropy FFI call.
- Advertise only `chacha20-poly1305@openssh.com`; no AES-CTR fallback.
- Native targets call the service-owned `infinity_ssh_random` adapter. No
  getrandom/host OS dependency on `target_os = "none"`.
- Ed25519 generation and X25519 exchange propagate entropy failure instead of
  using an RNG wrapper that panics. The X25519 secret is still taken once from
  its `Option` and zeroized on drop. Temporary seeds use `Zeroizing`.

Integration disables default/alloc/std/RSA/ECDSA features. Crypto wire formats,
signature verification, key derivation, and AEAD remain upstream implementations.
The fixed-capacity `larger` feature accommodates InfinityOS account handles;
it does not enable allocation or a host runtime.
