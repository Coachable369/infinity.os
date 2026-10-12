#[path = "../kernel/ui/mod.rs"] mod ui;
use ui::geometry::{Point, Rect};
use ui::window::{ContextId, SurfaceId, WindowError, WindowServer, WindowState, ZOrderClass};

// ------------------------=
// FUNC: main
// DESC: Exercises native window stacking, slot reuse, modal input exclusion and minimized capture release.
// ------------------=
fn main() {
    let mut server = WindowServer::new();
    let owner = ContextId(7);
    let area = Rect { x: 0, y: 0, width: 1200, height: 900 };
    let bounds = Rect { x: 10, y: 10, width: 640, height: 480 };
    let point = Point { x: 20, y: 20 };
    let first = server.create(owner, SurfaceId(1), bounds, ZOrderClass::Normal).unwrap();
    server.set_title(owner, first, "Café — native app").unwrap();
    assert_eq!(server.inspect(first).unwrap().title(), "Café — native app");
    assert_eq!(server.set_title(owner, first, "bad\nname"), Err(WindowError::InvalidTitle));
    assert_eq!(server.set_title(ContextId(99), first, "spoof"), Err(WindowError::AccessDenied));
    assert_eq!(server.inspect(first).unwrap().title(), "Café — native app");
    let second = server.create(owner, SurfaceId(2), bounds, ZOrderClass::Normal).unwrap();
    assert_eq!(server.hit_test(point), Some(second));
    server.focus(first).unwrap();
    assert_eq!(server.hit_test(point), Some(first));
    assert_eq!(server.ordered_window(1).unwrap().id, first);
    server.close(owner, second).unwrap();
    let replacement = server.create(owner, SurfaceId(3), bounds, ZOrderClass::Normal).unwrap();
    assert_eq!(server.hit_test(point), Some(replacement));
    server.focus(first).unwrap();
    server.capture_pointer(owner, first).unwrap();
    let resized = server.resize_window(owner, first, 500, 300, area).unwrap();
    server.set_state(owner, first, WindowState::Normal, area, area).unwrap();
    assert_eq!(server.inspect(first).unwrap().bounds, resized);
    server.set_state(owner, first, WindowState::Minimized, area, area).unwrap();
    assert_eq!(server.pointer_target(point), Some(replacement));
    assert_eq!(server.capture_pointer(owner, first), Err(WindowError::AccessDenied));
    server.set_state(owner, first, WindowState::Normal, area, area).unwrap();
    let modal = server.create(owner, SurfaceId(4), bounds, ZOrderClass::Modal).unwrap();
    server.set_modal_parent(owner, modal, first).unwrap();
    assert_eq!(server.set_modal_parent(owner, modal, modal), Err(WindowError::InvalidBounds));
    assert_eq!(server.focus(first), Err(WindowError::ModalBlocked));
    assert_eq!(server.capture_pointer(owner, first), Err(WindowError::ModalBlocked));
    server.focus(modal).unwrap();
    assert_eq!(server.hit_test(point), Some(modal));
    assert_eq!(server.ordered_window(2).unwrap().id, modal);
    assert_eq!(server.close(ContextId(99), modal), Err(WindowError::AccessDenied));
    assert_eq!(server.context_failed(owner), 3);
    assert_eq!(server.count(), 0);
    assert_eq!(server.pointer_target(point), None);
}
