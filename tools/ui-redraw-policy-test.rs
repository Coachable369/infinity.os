#[path = "../kernel/ui/redraw.rs"]
mod redraw;

use redraw::{
    authentication_controls_require_repaint, clock_change_requires_structural_redraw,
    desktop_app_content_requires_bounded_redraw, desktop_clock_requires_bounded_redraw,
    desktop_layer_focus_change_uses_bounded_reconstruction,
    desktop_window_move_requires_structural_redraw, desktop_window_move_uses_bounded_reconstruction,
    focus_change_requires_structural_redraw, onboarding_controls_require_repaint,
};

// ------------------------=
// FUNC: onboarding_pointer_focus_is_bounded
// DESC: Verifies pointer hover changes repaint onboarding controls without rebuilding the complete surface.
// ------------------=
fn onboarding_pointer_focus_is_bounded() {
    assert!(!focus_change_requires_structural_redraw(1, true, true));
    assert!(onboarding_controls_require_repaint(1, true, true));
    assert!(!focus_change_requires_structural_redraw(5, true, true));
    assert!(authentication_controls_require_repaint(5, true, true));
    assert!(!focus_change_requires_structural_redraw(6, true, true));
    assert!(authentication_controls_require_repaint(6, true, true));
}

// ------------------------=
// FUNC: non_pointer_focus_remains_structural
// DESC: Verifies keyboard focus and focus changes on other surfaces retain full structural presentation.
// ------------------=
fn non_pointer_focus_remains_structural() {
    assert!(focus_change_requires_structural_redraw(1, false, true));
    assert!(focus_change_requires_structural_redraw(2, true, true));
    assert!(focus_change_requires_structural_redraw(5, false, true));
    assert!(!onboarding_controls_require_repaint(1, false, true));
    assert!(!onboarding_controls_require_repaint(2, true, true));
    assert!(!authentication_controls_require_repaint(5, false, true));
    assert!(!authentication_controls_require_repaint(2, true, true));
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
    assert!(!clock_change_requires_structural_redraw(10, true));
    assert!(desktop_clock_requires_bounded_redraw(10, true));
    assert!(clock_change_requires_structural_redraw(5, true));
}

// ------------------------=
// FUNC: live_task_manager_updates_are_window_bounded
// DESC: Verifies animated task telemetry repaints only the active app window instead of the layered desktop.
// ------------------=
fn live_task_manager_updates_are_window_bounded() {
    assert!(desktop_app_content_requires_bounded_redraw(10, true));
    assert!(!desktop_app_content_requires_bounded_redraw(10, false));
}

// ------------------------=
// FUNC: launcher_updates_are_damage_limited
// DESC: Verifies launcher search, focus, and clock changes remain bounded to mutable overlay regions.
// ------------------=
fn launcher_updates_are_damage_limited() {
    assert!(!focus_change_requires_structural_redraw(7, true, true));
    assert!(!focus_change_requires_structural_redraw(7, false, true));
    assert!(!clock_change_requires_structural_redraw(7, true));
    assert!(desktop_clock_requires_bounded_redraw(7, true));
}

// ------------------------=
// FUNC: ordinary_desktop_pointer_motion_is_cursor_only
// DESC: Verifies pointer travel without a state transition never requests structural desktop repainting.
// ------------------=
fn ordinary_desktop_pointer_motion_is_cursor_only() {
    assert!(!focus_change_requires_structural_redraw(2, true, false));
    assert!(!clock_change_requires_structural_redraw(2, false));
    assert!(!desktop_clock_requires_bounded_redraw(2, false));
}

// ------------------------=
// FUNC: desktop_drag_reconstructs_the_scene
// DESC: Verifies translucent desktop window moves reconstruct only old-plus-new damage bounds.
// ------------------=
fn desktop_drag_reconstructs_the_scene() {
    assert!(desktop_window_move_uses_bounded_reconstruction(2, true));
    assert!(desktop_window_move_uses_bounded_reconstruction(4, true));
    assert!(desktop_window_move_uses_bounded_reconstruction(8, true));
    assert!(desktop_window_move_uses_bounded_reconstruction(9, true));
    assert!(desktop_window_move_uses_bounded_reconstruction(10, true));
    assert!(!desktop_window_move_requires_structural_redraw(2, true, true, false));
    assert!(desktop_window_move_requires_structural_redraw(
        3, true, true, false
    ));
    assert!(!desktop_window_move_uses_bounded_reconstruction(2, false));
}

// ------------------------=
// FUNC: desktop_click_focus_reconstructs_only_window_layers
// DESC: Verifies click-to-front transitions between native apps and File Navigator avoid a full desktop repaint.
// ------------------=
fn desktop_click_focus_reconstructs_only_window_layers() {
    assert!(desktop_layer_focus_change_uses_bounded_reconstruction(2, 8));
    assert!(desktop_layer_focus_change_uses_bounded_reconstruction(8, 9));
    assert!(desktop_layer_focus_change_uses_bounded_reconstruction(9, 10));
    assert!(desktop_layer_focus_change_uses_bounded_reconstruction(10, 2));
    assert!(!desktop_layer_focus_change_uses_bounded_reconstruction(8, 8));
    assert!(!desktop_layer_focus_change_uses_bounded_reconstruction(4, 9));
}

// ------------------------=
// FUNC: main
// DESC: Runs behavioral redraw-policy scenarios through the same functions used by the kernel presenter.
// ------------------=
fn main() {
    for screen in 0..=10 {
        assert!(!redraw::chat_requires_independent_widget_damage(screen, false));
        assert_eq!(redraw::chat_requires_independent_widget_damage(screen, true),
            matches!(screen, 4 | 8 | 9 | 10));
    }
    onboarding_pointer_focus_is_bounded();
    non_pointer_focus_remains_structural();
    visible_clock_updates_are_damage_limited();
    live_task_manager_updates_are_window_bounded();
    launcher_updates_are_damage_limited();
    ordinary_desktop_pointer_motion_is_cursor_only();
    desktop_drag_reconstructs_the_scene();
    desktop_click_focus_reconstructs_only_window_layers();
}
