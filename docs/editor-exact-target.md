# Editor visual acceptance target

Authoritative artwork: `assets/design/infinity-editor-ai-idesign-kit-v1.png`.
The main editor window in that sheet, not the surrounding design annotations,
defines the target. This is an acceptance target, not a claim of pixel parity.

- Default window: approximately 2:1, with the expanded assistant occupying one
  third. Keep real File/Edit/Selection/View menus and the syntax dropdown as
  explicitly requested by the user.
- Glass recipe: #0B1220 background, #0F1B2E surfaces, #142438 highlights,
  #213A55 soft one-pixel borders, white primary text, #9FB0C8 secondary text,
  #22D3EE accent. Eight-pixel gutters and rounded corners.
- Fine-line document and window controls, actual document path, compact tab,
  line-number gutter, cyan/green/yellow syntax and violet selection.
- Assistant: luminous four-point mark, compact heading, content-fitting user
  and response bubbles, separated proposed edits, reviewed actions and a slim
  composer. Collapse control is an X; the collapsed right-edge tab says AI.
- No invented conversation, fake code generation, inactive attachment buttons,
  or decorative multi-document controls. Missing capabilities remain explicit.

Review the actual installed render against the sheet. Build success does not
establish visual parity. Existing single-document and bounded-assistant limits
are documented in `editor-and-window-assistant.md`.

## Review outcome

The first installed render exposed a normalized minimum-height clamp that
stretched this design on a square firmware framebuffer. The second pass fixes
the shared constraint and the resize minimum; production geometry is now tested
against the target aspect ratio on four framebuffer sizes. Font ink is checked
against the actual pointer/caret cell width, including antialiased edge pixels.

Exact visual acceptance remains open. The left traffic-light group, reference
status-rail arrangement, multi-document behavior and complete AI diff/copy/
attachment interface are not implemented. The current reviewed-insertion card
is not equivalent to that full diff interface. Do not describe this revision as
pixel-identical or as full Sublime Text parity. Further visual expansion stops
after these two review passes rather than accepting these gaps silently.
