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
    clock_changed && !matches!(screen, 1 | 2 | 3 | 4 | 7 | 8 | 9 | 10)
}

// ------------------------=
// FUNC: desktop_clock_requires_bounded_redraw
// DESC: Selects a top-bar-only repaint when the visible desktop clock advances.
// ------------------=
pub const fn desktop_clock_requires_bounded_redraw(screen: u8, clock_changed: bool) -> bool {
    matches!(screen, 2 | 3 | 4 | 7 | 8 | 9 | 10) && clock_changed
}

// ------------------------=
// FUNC: desktop_app_content_requires_bounded_redraw
// DESC: Selects app-window-only repainting when editor, command, or live task telemetry content changes.
// ------------------=
pub const fn desktop_app_content_requires_bounded_redraw(
    screen: u8,
    content_changed: bool,
) -> bool {
    matches!(screen, 8 | 9 | 10) && content_changed
}

// ------------------------=
// FUNC: desktop_chat_content_requires_bounded_redraw
// DESC: Selects widget-column-only repainting when desktop AI chat state changes.
// ------------------=
pub const fn desktop_chat_content_requires_bounded_redraw(
    screen: u8,
    content_changed: bool,
    assistant_changed: bool,
) -> bool {
    screen == 2 && content_changed && !assistant_changed
}

// ------------------------=
// FUNC: assistant_requires_owner_damage
// DESC: Routes attached assistant changes to their app surface, never the separate desktop chat column.
// ------------------=
pub const fn assistant_requires_owner_damage(screen: u8, previous: u32, current: u32) -> bool {
    previous != current && matches!(screen, 2 | 4 | 8 | 9 | 10 | 11)
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
    window_moved
        && !matches!(screen, 2 | 4 | 8 | 9 | 10)
        && (screen != 0 || window_visible || window_maximized)
}

// ------------------------=
// FUNC: desktop_window_move_uses_bounded_reconstruction
// DESC: Selects old-plus-new damage reconstruction for movable installed desktop surfaces.
// ------------------=
pub const fn desktop_window_move_uses_bounded_reconstruction(
    screen: u8,
    window_moved: bool,
) -> bool {
    window_moved && matches!(screen, 2 | 4 | 8 | 9 | 10)
}

// ------------------------=
// FUNC: desktop_layer_focus_change_uses_bounded_reconstruction
// DESC: Selects clipped old-and-new window composition when focus moves between desktop application layers.
// ------------------=
pub const fn desktop_layer_focus_change_uses_bounded_reconstruction(
    previous_screen: u8,
    screen: u8,
) -> bool {
    previous_screen != screen
        && matches!(previous_screen, 2 | 8 | 9 | 10)
        && matches!(screen, 2 | 8 | 9 | 10)
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
// FUNC: chat_requires_independent_widget_damage
// DESC: Keeps chat updates visible beside active apps without expanding their window damage to the whole screen.
// ------------------=
pub const fn chat_requires_independent_widget_damage(screen: u8, changed: bool) -> bool {
    changed && matches!(screen, 2 | 4 | 8 | 9 | 10 | 11)
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
