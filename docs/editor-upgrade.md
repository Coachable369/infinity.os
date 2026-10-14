# Native editor upgrade

View → Color scheme selects ten independent document palettes: Infinity,
Charcoal, Monokai Night, Dracula Night, Nord, Solar Dark, Solar Light, Paper,
Ocean and Forest. These are native palettes inspired by familiar editor styles,
not imported Sublime theme files. Window borders/title/assistant remain unchanged.

View → Editor font: JetBrains Mono, Fira Code, IBM Plex Mono, Inconsolata.
View → Text size: 12, 14, 16, 18, 20, 24 pixels. Ctrl+= / Ctrl+- changes size.
Fonts removed from the user's library must be restored in Settings before use.
Appearance changes commit to a private session-authorized object before becoming
active. A failed save reports an error and preserves the previous appearance.

Edit → Text actions adds ten capabilities (paired directions count as one):

1. Select word at caret.
2. Select whole line (Ctrl+L).
3. Delete current/selected lines.
4. Move current/selected lines up or down.
5. Join selected lines, or current and following line (Ctrl+J).
6. Sort selected whole lines lexicographically.
7. Trim trailing whitespace on current/selected lines.
8. Uppercase/lowercase selection.
9. Toggle line comments for supported language modes (Ctrl+/).
10. Indent/outdent current/selected lines by four spaces (or remove one tab).

All modifying actions are atomic and undoable. No-selection case conversion is a
no-op. HTML, JSON, CSS and Plain text do not offer synthetic line-comment syntax.
The command palette also accepts the action's menu label, case-insensitively.
Select All before Sort or Trim to apply to the whole document.

Syntax selector adds Ruby, Go, Java, Swift, Kotlin, C#, SQL, Lua, YAML and TOML;
filename detection and manual selection both work. SQL keywords are insensitive
to case. These are lexical modes, not language servers or complete grammars.

The existing ASCII-only, 16 KiB document limit remains. Font ligatures, Unicode
editing, project indexing and multi-cursor editing are not claimed in this cycle.
The new size-specific font pack is embedded in both architecture kernels and
packaged in live/fresh-install payloads with a byte-for-byte parity assertion.

Design reference: existing Infinity editor IDesign kit. Research:
https://www.sublimetext.com/docs/color_schemes.html and
https://www.sublimetext.com/docs/settings.html.
