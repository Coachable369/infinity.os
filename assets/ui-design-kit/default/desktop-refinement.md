# Desktop refinement — implementation contract

Reference: `desktop-refinement-reference-v1.png`, generated with built-in imagegen.
This is a visual reference, not evidence of implemented features. Illustrative
Mail/Cloud destinations are not authorized integrations: Drop Compass must show
only real local compatible destinations. Cross-device clipboard is out of scope.

Reuse the default kit's smoked navy, Inter atlases, rounded controls, cyan focus,
and 8px spacing rhythm. No new icon family is needed. The new reference fills the
kit's missing multi-window focus, clipboard, drag-destination and capsule recipes.
Runtime controls remain native; never bake interactive text into a bitmap.

## Accepted scope and acceptance checklist

1. Focus Lens: dim behind the active window, finite 180ms transition, reversible,
   no repeated application rasterization, no work when settled.
2. Peek Through: temporary reveal without activating or modifying the rear app;
   restore on input/timeout, never deliver clicks through the invisible window.
3. Visual Clipboard: bounded searchable text/image history, explicit clearing,
   session isolation, sensitive sources excluded, actual preview and paste.
4. Drop Compass: only compatible native targets, real drop operation, cancel,
   edge-safe geometry and no service calls in paint.
5. Workspace Capsules: named durable app/location/layout collections; optional
   association with World Shift; restore without destroying unsaved documents.
6. Sculpted chrome: shared smooth geometry and crisp functional window controls.
7. Selection light: interrupted transitions start from current visual position;
   selection semantics and hit testing update immediately.
8. Typography: authored font sizes and baselines, no scaled bitmap labels.
9. Glass menus: readable contrast, consistent gutters, submenu edge containment.
10. Input fields: focus, caret, selection, disabled and error states, no geometry
    changes on focus.
11. Layout transitions: translate/clip retained surfaces, do not stretch text.
12. Feedback: bounded progress, success/error and pressed states, no false success.

All animations honor reduced motion. No idle animation loops. Controls must work
through both keyboard and pointer where applicable. Shared ARM64/x86 code and
fresh-installed parity are required; compilation alone does not close acceptance.

## State

Design reference complete. Focus Lens and Peek Through have shared native
controller/compositor implementations. The remaining ten items are not implemented
by this change. Installed screenshots, fresh-install parity and performance
acceptance remain pending; do not describe the twelve-feature release as complete.

Focus Lens: Ctrl+Shift+F or the top-bar search/spatial menu. Peek Through:
Ctrl+Shift+O or the same menu. Peek fades for 140ms, holds until 2.5 seconds from
activation, then restores. Escape dismisses; the first pointer click or scroll
dismisses without reaching the hidden window. Other keyboard input restores first
and then goes to the original focused app. Lock clears both effects. Focus Lens
is a session-local presentation mode, not a persistent user preference.

`./build-kit desktop-effects-tests` tests transition state, security guards in
shortcut routing, reduced motion, exact premultiplied pixels, and actual retained
surface reuse with paint callbacks that panic if an animation rerasterizes an app.
It does not prove installed interaction or frame rate.

## Visual Clipboard implementation checkpoint

The native sheet now supports eight session-local text entries, full-content
case-insensitive ASCII search, previews, paging, copy/paste, individual removal
and clearing. Ctrl+Shift+V opens it. In Text Editor, “Remember selection” in the
context menu or Ctrl+Shift+C explicitly retains selected text. Ordinary copy/cut
and browser copies do not retain history: application identity alone is not a
sensitive-content classifier. Lock/session changes erase retained content.

The sheet reuses the kit's smoked surface, cyan selection, rounded 8px cards and
8px gutters. Its logical footprint is 520×350, with a 36px search field, three
56px visible history cards and 32px footer actions. Gaps are not clickable.

Behavioral coverage: `tools/clipboard-history-test.rs` exercises bounded eviction,
deduplication, filtering beyond the preview, full-size restoration, denied reads,
stale IDs, expiration and browser exclusion. `tools/clipboard-cursor-test.rs`
exercises scaled action hit regions and overlay dismissal/erasure. Run both
through `build-kit run`. Shared ARM64/x86_64 compile checks passed; this is not
installed visual proof. Image history and installed visual verification remain
open acceptance items. This checkpoint does not complete Visual Clipboard or
the twelve-feature release.

## Generation prompt

Built-in imagegen; landscape reference board titled “InfinityOS Desktop
Refinement”, six sections: Focus Lens, Peek Through, Visual Clipboard, Drop
Compass, Workspace Capsules, Component States. Midnight navy #071b2a, Inter-like
type, text #dfedf5, muted #93b6c8, cyan #36cced, restrained violet glass rims,
8px spacing rhythm. Shippable UI proportions, no icy materials or giant glows.
