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

### Popup redraw correction

The destination edit changed both the File Navigator state and the shared input
hash. The presenter rejected the navigator's bounded repaint when that hash
changed, then repainted only the AI widget column. The navigator now retains
ownership of that repaint. The installed test compares actual pixels where the
old menu overlapped the destination popup, in addition to exercising validation,
copy, move, collision rejection and cancellation.

The patched installed ARM run passed (`20261007T125039363790Z-29367.json`),
and `builds/evidence/clipboard-browser/fixed-file-paste-to.png` was visually
reviewed. This resolves the popup defect above; this particular run updated a
disposable installed kernel and is not fresh-ISO parity evidence.

Explicit browser navigation/reload also replaces a failed document without
waiting for its old completion callback. The existing renderer-recovery path
already did this for renderer failures. Tab identity and healthy neighboring
tabs remain intact; the replaced failed document's history is not retained.

### Software gradient correction

The Yahoo native retest reached parsed article headings, then aborted in SWGL
shader binding. A minimal linear-gradient page reproduced the same abort without
networking. Servo enabled dithering, but the locked SWGL shader set does not
provide dithered gradient variants. Native renderer preparation now selects
non-dithered gradients, leaving the host GPU configuration unchanged.

The before fixture (`20261007T130708916146Z-30279.json`) failed. After rebuilding
the native engine, linear and radial interpolation pixels both passed
(`20261007T131004998366Z-34282.json`). This is native rendering evidence, not
by itself a Yahoo or installed-browser acceptance result.

Yahoo's follow-up (`20261007T131029172159Z-34307.json`) produced a real frame
without the gradient assertion. It still failed DOM/load acceptance and timed
out during cleanup; the captured page remains partially populated. Yahoo is
therefore **not** counted as a compatibility pass. Google post-challenge results
and full DuckDuckGo search remain separate open gates.

### Corrected media and installed acceptance

- Focused ARM and x86_64 ISO builds passed:
  `20261007T131728591911Z-34557.json` and
  `20261007T133931718722Z-44229.json`. These are dependency-aware focused builds,
  not a clean full-build claim.
- A fresh installation from the ARM QEMU boot media passed the clipboard suite
  with the ISO detached, including the popup pixel assertion:
  `20261007T135734400332Z-53488.json`. Its unmodified installed-kernel hash and
  clipboard receipt are retained in
  `build/browser-installed-1791381454497296000/clipboard-result.json`.
- A subsequent detached boot of that same disk, without a kernel update, passed
  real HTTPS linear/radial gradient pixels and another network request on reload:
  `20261007T140627679235Z-53627.json`. The reused-disk receipt does not repeat the
  fresh-install parity claim. Both popup and gradient screenshots were reviewed.
- Native combined progressive-paint, clipboard, gradient, and failure-recovery
  fixtures passed: `20261007T131443544797Z-34443.json`.
- Generated design-kit payload and embedded sprite bytes matched in the ARM,
  ARM QEMU, and x86_64 installed kernels:
  `20261007T141020336573Z-53670.json`.

The downloadable ARM VirtualBox path and installed x86_64 runtime were not
exercised in this correction pass. Yahoo still fails full-page acceptance;
Google post-challenge results, full DuckDuckGo, and OS-wide busy-cursor coverage
remain incomplete.
