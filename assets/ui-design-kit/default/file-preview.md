# Native file preview

Reuse the existing compact application menu, native Inter labels, OS primary/accent
colors, eight-pixel corners and image aspect-fit recipe. No new artwork or icon
family is needed. Preview must never launch file associations or edit a source.

The centered quick-look sheet has a 48px title region, 20px content inset,
24px text rows and a separate 8px scroll rail. Close uses the existing fine X
glyph. Title drag moves the sheet; left/right/bottom edges resize it. Restore the
union of old/new bounds on movement and dismissal. Escape closes the sheet.

States: text, bitmap, PNG, folder listing, unavailable/denied, oversized, and
unsupported. Never display unsupported binary content as text. All previews are
session-owned and clear on lock/logout. Respect existing Personal Space ownership.

Initial supported formats: validated 24/32-bit BMP; noninterlaced 8-bit grayscale,
RGB, grayscale-alpha and RGBA PNG; UTF-8 text including source, Markdown, CSV and
JSON (plain-text preview). Files and decoded PNG scanlines are bounded to 8 MiB.
JPEG, palette/interlaced PNG, PDF and Office document rendering are not implemented.
These formats show an explicit unsupported state, not a fabricated thumbnail.

Entry points: File Navigator object context menu → Preview; shell `preview <path>`
(relative to the current namespace, quoted names accepted); desktop file and
namespace-reference context menu → Preview. Up to sixteen authorized regular
files in the existing `/home/default/desktop` namespace appear below the built-in
collection row. This projection refreshes on desktop entry and desktop context
interaction; it does not introduce a filesystem watcher or new file-management UI.

Host validation: focused decoding, rejection, wrapping, scrolling, authorization
and teardown tests; File Navigator workspace/menu tests; existing InfinityUI
regressions. `tools/file-preview-proof.rs` calls the production sheet painter
using a host framebuffer adapter and authored Inter atlases; its rendered review
is not installed-VM proof. Use `./build-kit file-preview-tests` for focused tests.
ARM64 and x86_64 checks and a fresh release remain separate from installed desktop
interaction acceptance. No installed-runtime verification is claimed here.
