//! Architecture-neutral redraw policy for bounded system-surface updates.

// ------------------------=
// FUNC: focus_change_requires_structural_redraw
// DESC: Determines whether a focus transition requires rebuilding the complete system surface.
// ------------------=
pub const fn focus_change_requires_structural_redraw(
    screen: u8,
    pointer_changed: bool,
    focus_changed: bool,
) -> bool {
    focus_changed && !(screen == 1 && pointer_changed)
}

// ------------------------=
// FUNC: clock_change_requires_structural_redraw
// DESC: Determines whether a clock transition affects visible content on the active system surface.
// ------------------=
pub const fn clock_change_requires_structural_redraw(screen: u8, clock_changed: bool) -> bool {
    clock_changed && screen != 1
}

// ------------------------=
// FUNC: onboarding_controls_require_repaint
// DESC: Selects a bounded onboarding-control repaint for pointer-driven focus transitions.
// ------------------=
pub const fn onboarding_controls_require_repaint(
    screen: u8,
    pointer_changed: bool,
    focus_changed: bool,
) -> bool {
    screen == 1 && pointer_changed && focus_changed
}
