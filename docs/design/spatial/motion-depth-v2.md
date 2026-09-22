# Spatial desktop: motion and depth acceptance

The September 22 installed build opens Spatial Desktop without the reported
checkpoint-key panic and can launch File Navigator from its overview. That is
functional smoke evidence, not completion of the five spatial workflows.

The current uniform glass-card presentation fails the requested visual standard.
`idesign-motion-depth-v2.png` is the revised AI-generated reference board, not a
runtime asset, screenshot, or proof of implementation. Its fictional documents,
counts and workspace names must not be rendered as fabricated user content.

## Required native presentation

| Workflow | Visual and interaction acceptance |
| --- | --- |
| Holographic Workspace | Large retained window previews in a depth arrangement; selected window advances while neighbors recede; exact instance activation; real app content, no screenshot stand-ins. |
| Worldshift | Readable scene previews on a luminous trajectory; clear current destination; continuous departure/arrival transition; cancellation and unsaved-work preservation. |
| Gravity Well | Visible collection nucleus and elliptical rails; real draggable references; destination attraction and settling; explicit confirmation before filesystem operations. |
| Matter Shelf | Compact bottom ribbon; lifted dragged object, clear insertion target and settling; keyboard parity; content remains visible rather than buried in another dialog. |
| Constellations | Spacious positioned graph with readable node labels, luminous connections and bounded pulse effects; direct linking/unlinking and stale-source handling. |

## Performance and verification gates

- Native geometry, selected installed icon pack and installed fonts; no new UI framework.
- Motion communicates selection, movement or state. No perpetual full-screen repaint.
- Cache static lighting/backdrops. Reuse application surfaces. Bound effect damage.
- Elapsed-time animation, reduced-motion parity, immediate cancellation.
- Empty states must be intentional and useful, without tiny icons in oversized blank cards.
- Capture installed before/after screenshots and verify actual interactions and persistence.
- Record frame timing during transitions and pointer handling; do not infer performance from compilation.
- Fresh-install ISO and installed-System-Generation parity remain mandatory.

## Current QA environment

Fresh install on `infinityos-4-fresh-spatial-qa.vdi`, ISO detached, account setup
completed. VirtualBox USB tablet replaced captured relative USB mouse for reliable
absolute GUI test input. Login, top-bar search menu, spatial opening, File Navigator
launch and its retained overview preview were observed. Complete workflow and
visual/performance acceptance are still pending. A stale switching notice was also
observed when reopening the overview and requires correction.

## Reference generation

Built-in image generation, ui-mockup. Prompt: professional InfinityOS Spatial
IDesign Kit with five distinct native spatial views, midnight navy glass,
ice-cyan illumination, readable warm-white typography, crystalline icons,
large layered live-window previews, scene trajectory, orbital collection,
compact drag-out ribbon and relationship graph. Include hover/focus/drag/
confirmation/reduced-motion states and anticipation-transition-settle storyboard.
Avoid ordinary form panels, decorative HUD clutter and fabricated runtime content.

## September 22 native motion pass

Implemented depth-staged retained previews with a filmstrip, staggered Worldshift
cards with real saved-layout miniatures, orbital Gravity rails, circular
Constellation nodes with curved links, and a compact Matter Shelf. Large icons
use the installed high-resolution launcher atlas and correct center coordinates.
The desktop stage is captured, dimmed and defocused once per scene refresh;
damaged rows reuse this cache. The additional stage reserves 31.6 MiB of BSS.

Scene reveals use finite 240 ms elapsed-time tracks. Collection drops use a
180 ms damped-spring keyframe track with bounded overshoot and exact rest.
Reduced motion bypasses transitions. No perpetual orbital repaint is scheduled.
Text editing damages its input region instead of repainting the whole stage.

Behavioral tests cover spring endpoints/overshoot/reversal, clipped stage pixels,
retained previews, state serialization and final drag coordinates when held
motion samples are coalesced away. The controller now applies release coordinates
before committing a graph placement.

Installed disk-only observations on the motion-depth QA clones: login, opening
all five views, retained File Navigator preview, saving a world, creating text
clippings, inserting a clipping into Text Editor, and connecting two real nodes.
The `Orbit` clipping survived a power-off/reboot and installed-kernel update.
An earlier host diagnostic command aborted VirtualBox; that run is not valid
evidence of a native persistence defect.

Remaining acceptance must not be inferred from these observations: reliable
pointer dragging after the release-coordinate fix, complete Worldshift unsaved
document round-trip, confirmed/cancelled cross-app drops, and measured installed
animation frame times. Pointer clicks were intermittently missed during GUI QA;
the raw-tablet mirror arbitration regression passes on the host, but complete
installed input acceptance remains open. This document is not a completion claim.

The sixth motion-depth test disk booted and signed in with the provisioning
default xHCI USB mouse/keyboard profile and no optical media. The final graph
release-coordinate fix is compiled and host-tested, but installed dragging could
not be verified reliably. Ctrl+Shift+K also failed to open Spatial Desktop through
the available VM control path. Further speculative input rewrites stopped at the
six-loop correction limit. These are remaining acceptance failures, not requests
for another password or manual setup.

Release build: `sh build.sh` completed with exit status 0 on September 22 at
17:44 America/Chicago. It regenerated `builds/InfinityOS-aarch64.iso` (8.0 GiB),
`builds/InfinityOS-x86_64.iso` (1.7 GiB), the provisioning model ISO, and
`builds/SHA256SUMS`. The build's ARM-copy binary equality and boot-payload
extraction checks passed. Temporary build evidence:
`/tmp/spatial-motion-final-build.log`. This successful build does not close the
installed interaction/performance failures listed above.
