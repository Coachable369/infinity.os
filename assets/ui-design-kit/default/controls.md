# Eclipse native controls

Reuse the Precision reference surfaces and Access glyph atlas; no substitute
icon family or alternate panel border is introduced.

- Checkbox: 20 logical pixels, shared Field surface, inset three pixels, Access
  glyph 15 (check) sampled with coverage filtering. Checked and unchecked remain
  distinguishable without color. The whole labelled settings row is clickable.
- Radio: Current/Inactive circular atlas states, one selected value per group.
  Selecting an already selected option does not emit a change.
- Choice: shared Secondary surface, current value, native disclosure indicator;
  circular selectors wrap directly on activation (including the five voices).
  Explicit selection must stay within the available options.
- Expander: the existing antialiased `app_disclosure` chevron; activation toggles
  details, collapse is idempotent. It never also changes the setting value.
- Disabled: half-coverage artwork, no focus animation and no value mutation.
- Focus/pressed: use the existing Precision states, never alter hit geometry.

`kernel/ui/controls.rs` is the shared interaction contract. It proposes a new
value; the owning service must authorize and persist it before repainting the
committed value. Mouse, keyboard and accessibility activation use the same
transitions. `panel_view.rs` owns checkbox/radio composition. Settings retains
its native service bindings and shared authored row geometry.

This catalog does not imply every Settings section has completed migration.
