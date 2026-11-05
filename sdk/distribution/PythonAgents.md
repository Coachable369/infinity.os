# Python application agent instructions

Read `/InfinitySDK/AGENTS.md` first. Start from a relevant Python entry in
`/InfinitySDK/Examples/catalog.json`; copy it into the user's workspace.

- Use the native `infinity` module for UIKit and AI actions. Read its reference
  before choosing controls. Prefer bundled standard-library modules; do not
  assume desktop CPython packages or host OS commands exist.
- The counter demonstrates `window`, `run(render, handle)`, `state`, `choice`
  and `publish`. Share handlers between click and AI; return true only after
  applying a valid action. Do not block the dispatch loop with long work.
- Validate syntax before execution and keep the editor's source authoritative.
  Preserve tracebacks for diagnosis, but never leak secrets to logs.
- Use `infinity_log` for structured diagnostics. `infinity_network.connect`
  requires an explicit network grant and verified TLS for HTTPS. Close sockets
  on every path. The network example is a bounded HEAD demonstration, not a
  complete HTTP client or browser.
- Audio tone generation produces a WAV file; it does not demonstrate native
  microphone capture or playback. Document this distinction in derived apps.
- Test behavior, not generated prose. Never claim an app runs merely because
  imports or syntax validation passed.
