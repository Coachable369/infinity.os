//! Native shell geometry from the IDesign Kit. Coordinates are window-local pixels.
use crate::Viewport;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control { Back, Forward, Reload, Address, Go, Content }

pub struct Layout {
    pub title: Viewport,
    pub back: Viewport,
    pub forward: Viewport,
    pub reload: Viewport,
    pub address: Viewport,
    pub go: Viewport,
    pub content: Viewport,
    pub status: Viewport,
}

impl Layout {
    // ------------------------=
    // FUNC: new
    // DESC: Lays out scalable shell chrome while preserving distinct hit targets and page bounds.
    // ------------------=
    pub fn new(width: u32, height: u32, scale: u32) -> Option<Self> {
        if !(1..=4).contains(&scale) || width > i32::MAX as u32 || height > i32::MAX as u32
            || width < 640 * scale || height < 240 * scale { return None; }
        let gap = 8 * scale;
        let gutter = 16 * scale;
        let control = 44 * scale;
        let title_height = 40 * scale;
        let toolbar_height = control + 2 * gap;
        let status_height = 24 * scale;
        let y = title_height + gap;
        let rect = |x: u32, y: u32, width, height| Viewport { x: x as i32, y: y as i32, width, height };
        let address_x = gutter + 3 * (control + gap);
        let go_width = 72 * scale;
        let go_x = width - gutter - go_width;
        let content_y = title_height + toolbar_height;
        Some(Self {
            title: rect(0, 0, width, title_height),
            back: rect(gutter, y, control, control),
            forward: rect(gutter + control + gap, y, control, control),
            reload: rect(gutter + 2 * (control + gap), y, control, control),
            address: rect(address_x, y, go_x - gap - address_x, control),
            go: rect(go_x, y, go_width, control),
            content: rect(0, content_y, width, height - content_y - status_height),
            status: rect(0, height - status_height, width, status_height),
        })
    }
    // ------------------------=
    // FUNC: hit
    // DESC: Routes page versus native chrome clicks without treating padding as a control.
    // ------------------=
    pub fn hit(&self, x: i32, y: i32) -> Option<Control> {
        for (bounds, control) in [
            (self.back, Control::Back), (self.forward, Control::Forward),
            (self.reload, Control::Reload), (self.address, Control::Address),
            (self.go, Control::Go), (self.content, Control::Content),
        ] {
            if bounds.local(x, y).is_some() { return Some(control); }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: resizing_preserves_chrome_and_changes_content_bounds
    // DESC: Verifies proportional sizing, nonoverlap and correct hit targets at each supported scale.
    // ------------------=
    #[test]
    fn resizing_preserves_chrome_and_changes_content_bounds() {
        for scale in 1..=4 {
            let small = Layout::new(640 * scale, 480 * scale, scale).unwrap();
            let large = Layout::new(1040 * scale, 760 * scale, scale).unwrap();
            assert_eq!(large.address.width - small.address.width, 400 * scale);
            assert_eq!(large.content.height - small.content.height, 280 * scale);
            assert_eq!(small.hit(small.back.x, small.back.y), Some(Control::Back));
            assert_eq!(small.hit(small.back.x + small.back.width as i32, small.back.y), None);
            assert_eq!(small.hit(0, small.content.y), Some(Control::Content));
            assert_eq!(small.hit(0, small.status.y), None);
            assert_eq!(small.content.y as u32 + small.content.height, small.status.y as u32);
            assert!(small.address.width >= 300 * scale);
        }
        assert!(Layout::new(639, 480, 1).is_none());
        assert!(Layout::new(u32::MAX, 480, 1).is_none());
        assert!(Layout::new(640, 480, 0).is_none());
    }
}
