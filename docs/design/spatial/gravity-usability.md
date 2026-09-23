# Gravity Well interaction refinement

Historical interaction pass; superseded by [Gravity Wall](gravity-wall.md).

Reuse the existing Spatial IDesign Kit, icon pack, orbit stage, glass buttons,
and settling animation. No new bitmap assets or replacement visual language.

- Add selected file captures the File Navigator selection as a shortcut.
- Add note creates a clipping directly, without visiting Matter Shelf.
- Select a shortcut, then click a named collection or press 1–4. Dragging
  onto the same shared 190-by-60 destination also previews the move.
- The active collection is highlighted. Saved world names appear consistently
  on destinations, shortcut captions, and confirmation.
- Confirmation identifies both the shortcut and destination. Escape cancels.
- Remove shortcut removes metadata only; original files remain untouched.

The change uses existing persisted spatial metadata and native installed-kernel
paths; no new payload component or state format is introduced.
