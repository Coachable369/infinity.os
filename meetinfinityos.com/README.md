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

Run `npm test` (no installation needed). Eight tests exercise production motion
state, preference persistence, restricted storage, source-artifact provenance,
mesh depth and UV mapping, contour area, scroll pose, electrical routes, and
shared navigation including keyboard dismissal and focus restoration.

`verification/browser-results.json` records actual browser state and pixel checks.
Desktop and mobile screenshots are visual evidence, not textual test oracles.
Interactive preview content is explicitly illustrative; it is not a live OS session.

## Whitepaper

The shared top navigation links to `whitepaper.html`. It presents a public
design-direction overview with linked contents, responsive reading typography,
and a print stylesheet. It reuses the existing kit and emblem, works without
WebGL, and remains readable with JavaScript disabled. `navigation.js` owns the
same mobile-menu behavior on both pages; it does not load the homepage scene.

## Deployment

Upload only the runtime HTML, CSS, JavaScript modules and allowlisted assets to
`/var/www/meetinfinityos.com/public_html` on the existing Apache host.
Do not upload tests, design boards, verification files, or credentials.
Preserve a server-side backup before replacing the published files.
No DNS, Apache-vhost, TLS, database, or OS-image change is needed.
`bash deploy.sh` performs this allowlisted upload, keeps a timestamped server
backup, publishes the HTML last, and checks both HTTPS hostnames. Credentials
are read from the existing deployment environment and are never stored here.

This standalone website is not an installed InfinityOS component. The user
explicitly exempted website work from the native build kit on 2026-10-20;
the running native build and its lock must remain untouched.

## Reproducing The Contours

`design/extract-emblem.cjs` uses Sharp as an authoring dependency to derive
`assets/emblem-outline.json` from `design/emblem-reference.png`. It extracts the
largest alpha component above the floor reflection, retains both openings,
and smooths raster edge noise. The current contour area differs from its
extracted mask by less than 0.2%; tests enforce a 0.5% bound.
Sharp is not shipped to the browser or required for preview/testing.
