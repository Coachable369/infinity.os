# Active desktop performance audit — 2026-09-07

Scope: the installed native DisplayDevice/system_ui_present path, not only the
separate SoftwareCompositor. Runtime guest performance is not signed off.

## Ordered checks

1. Cursor-only movement: unchanged desktop state restores the saved cursor pixels,
   saves/draws the new cursor, and presents dirty rectangles. The executable redraw
   policy test passes for this condition. This is not a guest event trace; focus,
   caret/content changes or other state changes can enter additional repaint paths.
2. Window movement: FAIL. bounded_scene_geometry_change calls system_ui_frame for
   each damage clip. Application content is rebuilt; persistent application surface
   composition is not wired into this active path.
3. Back buffer: static PRIMARY_BACK_BUFFER persists between frames. Above
   stride*height=2560*1600 pixels, initialization falls back to direct front-buffer
   rendering. This is a separate unsupported-resolution/flicker risk.
4. Damage percentage: guest measurement outstanding. At scale 2 a saved cursor is
   at most 56x56 pixels; two separate cursor bounds total 0.153125% of 2560x1600.
   Touching bounds may become a larger enclosing rectangle. Clock/caret/hover work
   must be classified separately. These are geometric bounds, not measured frames.
5. Damage expansion: capacity is eight rectangles; overflow collapses to an
   enclosing rectangle. Settings also clears/replaces nested render clips instead
   of consistently restoring the caller clip. Bounded scene reconstruction is
   therefore not a guarantee that subsequent painting stays inside that damage.
6. Paint work: glyph atlases are prebuilt, but glyph resampling/blending, kerning,
   text measurement/wrapping, bitmap scaling and software glass blending remain
   in paint. File Navigator calls namespace_child_count and repeated
   namespace_child_nth_sorted during rendering. No AI icon generation runs there.
7. Synchrony: these namespace operations are synchronous in-memory scans, not
   evidence of blocking IOP/disk reads. Drag release checkpoints layout through
   storage synchronously. Full service/event-loop timing remains unmeasured.
8. Presentation: only dirty rows are copied when back-buffered. Original global
   memcpy/memmove/memset were volatile byte loops; word batching is now implemented.
   Host throughput does not establish guest framebuffer memory throughput.
9. Instrumentation: current frame publisher uses fixed storage and a single
   nonwaiting compare_exchange; contention drops a sample. No per-frame formatted
   serial logging was found in this presenter/telemetry path. Pixel-level damage
   bookkeeping still costs CPU in glyph/bitmap paths.
10. Blending: substantial software recomposition remains during drag. Persistent
    app surfaces and cached unchanged translucent layers are still required.

## Verified local changes, not end-to-end acceptance

- Native-word memory loops preserve volatile access and overlap semantics.
  Exhaustive offset/length tests compare full buffers including sentinel bytes.
- Actual production primitive fixture compares clipped results against full-render
  pixels in both RGB/BGR formats, including untouched pixels outside the clip.
- 16,384,000-byte memcpy fixture: previous average 6.31 ms / p95 6.74 ms;
  latest average 1.69 ms / p95 1.89 ms (60 host samples, opt-level=z).
- Production painter fixture, 900-pixel clip: previous average about 33 ms;
  latest 6.19–6.36 ms. 32-pixel clip: previous about 26 ms; latest 7.9–8.5 us.
  Its damage hook counts submissions; it does not simulate production merge cost.
- ARM64 release installed/live kernels and ISOs build successfully.
- No claim that the running VM uses these changes. Its original disk remains
  unchanged. Backup: builds/backups/infinityos-4-before-performance-20260907.vdi.

Next acceptance must first trace actual cursor-event damage, then replace active
drag repainting with persistent surfaces. Do not treat the host improvements above
as completion of the responsive-desktop milestone.
