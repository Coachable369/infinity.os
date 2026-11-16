#![allow(dead_code)]
#[path="../kernel/ui/desktop_template.rs"] mod desktop_template;
#[path="../kernel/ui/installer_template.rs"] mod installer_template;
#[path="../kernel/ui/installer_layout.rs"] mod installer_layout;
#[path="../kernel/ui/bitmap.rs"] mod bitmap;
#[path = "../kernel/ui/desktop_widgets.rs"]
mod desktop_widgets;
#[path = "../kernel/ui/geometry.rs"]
mod geometry;
use desktop_widgets::State;
use geometry::Point;
// ------------------------=
// FUNC: visible_brand_opens_menu_and_ai_clears_dock
// DESC: Exercises the packaged icon hit target and responsive widget footprint at supported display sizes.
// ------------------=
#[test]
fn visible_brand_opens_menu_and_ai_clears_dock() {
    for (width,height,scale) in [(1280,800,1),(1920,1080,1),(3840,2160,2)] {
        let brand=desktop_template::bounds(1,49,width,height).unwrap();
        assert_eq!(desktop_template::navigation_hit(Point{x:brand.x+brand.width as i32/2,
            y:brand.y+brand.height as i32/2},width,height),Some(49));
        let panel=State::new().rect(1,width,height,scale,false);
        assert!(panel.bottom() <= height.saturating_sub(82*scale) as i32);
        let composer=desktop_template::bounds(3,68,width,height).unwrap();
        let base=desktop_template::bounds(3,2,width,height).unwrap();
        assert!(composer.bottom()+panel.y-base.y <= panel.bottom());
    }
}
// ------------------------=
// FUNC: authored_defaults_match_preview_at_every_resolution
// DESC: Ensures saved scene geometry is not silently shifted by runtime drag constraints.
// ------------------=
#[test]
fn authored_defaults_match_preview_at_every_resolution() {
    for (w,h,s) in [(800,600,1),(1280,800,1),(1536,1024,1),(3840,2160,2)] {
        let mut state=State::new();
        assert_eq!(state.visible,2);
        assert_eq!(state.rect(1,w,h,s,false).y,state.rect(0,w,h,s,false).y);
        state.visible=3;
        let overview=state.rect(0,w,h,s,false);
        let ai=state.rect(1,w,h,s,false);
        assert_eq!(ai,desktop_template::bounds(3,2,w,h).unwrap());
        assert_eq!(overview.height,desktop_template::bounds(2,2,w,h).unwrap().height);
        assert!(!overview.intersects(ai));
        let first_detail=desktop_template::bounds(2,60,w,h).unwrap();
        let chart_height=desktop_template::overview_chart_height(overview.width,h);
        assert!(first_detail.y >= overview.y+chart_height as i32);
    }
}
// ------------------------=
// FUNC: browser_suppresses_overview_without_mutating_preferences
// DESC: Verifies the browser removes the overview hit/paint surface while retaining AI and saved desktop state.
// ------------------=
#[test]
fn browser_suppresses_overview_without_mutating_preferences() {
    for visible in 0..4 {
        let mut state = State::new();
        state.visible = visible;
        let saved = state.encode();
        assert!(!state.shown(0, true));
        assert_eq!(state.shown(1, true), visible & 2 != 0);
        assert_eq!(state.shown(0, false), visible & 1 != 0);
        assert_eq!(state.encode(), saved);
        assert!(!state.shown(2, true));
    }
}
#[test]
// ------------------------=
// FUNC: drag_snap_visibility_and_persistence
// DESC: Tests production pointer capture, release snapping, bounded layout and durable hidden state.
// ------------------=
fn drag_snap_visibility_and_persistence() {
    for (w, h, s) in [(800, 600, 1), (1920, 1080, 1), (3840, 2160, 2)] {
        let mut state = State::new();
        for id in 0..3 {
            let r = state.rect(id, w, h, s, false);
            state.drag = Some((id, 10, 10));
            state.move_pointer(Point { x: 110, y: 110 }, true, w, h, s, false);
            assert!(state.drag.is_some());
            assert_ne!(r, state.rect(id, w, h, s, false));
            state.move_pointer(
                Point {
                    x: 10 + (8 * s) as i32,
                    y: 110,
                },
                false,
                w,
                h,
                s,
                false,
            );
            let r = state.rect(id, w, h, s, false);
            assert!(state.drag.is_none());
            assert!((r.x - (8 * s) as i32).abs() <= 4);
            assert!(r.x >= 0 && r.y >= 0 && r.right() <= w as i32 && r.bottom() <= h as i32);
        }
        state.visible = 0;
        state.menu = Some(Point {
            x: w as i32,
            y: h as i32,
        });
        let menu = state.menu_rect(w, h, s).unwrap();
        assert!(menu.right() <= w as i32 && menu.bottom() <= h as i32);
        let restored = State::decode(state.encode());
        assert_eq!(restored.positions, state.positions);
        assert_eq!(restored.visible, 0);
        assert_eq!(restored.menu, None);
        state.transparency=3<<4;
        assert_eq!(state.material_alpha(2,240),60);
        assert_eq!(state.material_alpha(1,240),240);
        state.visibility_menu=true;
        assert!(state.menu_rect(w,h,s).unwrap().height>menu.height);
        assert_eq!(State::decode(0), State::new());
    }
}
#[test]
// ------------------------=
// FUNC: damage_is_bounded_and_idle_is_clean
// DESC: Proves header capture alone does not repaint and a small move damages only that widget's coverage.
// ------------------=
fn damage_is_bounded_and_idle_is_clean() {
    use desktop_widgets::*;
    publish(State::new());
    let _ = take_damage(1920, 1080, 1);
    assert_eq!(take_damage(1920, 1080, 1), None);
    let mut state = current();
    state.visible=3;
    publish(state);
    let _=take_damage(1920,1080,1);
    state.drag = Some((0, 10, 10));
    publish(state);
    assert_eq!(take_damage(1920, 1080, 1), None);
    let old = state.rect(0, 1920, 1080, 1, false);
    state.move_pointer(
        Point {
            x: old.x + 20,
            y: old.y + 20,
        },
        true,
        1920,
        1080,
        1,
        false,
    );
    publish(state);
    let damage = take_damage(1920, 1080, 1).unwrap();
    assert!((damage.width as usize * damage.height as usize) < 1920 * 1080 / 4);
    assert!(damage.contains(Point { x: old.x, y: old.y }));
    assert_eq!(take_damage(1920, 1080, 1), None);
}

// ------------------------=
// FUNC: trash_placement_is_personal_and_clamped
// DESC: Verifies legacy defaults, damage, cross-resolution placement and durable round trip.
// ------------------=
#[test]
fn trash_placement_is_personal_and_clamped() {
    let mut state=State::new();
    let tile=geometry::Rect{x:680,y:500,width:80,height:70};
    assert_eq!(State::decode(state.encode()).position_trash(tile,800,600),tile);
    state.trash_position=Some([250,500]);
    let restored=State::decode(state.encode());
    let placed=restored.position_trash(tile,1600,900);
    assert_eq!((placed.x,placed.y),(400,450));
    state.trash_position=Some([1000,1000]);
    assert_eq!(state.position_trash(tile,800,600).right(),800);
    assert_eq!(state.position_trash(tile,800,600).bottom(),600);
}

// ------------------------=
// FUNC: weather_visibility_round_trip
// DESC: Verifies newly enabled forecasts survive a session reload without moving previously saved desktop objects.
// ------------------=
#[test]
fn weather_visibility_round_trip(){
 for flags in 0..8 { for trash in [None,Some([0,0]),Some([1000,1000])] {
    let mut s=State::new();s.positions=[[1001,1001],[872,504]];s.trash_position=trash;s.visible=flags;
    assert_eq!(State::decode(s.encode()),s);
 }}
 assert_eq!(State::decode(0),State::new());
}

#[test]
// ------------------------=
// FUNC: widgets_share_width_and_snap_into_stacks
// DESC: Drags the weather panel near the AI panel and verifies both edges snap on release at three resolutions.
// ------------------=
fn widgets_share_width_and_snap_into_stacks() {
    for (w,h,scale) in [(1280,800,1),(1920,1080,1),(3840,2160,2)] {
        let mut state=State::new();
        state.visible=6;
        let ai=state.rect(1,w,h,scale,false);
        assert_eq!(ai.width,state.rect(0,w,h,scale,false).width);
        assert_eq!(ai.width,state.rect(2,w,h,scale,false).width);
        state.drag=Some((2,4,8));
        let drop=Point{x:ai.x+24*scale as i32+4,y:ai.bottom()+20*scale as i32+8};
        state.move_pointer(drop,false,w,h,scale,false);
        let weather=state.rect(2,w,h,scale,false);
        assert!((weather.x-ai.x).abs()<=w.div_ceil(1000) as i32);
        assert!((weather.y-ai.bottom()-12*scale as i32).abs()<=h.div_ceil(1000) as i32);
    }
}
