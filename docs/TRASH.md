# Trash

Trash is a first-class safe-delete lifecycle. Moving `/home/default/report` to Trash preserves its ObjectId and exact original path through the reversible `/trash/home/default/report` Namespace projection. Restore removes the Trash prefix and returns the same ObjectId to its original reference unless a conflict exists.

`trash delete` permanently removes the selected Trash reference and tombstones an object only when no other reference remains. `trash empty` repeats that bounded operation. This is distinct from explicit `object destroy`, which targets Object identity and requires stronger authority and confirmation.
