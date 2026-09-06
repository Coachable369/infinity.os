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
    focus_changed && !((matches!(screen, 1 | 5 | 6) && pointer_changed) || screen == 7)
}

// ------------------------=
// FUNC: clock_change_requires_structural_redraw
// DESC: Determines whether a clock transition affects visible content on the active system surface.
// ------------------=
pub const fn clock_change_requires_structural_redraw(screen: u8, clock_changed: bool) -> bool {
    clock_changed && !matches!(screen, 1 | 2 | 3 | 4 | 7 | 8 | 9)
}

// ------------------------=
// FUNC: desktop_clock_requires_bounded_redraw
// DESC: Selects a top-bar-only repaint when the visible desktop clock advances.
// ------------------=
pub const fn desktop_clock_requires_bounded_redraw(screen: u8, clock_changed: bool) -> bool {
    matches!(screen, 2 | 3 | 4 | 7 | 8 | 9) && clock_changed
}

// ------------------------=
// FUNC: desktop_app_content_requires_bounded_redraw
// DESC: Selects app-window-only repainting when editor text or command output changes.
// ------------------=
pub const fn desktop_app_content_requires_bounded_redraw(
    screen: u8,
    content_changed: bool,
) -> bool {
    matches!(screen, 8 | 9) && content_changed
}

// ------------------------=
// FUNC: appearance_change_requires_structural_redraw
// DESC: Invalidates the complete composed surface when the active semantic accent changes.
// ------------------=
pub const fn appearance_change_requires_structural_redraw(
    previous_accent_rgb: u32,
    accent_rgb: u32,
) -> bool {
    previous_accent_rgb != accent_rgb
}

// ------------------------=
// FUNC: desktop_window_move_requires_structural_redraw
// DESC: Requires coherent scene reconstruction for translucent window geometry transitions.
// ------------------=
pub const fn desktop_window_move_requires_structural_redraw(
    screen: u8,
    window_moved: bool,
    window_visible: bool,
    window_maximized: bool,
) -> bool {
    window_moved && (screen != 0 || window_visible || window_maximized)
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

// ------------------------=
// FUNC: authentication_controls_require_repaint
// DESC: Selects bounded login-card and utility-tray repainting for pointer-driven focus transitions.
// ------------------=
pub const fn authentication_controls_require_repaint(
    screen: u8,
    pointer_changed: bool,
    focus_changed: bool,
) -> bool {
    matches!(screen, 5 | 6) && pointer_changed && focus_changed
}

// ------------------------=
// FUNC: desktop_menu_change_requires_bounded_redraw
// DESC: Selects saved-region composition for desktop menu open, hover, switch, and close transitions.
// ------------------=
pub const fn desktop_menu_change_requires_bounded_redraw(
    previous_screen: u8,
    screen: u8,
    previous_menu: usize,
    menu: usize,
    focus_changed: bool,
) -> bool {
    matches!(previous_screen, 2 | 3)
        && matches!(screen, 2 | 3)
        && (previous_screen != screen || (screen == 3 && (previous_menu != menu || focus_changed)))
}
