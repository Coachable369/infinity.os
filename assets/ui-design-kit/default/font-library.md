# Font library

Reuse the native Settings accordion, smoked navy cards, cyan selection border,
16px labels, 8px rhythm and 16px gutters from the default kit. No new artwork or
icon family is needed. Location: Themes & Skins → Fonts, replacing the read-only
automatic UI-scale explanation (automatic display scaling is unchanged).

Six 56px rows per page. Each row presents the family name, a live sample using
that family's real glyphs, a Use action and a Remove/Install action. The bundled
Roboto fallback cannot be removed. Removing the active family selects Roboto in
the same durable transaction. A footer provides Previous/Next and save feedback.
Removed bundled fonts remain recoverable from the immutable System Generation;
removal changes profile availability, not shared system payload bytes.

Settings uses the same measured/clipped geometry for painting and input. Selection
is published only after successful object-store persistence. No host font services,
runtime filesystem mounts or paint-time font decoding are introduced.

This panel manages the shipped OFL repository; arbitrary external TTF/OTF import
is a separate acceptance item, not implied by the bundled Install action.
