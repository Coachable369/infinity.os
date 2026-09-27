//! Allocation-free desktop window gestures; geometry uses the shell's 0..1000 units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
#[derive(Clone, Copy)]
pub(super) enum Arrange {
    Left,
    Right,
    Center,
    Grow,
    Shrink,
}
// ------------------------=
// FUNC: arrange
// DESC: Fits a window inside the usable desktop while retaining application minimum sizes.
// ------------------=
pub(super) fn arrange(
    mut b: Bounds,
    action: Arrange,
    min: (i32, i32),
    top: i32,
    bottom: i32,
) -> Bounds {
    let available = (bottom - top).max(min.1);
    b.width = b.width.clamp(min.0, 900);
    b.height = b.height.clamp(min.1, available.min(820).max(min.1));
    match action {
        Arrange::Left | Arrange::Right => {
            b.width = 480.max(min.0);
            b.height = available.min(820).max(min.1);
            b.x = if matches!(action, Arrange::Left) {
                10
            } else {
                990 - b.width
            };
            b.y = top;
        }
        Arrange::Center => {
            b.x = (1000 - b.width) / 2;
            b.y = top + (available - b.height) / 2;
        }
        Arrange::Grow | Arrange::Shrink => {
            let step = if matches!(action, Arrange::Grow) {
                25
            } else {
                -25
            };
            b.width = (b.width + step).clamp(min.0, 900);
            b.height = (b.height + step).clamp(min.1, available.min(820).max(min.1));
        }
    }
    b.x = b.x.clamp(0, 1000 - b.width);
    b.y = b.y.clamp(top, (bottom - b.height).max(top));
    b
}
// ------------------------=
// FUNC: next_window
// DESC: Cycles stable window identities so raising a window cannot trap cycling between two apps.
// ------------------=
pub(super) fn next_window<const N:usize>(visible: [bool; N], active: usize) -> Option<usize> {
    (1..=N)
        .map(|step| (active % N + step) % N)
        .find(|id| visible[*id])
}
pub(super) struct TitleClicks {
    last: Option<(usize, u64, i32, i32)>,
}
impl TitleClicks {
    // ------------------------=
    // FUNC: new
    // DESC: Starts with no title gesture armed.
    // ------------------=
    pub const fn new() -> Self {
        Self { last: None }
    }
    // ------------------------=
    // FUNC: click
    // DESC: Accepts a same-window, nearby second click within 450ms; triples begin a new gesture.
    // ------------------=
    pub fn click(&mut self, id: usize, now: Option<u64>, x: i32, y: i32) -> bool {
        let previous = self.last.take();
        let Some(now) = now else {
            return false;
        };
        if previous.is_some_and(|(old, t, px, py)| {
            old == id
                && now >= t
                && now - t <= 450_000_000
                && x.abs_diff(px) <= 4
                && y.abs_diff(py) <= 4
        }) {
            return true;
        }
        self.last = Some((id, now, x, y));
        false
    }
    // ------------------------=
    // FUNC: cancel
    // DESC: Prevents a title click separated by another control or drag from becoming a double click.
    // ------------------=
    pub fn cancel(&mut self) {
        self.last = None;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: arrangements_respect_work_area_and_minimums
    // DESC: Exercises tiling, centering, resizing and off-screen recovery across application sizes.
    // ------------------=
    #[test]
    fn arrangements_respect_work_area_and_minimums() {
        let b = Bounds {
            x: 980,
            y: 900,
            width: 680,
            height: 500,
        };
        let left = arrange(b, Arrange::Left, (420, 300), 70, 900);
        let right = arrange(b, Arrange::Right, (420, 300), 70, 900);
        assert!(left.x + left.width < right.x);
        assert_eq!(right.x + right.width, 990);
        for min in [(300, 260), (420, 350), (600, 420)] {
            for action in [
                Arrange::Left,
                Arrange::Right,
                Arrange::Center,
                Arrange::Grow,
                Arrange::Shrink,
            ] {
                let n = arrange(b, action, min, 70, 900);
                assert!(n.x >= 0 && n.y >= 70 && n.x + n.width <= 1000 && n.y + n.height <= 900);
                assert!(n.width >= min.0 && n.height >= min.1);
            }
        }
        let center = arrange(b, Arrange::Center, (420, 300), 70, 900);
        assert_eq!(center.x * 2 + center.width, 1000);
        let grown = arrange(center, Arrange::Grow, (420, 300), 70, 900);
        assert_eq!(grown.width, center.width + 25);
        assert_eq!(arrange(grown, Arrange::Shrink, (420, 300), 70, 900), center);
        assert_eq!(next_window([false; 5], 2), None);
        let visible = [true, false, true, true, false];
        assert_eq!(next_window(visible, 0), Some(2));
        assert_eq!(next_window(visible, 2), Some(3));
        assert_eq!(next_window(visible, 3), Some(0));
        let browser_visible=[true,false,false,false,false,true];
        assert_eq!(next_window(browser_visible,0),Some(5));
        assert_eq!(next_window(browser_visible,5),Some(0));
        assert_eq!(next_window([false;0],0),None);
    }
    // ------------------------=
    // FUNC: title_gestures_are_bounded_and_isolated
    // DESC: Rejects cross-window clicks, distant movement, long delays and intervening control clicks.
    // ------------------=
    #[test]
    fn title_gestures_are_bounded_and_isolated() {
        let mut t = TitleClicks::new();
        assert!(!t.click(1, Some(0), 10, 10));
        assert!(t.click(1, Some(100_000_000), 12, 11));
        assert!(!t.click(1, Some(120_000_000), 12, 11));
        assert!(!t.click(2, Some(150_000_000), 12, 11));
        assert!(!t.click(2, Some(160_000_000), 30, 11));
        t.cancel();
        assert!(!t.click(2, Some(170_000_000), 30, 11));
        assert!(!t.click(2, Some(900_000_000), 30, 11));
        assert!(!t.click(2, None, 30, 11));
    }
}
