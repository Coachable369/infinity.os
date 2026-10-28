# Browser tabs and downloads v2

Reference: `browser-tabs-downloads-v2-reference.png`, generated with built-in
imagegen. This is a design target, not runtime acceptance evidence.

For browser tabs use `browser-tabs-soft-v4.md`. This document and its reference
remain the active Downloads design contract. Preserve the assistant side tab.

Downloads reuse native menu/panel surfaces, Inter labels, 8/16px gutters and
semantic cyan progress, red failure and green completion. Never report success
before durable object commit. Do not show guessed transfer times. Image actions
must operate on the element under the original right click, not the pointer's
later position. Website filenames confer no path or overwrite authority.

The generated board includes illustrative Inspect/Copy Image controls; those
are not authorized completion claims or a substitute for implemented actions.
