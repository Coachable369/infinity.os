//! Shared compact application-menu geometry and scrolling policy.

pub const ROW_HEIGHT: usize = 28;
pub const SCROLL_ROW_HEIGHT: usize = 24;
pub const PANEL_PADDING: usize = 6;
pub const MAX_VISIBLE_ROWS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub first: usize,
    pub visible: usize,
    pub overflow: bool,
    pub can_scroll_up: bool,
    pub can_scroll_down: bool,
}

// ------------------------=
// FUNC: window
// DESC: Keeps the focused command visible in a continuous vertical menu without exposing pagination controls.
// ------------------=
pub fn window(count: usize, capacity: usize, focus: usize) -> Window {
    let visible = count.min(capacity.max(1));
    let maximum_first = count.saturating_sub(visible);
    let focus = focus.min(count.saturating_sub(1));
    let first = focus.saturating_sub(visible.saturating_sub(1)).min(maximum_first);
    Window {
        first,
        visible,
        overflow: count > visible,
        can_scroll_up: first > 0,
        can_scroll_down: first + visible < count,
    }
}

// ------------------------=
// FUNC: panel_height
// DESC: Returns the reusable glass-menu height for compact rows and optional top and bottom scroll affordances.
// ------------------=
pub const fn panel_height(rows: usize, overflow: bool) -> usize {
    PANEL_PADDING * 2
        + rows * ROW_HEIGHT
        + if overflow { SCROLL_ROW_HEIGHT * 2 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------=
    // FUNC: focused_rows_scroll_continuously
    // DESC: Verifies every focus remains visible and scroll affordances represent the remaining content.
    // ------------------=
    #[test]
    fn focused_rows_scroll_continuously() {
        for focus in 0..14 {
            let view = window(14, 8, focus);
            assert!(focus >= view.first && focus < view.first + view.visible);
            assert_eq!(view.can_scroll_up, view.first > 0);
            assert_eq!(view.can_scroll_down, view.first + view.visible < 14);
        }
        assert_eq!(window(4, 8, 3), Window {
            first: 0, visible: 4, overflow: false,
            can_scroll_up: false, can_scroll_down: false,
        });
    }
}
