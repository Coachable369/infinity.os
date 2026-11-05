# Eclipse native controls

Reuse the Precision reference surfaces and Access glyph atlas; no substitute
icon family or alternate panel border is introduced.

- Text input: use the Node Name field in the configuration template for the
  inset, border, radius and material. A window resize repositions the field
  without magnifying its artwork or type.
- Boolean toggle: compose the Secondary track and circular Current/Inactive
  thumb. Off places the thumb on the left, On on the right. The whole labeled
  row is clickable; a toggle does not rely on color alone.
- Radio: Current/Inactive circular atlas states, one selected value per group.
  Selecting an already selected option does not emit a change.
- Choice: shared Secondary surface, current value, native disclosure indicator;
  opening a list preserves the value. Arrows highlight; Enter or clicking an option commits; Escape cancels.
  Explicit selection must stay within the available options.
- Expander: the existing antialiased `app_disclosure` chevron; activation toggles
  details, collapse is idempotent. It never also changes the setting value.
- Disabled: half-coverage artwork, no focus animation and no value mutation.
- Focus/pressed: use the existing Precision states, never alter hit geometry.

`kernel/ui/controls.rs` is the shared interaction contract. It proposes a new
value; the owning service must authorize and persist it before repainting the
committed value. Mouse, keyboard and accessibility activation use the same
transitions. `panel_view.rs` owns toggle/radio composition. Settings retains
its native service bindings and shared authored row geometry.

This catalog does not imply every Settings section has completed migration.

The Precision choice field uses a left-aligned value and a separate trailing
chevron well. Its text, inset, and indicator share one geometry calculation so
long values cannot paint underneath the indicator. The field uses the kit's
Secondary surface and focus state. Boolean controls use a separate trailing
checkbox. Open menus use a Panel surface and an inset, image-backed selection
row. Row highlighting changes paint only and preserves the full hit target.
