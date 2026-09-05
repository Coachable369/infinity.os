#[path = "../kernel/ui/redraw.rs"]
mod redraw;

use redraw::{
    clock_change_requires_structural_redraw, desktop_clock_requires_bounded_redraw,
    desktop_window_move_requires_structural_redraw, focus_change_requires_structural_redraw,
    onboarding_controls_require_repaint,
};

// ------------------------=
// FUNC: onboarding_pointer_focus_is_bounded
// DESC: Verifies pointer hover changes repaint onboarding controls without rebuilding the complete surface.
// ------------------=
fn onboarding_pointer_focus_is_bounded() {
    assert!(!focus_change_requires_structural_redraw(1, true, true));
    assert!(onboarding_controls_require_repaint(1, true, true));
}

// ------------------------=
// FUNC: non_pointer_focus_remains_structural
// DESC: Verifies keyboard focus and focus changes on other surfaces retain full structural presentation.
// ------------------=
fn non_pointer_focus_remains_structural() {
    assert!(focus_change_requires_structural_redraw(1, false, true));
    assert!(focus_change_requires_structural_redraw(2, true, true));
    assert!(!onboarding_controls_require_repaint(1, false, true));
    assert!(!onboarding_controls_require_repaint(2, true, true));
}

// ------------------------=
// FUNC: visible_clock_updates_are_damage_limited
// DESC: Verifies onboarding ignores clock ticks and the desktop repaints only its clock region.
// ------------------=
fn visible_clock_updates_are_damage_limited() {
    assert!(!clock_change_requires_structural_redraw(1, true));
    assert!(!desktop_clock_requires_bounded_redraw(1, true));
    assert!(!clock_change_requires_structural_redraw(2, true));
    assert!(desktop_clock_requires_bounded_redraw(2, true));
    assert!(!clock_change_requires_structural_redraw(2, false));
    assert!(!desktop_clock_requires_bounded_redraw(2, false));
    assert!(!clock_change_requires_structural_redraw(3, true));
    assert!(desktop_clock_requires_bounded_redraw(3, true));
    assert!(!clock_change_requires_structural_redraw(4, true));
    assert!(desktop_clock_requires_bounded_redraw(4, true));
    assert!(clock_change_requires_structural_redraw(5, true));
}

// ------------------------=
// FUNC: desktop_drag_uses_bounded_damage
// DESC: Verifies ordinary visible-window motion avoids a structural redraw while unsupported states retain the safe fallback.
// ------------------=
fn desktop_drag_uses_bounded_damage() {
    assert!(!desktop_window_move_requires_structural_redraw(2, true, true, false));
    assert!(desktop_window_move_requires_structural_redraw(2, true, false, false));
    assert!(desktop_window_move_requires_structural_redraw(2, true, true, true));
    assert!(desktop_window_move_requires_structural_redraw(3, true, true, false));
    assert!(!desktop_window_move_requires_structural_redraw(2, false, true, false));
}

// ------------------------=
// FUNC: main
// DESC: Runs behavioral redraw-policy scenarios through the same functions used by the kernel presenter.
// ------------------=
fn main() {
    onboarding_pointer_focus_is_bounded();
    non_pointer_focus_remains_structural();
    visible_clock_updates_are_damage_limited();
    desktop_drag_uses_bounded_damage();
}
