# Accessibility

Accessibility is derived from the retained semantic tree, not inferred from
pixels. Every interactive element declares a role, label ID, state, actions, and
bounds. The login surface provides deterministic Tab/Shift-Tab traversal,
visible focus, keyboard activation, and a dedicated Accessibility entry point.

SafeSkin provides high contrast and zero motion. Reduced-motion policy exists in
the platform contract. English strings resolve through stable IDs so a future
locale can change copy without changing actions or authority.

Semantic metadata and focus traversal are TESTED. Screen-reader speech output,
switch control, magnification, and complete locale packs are PLANNED.
