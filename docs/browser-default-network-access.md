# Browser network defaults

Normal browser launch now obtains a renewable, ten-minute network capability
lease for the active signed-in session without an operator confirmation dialog.
The lease covers connect, send, receive and DNS only. Endpoint policy, TLS
validation, downloads and sensitive site permissions are unchanged.

System Settings → Network remains authoritative: Standard permits internet
access; Restricted and Offline deny browser access. The browser bridge checks
the active session and profile while pumping requests, cancels an active request
when access is withdrawn, and refuses queued requests. Existing saved profile
choices are not overwritten. Returning to Standard allows subsequent requests.

The implementation is shared by ARM64 and x86_64 and compiled into both live and
installed kernels; there is no separate live-only preference or asset.

## Verification

- `./build-kit run sh tools/network-test.sh`: passed. Behavior checks cover
  default grants, session denial, holder isolation, renewal without capability
  table growth, Restricted/Offline persistence and re-enabling Standard.
- Native release checks with `native-browser` passed for ARM64 and x86_64.
- `./build-kit run cargo test --release --manifest-path tools/behavior-harness/Cargo.toml --bin https-service-test`: 114 passed.
- New ISO packaging and cold-installed UI verification are not yet performed.

This change does not claim to resolve the separately reported tab appearance.
