#[path = "../sdk/infinity-browser-core/scrollbars.rs"]
mod scrollbars;
use scrollbars::*;

// ------------------------=
// FUNC: rails
// DESC: Builds an overflowing viewport at a known scroll offset for input assertions.
// ------------------=
fn rails(offset: [f32; 2]) -> [Option<Rail>; 2] {
    layout(Rect { x: 10.0, y: 20.0, width: 320.0, height: 240.0 }, [320.0, 240.0], [640.0, 720.0], offset, 1.0)
}

#[test]
// ------------------------=
// FUNC: rails_fit_without_corner_overlap
// DESC: Checks both axes, proportional minimum thumbs, exact endpoints and stable device-scale bounds.
// ------------------=
fn rails_fit_without_corner_overlap() {
    let [Some(x), Some(y)] = rails([0.0, 0.0]) else { panic!() };
    assert_eq!(x.track.width, 306.0); assert_eq!(y.track.height, 226.0);
    assert_eq!(x.thumb.x, x.track.x); assert_eq!(y.thumb.y, y.track.y);
    assert!(!x.track.contains(y.track.x, y.track.y + y.track.height));
    let end = rails([640.0, 720.0]);
    let bottom = end[1].unwrap();
    assert_eq!(bottom.thumb.y + bottom.thumb.height, bottom.track.y + bottom.track.height);
    let empty = layout(Rect { width: 320.0, height: 240.0, ..Rect::default() }, [320.0, 240.0], [0.0, 0.0], [0.0;2], 1.0);
    assert!(empty.iter().all(Option::is_none));
    assert!(layout(Rect::default(), [1.0;2], [100.0;2], [0.0;2], 1.0).iter().all(Option::is_none));
    let thin = layout_styled(Rect {width:320.0,height:240.0,..Rect::default()}, [320.0,240.0], [640.0,720.0], [0.0;2], 1.0, true);
    assert_eq!(thin[1].unwrap().track.width, 8.0);
    assert_eq!(thin[1].unwrap().track.height, 232.0);
}

#[test]
// ------------------------=
// FUNC: drag_bottom_then_absolute_top
// DESC: Requires exact zero after a bottom-to-top captured drag, without initial pointer jumps or DOM release leakage.
// ------------------=
fn drag_bottom_then_absolute_top() {
    let mut state = Interaction::default();
    let y = rails([0.0;2])[1].unwrap();
    assert!(state.pointer(rails([0.0;2]), Pointer::Down, y.thumb.x+5.0, y.thumb.y+8.0).consumed);
    assert_eq!(state.pointer(rails([0.0;2]), Pointer::Move, y.thumb.x+5.0, y.thumb.y+8.0).scroll, Some((Axis::Vertical, 0.0)));
    assert_eq!(state.pointer(rails([0.0;2]), Pointer::Move, 500.0, 9999.0).scroll, Some((Axis::Vertical, 720.0)));
    assert_eq!(state.pointer(rails([0.0,720.0]), Pointer::Move, 500.0, -100.0).scroll, Some((Axis::Vertical, 0.0)));
    assert!(state.pointer(rails([0.0;2]), Pointer::Up, 500.0, -100.0).consumed);
    assert!(!state.pointer(rails([0.0;2]), Pointer::Move, 150.0, 100.0).consumed);
}

#[test]
// ------------------------=
// FUNC: track_pages_and_hidden_axes_leave_page_input_alone
// DESC: Track clicks page one axis, ordinary page clicks pass through, and release or focus loss clears capture.
// ------------------=
fn track_pages_and_hidden_axes_leave_page_input_alone() {
    let mut state = Interaction::default();
    assert!(!state.pointer(rails([0.0;2]), Pointer::Down, 100.0, 100.0).consumed);
    let y = rails([0.0;2])[1].unwrap();
    assert_eq!(state.pointer(rails([0.0;2]), Pointer::Down, y.track.x+5.0, y.track.y+200.0).scroll, Some((Axis::Vertical, 216.0)));
    assert!(state.pointer(rails([0.0,216.0]), Pointer::Up, 100.0, 100.0).consumed);
    assert!(state.pressed.is_none());
    state.pointer(rails([0.0;2]), Pointer::Down, y.thumb.x+5.0, y.thumb.y+8.0);
    state.pointer(rails([0.0;2]), Pointer::Leave, 0.0, 0.0);
    assert!(state.pressed.is_none());
    state.pointer(rails([0.0;2]), Pointer::Cancel, 0.0, 0.0);
    assert!(!state.pointer([None,None], Pointer::Down, y.thumb.x+5.0, y.thumb.y+8.0).consumed);
}

#[test]
// ------------------------=
// FUNC: horizontal_and_resize_use_fresh_metrics
// DESC: Horizontal capture keeps its axis and clamps through resized tracks without invalid geometry.
// ------------------=
fn horizontal_and_resize_use_fresh_metrics() {
    let mut state = Interaction::default();
    let x = rails([0.0;2])[0].unwrap();
    state.pointer(rails([0.0;2]), Pointer::Down, x.thumb.x+4.0, x.thumb.y+4.0);
    assert_eq!(state.pointer(rails([0.0;2]), Pointer::Move, 9999.0, 0.0).scroll, Some((Axis::Horizontal,640.0)));
    let resized = layout(Rect {width:160.0,height:120.0,..Rect::default()}, [80.0,60.0], [880.0,900.0], [640.0,0.0], 1.0);
    assert_eq!(state.pointer(resized, Pointer::Move, -20.0, 0.0).scroll, Some((Axis::Horizontal,0.0)));
    assert_eq!(resized[0].unwrap().track.height, 14.0);
}
