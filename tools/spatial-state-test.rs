#[path = "../kernel/ui/session_state.rs"]
mod session_state;
#[path = "../kernel/ui/spatial.rs"]
mod spatial;
use spatial::*;
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
