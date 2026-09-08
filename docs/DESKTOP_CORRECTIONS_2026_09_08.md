# Desktop correction status

## Source fixes and behavioral checks

1. Scrollbar thumbs share their track's horizontal grab tolerance. Launcher, Settings and Text Editor track clicks position the thumb and begin a captured drag; all three consume final release coordinates before releasing capture. Geometry tests cover increasing/decreasing offsets and padded thumb hits.
2. Resize grips take precedence over overlapping chat-widget click handlers. Native, Navigator and Settings resizing consume release coordinates. Tests cover the complete lower-right grip at both UI scales and two-axis size changes.
3. Boot no longer starts a File Navigator task or defaults its window to visible. Login recreates tasks for saved visible Navigator, Text Editor, Command Window and Task Manager windows; unlocking does not duplicate running tasks. Existing per-user geometry persistence remains in use. This does not add persistence for unsaved editor contents, multiple Navigator paths, or a separate minimized-versus-closed application record.
4. Adapter rediscovery updates link state. The Network panel distinguishes disabled adapters, offline policy, unknown/disconnected links and links without addresses. Changes to observed network state invalidate the Settings surface without requiring a full-desktop repaint. This does not fabricate connectivity when the device/transport has not established it.
5. DNS Apply commits an active field before switching controls and persists the configuration. Resolver status uses the actual enabled flag, and server changes invalidate cached answers.

## Remaining acceptance gaps

- Live drag, resize, session reboot and network visual tests have not run against this build: the VirtualBox session locked during reproduction and must be unlocked before continuing.
- Outbound DNS is not implemented in the current resolver. It accepts hashed names and resolves local/cache entries; cache misses return `ResolverUnavailable`. Real DNS needs hostname query encoding, asynchronous transport, response validation and timeout handling. The current Settings fixes do not supply that implementation.
- The native network packet pump is x86-64 E1000-only. An ARM64 VirtualBox-discovered firmware adapter is not proof of a working guest packet transport, DHCP lease, route or DNS service.

Checks: launcher/input regressions, resize geometry, per-user session round-trip and task restoration, typed network transitions and resolver cache invalidation. Builds and artifact parity are separate from live VM proof.
