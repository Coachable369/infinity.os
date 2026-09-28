//! Native shell geometry from the IDesign Kit. Coordinates are window-local pixels.
use crate::Viewport;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control { Back, Forward, Reload, Address, Go, Downloads, Menu, Minimize, Maximize, Close, Content, NewTab }

pub struct Layout {
    pub title: Viewport,
    pub tabs: Viewport,
    pub new_tab: Viewport,
    pub back: Viewport,
    pub forward: Viewport,
    pub reload: Viewport,
    pub address: Viewport,
    pub go: Viewport,
    pub downloads: Viewport,
    pub menu: Viewport,
    pub minimize: Viewport,
    pub maximize: Viewport,
    pub close: Viewport,
    pub content: Viewport,
    pub status: Viewport,
    pub download_card: Viewport,
    pub download_save: Viewport,
    pub download_discard: Viewport,
}

impl Layout {
    // ------------------------=
    // FUNC: with_tab_count
    // DESC: Keeps the add-tab control immediately after the last visible tab without moving window controls.
    // ------------------=
    pub fn with_tab_count(mut self,count:usize)->Self {
        if let Some((last,_))=count.checked_sub(1).and_then(|index|self.tab(index,count)) {
            self.new_tab.x=last.x+last.width as i32+(8*self.tabs.height/36) as i32;
        }
        self
    }
    // ------------------------=
    // FUNC: new
    // DESC: Lays out scalable shell chrome while preserving distinct hit targets and page bounds.
    // ------------------=
    pub fn new(width: u32, height: u32, scale: u32) -> Option<Self> {
        if !(1..=4).contains(&scale) || width > i32::MAX as u32 || height > i32::MAX as u32
            || width < 760 * scale || height < 240 * scale { return None; }
        let gap = crate::skin::GAP * scale;
        let gutter = crate::skin::GUTTER * scale;
        let control = crate::skin::CONTROL_HEIGHT * scale;
        let title_height = 48 * scale;
        let toolbar_height = control + 2 * gap;
        let status_height = 24 * scale;
        let tab_height = 36 * scale;
        let y = title_height + gap;
        let rect = |x: u32, y: u32, width, height| Viewport { x: x as i32, y: y as i32, width, height };
        let address_x = gutter + 3 * (control + gap);
        let go_width = 32 * scale;
        let menu_x = width - gutter - control;
        let downloads_x = menu_x - gap - control;
        let go_x = downloads_x - gap - go_width;
        let window_control = 32 * scale;
        let close_x = width - gutter - window_control;
        let content_y = title_height + toolbar_height;
        Some(Self {
            title: rect(0, 0, width, title_height),
            tabs: rect(104 * scale, 12 * scale, width - 264 * scale, tab_height),
            new_tab: rect(width - 152 * scale, 12 * scale, 32 * scale, 32 * scale),
            back: rect(gutter, y, control, control),
            forward: rect(gutter + control + gap, y, control, control),
            reload: rect(gutter + 2 * (control + gap), y, control, control),
            address: rect(address_x, y, go_x - gap - address_x, control),
            go: rect(go_x, y, go_width, control),
            downloads: rect(downloads_x, y, control, control),
            menu: rect(menu_x, y, control, control),
            minimize: rect(close_x - 2 * (window_control + gap), 8 * scale, window_control, window_control),
            maximize: rect(close_x - window_control - gap, 8 * scale, window_control, window_control),
            close: rect(close_x, 8 * scale, window_control, window_control),
            content: rect(0, content_y, width, height - content_y - status_height),
            status: rect(0, height - status_height, width, status_height),
            download_card: rect(gutter,height-status_height-80*scale,width-2*gutter,72*scale),
            download_save: rect(width-gutter-212*scale,height-status_height-60*scale,96*scale,36*scale),
            download_discard: rect(width-gutter-108*scale,height-status_height-60*scale,96*scale,36*scale),
        })
    }
    // ------------------------=
    // FUNC: hit
    // DESC: Routes page versus native chrome clicks without treating padding as a control.
    // ------------------=
    pub fn hit(&self, x: i32, y: i32) -> Option<Control> {
        let scale=self.tabs.height/36;
        for (index,control) in [Control::Close,Control::Minimize,Control::Maximize].into_iter().enumerate() {
            let bounds=Viewport{x:((14+index as u32*22)*scale) as i32,y:(14*scale) as i32,width:20*scale,height:20*scale};
            if bounds.local(x,y).is_some() {return Some(control);}
        }
        for (bounds, control) in [
            (self.back, Control::Back), (self.forward, Control::Forward),
            (self.reload, Control::Reload), (self.address, Control::Address),
            (self.go, Control::Go), (self.content, Control::Content),
            (self.downloads, Control::Downloads), (self.menu, Control::Menu),
            (self.minimize, Control::Minimize), (self.maximize, Control::Maximize),
            (self.close, Control::Close),
            (self.new_tab, Control::NewTab),
        ] {
            if bounds.local(x, y).is_some() { return Some(control); }
        }
        None
    }
    // ------------------------=
    // FUNC: tab
    // DESC: Gives each of up to eight native tab slots distinct title and close hit areas at every supported scale.
    // ------------------=
    pub fn tab(&self, index: usize, count: usize) -> Option<(Viewport, Viewport)> {
        if count == 0 || count > 8 || index >= count {return None;}
        let scale = self.tabs.height / 36;
        let slot = (self.tabs.width / count as u32).min(232 * scale);
        let tab = Viewport { x: self.tabs.x + (index as u32 * slot) as i32,
            y: self.tabs.y, width: slot - 8 * scale, height: 36 * scale };
        let close_inset = (44 * scale).min(tab.width / 2 + 12 * scale);
        let close = Viewport { x: tab.x + tab.width as i32 - close_inset as i32,
            y: tab.y + (6 * scale) as i32, width: 24 * scale, height: 24 * scale };
        Some((tab, close))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // ------------------------=
    // FUNC: tab_targets_never_overlap_navigation
    // DESC: Exercises all supported tab counts and scales, including separate close and new-tab targets.
    // ------------------=
    #[test]
    fn tab_targets_never_overlap_navigation() {
        for scale in 1..=4 {
            let layout=Layout::new(760*scale,240*scale,scale).unwrap();
            for count in 1..=8 {
                let mut right=layout.tabs.x;
                for index in 0..count {
                    let (tab,close)=layout.tab(index,count).unwrap();
                    assert!(tab.x>=right);
                    assert!(tab.local(close.x,close.y).is_some());
                    assert!(tab.local(close.x+close.width as i32-1,close.y+close.height as i32-1).is_some());
                    for y in 0..close.height {for x in 0..close.width {
                        assert!(crate::tab_style::contains(tab.width,tab.height,
                            (close.x-tab.x) as u32+x,(close.y-tab.y) as u32+y));
                    }}
                    assert!(tab.y+tab.height as i32<=layout.address.y);
                    right=tab.x+tab.width as i32;
                }
                assert!(right<layout.new_tab.x);
                let fitted=Layout::new(760*scale,240*scale,scale).unwrap().with_tab_count(count);
                assert_eq!(fitted.new_tab.x,right+(8*scale) as i32);
                assert_eq!(fitted.hit(fitted.new_tab.x,fitted.new_tab.y),Some(Control::NewTab));
                for y in 0..fitted.new_tab.height {for x in 0..fitted.new_tab.width {
                    assert_eq!(fitted.hit(fitted.new_tab.x+x as i32,fitted.new_tab.y+y as i32),Some(Control::NewTab));
                }}
            }
            assert_eq!(layout.hit(layout.new_tab.x,layout.new_tab.y),Some(Control::NewTab));
            assert!(layout.tab(0,0).is_none());
            assert!(layout.tab(0,9).is_none());
        }
    }
    // ------------------------=
    // FUNC: download_consent_stays_inside_viewport_with_separate_actions
    // DESC: Checks native save-card geometry at minimum size and every supported scale.
    // ------------------=
    #[test]
    fn download_consent_stays_inside_viewport_with_separate_actions() {
        for scale in 1..=4 {
            let layout=Layout::new(760*scale,240*scale,scale).unwrap();
            for button in [layout.download_save,layout.download_discard] {
                assert!(layout.download_card.local(button.x,button.y).is_some());
                assert!(layout.download_card.local(button.x+button.width as i32-1,button.y+button.height as i32-1).is_some());
            }
            assert!(layout.content.local(layout.download_card.x,layout.download_card.y).is_some());
            assert!(layout.content.local(layout.download_card.x+layout.download_card.width as i32-1,
                layout.download_card.y+layout.download_card.height as i32-1).is_some());
            assert!(layout.download_save.x+(layout.download_save.width as i32)<layout.download_discard.x);
        }
    }
    // ------------------------=
    // FUNC: resizing_preserves_chrome_and_changes_content_bounds
    // DESC: Verifies proportional sizing, nonoverlap and correct hit targets at each supported scale.
    // ------------------=
    #[test]
    fn resizing_preserves_chrome_and_changes_content_bounds() {
        for scale in 1..=4 {
            let small = Layout::new(760 * scale, 480 * scale, scale).unwrap();
            let large = Layout::new(1160 * scale, 760 * scale, scale).unwrap();
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
    // ------------------------=
    // FUNC: every_control_has_an_independent_hit_target
    // DESC: Verifies every design-kit control and its outer edges at all supported scales.
    // ------------------=
    #[test]
    fn every_control_has_an_independent_hit_target() {
        for scale in 1..=4 {
            let layout = Layout::new(760 * scale, 480 * scale, scale).unwrap();
            let controls = [
                (layout.back, Control::Back), (layout.forward, Control::Forward),
                (layout.reload, Control::Reload), (layout.address, Control::Address),
                (layout.go, Control::Go), (layout.downloads, Control::Downloads),
                (layout.menu, Control::Menu), (layout.minimize, Control::Minimize),
                (layout.maximize, Control::Maximize), (layout.close, Control::Close),
            ];
            for (bounds, control) in controls {
                assert_eq!(layout.hit(bounds.x, bounds.y), Some(control));
                assert_eq!(layout.hit(bounds.x + bounds.width as i32 - 1,
                    bounds.y + bounds.height as i32 - 1), Some(control));
                assert_eq!(layout.hit(bounds.x + bounds.width as i32, bounds.y), None);
                for (other, other_control) in controls {
                    if control != other_control {
                        assert!(other.local(bounds.x, bounds.y).is_none());
                    }
                }
            }
        }
    }
}
