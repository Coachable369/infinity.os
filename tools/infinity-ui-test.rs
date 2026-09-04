#[path = "../kernel/ui/mod.rs"]
mod ui;

use ui::geometry::{fit_cover, Insets, Point, Rect, Scale, Size};
use ui::input::{ElementId, FocusManager, PointerAccelerator};
use ui::async_model::{AsyncError, AsyncState};
use ui::clipboard::{ClipboardError, ClipboardKind};
use ui::localization::{resolve, StringId, EN_US};
use ui::platform::{CacheBudget, FrameClock};
use ui::scene::{linear_layout, AccessibilityRole, Axis, ElementKind, ElementState, SemanticElement};
use ui::skin::{decode_header, diagnostic_light_skin, encode_header, AppearanceScope, SkinId, SkinRegistry, SkinError};
use ui::system_layout::{
    DesktopTarget, OnboardingTarget, SettingsTarget, SystemLayout, SystemMenuTarget,
};
use ui::trusted::{TrustedSurface, TrustedUiError};
use ui::vector::{semantic_name, validate, IconId, VectorCommand, VectorError, VectorIcon, MAX_VECTOR_COMMANDS};
use ui::window::{ContextId, SurfaceId, WindowError, WindowServer, ZOrderClass};

// ------------------------=
// FUNC: main
// DESC: Runs deterministic InfinityUI parser, layout, input, damage, skin, and isolation acceptance tests.
// ------------------=
fn main() {
    geometry_test();
    skin_test();
    focus_and_pointer_test();
    installed_system_hit_geometry_test();
    scene_and_damage_test();
    window_isolation_test();
    service_foundation_test();
    println!("InfinityUI native runtime: PASS");
}

// ------------------------=
// FUNC: installed_system_hit_geometry_test
// DESC: Proves visible installed-system controls retain exact pointer targets across wide, square, and HiDPI modes.
// ------------------=
fn installed_system_hit_geometry_test() {
    let wide = SystemLayout::new(1920, 1080);
    assert_eq!(wide.onboarding_target(0, 200, 790), Some(OnboardingTarget::Primary));
    assert_eq!(wide.onboarding_target(2, 80, 790), Some(OnboardingTarget::Back));
    assert_eq!(wide.onboarding_target(2, 210, 790), Some(OnboardingTarget::Primary));
    assert_eq!(wide.onboarding_target(2, 200, 450), Some(OnboardingTarget::Input));

    let square = SystemLayout::new(1600, 1600);
    assert_eq!(square.onboarding_target(0, 200, 700), Some(OnboardingTarget::Primary));
    assert_eq!(square.onboarding_target(3, 200, 470), Some(OnboardingTarget::Input));
    assert_eq!(square.authentication_target(200, 420), Some(0));
    assert_eq!(square.authentication_target(200, 500), Some(1));
    assert_eq!(square.authentication_target(200, 560), Some(2));
    assert_eq!(square.desktop_target(30, 20, 30, 500), Some(DesktopTarget::InfinityMenu));
    assert_eq!(square.desktop_target(120, 15, 30, 500), Some(DesktopTarget::TopMenu(1)));
    assert_eq!(square.desktop_target(860, 15, 30, 500), Some(DesktopTarget::Status(0)));
    assert_eq!(square.desktop_target(100, 510, 30, 500), Some(DesktopTarget::HomeTitle));
    assert_eq!(square.desktop_target(260, 970, 30, 500), Some(DesktopTarget::Dock(0)));
    assert_eq!(square.system_menu_target(0, 50, 50), SystemMenuTarget::Item(0));
    assert_eq!(square.system_menu_target(0, 500, 500), SystemMenuTarget::Dismiss);
    assert_eq!(square.settings_target(200, 335), Some(SettingsTarget::Section(0)));
    assert_eq!(square.settings_target(500, 390), Some(SettingsTarget::ContentRow(0)));

    let hidpi = SystemLayout::new(2560, 1440);
    assert_eq!(hidpi.onboarding_target(0, 220, 810), Some(OnboardingTarget::Primary));
    assert_eq!(hidpi.onboarding_target(4, 220, 560), Some(OnboardingTarget::Input));
}

// ------------------------=
// FUNC: geometry_test
// DESC: Verifies required scale factors, stable cover layout, and bounded row geometry.
// ------------------=
fn geometry_test() {
    for scale in [Scale::ONE, Scale::ONE_QUARTER, Scale::ONE_HALF, Scale::TWO] {
        assert!(scale.valid());
    }
    assert_eq!(Scale::TWO.pixels(24), 48);
    let cover = fit_cover(Size { width: 1920, height: 1080 }, Rect { x: 0, y: 0, width: 1536, height: 1024 });
    assert_eq!(cover.height, 1024);
    assert!(cover.width >= 1536);
    let mut rows = [Rect::default(); 4];
    assert_eq!(linear_layout(Rect { x: 0, y: 0, width: 400, height: 100 }, Insets::default(), Axis::Horizontal, 4, 8, &mut rows), 4);
    assert_eq!(rows[1].x - rows[0].right(), 8);
}

// ------------------------=
// FUNC: skin_test
// DESC: Verifies package encoding, corruption rejection, alternate activation, rollback, and SafeSkin fallback.
// ------------------=
fn skin_test() {
    let dark = ui::skin::default_dark_skin();
    let mut header = [0u8; ui::skin::SKIN_HEADER_BYTES];
    encode_header(&dark, &mut header);
    assert_eq!(decode_header(&header, dark.tokens).unwrap().id.as_bytes(), b"infinity.default.dark");
    let mut corrupt = header;
    corrupt[0] = b'X';
    assert!(matches!(decode_header(&corrupt, dark.tokens), Err(SkinError::InvalidMagic)));
    let mut registry = SkinRegistry::new();
    registry.activate(SkinId::from_bytes(b"infinity.diagnostic.light"), AppearanceScope::User).unwrap();
    assert_eq!(registry.active().id.as_bytes(), b"infinity.diagnostic.light");
    registry.rollback();
    assert_eq!(registry.active().id.as_bytes(), b"infinity.default.dark");
    registry.enter_safe_mode();
    assert_eq!(registry.active().id.as_bytes(), b"infinity.safe");
    assert_eq!(registry.register(diagnostic_light_skin()), Err(SkinError::Duplicate));
}

// ------------------------=
// FUNC: focus_and_pointer_test
// DESC: Verifies keyboard traversal wraps and adaptive pointer acceleration preserves small gestures.
// ------------------=
fn focus_and_pointer_test() {
    let mut focus = FocusManager::new();
    focus.register(ElementId(1)).unwrap();
    focus.register(ElementId(2)).unwrap();
    assert_eq!(focus.current(), Some(ElementId(1)));
    assert_eq!(focus.move_next(false), Some(ElementId(2)));
    assert_eq!(focus.move_next(false), Some(ElementId(1)));
    assert_eq!(focus.move_next(true), Some(ElementId(2)));
    let mut pointer = PointerAccelerator::new();
    assert_eq!(pointer.apply(1, -1), Point { x: 1, y: -1 });
    assert!(pointer.apply(12, 12).x > 12);
}

// ------------------------=
// FUNC: scene_and_damage_test
// DESC: Verifies retained hit testing and old-plus-new bounds invalidation without full-screen repaint.
// ------------------=
fn scene_and_damage_test() {
    let mut runtime = ui::InfinityUiRuntime::new();
    runtime.scene.insert(element(1, None, Rect { x: 0, y: 0, width: 800, height: 600 }, 0, 0)).unwrap();
    runtime.scene.insert(element(2, Some(ElementId(1)), Rect { x: 50, y: 50, width: 200, height: 60 }, 1, 2)).unwrap();
    assert_eq!(runtime.scene.hit_test(Point { x: 70, y: 70 }), Some(ElementId(2)));
    runtime.begin_frame();
    runtime.scene.get_mut(ElementId(2)).unwrap().bounds.x = 80;
    let damage = runtime.commit_frame();
    assert!(!damage.is_empty());
    assert!(damage.iter().all(|rect| rect.width < 800 || rect.height < 600));
}

// ------------------------=
// FUNC: element
// DESC: Constructs a compact semantic test element with accessibility metadata.
// ------------------=
fn element(id: u32, parent: Option<ElementId>, bounds: Rect, actions: u16, z_order: i16) -> SemanticElement {
    SemanticElement {
        id: ElementId(id), parent, kind: if id == 1 { ElementKind::Root } else { ElementKind::Button },
        accessibility_role: if id == 1 { AccessibilityRole::Application } else { AccessibilityRole::Button },
        bounds, previous_bounds: bounds, state: ElementState(ElementState::ENABLED), label_id: id,
        action_mask: actions, z_order, visible: true,
    }
}

// ------------------------=
// FUNC: window_isolation_test
// DESC: Proves cross-context mutation and pointer capture denial plus crashed-owner cleanup.
// ------------------=
fn window_isolation_test() {
    let mut server = WindowServer::new();
    let first = server.create(ContextId(1), SurfaceId(11), Rect { x: 0, y: 0, width: 300, height: 200 }, ZOrderClass::Normal).unwrap();
    let second = server.create(ContextId(2), SurfaceId(12), Rect { x: 20, y: 20, width: 300, height: 200 }, ZOrderClass::Floating).unwrap();
    assert!(matches!(server.mutate(ContextId(2), first), Err(WindowError::AccessDenied)));
    assert_eq!(server.capture_pointer(ContextId(2), first), Err(WindowError::AccessDenied));
    assert_eq!(server.hit_test(Point { x: 30, y: 30 }), Some(second));
    let work_area = Rect { x: 0, y: 40, width: 1280, height: 680 };
    assert_eq!(server.capture_pointer(ContextId(1), first), Ok(()));
    assert_eq!(server.move_window(ContextId(1), first, Point { x: 1200, y: 700 }, work_area).unwrap(), Rect { x: 980, y: 520, width: 300, height: 200 });
    assert_eq!(server.resize_window(ContextId(1), first, 500, 300, work_area).unwrap(), Rect { x: 980, y: 520, width: 300, height: 200 });
    assert_eq!(server.release_pointer(ContextId(1)), Ok(()));
    assert_eq!(server.move_window(ContextId(2), first, Point { x: 10, y: 40 }, work_area), Err(WindowError::AccessDenied));
    assert_eq!(server.context_failed(ContextId(2)), 1);
    assert_eq!(server.hit_test(Point { x: 1000, y: 540 }), Some(first));
}

// ------------------------=
// FUNC: service_foundation_test
// DESC: Verifies secure input, bounded clipboard, async cancellation, frame pacing, cache limits, localization, and vectors.
// ------------------=
fn service_foundation_test() {
    let mut runtime = ui::InfinityUiRuntime::new();
    assert_eq!(runtime.clipboard.write(false, 7, ClipboardKind::Utf8Text, b"private"), Err(ClipboardError::AccessDenied));
    runtime.clipboard.write(true, 7, ClipboardKind::Utf8Text, b"typed text").unwrap();
    assert_eq!(&runtime.clipboard.read(true).unwrap().bytes[..10], b"typed text");

    assert_eq!(runtime.trusted.acquire_secure_input(false, 7, TrustedSurface::Authentication, 100), Err(TrustedUiError::AccessDenied));
    let lease = runtime.trusted.acquire_secure_input(true, 7, TrustedSurface::Authentication, 100).unwrap();
    assert_eq!(runtime.trusted.acquire_secure_input(true, 8, TrustedSurface::Lock, 100), Err(TrustedUiError::Busy));
    assert!(runtime.trusted.expire(100));
    assert_eq!(runtime.trusted.release_secure_input(lease), Err(TrustedUiError::AccessDenied));

    let token = runtime.async_tasks.begin(ElementId(9), 1, 100).unwrap();
    let stale = ui::async_model::TaskToken { id: token.id, generation: token.generation + 1 };
    assert_eq!(runtime.async_tasks.complete(stale, 2, None), Err(AsyncError::Stale));
    runtime.async_tasks.complete(token, 3, None).unwrap();
    let pending = runtime.async_tasks.begin(ElementId(10), 4, 100).unwrap();
    assert_eq!(runtime.async_tasks.cancel_owner(ElementId(10)), 1);
    assert!(runtime.async_tasks.complete(pending, 5, None).is_ok());

    let mut clock = FrameClock::new(1);
    assert_eq!(clock.refresh_hz, 30);
    clock.advance(1, 1_000);
    clock.advance(100, 1_000);
    assert!(clock.dropped_frames > 0);
    let mut budget = CacheBudget { total_bytes: 128, used_bytes: 0, evictions: 0 };
    assert!(budget.reserve(96));
    assert!(!budget.reserve(64));
    assert_eq!(budget.evictions, 1);
    budget.release(96);

    assert_eq!(resolve(EN_US, StringId::Welcome), b"Welcome to InfinityOS");
    assert_eq!(semantic_name(IconId::Eye), b"eye");
    let mut commands = [None; MAX_VECTOR_COMMANDS];
    commands[0] = Some(VectorCommand::Move(Point { x: 0, y: 0 }));
    commands[1] = Some(VectorCommand::Line(Point { x: 24, y: 24 }));
    let icon = VectorIcon { id: IconId::Infinity, view_box: Rect { x: 0, y: 0, width: 24, height: 24 }, commands, command_count: 2, stroke_width: 2, filled: false };
    assert_eq!(validate(&icon), Ok(()));
    let empty = VectorIcon { command_count: 0, ..icon };
    assert_eq!(validate(&empty), Err(VectorError::Empty));
    let _ = AsyncState::Cancelled;
}
