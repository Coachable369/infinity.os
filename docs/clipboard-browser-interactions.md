# Clipboard and browser interactions

## Acceptance scope

- Both Command keys and Control select all in native editing and the browser.
- Text context actions preserve selection and invoke real copy/cut/paste,
  select-all, clipboard inspection, and Google search.
- File Paste To uses the native destination popup, validates a Personal Space
  folder, refuses collisions, preserves a staged cut on failure, and closes only
  after a successful transfer or explicit cancellation.
- Streaming paints before response completion. Reload performs another request
  and immediately publishes loading state.
- Magnifiers use generated theme role 27. The browser no longer draws its own.
- Google joins the configurable search providers and is the new-profile default;
  existing provider records keep their meaning.
- A generated sixteen-frame infinity ribbon animates at 80ms/frame during browser
  loading, including reload, without invalidating the application surface.

## Security and packaging

Native menus reuse the session clipboard, expiry, lock clearing and browser
gesture grants. The clipboard viewer never writes a document or persists data.
Selection search does not modify the clipboard and excludes password controls.
Queries are bounded and percent encoded. Website challenges are not bypassed.
The generated cursor master and RGBA sheet join the existing design-kit payload
list; both kernels embed the same runtime sprite.

## Verification

`tools/clipboard-cursor-test.sh` tests menu bounds/hits, damage restoration state,
viewer erasure, frame timing and sixteen distinct visible cursor frames.
Browser-core tests cover all four provider records and literal selection queries.
The native engine fixture verifies early pixels before EOF, another request on
reload, clipboard editing, and failure recovery. Installed acceptance is exposed
through `run-desktop-installed.py --clipboard-ui`, including both Command keys,
context-menu mutations and Paste To copy/move/collision behavior.

Native fixture success is not a claim that every external website renders.
Yahoo/DuckDuckGo compatibility and Google challenge/search results require
separate real-site verification. ISO builds and installed-runtime results are
recorded separately in build-kit manifests.

### October 7 verification

- Browser core: 48 tests passed. Native engine fixtures passed progressive
  pixels, reload requests, selection operations, and network/renderer recovery.
- Input regression and launcher interaction suites passed after correcting the
  ninth file-context row's geometry and hit target.
- Focused aarch64 and x86_64 ISO profiles passed. The earlier incremental run
  failed at that ninth-row test; it is not recorded as a passing full pipeline.
- A fresh aarch64 QEMU/HVF installation passed media-detached clipboard tests,
  both Command keys, Control+A, destination validation, copy/move, and collisions.
  Receipt: `build/browser-installed-1791353749755250000/result.json`.
- Visual acceptance is **not complete**: the old file context menu obscures the
  Paste To destination popup despite correct editing and transfer state. Text
  context menus and the clipboard viewer render correctly in captured frames.
- No installed x86_64 runtime acceptance or VirtualBox acceptance is claimed.
  The wait cursor currently follows browser loading, not every OS operation.
  Real-site Google post-challenge results, Yahoo and full DuckDuckGo remain open.
