# Browser page scrollbars

The pinned Servo engine implements scrolling but does not supply scrollbar
painting or a preference that enables it. InfinityOS supplies root-page rails
inside the native Servo painter. No DOM nodes, page styles, browser chrome
overlays, or host window services are used.

Reuse the existing compact application/file-preview rail vocabulary: a quiet
navy track, proportional eight-pixel thumb, and Precision blue interaction
states. No new artwork or outer panel recipe is needed.

- Track: 14 logical pixels, always visible while that axis has input-scrollable
  root overflow. Page zoom does not magnify it; native HiDPI scaling does.
- Root `scrollbar-width:none` suppresses the rails; `thin` uses an eight-pixel
  track with a four-pixel thumb. These choices come from computed layout style,
  not injected scripts. Default `auto` uses the normal kit geometry.
- Thumb: eight pixels wide, three-pixel cross-axis inset, four-pixel rounded
  ends, proportional length with a 24-pixel minimum.
- Colors: Precision surface #101B3A, neutral thumb #6C89AE, hover #86C7FF,
  pressed #3E6FFF. These are engine content controls, not new window borders.
- Both rails leave the bottom-right 14-pixel corner clear. Tracks and thumbs
  remain inside the engine viewport; no root rail appears for a fitting page
  or an axis whose scroll tree disallows input scrolling.
- Thumb drag preserves its initial grab position and clamps to both endpoints.
  Track clicks move 90 percent of the visible page. The captured release is
  consumed even outside the track. Leaving the viewport cancels capture.
- Layout, script scroll, wheel, keyboard scroll, and page zoom update the same
  authoritative tree used by painting. Returning to zero explicitly updates
  WebRender's retained offsets.
- Only root rail gestures are intercepted before document hit testing. Existing
  page controls and nested scroll containers retain Servo's native dispatch;
  this implementation does not introduce nested-container rails.

`sdk/infinity-browser-core/scrollbars.rs` owns geometry and capture policy.
`tools/servo-platform-probe/native-scrollbars.rs` integrates with Servo's tree
and production painter. Software-renderer staging composes these hooks with
its native rasterizer options rather than overwriting either transform.

Behavioral proof: `tools/browser-scrollbars-test.rs` and the real-engine
`engine-scrollbars.rs` fixture. The latter verifies persistent pixels, track
paging, bottom-to-top pixels and DOM offsets, script/wheel scrolling, nested
input, zoom, resize and removal of overflow. Guest proof does not establish
fresh-installed desktop acceptance or nested-container scrollbar support.
