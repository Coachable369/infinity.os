# Meet InfinityOS

Static public website for `https://meetinfinityos.com/`. No application framework,
package installation, API key, tracking, cookies, or build step is required.
Serve the directory over HTTP for the self-hosted Three.js scene and ES modules.
Opening `index.html` through `file://` shows the static fallback, not the 3D scene.

Local preview: `python3 -m http.server 4178 --bind 127.0.0.1 --directory .`
Then open `http://127.0.0.1:4178/`.

## Design

Start with `design/continuum.md` and `design/IDesign-Kit.png`.
The authoritative symbol is `../assets/boot/infinity-emblem-v2.bmp`.
The same-named PNG is DIFFERENT artwork and is not a valid substitute.
Header, small marks and fallback use pixels decoded from the BMP itself.
The rotating model is a beveled 3D relief traced from that emblem's outer
silhouette and two openings, with the original artwork mapped to its faces.
It is not a generic tube, torus knot, or an original source 3D model.
The baked floor reflection is excluded from the mesh. Its unseen reverse and
edge depth are reconstructed. Tool icons are Lucide; licenses are included.

The lossless 1672x941 web emblem is pixel-identical to `emblem-reference.png`.
Generated art supplies the design board and secondary macro environment only.
Three.js r180 is pinned and self-hosted, with one geometry helper whose import
path is adjusted for local hosting. The user approved frameworks on 2026-10-20.
The scene caps its framebuffer at 1.6 million pixels and suspends offscreen.
Inter is self-hosted. No external runtime CDN requests are made.

## Verification

Run `./build-kit run npm --prefix meetinfinityos.com test` from the repository
root (no installation needed). Eight tests exercise production motion
state, preference persistence, restricted storage, source-artifact provenance,
mesh depth and UV mapping, contour area, scroll pose, electrical routes, and
shared navigation including keyboard dismissal and focus restoration.

`verification/browser-results.json` records actual browser state and pixel checks.
Desktop and mobile screenshots are visual evidence, not textual test oracles.
The experience section contains actual OS screenshots, not a live OS session.

## InfinityOS in Action

The original two-column experience section and Voice/Create/Explore previews
remain intact. The left selector now also includes a Screenshots group:
Sign-in, Desktop & AI, and App library. Each selection displays its actual
October 28, 2026 capture, factual description and full-resolution link in the
same right-hand preview. Actual screenshots and illustrative scenes have
distinct captions. Images are uncropped and unretouched.

To add another screenshot, append a `.screenshot-tab` to `.feature-selector`
and a matching `.screenshot-scene` inside `#feature-panel`. Match the button's
`data-feature` to the scene's `data-screenshot-scene`, supply unique accessible
IDs, and include a heading, description, responsive image and full-resolution
link. The controller discovers these entries without new mode-specific code.
Add related detail subsections within that scene and a fallback link in
`.screenshot-fallback`. Export 960px/1600px WebP derivatives and a lossless
full-size WebP to `assets/screenshots/`; add exact files to `deploy.sh`.

`tests/experience-browser.cjs` uses authoring-only Playwright and Sharp.
Run through `./build-kit run node meetinfinityos.com/tests/experience-browser.cjs`
with those dependencies on `NODE_PATH`. The check owns and closes a localhost
preview on port 4178, exercises all six selections, keyboard navigation,
full-resolution links, image geometry, five viewport widths, mobile navigation,
motion controls and no-script fallback links. Evidence is written to
`verification/experience/`. Set `EXPERIENCE_URL=https://meetinfinityos.com/`
to run against production instead.

## Whitepapers

The shared top navigation links to `whitepaper.html`. Its public knowledge
library has four explicitly requested forthcoming placeholders. No PDF files,
download links, dates, sizes, or titles have been invented. The existing public
design-direction overview remains below it with linked contents, responsive
reading typography, and a print stylesheet. It reuses the kit, works without
WebGL, and remains readable with JavaScript disabled. `navigation.js` owns the
same mobile-menu behavior across the site; it does not load the homepage scene.

When a paper is ready, add the approved public PDF under `publications/`, replace
one `.publication-entry` with its real title, summary, version/date, page count,
file size and a named `<a download>` link. Add the exact PDF path to `deploy.sh`'s
allowlist and staging/publish steps; no private or draft documents belong there.
Verify the downloaded file and its `application/pdf` response before publishing.

## Investors

`invest.html` is linked from the top menu on all three pages. It reuses the
reading-page stylesheet, shared navigation, and original emblem. Its investment
thesis covers workflow value, category context, commercial hypotheses, proposed
diligence milestones, competitive risks and development status. Official Apple
and Microsoft references support category context, not InfinityOS traction.
Working links connect the library and product experience.
It does not collect investment commitments, publish financial terms, or invent
a contact address. Deployment includes this page in the runtime allowlist.

## Deployment

Upload only the runtime HTML, CSS, JavaScript modules and allowlisted assets to
`/var/www/meetinfinityos.com/public_html` on the existing Apache host.
Do not upload tests, design boards, verification files, or credentials.
Preserve a server-side backup before replacing the published files.
No DNS, Apache-vhost, TLS, database, or OS-image change is needed.
`bash deploy.sh` performs this allowlisted upload, keeps a timestamped server
backup, publishes the HTML last, and checks both HTTPS hostnames. Credentials
are read from the existing deployment environment and are never stored here.

This standalone website is not an installed InfinityOS component and requires
no ISO rebuild. Run bounded verification through the repository build kit;
never disturb a running native build or its lock.

## Reproducing The Contours

`design/extract-emblem.cjs` uses Sharp as an authoring dependency to derive
`assets/emblem-outline.json` from `design/emblem-reference.png`. It extracts the
largest alpha component above the floor reflection, retains both openings,
and smooths raster edge noise. The current contour area differs from its
extracted mask by less than 0.2%; tests enforce a 0.5% bound.
Sharp is not shipped to the browser or required for preview/testing.
