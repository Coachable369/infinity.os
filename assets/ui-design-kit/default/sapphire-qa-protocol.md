# OS-wide Sapphire QA protocol

The working inventory is `sapphire-coverage.json`. Its initial 76 entries and
650 candidate state checks are a discovery baseline, not proof of exhaustive
coverage or passed acceptance. Launcher shortcuts may share an application;
retain their activation checks while linking shared visual evidence.

## Order of work

1. Inventory Studio screens, launcher entries, application menus, dialogs,
   desktop context menus and runtime-only surfaces. Reconcile the inventory
   against installed application/service discovery. Add missing surfaces before
   declaring coverage complete.
2. Review shared components first: chrome, buttons, text fields, dropdowns,
   toggles, sliders, tabs, scrollbars, menus, typography and icons. Record the
   consumers of each component so a shared fix reopens affected checks.
3. Review each Settings page, then desktop navigation/launcher/dock/tray,
   widgets, native apps, Python/C apps, login and setup screens.
4. Exercise interaction and resizing, then installed ARM and x86 paths,
   persistence across login/reboot, and Studio-to-installed asset parity.
5. Publish ARM first only after its release gates pass. Report visual and
   functional acceptance independently from build/package success.

## Evidence for every applicable state

- Reference asset and component/state identifier; explicit Studio overrides use
  their chosen recipe, not the default palette.
- Source revision and pending-input hashes; template, font and atlas versions.
- Renderer/environment, architecture, window size, scale, backdrop, fixture
  data and animation time. Preserve normal, maximized, restored and zoom views.
- Full screenshot plus element bounds/crop. Inspect silhouette, spacing,
  baseline, typography, gradient, edge softness, contrast and clipping.
- Numerical differences only for aligned, comparable reference pixels. Do not
  compare dissimilar fixture text or claim a percentage from visual judgment.
  Do not generate the expected image from the same implementation being tested.
- Interaction result: hit target, focus order, selection, cancellation,
  scrolling, persistence and error recovery as applicable.
- Result: unverified, failed, blocked, or passed; defect and evidence paths.
  Inapplicable states require a written reason. A screenshot capture alone does
  not turn an unverified entry into a pass.

## Acceptance and correction

Every element/state must independently meet the user's 95% visual threshold;
blank space and passing controls cannot compensate for failures. Functional
defects block acceptance regardless of visual score. Missing references or
non-comparable artwork remain blocked for quantitative acceptance until a
properly specified reference exists.

For each defect, record the smallest affected component, correct it, repeat its
failed checks and the consumers affected by that change. Preserve passing areas
otherwise. Stop after six unsuccessful correction loops and report precise
remaining failures. Do not change the frozen desktop audio pipeline during UI QA.

## Current checkpoint

Inventory created. Themes geometry, expansion, scrolling and reference-size
clipping have been corrected and verified in production-painter host captures.
Shared opacity and Python disabled-state defects are corrected. Native painter
and Python GUI behavioral suites pass. Per-element 95% visual scores and
installed-runtime acceptance remain open; see `sapphire-defects.md` for evidence.
