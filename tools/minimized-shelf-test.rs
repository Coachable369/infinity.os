#![allow(dead_code)]
#[path = "../kernel/ui/mod.rs"]
mod ui;
use ui::geometry;
#[path = "../kernel/ui/minimized_shelf.rs"]
mod shelf;
use geometry::Point;
use shelf::*;
// ------------------------=
// FUNC: default_shelf_is_left_docked
// DESC: Verifies fresh desktop state docks the task shelf on the left without overriding subsequent dragging.
// ------------------=
#[test]
fn default_shelf_is_left_docked() {
    let mut state = State::new();
    assert!(state.left);
    assert_eq!(state.floating, [0, 0]);
    state.drag = Some((0, 60, 0, 0));
    state.move_drag(950, 100, false, 50);
    assert!(!state.left);
}
#[test]
// ------------------------=
// FUNC: native_tray_slots_survive_stacking_changes
// DESC: Keeps restore targets stable when stacking order changes or a window leaves the tray.
// ------------------=
fn native_tray_slots_survive_stacking_changes() {
    let mut state = State::new();
    let a = ui::window::WindowId(100);
    let b = ui::window::WindowId(101);
    let mut windows = [None; ui::window::MAX_WINDOWS];
    windows[0] = Some(a);
    windows[1] = Some(b);
    state.sync_native(windows);
    windows.swap(0, 1);
    state.sync_native(windows);
    assert_eq!(state.native[0], Some(a));
    assert_eq!(state.native[1], Some(b));
    windows[1] = None;
    state.sync_native(windows);
    assert_eq!(state.native[0], None);
    assert_eq!(state.native[1], Some(b));
    assert_eq!(state.count(), 1);
}

#[test]
// ------------------------=
// FUNC: all_native_windows_fit_without_displacing_system_apps
// DESC: Verifies all native slots retain independent tray identities alongside built-in applications.
// ------------------=
fn all_native_windows_fit_without_displacing_system_apps() {
    let mut state=State::new();
    state.set(EDITOR,true);
    for slot in 0..ui::window::MAX_WINDOWS {
        state.native[slot]=Some(ui::window::WindowId(100+slot as u32));
        state.set(NATIVE+slot,true);
    }
    assert_eq!(state.count(),17);
    state.set(NATIVE+15,false);
    assert_eq!(state.count(),16);
    assert_eq!(state.item(0),Some(EDITOR));
    assert_eq!(state.native[0],Some(ui::window::WindowId(100)));
}
#[test]
// ------------------------=
// FUNC: browser_minimize_has_an_independent_restore_identity
// DESC: Verifies browser minimization survives alongside other apps and restores without clearing their entries.
// ------------------=
fn browser_minimize_has_an_independent_restore_identity() {
    let mut state=State::new();
    state.set(EDITOR,true);
    state.set(BROWSER,true);
    assert_eq!(state.count(),2);
    assert_eq!(state.item(1),Some(BROWSER));
    state.set(BROWSER,false);
    assert_eq!(state.count(),1);
    assert_eq!(state.item(0),Some(EDITOR));
}
#[test]
// ------------------------=
// FUNC: drop_interior_floats_and_edges_anchor
// DESC: Exercises the same coalesced drag/release state transition used by native pointer routing.
// ------------------=
fn drop_interior_floats_and_edges_anchor() {
    let mut s = State::new();
    s.drag = Some((900, 180, 10, 10));
    s.move_drag(410, 250, true, 60);
    assert_eq!(s.drag, Some((400, 240, 10, 10)));
    s.move_drag(420, 270, false, 60);
    assert_eq!(s.floating, [411, 261]);
    assert_eq!(s.drag, None);
    for (x, left) in [(20, true), (990, false)] {
        s.drag = Some((410, 260, 10, 10));
        s.move_drag(x, 250, false, 60);
        assert_eq!(s.floating, [0, 0]);
        assert_eq!(s.left, left);
    }
}
#[test]
// ------------------------=
// FUNC: drawer_anchors_and_tracks_drag_without_losing_apps
// DESC: Verifies both edges, inward menus, and live drag geometry at different display sizes.
// ------------------=
fn drawer_anchors_and_tracks_drag_without_losing_apps() {
    for (w, h) in [(800, 600), (1920, 1080), (3840, 2160)] {
        let mut state = State::new();
        assert!(state.left);
        assert_eq!(state.floating, [0, 0]);
        assert!(Geometry::new(w, h, state).rail.x < (w / 2) as i32);
        state.set(EDITOR, true);
        state.menu = Some(EDITOR);
        state.left = false;
        let right = Geometry::new(w, h, state);
        assert!(right.menu.right() < right.rail.x);
        state.left = true;
        let left = Geometry::new(w, h, state);
        assert!(left.menu.x > left.rail.right());
        assert_eq!(left.rail.x, (w / 100) as i32);
        state.drag = Some((400, 180, 10, 10));
        let moving = Geometry::new(w, h, state);
        assert_eq!(moving.rail.x, (w * 400 / 1000) as i32);
        assert_eq!(state.item(0), Some(EDITOR));
        state.drag = None;
        assert_eq!(Geometry::new(w, h, state).rail, left.rail);
    }
}

#[test]
// ------------------------=
// FUNC: lifecycle_keeps_exact_minimized_identities
// DESC: Verifies minimize, restore, close, and menu dismissal without manufacturing closed apps.
// ------------------=
fn lifecycle_keeps_exact_minimized_identities() {
    let mut state = State::new();
    assert_eq!(state.count(), 0);
    state.set(2, true);
    state.set(EDITOR, true);
    state.set(COMMAND, true);
    assert_eq!(state.item(0), Some(2));
    assert_eq!(state.item(1), Some(COMMAND));
    assert_eq!(state.item(2), Some(EDITOR));
    state.menu = Some(EDITOR);
    state.row = Some(2);
    state.set(EDITOR, false);
    assert_eq!(state.menu, None);
    assert_eq!(state.row, None);
    assert_eq!(state.count(), 2);
    state.set(2, false);
    assert_eq!(state.item(0), Some(COMMAND));
    state.set(COMMAND, false);
    assert_eq!(state.count(), 0);
    state.set(COUNT, true);
    assert_eq!(state.count(), 0);
}

#[test]
// ------------------------=
// FUNC: shelf_overflow_geometry_and_menu_actions_are_reachable
// DESC: Tests representative resolutions, exact targets, bounded overflow, and all three action rows.
// ------------------=
fn shelf_overflow_geometry_and_menu_actions_are_reachable() {
    for (width, height) in [
        (800, 600),
        (1024, 768),
        (1440, 900),
        (1920, 1080),
        (2560, 1440),
        (3440, 1440),
    ] {
        let mut state = State::new();
        for id in 0..COUNT {
            state.set(id, true);
        }
        let g = Geometry::new(width, height, state);
        assert!(g.rail.height > g.rail.width);
        assert!(g.rail.right() < width as i32);
        assert!(g.rail.bottom() < height as i32 * 85 / 100);
        for offset in 0..COUNT {
            state.scroll(1, g.capacity);
            assert!(state.offset <= state.count().saturating_sub(g.capacity));
            for row in 0..g.capacity {
                let tile = g.tile_rect(row);
                assert!(tile.bottom() <= g.rail.bottom());
                let point = Point {
                    x: tile.x + tile.width as i32 / 2,
                    y: tile.y + tile.height as i32 / 2,
                };
                assert_eq!(g.hit(state, point), state.item(state.offset + row));
            }
            state.menu = state.item(state.offset);
            let menu = Geometry::new(width, height, state);
            assert!(menu.menu.x > g.rail.right());
            assert!(menu.menu.bottom() <= height as i32);
            for action in 0..3 {
                assert_eq!(
                    menu.menu_row(Point {
                        x: menu.menu.x + 20,
                        y: menu.menu.y + (action as i32 + 1) * menu.row_height as i32 + 5
                    }),
                    Some(action)
                );
            }
            assert!(
                menu.damage(state).width * menu.damage(state).height < (width * height / 3) as u32,
                "resolution {width}x{height}, offset {offset}"
            );
        }
        state.scroll(-100, g.capacity);
        assert_eq!(state.offset, 0);
    }
}

#[test]
// ------------------------=
// FUNC: unchanged_pointer_state_has_no_scene_damage
// DESC: Tests the production dirty-state gate and bounded coverage for opening and dismissing a menu.
// ------------------=
fn unchanged_pointer_state_has_no_scene_damage() {
    publish(State::new());
    let _ = take_damage(1920, 1080);
    assert_eq!(take_damage(1920, 1080), None);
    set(EDITOR, true);
    let damage = take_damage(1920, 1080).unwrap();
    assert!(damage.width < 1920 / 8);
    assert_eq!(take_damage(1920, 1080), None);
    let mut state = current();
    state.menu = Some(EDITOR);
    publish(state);
    let menu = Geometry::new(1920, 1080, state).menu;
    assert!(take_damage(1920, 1080).unwrap().intersection(menu) == menu);
    state.menu = None;
    publish(state);
    assert!(take_damage(1920, 1080).unwrap().intersection(menu) == menu);
    assert_eq!(take_damage(1920, 1080), None);
}

#[test]
// ------------------------=
// FUNC: expanded_motion_preserves_rows_and_reverses_without_jumps
// DESC: Verifies timed expansion, exact targets, sliding row selection, bounded action regions and reduced-motion endpoints.
// ------------------=
fn expanded_motion_preserves_rows_and_reverses_without_jumps() {
    publish(State::new()); animate(0,true);
    let mut s=State::new();s.set(EDITOR,true);s.set(COMMAND,true);s.hover=Some(COMMAND);s.row=Some(0);s.expanded=true;publish(s);
    let initial=Geometry::new(1440,900,s);
    animate(100,false);animate(220,false);
    let mid=Geometry::new(1440,900,s);
    assert!(mid.rail.width>initial.rail.width);
    assert_eq!(mid.tile_rect(0).y,initial.tile_rect(0).y);
    assert_eq!(mid.tile_rect(0).height,initial.tile_rect(0).height);
    let before=presentation();s.expanded=false;publish(s);animate(220,false);assert_eq!(presentation(),before);
    animate(460,false);assert_eq!(presentation().0,0);
    s.expanded=true;publish(s);animate(500,true);assert_eq!(presentation().0,255);
    let full=Geometry::new(1440,900,s);
    for i in 0..ACTIONS.len() {let r=full.action_rect(i);let p=Point{x:r.x+1,y:r.y+1};assert_eq!(full.action_hit(p),Some(i));assert_eq!(full.hit(s,p),None);}
    s.row=Some(1);s.selected_row=1;publish(s);animate(600,false);animate(690,false);assert!(presentation().1>0 && presentation().1<256);animate(920,false);assert_eq!(presentation().1,256);
    publish(State::new());animate(940,true);
}

#[test]
// ------------------------=
// FUNC: connected_bubble_has_concave_shoulders
// DESC: Verifies the selection flows into the spine without gaps and tapers tangentially outside the row.
// ------------------=
fn connected_bubble_has_concave_shoulders() {
    assert_eq!(bubble_span(80,100,140,10,200,20),Some((200,201)));
    assert_eq!(bubble_span(99,100,140,10,200,20),Some((186,201)));
    assert_eq!(bubble_span(120,100,140,10,200,20),Some((10,201)));
    assert_eq!(bubble_span(160,100,140,10,200,20),None);
    for y in 80..160 {let (l,r)=bubble_span(y,100,140,10,200,20).unwrap();assert!(l<=200 && r==201);}
}

#[test]
// ------------------------=
// FUNC: hover_bubble_grows_and_reverses_continuously
// DESC: Verifies intermediate hover growth, independently moving selection edges, reversal and reduced-motion settling.
// ------------------=
fn hover_bubble_grows_and_reverses_continuously() {
    let mut state=State::new();state.set(EDITOR,true);state.hover=Some(EDITOR);state.expanded=true;state.row=Some(0);
    publish(state);animate(0,true);
    state.row=Some(3);publish(state);animate(100,false);animate(210,false);
    let (hover,tail)=bubble_state();assert!(hover[3]>0 && hover[3]<255);assert_eq!(presentation().1,0);assert_eq!(tail,0);
    let before=hover[3];state.row=Some(1);publish(state);animate(210,false);assert_eq!(bubble_state().0[3],before);
    animate(350,false);assert_eq!(bubble_state().0[1],255);assert_eq!(bubble_state().0[3],0);
    state.row=Some(4);state.selected_row=4;publish(state);animate(601,true);assert_eq!(bubble_state().0[4],255);assert_eq!(bubble_state().1,1024);
    publish(State::new());animate(602,true);
}

#[test]
// ------------------------=
// FUNC: hover_damage_excludes_closed_context_menu
// DESC: Hover repaints only the rail; an opened context menu extends its damage.
// ------------------=
fn hover_damage_excludes_closed_context_menu() {
    let mut state=State::new(); state.set(EDITOR,true); state.hover=Some(EDITOR);
    let g=Geometry::new(1440,900,state);
    let hover=g.damage(state);
    assert_eq!(hover.width,g.rail.width+24);
    assert_eq!(hover.height,g.rail.height+24);
    state.menu=Some(EDITOR);
    let menu=g.damage(state);
    assert!(menu.width>=hover.width);
    assert!(menu.contains(Point{x:g.menu.x,y:g.menu.y}));
}

#[test]
// ------------------------=
// FUNC: settled_hover_damage_is_bounded_to_actions
// DESC: Open-tray hover does not repaint its static header or full-height spine.
// ------------------=
fn settled_hover_damage_is_bounded_to_actions() {
    let mut s=State::new();s.set(EDITOR,true);s.hover=Some(EDITOR);s.expanded=true;
    publish(s);animate(0,true);
    let g=Geometry::new(1440,900,s);let r=g.animation_damage(s);
    assert!(r.y>g.rail.y);
    assert!(r.height<g.rail.height);
    for i in 0..ACTIONS.len() {let a=g.action_rect(i);assert!(r.contains(Point{x:a.x,y:a.y}));assert!(r.contains(Point{x:a.right()-1,y:a.bottom()-1}));}
    publish(State::new());animate(0,true);
}
