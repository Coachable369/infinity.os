//! Keyboard polish for the existing launcher; focus 0 is its search field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction { Up, Down, Left, Right, Previous, Next }

// ------------------------=
// FUNC: next_focus
// DESC: Moves arrows geometrically and Tab sequentially without losing focus on incomplete rows.
// ------------------=
pub fn next_focus(current: usize, apps: usize, categories: usize, columns: usize, direction: Direction) -> usize {
    let total = apps + categories + 1;
    let current = current.min(total - 1);
    let columns = columns.max(1);
    match direction {
        Direction::Previous | Direction::Left => (current + total - 1) % total,
        Direction::Next | Direction::Right => (current + 1) % total,
        Direction::Up if current == 0 => 0,
        Direction::Down if current == 0 => usize::from(total > 1),
        Direction::Up if current <= apps => current.saturating_sub(columns),
        Direction::Down if current <= apps => {
            let target = current + columns;
            if target <= apps { target }
            else if (current - 1) / columns < apps.saturating_sub(1) / columns { apps }
            else if categories > 0 { apps + 1 + ((current - 1) % columns).min(categories - 1) }
            else { current }
        }
        Direction::Up => {
            if apps == 0 { 0 }
            else { ((apps - 1) / columns * columns + 1 + current - apps - 1).min(apps) }
        }
        Direction::Down => current,
    }
}

// ------------------------=
// FUNC: activation_focus
// DESC: Makes Return in search choose the first result, never an unrelated category when empty.
// ------------------=
pub fn activation_focus(focus: usize, apps: usize) -> Option<usize> {
    if focus == 0 { (apps > 0).then_some(1) } else { Some(focus) }
}

// ------------------------=
// FUNC: clear_before_dismiss
// DESC: Keeps filtered launcher results recoverable with a first Escape before closing on a second.
// ------------------=
pub fn clear_before_dismiss(query_length: usize) -> bool { query_length > 0 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: grid_and_partial_rows
    // DESC: Exercises actual geometric focus transitions and category boundaries.
    // ------------------=
    fn grid_and_partial_rows() {
        assert_eq!(next_focus(1, 13, 5, 6, Direction::Down), 7);
        assert_eq!(next_focus(7, 13, 5, 6, Direction::Up), 1);
        assert_eq!(next_focus(12, 13, 5, 6, Direction::Down), 13);
        assert_eq!(next_focus(13, 13, 5, 6, Direction::Down), 14);
        assert_eq!(next_focus(18, 13, 5, 6, Direction::Up), 13);
        assert_eq!(next_focus(1, 13, 5, 6, Direction::Up), 0);
        for apps in 0..20 { for current in 0..apps + 6 {
            for direction in [Direction::Up, Direction::Down, Direction::Left, Direction::Right, Direction::Previous, Direction::Next] {
                assert!(next_focus(current, apps, 5, 6, direction) < apps + 6);
            }
        }}
        assert_eq!(next_focus(0, 0, 0, 6, Direction::Down), 0);
    }
    #[test]
    // ------------------------=
    // FUNC: search_and_escape
    // DESC: Verifies empty-result Enter is a no-op and Escape preserves an open cleared search.
    // ------------------=
    fn search_and_escape() {
        assert_eq!(activation_focus(0, 0), None);
        assert_eq!(activation_focus(0, 3), Some(1));
        assert_eq!(activation_focus(4, 3), Some(4));
        assert!(clear_before_dismiss(3));
        assert!(!clear_before_dismiss(0));
        assert_eq!(next_focus(0, 13, 5, 6, Direction::Previous), 18);
        assert_eq!(next_focus(18, 13, 5, 6, Direction::Next), 0);
    }
}
