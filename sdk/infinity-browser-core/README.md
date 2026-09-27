# Infinity Browser core — not yet integrated

Allocation-free lifecycle and bounded input/download staging for the future
native shell. This is not Servo, a network client, a permission grant, or a
browser implementation. No launcher entry or installed browser is advertised.

The generated Sapphire/Titanium reference and unique globe/infinity icon are
specified in `docs/infinity-browser-design.md`. `skin.rs` owns interaction colors
and navigation-atlas cell geometry; `layout.rs` owns disjoint chrome hit targets.
These are shell building blocks, not a renderer. Native painting and screenshot
comparison, functional controls, default URL registration and packaging remain
required before the browser can ship.

The shell must serialize Session access on its event loop. Navigation IDs label
all callbacks; supersession/close invalidates older results. The caller must
also cancel the corresponding engine/network work and release its resources.
Rejecting stale results does not itself interrupt a worker. Drive `tick` from
the monotonic clock and drain a bounded number of inputs per UI iteration.
Backpressure is explicit: the caller must not discard key releases on Full.

Viewport coordinates use compositor pixels; a Servo device-pixel ratio conversion
must be applied consistently by the eventual renderer adapter.

Download staging must be allocated off the UI stack with a service-governed
capacity. Validate a response-derived basename and normalized media type, strip
MIME parameters first, and never decode/reinterpret the accepted name as a path.
After successful finish, revalidate native ObjectWrite/namespace authority and
commit through the object service with collision-safe naming. Report completion
only after a durable object/namespace commit, not after Download::finish.
Native storage wiring and metadata persistence remain unimplemented.

Focused behavioral tests (no engine simulation):
`./build-kit run cargo test --manifest-path sdk/infinity-browser-core/Cargo.toml`
