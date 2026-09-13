# Native geturl status

## Beta decision — 2026-09-13

The user accepted the current implementation as sufficient **for a limited beta**
and requested that remaining work be recorded for a future return. Full curl
compatibility work is deferred, not completed; the earlier 1:1 feature-test
requirement remains the future full-release target.

The beta baseline is the installed, ISO-detached native HTTPS GET verified below
against `https://example.com/`: HTTP 200, 559 bytes, and a SHA-256 matching curl.
It requires manual network configuration and policy/session permission setup.
It is limited to an 8 KiB response and a thirty-second native deadline, with
Console presentation rather than binary stdout or file downloads.

### Resume checklist

1. Fix automatic fresh-install network readiness and integrate Ask-policy consent.
2. Add binary-safe stdout/file output, pipelines and real command exit status.
3. Implement methods, headers/bodies, redirects, authentication, cookies,
   uploads, retries, resume and configurable timeouts.
4. Inventory the target curl version's protocols/options and build a behavioral
   comparison matrix against real endpoints; one matching GET is not full parity.
5. Reinstall the final corrected ISO and verify the installed system without media.
6. Complete application/IOP/weather integration as tracked in `native-http.md`.

Resume from the verified GET baseline and inspect failed or missing acceptance
items; do not repeat passing work unless subsequent changes affect it.

## Current interface

Initial Console built-in, **not a complete curl clone or standalone executable**.
Compatibility target: [upstream curl](https://curl.se/docs/manpage.html).

```text
geturl --help
geturl --version
geturl [-fsS4] [--http1.1] https://HOST/PATH
```

Long forms `--fail`, `--silent`, `--show-error`, and `--ipv4` are accepted.
Supports one HTTPS URL on port 443. A missing path becomes `/`; fragments are
not sent. Query-only URLs lacking `/`, quoting, globbing, multiple URLs,
credentials, other ports and unimplemented flags fail explicitly.

The native actor runs cooperatively, without blocking input on network IO.
Complete responses enter bounded Console scrollback, with wrapped lines and
sanitized controls: **not binary stdout, file output, or pipelines**. Limit:
8192 response bytes and a 30-second deadline. `-s` suppresses errors, not body
data; `-S` restores error display. `-f` suppresses bodies for status >=400.

`INFINITY_GETURL_EXIT` is diagnostic state, not a shell process exit status:
pending UINT32_MAX; success 0; unsupported scheme 1; arguments 2; URL 3;
resolution 6; generic connection/service error 7; HTTP fail 22; timeout 28;
cancellation 42. Detailed TLS error mapping remains incomplete.

## Permission workflow

`geturl` only uses existing session capabilities and Network policy. A privileged
authenticated operator can run `https authorize` to inspect the temporary scope,
then `https authorize confirm=true` to approve. The protected capability-consent
surface grants missing network rights for 60 seconds, to the current session.
Existing valid grants are reused. Scope covers destinations still allowed by
Network policy, **not one hostname**; policy is never overridden. Grants are not
persistent. Revocation and locking cancel traffic; `https cancel` also cancels.

## Verification and remaining acceptance

Behavioral parser tests check flags, URL components, status mapping and invalid
inputs. The real Runtime/actor command harness checks admission, cancellation,
typed status, nonblocking presentation, denied/nonconfirmed authorization, and
successful privileged consent with an enforced sixty-second lease expiry.
Its NIC is a fixture, not successful-download or installed-system evidence.
Code is included in the installed kernel, not just the live ISO.
ARM and x86_64 installed kernels link; byte-for-byte installer kernel and loader
artifact parity passes on both architectures.

### Installed public-URL result (2026-09-13)

The isolated QEMU test completed a fresh installation, onboarding and an
ISO-detached cold boot. Initial networking was LinkOnly; static IPv4, gateway
and DNS were configured through Settings. Standard policy's Ask action also
required explicit approval in Settings, separately from the session capability
lease. These remain default-setup/usability gaps, not automatic HTTPS readiness.

Real traffic exposed a missing completion poll in the PS/2 desktop loop: the
network actor ran but Console never collected its result. Both idle and active
PS/2 paths now service background completion, matching the UEFI input path.
The rebuilt kernel was applied to the stopped disposable installed disk after
backup, with CRC/reference checks and unchanged-byte verification. The saved
VirtualBox VM was not modified. This final test used an updated installation;
it is not a claim that the corrected ISO was freshly installed again.

The subsequent installed request to `https://example.com/` passed:

| Value | curl reference | Native installed geturl |
| --- | --- | --- |
| Exit | 0 | 0 (diagnostic completion status) |
| HTTP status | 200 | 200 |
| Body bytes | 559 | 559 |
| SHA-256 | `ff67a9d764d6a2367a187734e697f6a53217db9a21c101d410a113ca871a299d` | identical |

The native client performed DNS/TCP/TLS/HTTP inside the guest using system
certificate roots, firmware time and boot entropy. Host curl is only the
independent reference. Body comparison observes the downloaded bytes, **not
binary stdout equivalence**. Evidence from this run is retained at
`/tmp/geturl-real-20260913-a/result-1789286874315566000.json` and the adjacent
packet captures; temporary evidence is not a durable repository artifact.

`tools/geturl-installed-test.py --output NEW_DIRECTORY` recreates the initial
fresh-install test. `--configure-nat` explicitly sets this harness's QEMU NAT
address/gateway/DNS and changes a fresh Standard profile from Ask to Allow via
Settings; do not repeat that policy-toggle setup on an already configured
guest. `--reuse-installed` reuses only the harness disk and matching ELF files.
The test compares structured status/length/digest, preserves failure receipts,
captures packets/screenshots, and never counts console wording as acceptance.
It deliberately reports `full_curl_parity: false` even when GET passes.

Still required: automatic fresh-install network readiness, integrated Ask-policy
consent, application/IOP/weather integration, and full curl protocol/option parity.
Missing areas include request methods/bodies/headers, redirects, authentication,
cookies, proxies, uploads/downloads, resume/retry, adjustable timeouts, binary
streams/files, quoting, pipelines and shell process exit status. Do not present
this initial implementation as fulfillment of the full compatibility request.
