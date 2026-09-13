# Native geturl status

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
artifact parity passes on both architectures. No installed download is claimed.

Still required: ISO-detached fresh-install command execution, real HTTPS output,
application/IOP/weather integration, and full curl protocol/option parity.
Missing areas include request methods/bodies/headers, redirects, authentication,
cookies, proxies, uploads/downloads, resume/retry, adjustable timeouts, binary
streams/files, quoting, pipelines and shell process exit status. Do not present
this initial implementation as fulfillment of the full compatibility request.
