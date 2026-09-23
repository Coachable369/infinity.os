#[path = "../kernel/ui/session_state.rs"]
mod session_state;
#[path = "../kernel/ui/spatial.rs"]
mod spatial;
use spatial::*;
#[test]
// ------------------------=
// FUNC: collection_controls_share_drop_geometry_and_preserve_sources
// DESC: Tests destination boundaries, non-mutating previews, and confirmed metadata-only organization.
// ------------------=
fn collection_controls_share_drop_geometry_and_preserve_sources() {
    let owner = [1; 16];
    let mut state = SpatialState::new(owner);
    let index = state.gather(owner, clipping()).unwrap();
    let source = state.items[index].unwrap();
    for group in 0..WORLD_COUNT {
        let (x, y, w, h) = collection_card(group);
        assert_eq!(collection_hit(x as i32, y as i32), Some(group as u8));
        assert_eq!(
            collection_hit((x + w - 1) as i32, (y + h - 1) as i32),
            Some(group as u8)
        );
        assert_eq!(collection_hit((x + w) as i32, y as i32), None);
        assert_eq!(collection_hit(x as i32, (y + h) as i32), None);
        let before = state;
        let request = DropRequest::new(&state, index, DropTarget::Collection(group as u8)).unwrap();
        assert_eq!(state, before);
        let DropTarget::Collection(destination) = request.target else {
            panic!()
        };
        state
            .place(owner, index, destination, x.min(710) as u16, 230)
            .unwrap();
        let result = state.items[index].unwrap();
        assert_eq!(result.collection, group as u8);
        assert_eq!(
            (result.object, result.path, result.text),
            (source.object, source.path, source.text)
        );
    }
    assert_eq!(collection_hit(-1, 750), None);
    assert_eq!(collection_hit(1000, 750), None);
}
#[test]
// ------------------------=
// FUNC: carousel_endpoints_retarget_depth_and_activation
// DESC: Exercises all window counts and focus pairs, visible-position retargeting, matching depth hits, and second-click activation.
// ------------------=
fn carousel_endpoints_retarget_depth_and_activation() {
    for zoom in [0, 128, 255] {
        for count in 1..=OVERVIEW_COUNT {
            for first in 0..count {
                let initial = OverviewFrame::new(None, first, zoom, count, 255);
                assert_eq!(initial.order[count - 1], first);
                for next in 0..count {
                    let start = OverviewFrame::new(Some(&initial.bounds), next, zoom, count, 0);
                    assert_eq!(start.bounds, initial.bounds);
                    let end = OverviewFrame::new(Some(&initial.bounds), next, zoom, count, 255);
                    assert_eq!(
                        end.bounds,
                        OverviewFrame::new(None, next, zoom, count, 255).bounds
                    );
                    assert_eq!(end.order[count - 1], next);
                    for progress in [1, 64, 128, 192, 254] {
                        let frame =
                            OverviewFrame::new(Some(&initial.bounds), next, zoom, count, progress);
                        let retarget =
                            OverviewFrame::new(Some(&frame.bounds), first, zoom, count, 0);
                        assert_eq!(frame.bounds, retarget.bounds);
                        let front = frame.order[count - 1];
                        let (x, y, w, h) = frame.bounds[front];
                        assert_eq!(
                            frame.hit((x + w / 2) as i32, (y + h / 2) as i32),
                            Some(front)
                        );
                        assert!(!overview_activates(next, next, true));
                    }
                    assert_eq!(overview_activates(next, first, false), next == first);
                    // Native-resolution card atlases remain bounded at supported 4K size.
                    let pixels: usize = (0..count)
                        .map(|i| {
                            let a = initial.bounds[i];
                            let b = end.bounds[i];
                            (a.2.max(b.2) * 3840 / 1000 + 24) * (a.3.max(b.3) * 2160 / 1000 + 24)
                        })
                        .sum();
                    assert!(pixels <= 12 * 1024 * 1024);
                }
            }
        }
    }
}
#[test]
// ------------------------=
// FUNC: queued_refresh_waits_for_transition_then_runs
// DESC: Verifies moving and closing scenes retain their pixels without discarding pending refresh work.
// ------------------=
fn queued_refresh_waits_for_transition_then_runs() {
    for elapsed in [0, 99, 100, 220, 1000] {
        assert!(!refresh_due(true, elapsed, true, false));
        assert!(!refresh_due(true, elapsed, false, true));
        assert!(!refresh_due(false, elapsed, false, false));
        assert_eq!(refresh_due(true, elapsed, false, false), elapsed >= 100);
    }
}
#[test]
// ------------------------=
// FUNC: depth_staged_overview_keeps_every_window_reachable
// DESC: Checks containment and exposed hit areas for all small-session stack orders and zoom levels.
// ------------------=
fn depth_staged_overview_keeps_every_window_reachable() {
    for count in 1..=5 {
        for focus in 0..count {
            for zoom in [0, 128, 255] {
                let mut exposed = [false; 5];
                for y in (230..780).step_by(5) {
                    for x in (80..920).step_by(5) {
                        let contains = |index| {
                            let (a, b, w, h) = overview_bounds(index, focus, zoom, count);
                            assert!(a >= 80 && a + w <= 920 && b >= 230 && b + h < 790);
                            x >= a && x < a + w && y >= b && y < b + h
                        };
                        let hit = if contains(focus) {
                            Some(focus)
                        } else {
                            (0..count).find(|&index| contains(index))
                        };
                        if let Some(index) = hit {
                            exposed[index] = true;
                        }
                    }
                }
                assert!(exposed[..count].iter().all(|&visible| visible));
            }
        }
    }
}
#[test]
// ------------------------=
// FUNC: keyboard_navigation_skips_removed_references
// DESC: Exercises sparse collections, reverse traversal, wrapping and the empty state.
// ------------------=
fn keyboard_navigation_skips_removed_references() {
    let mut state = SpatialState::new([1; 16]);
    assert_eq!(next_reference(&state, 15, false), 0);
    state.items[3] = Some(clipping());
    state.items[11] = Some(clipping());
    assert_eq!(next_reference(&state, 0, false), 3);
    assert_eq!(next_reference(&state, 3, false), 11);
    assert_eq!(next_reference(&state, 11, false), 3);
    assert_eq!(next_reference(&state, 3, true), 11);
    state.items[3] = None;
    assert_eq!(next_reference(&state, 11, false), 11);
}
#[test]
// ------------------------=
// FUNC: coalesced_drag_release_preserves_final_placement
// DESC: Verifies final position is identical with zero or many held samples and remains inside the interactive stage.
// ------------------=
fn coalesced_drag_release_preserves_final_placement() {
    let origin = (290, 230);
    let press = (385, 270);
    let release = (600, 520);
    let direct = drag_position(origin, press, release);
    assert_eq!(direct, (505, 480));
    for steps in 1..32 {
        for step in 0..steps {
            let _ = drag_position(
                origin,
                press,
                (385 + 215 * step / steps, 270 + 250 * step / steps),
            );
        }
        assert_eq!(drag_position(origin, press, release), direct);
    }
    assert_eq!(
        drag_position(origin, press, (i32::MIN, i32::MAX)),
        (80, 620)
    );
}
#[test]
// ------------------------=
// FUNC: independent_overview_fits_every_window_and_keeps_zoom_bounded
// DESC: Verifies ten independent identities fit above actions and focused zoom preserves surrounding cards.
// ------------------=
fn independent_overview_fits_every_window_and_keeps_zoom_bounded() {
    for count in 6..=OVERVIEW_COUNT {
        for index in 0..count {
            let r = overview_bounds(index, 0, 0, count);
            assert!(r.0 >= 80 && r.0 + r.2 <= 920);
            assert!(r.1 >= 230 && r.1 + r.3 < 805);
            for other in index + 1..count {
                let s = overview_bounds(other, 0, 0, count);
                assert!(
                    r.0 + r.2 <= s.0 || s.0 + s.2 <= r.0 || r.1 + r.3 <= s.1 || s.1 + s.3 <= r.1
                );
            }
            assert_eq!(
                overview_bounds(index, index, 255, count),
                (130, 230, 740, 445)
            );
            if index != 0 {
                assert_eq!(overview_bounds(index, 0, 255, count), r);
            }
        }
    }
}
#[test]
// ------------------------=
// FUNC: drop_proposals_are_non_mutating_and_reject_invalid_destinations
// DESC: Exercises confirmation staging without modifying references or source identities.
// ------------------=
fn drop_proposals_are_non_mutating_and_reject_invalid_destinations() {
    let mut state = SpatialState::new([1; 16]);
    let index = state.gather([1; 16], clipping()).unwrap();
    let before = state;
    let request = DropRequest::new(&state, index, DropTarget::Collection(3)).unwrap();
    assert_eq!(request.index, index);
    assert_eq!(state, before);
    assert!(DropRequest::new(&state, index, DropTarget::Collection(4)).is_none());
    assert!(DropRequest::new(&state, 16, DropTarget::Editor).is_none());
    assert!(DropRequest::new(&state, 1, DropTarget::Editor).is_none());
    let mut folder = Label::empty();
    folder.set(b"/home/default");
    assert!(DropRequest::new(&state, index, DropTarget::Folder(folder)).is_none());
    state.items[index].as_mut().unwrap().object = [7; 16];
    assert!(DropRequest::new(&state, index, DropTarget::Folder(folder)).is_some());
    folder.set(b"relative");
    assert!(DropRequest::new(&state, index, DropTarget::Folder(folder)).is_none());
}
#[test]
// ------------------------=
// FUNC: shelf_pages_keep_gutters_and_hidden_items_noninteractive
// DESC: Checks shared rendering and hit-test geometry for every shelf page and boundary.
// ------------------=
fn shelf_pages_keep_gutters_and_hidden_items_noninteractive() {
    for focus in 0..16 {
        let mut visible = 0;
        for index in 0..16 {
            if let Some(rect) = shelf_card(index, focus) {
                visible += 1;
                assert_eq!(index / 4, focus / 4);
                assert!(contains(rect, rect.0 as i32, 690));
                assert!(!contains(rect, (rect.0 + rect.2) as i32, 690));
                assert!(!contains(rect, rect.0 as i32, 790));
                assert!(rect.1 >= 690 && rect.1 + rect.3 < 805);
            }
        }
        assert_eq!(visible, 4);
    }
    assert_eq!(shelf_card(16, 16), None);
}
// ------------------------=
// FUNC: clipping
// DESC: Builds a deliberately collected text clipping for behavioral tests.
// ------------------=
fn clipping() -> Item {
    let mut name = Label::empty();
    assert!(name.set(b"Research"));
    let mut text = Label::empty();
    assert!(text.set(b"Collected deliberately"));
    Item {
        object: [0; 16],
        path: Label::empty(),
        name,
        text,
        collection: 0,
        x: 300,
        y: 400,
        links: 0,
    }
}
#[test]
// ------------------------=
// FUNC: ownership_relationships_and_roundtrip
// DESC: Verifies cross-user denial, symmetric links, non-destructive removal and durable reload.
// ------------------=
fn ownership_relationships_and_roundtrip() {
    let owner = [1; 16];
    let mut state = SpatialState::new(owner);
    assert_eq!(state.gather([2; 16], clipping()), Err(Error::Owner));
    let a = state.gather(owner, clipping()).unwrap();
    let b = state.gather(owner, clipping()).unwrap();
    state.connect(owner, a, b).unwrap();
    state.place(owner, b, 3, 900, 100).unwrap();
    assert_eq!(state.items[a].unwrap().links, 1 << b);
    assert_eq!(state.items[b].unwrap().links, 1 << a);
    let bytes = state.encode(owner).unwrap();
    assert_eq!(SpatialState::decode(owner, &bytes), Ok(state));
    assert_eq!(SpatialState::decode([2; 16], &bytes), Err(Error::Owner));
    state.remove(owner, a).unwrap();
    assert_eq!(state.items[b].unwrap().links, 0);
    let before = state;
    assert_eq!(state.place(owner, b, 4, 0, 0), Err(Error::Invalid));
    assert_eq!(state, before);
    let mut corrupt = bytes;
    corrupt[500] ^= 1;
    assert_eq!(SpatialState::decode(owner, &corrupt), Err(Error::Corrupt));
    assert_eq!(
        SpatialState::decode(owner, &bytes[..100]),
        Err(Error::Corrupt)
    );
}
#[test]
// ------------------------=
// FUNC: capacity_and_clear_are_bounded
// DESC: Verifies fixed resource limits, explicit removal and slot reuse without overwriting content.
// ------------------=
fn capacity_and_clear_are_bounded() {
    let owner = [1; 16];
    let mut state = SpatialState::new(owner);
    for i in 0..ITEM_COUNT {
        assert_eq!(state.gather(owner, clipping()), Ok(i));
    }
    let before = state;
    assert_eq!(state.gather(owner, clipping()), Err(Error::Capacity));
    assert_eq!(state, before);
    state.remove(owner, 7).unwrap();
    assert_eq!(state.gather(owner, clipping()), Ok(7));
    assert_eq!(state.connect(owner, 1, 1), Err(Error::Missing));
    assert!(!Label::<4>::empty().set(b"oversized"));
}

#[test]
// ------------------------=
// FUNC: geometry_matches_hits_and_clamps_placement
// DESC: Tests actual card geometry at zoom endpoints, gutters, free placement and z-order.
// ------------------=
fn geometry_matches_hits_and_clamps_placement() {
    for zoom in [0, 64, 128, 255] {
        for focus in 0..5 {
            for i in 0..5 {
                let rect = overview_card(i, focus, zoom);
                assert!(contains(rect, rect.0 as i32 + 1, rect.1 as i32 + 1));
                assert!(!contains(rect, (rect.0 + rect.2) as i32, rect.1 as i32));
                assert!(rect.0 + rect.2 <= 900 && rect.1 + rect.3 <= 740);
            }
        }
    }
    for i in 0..4 {
        let rect = world_card(i);
        assert!(!contains(rect, (rect.0 + rect.2 + 1) as i32, rect.1 as i32));
    }
    let mut state = SpatialState::new([1; 16]);
    let a = state.gather([1; 16], clipping()).unwrap();
    let b = state.gather([1; 16], clipping()).unwrap();
    assert_eq!(hit_item(&state, 301, 401), Some(b));
    state.remove([1; 16], b).unwrap();
    assert_eq!(hit_item(&state, 301, 401), Some(a));
    state.place([1; 16], a, 2, 1000, 1000).unwrap();
    assert_eq!(item_card(a, &state.items[a].unwrap()), (710, 620, 190, 110));
    assert_eq!(hit_item(&state, 79, 229), None);
}

#[test]
// ------------------------=
// FUNC: world_layout_survives_roundtrip
// DESC: Verifies per-world app visibility, placement, identity label and navigation state survive serialization.
// ------------------=
fn world_layout_survives_roundtrip() {
    use session_state::{DesktopResumeSurface, DesktopSessionLayout, WindowPlacement};
    let mut state = SpatialState::new([3; 16]);
    let placement = WindowPlacement::new(100, 120, 600, 500, false, true);
    let layout = DesktopSessionLayout {
        home: placement,
        editor: placement,
        command: placement,
        task_manager: placement,
        settings: placement,
        desktop_item_positions: [[0; 2]; 7],
        focused_surface: DesktopResumeSurface::TextEditor,
        settings_section: 0,
        settings_expanded_row: None,
        settings_scroll_offset: 0,
        input_preferences: [0; 8],
    };
    state.worlds[2].layout = Some(layout);
    state.worlds[2].name.set(b"Research");
    state.worlds[2].location.set(b"/home/default/documents");
    state.worlds[2]
        .editor
        .set(b"/home/default/documents/research.txt");
    state.active_world = 2;
    state.reduced_motion = true;
    let bytes = state.encode([3; 16]).unwrap();
    assert_eq!(SpatialState::decode([3; 16], &bytes), Ok(state));
    state.active_world = 4;
    assert_eq!(state.encode([3; 16]), Err(Error::Invalid));
}
