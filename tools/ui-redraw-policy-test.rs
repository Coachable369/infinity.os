#[path = "../kernel/ui/redraw.rs"]
mod redraw;

use redraw::{
    clock_change_requires_structural_redraw, focus_change_requires_structural_redraw,
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
// FUNC: onboarding_clock_is_not_visible_damage
// DESC: Verifies clock ticks do not rebuild onboarding while visible desktop clocks still request presentation.
// ------------------=
fn onboarding_clock_is_not_visible_damage() {
    assert!(!clock_change_requires_structural_redraw(1, true));
    assert!(clock_change_requires_structural_redraw(2, true));
    assert!(!clock_change_requires_structural_redraw(2, false));
}

// ------------------------=
// FUNC: main
// DESC: Runs behavioral redraw-policy scenarios through the same functions used by the kernel presenter.
// ------------------=
fn main() {
    onboarding_pointer_focus_is_bounded();
    non_pointer_focus_remains_structural();
    onboarding_clock_is_not_visible_damage();
}
