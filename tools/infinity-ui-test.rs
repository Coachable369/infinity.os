#[path = "../kernel/ui/mod.rs"]
mod ui;

use ui::app_launcher::{
    launcher_visible_count, launcher_visible_entry, DockAction, LauncherAction,
    DESKTOP_DOCK_ENTRIES, LAUNCHER_APPS,
};
use ui::async_model::{AsyncError, AsyncState};
use ui::clipboard::{ClipboardError, ClipboardKind};
use ui::compositor::{CompositorError, SoftwareCompositor, SurfaceFrame};
use ui::geometry::{fit_cover, Insets, Point, Rect, Scale, Size};
use ui::input::{ElementId, FocusManager, PointerAccelerator};
use ui::input_router::{InputDestination, InputRouter};
use ui::localization::{resolve, StringId, EN_US};
use ui::platform::{AdaptiveQualityController, CacheBudget, FrameClock, QualityLevel};
use ui::scene::{
    linear_layout, AccessibilityRole, Axis, DamageClass, DamageRecord, DamageTracker, ElementKind,
    ElementState, SemanticElement, MAX_DAMAGE_REGIONS,
};
use ui::skin::{
    decode_header, diagnostic_light_skin, encode_header, hsv_to_rgb, rgb_to_hsv, AccentSurface,
    AppearanceScope, ColorRole, SkinError, SkinId, SkinRegistry,
};
use ui::surface::{PixelFormat, SurfaceError, SurfaceRegistry, SurfaceSecurityClass};
use ui::system_layout::{
    resize_home_window, resize_native_window, window_transition_damage, AiChatTarget,
    AppLauncherTarget, DesktopAppWindowTarget, DesktopTarget, EditorDialogTarget,
    EditorScrollTarget, OnboardingTarget, SettingsAccentTarget, SettingsTarget,
    SettingsWindowState, SystemLayout, SystemMenuTarget, DESKTOP_FOREGROUND_DOCK,
    DESKTOP_FOREGROUND_WIDGETS,
};
use ui::text_editor::{document_path, visual_line_count, visual_line_start, TextDocument};
use ui::trusted::{TrustedSurface, TrustedUiError};
use ui::vector::{
    semantic_name, validate, IconId, VectorCommand, VectorError, VectorIcon, MAX_VECTOR_COMMANDS,
};
use ui::window::{
    ContextId, SurfaceId, WindowError, WindowEventKind, WindowServer, WindowState, ZOrderClass,
    MAX_WINDOW_EVENTS,
};

// ------------------------=
// FUNC: main
// DESC: Runs deterministic InfinityUI parser, layout, input, damage, skin, and isolation acceptance tests.
// ------------------=
fn main() {
    geometry_test();
    skin_test();
    focus_and_pointer_test();
    installed_system_hit_geometry_test();
    app_launcher_behavior_test();
    desktop_foreground_damage_test();
    desktop_ai_chat_layout_test();
    window_move_composition_test();
    independent_window_state_test();
    scene_and_damage_test();
    surface_and_compositor_test();
    semantic_damage_storm_test();
    context_recovery_test();
    window_server_restart_test();
    window_isolation_test();
    drag_path_test();
    service_foundation_test();
    println!("InfinityUI native runtime: PASS");
}

// ------------------------=
// FUNC: independent_window_state_test
// DESC: Verifies moving and resizing the focused window cannot mutate the retained geometry of another layered window.
// ------------------=
fn independent_window_state_test() {
    let editor = ui::system_layout::DesktopAppWindowState::new(140, 170, 520, 560);
    let command = ui::system_layout::DesktopAppWindowState::new(290, 250, 460, 440);
    let moved_editor = ui::system_layout::DesktopAppWindowState {
        x: 330,
        y: 120,
        width: 610,
        height: 650,
        ..editor
    };
    assert_eq!(
        (command.x, command.y, command.width, command.height),
        (290, 250, 460, 440)
    );
    assert_ne!(
        (
            moved_editor.x,
            moved_editor.y,
            moved_editor.width,
            moved_editor.height
        ),
        (editor.x, editor.y, editor.width, editor.height)
    );
    let display = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let damage = window_transition_damage(
        Rect {
            x: editor.x,
            y: editor.y,
            width: editor.width as u32,
            height: editor.height as u32,
        },
        Rect {
            x: moved_editor.x,
            y: moved_editor.y,
            width: moved_editor.width as u32,
            height: moved_editor.height as u32,
        },
        display,
        16,
    );
    assert!(damage.contains(Point {
        x: editor.x,
        y: editor.y,
    }));
    assert!(damage.contains(Point {
        x: moved_editor.x + moved_editor.width - 1,
        y: moved_editor.y + moved_editor.height - 1,
    }));
}

// ------------------------=
// FUNC: window_move_composition_test
// DESC: Verifies bounded old-plus-new reconstruction clears stale pixels and restores foreground layers over a moved app.
// ------------------=
fn window_move_composition_test() {
    let display = Rect {
        x: 0,
        y: 0,
        width: 12,
        height: 6,
    };
    let old = Rect {
        x: 1,
        y: 1,
        width: 4,
        height: 3,
    };
    let new = Rect {
        x: 5,
        y: 1,
        width: 4,
        height: 3,
    };
    let damage_rect = window_transition_damage(old, new, display, 0);
    assert_eq!(
        damage_rect,
        Rect {
            x: 1,
            y: 1,
            width: 8,
            height: 3,
        }
    );
    assert!(damage_rect.width < display.width || damage_rect.height < display.height);

    let mut surfaces = SurfaceRegistry::new(1024);
    let app_id = surfaces
        .create(
            ContextId(11),
            Size {
                width: 4,
                height: 3,
            },
            PixelFormat::Xrgb8888,
            SurfaceSecurityClass::Application,
            false,
        )
        .unwrap();
    let foreground_id = surfaces
        .create(
            ContextId(1),
            Size {
                width: 5,
                height: 1,
            },
            PixelFormat::Xrgb8888,
            SurfaceSecurityClass::System,
            true,
        )
        .unwrap();
    let app_pixels = [0xff22_66aa; 12];
    let foreground_pixels = [0xffee_cc44; 5];
    let foreground = SurfaceFrame {
        descriptor: *surfaces.inspect(foreground_id).unwrap(),
        pixels: &foreground_pixels,
        bounds: Rect {
            x: 3,
            y: 2,
            width: 5,
            height: 1,
        },
        opacity: 255,
        z_class: ZOrderClass::Floating,
        visible: true,
    };
    let mut compositor = SoftwareCompositor::new(
        Size {
            width: 12,
            height: 6,
        },
        12,
        0xff00_0000,
    );
    let mut back = [0u32; 72];
    let mut front = [0u32; 72];
    let mut initial_damage = DamageTracker::new();
    initial_damage.add_semantic(display, DamageClass::Geometry, 0, 100);
    compositor
        .compose(
            &mut back,
            &[
                SurfaceFrame {
                    descriptor: *surfaces.inspect(app_id).unwrap(),
                    pixels: &app_pixels,
                    bounds: old,
                    opacity: 255,
                    z_class: ZOrderClass::Normal,
                    visible: true,
                },
                foreground,
            ],
            &initial_damage,
        )
        .unwrap();
    compositor
        .present(&mut front, &back, &initial_damage)
        .unwrap();

    let pixels_before_move = compositor.metrics().presented_pixels;
    let mut move_damage = DamageTracker::new();
    move_damage.add_semantic(damage_rect, DamageClass::Geometry, app_id.0, 180);
    compositor
        .compose(
            &mut back,
            &[
                SurfaceFrame {
                    descriptor: *surfaces.inspect(app_id).unwrap(),
                    pixels: &app_pixels,
                    bounds: new,
                    opacity: 255,
                    z_class: ZOrderClass::Normal,
                    visible: true,
                },
                foreground,
            ],
            &move_damage,
        )
        .unwrap();
    compositor.present(&mut front, &back, &move_damage).unwrap();

    assert_eq!(front[1 * 12 + 1], 0xff00_0000);
    assert_eq!(front[1 * 12 + 8], 0xff22_66aa);
    assert_eq!(front[2 * 12 + 4], 0xffee_cc44);
    assert_eq!(front[2 * 12 + 7], 0xffee_cc44);
    assert_eq!(
        compositor.metrics().presented_pixels - pixels_before_move,
        u64::from(damage_rect.width) * u64::from(damage_rect.height)
    );
}

// ------------------------=
// FUNC: desktop_foreground_damage_test
// DESC: Verifies moved-window damage identifies the dock and right widgets as persistent foreground layers.
// ------------------=
fn desktop_foreground_damage_test() {
    let layout = SystemLayout::new(1920, 1080);
    let foreground = layout.desktop_foreground_geometry();
    assert_eq!(
        layout.desktop_foreground_layers_for_rect(Rect {
            x: 200,
            y: 200,
            width: 300,
            height: 300,
        }),
        0
    );
    assert_eq!(
        layout.desktop_foreground_layers_for_rect(Rect {
            x: foreground.widgets.x - 10,
            y: foreground.widgets.y + 20,
            width: 40,
            height: 40,
        }),
        DESKTOP_FOREGROUND_WIDGETS
    );
    assert_eq!(
        layout.desktop_foreground_layers_for_rect(Rect {
            x: foreground.dock.x + 20,
            y: foreground.dock.y - 10,
            width: 40,
            height: 40,
        }),
        DESKTOP_FOREGROUND_DOCK
    );
    assert_eq!(
        layout.desktop_foreground_layers_for_rect(Rect {
            x: foreground.widgets.x,
            y: foreground.widgets.y,
            width: foreground.widgets.width,
            height: (foreground.dock.bottom() - foreground.widgets.y) as u32,
        }),
        DESKTOP_FOREGROUND_WIDGETS | DESKTOP_FOREGROUND_DOCK
    );
}

// ------------------------=
// FUNC: desktop_ai_chat_layout_test
// DESC: Verifies responsive chat geometry and pointer targeting for every interactive control.
// ------------------=
fn desktop_ai_chat_layout_test() {
    let layout = SystemLayout::new(1920, 1080);
    let geometry = layout.ai_chat_geometry(false);
    let foreground = layout.desktop_foreground_geometry();
    assert!(geometry.panel.height > 300);
    assert!(geometry.panel.bottom() <= foreground.widgets.bottom());
    assert!(geometry.timeline.bottom() <= geometry.composer.y);

    let normalized = |rect: Rect| {
        (
            (rect.x + rect.width as i32 / 2) * 1000 / 1920,
            (rect.y + rect.height as i32 / 2) * 1000 / 1080,
        )
    };
    for (rect, target) in [
        (geometry.model, AiChatTarget::Model),
        (geometry.timeline, AiChatTarget::Timeline),
        (geometry.composer, AiChatTarget::Composer),
        (geometry.send, AiChatTarget::Send),
        (geometry.minimize, AiChatTarget::Minimize),
        (geometry.close, AiChatTarget::Close),
    ] {
        let (x, y) = normalized(rect);
        assert_eq!(layout.ai_chat_target(x, y, false), Some(target));
    }

    let minimized = layout.ai_chat_geometry(true);
    assert!(minimized.panel.height < geometry.panel.height);
    let (model_x, model_y) = normalized(minimized.model);
    assert_eq!(layout.ai_chat_target(model_x, model_y, true), None);
    let (close_x, close_y) = normalized(minimized.close);
    assert_eq!(
        layout.ai_chat_target(close_x, close_y, true),
        Some(AiChatTarget::Close)
    );
    assert!(ui::redraw::desktop_chat_content_requires_bounded_redraw(
        2, true
    ));
    assert!(!ui::redraw::desktop_chat_content_requires_bounded_redraw(
        4, true
    ));
}

// ------------------------=
// FUNC: app_launcher_behavior_test
// DESC: Verifies typed search filtering and shared pointer geometry for the installed native launcher.
// ------------------=
fn app_launcher_behavior_test() {
    assert_eq!(launcher_visible_count(b"sett"), 1);
    assert_eq!(
        launcher_visible_entry(b"SETT", 0).map(|entry| entry.action),
        Some(LauncherAction::Settings(0))
    );
    assert_eq!(launcher_visible_count(b"not-an-installed-app"), 0);
    assert_eq!(
        LAUNCHER_APPS
            .iter()
            .filter(|entry| entry.action == LauncherAction::TextEditor)
            .count(),
        1
    );
    assert_eq!(DESKTOP_DOCK_ENTRIES.len(), 8);
    assert_eq!(DESKTOP_DOCK_ENTRIES[0].action, DockAction::Launcher);
    assert_eq!(DESKTOP_DOCK_ENTRIES[1].action, DockAction::Files);
    assert_eq!(DESKTOP_DOCK_ENTRIES[7].action, DockAction::Trash);
    assert_eq!(
        LAUNCHER_APPS
            .iter()
            .filter(|entry| entry.action == LauncherAction::CommandWindow)
            .count(),
        1
    );

    let layout = SystemLayout::new(1920, 1080);
    let geometry = layout.app_launcher_geometry();
    let normalized = |x: i32, y: i32| (x * 1000 / 1920, y * 1000 / 1080);
    let (search_x, search_y) = normalized(
        geometry.search.x + geometry.search.width as i32 / 2,
        geometry.search.y + geometry.search.height as i32 / 2,
    );
    assert_eq!(
        layout.app_launcher_target(search_x, search_y, 12),
        AppLauncherTarget::Search
    );
    let (app_x, app_y) = normalized(
        geometry.grid_left as i32 + geometry.grid_cell_width as i32 / 2,
        geometry.grid_top as i32 + geometry.grid_row_height as i32 / 2,
    );
    assert_eq!(
        layout.app_launcher_target(app_x, app_y, 12),
        AppLauncherTarget::App(0)
    );
    let (category_x, category_y) = normalized(
        geometry.category_left as i32 + geometry.category_width as i32 / 2,
        geometry.category_top as i32 + geometry.category_height as i32 / 2,
    );
    assert_eq!(
        layout.app_launcher_target(category_x, category_y, 12),
        AppLauncherTarget::Category(0)
    );
    assert_eq!(
        layout.app_launcher_target(1, 500, 12),
        AppLauncherTarget::Dismiss
    );
    assert_eq!(
        layout.app_launcher_target(240, 960, 12),
        AppLauncherTarget::DockToggle
    );
    let (close_x, close_y) = normalized(
        geometry.close.x + geometry.close.width as i32 / 2,
        geometry.close.y + geometry.close.height as i32 / 2,
    );
    assert_eq!(
        layout.app_launcher_target(close_x, close_y, 12),
        AppLauncherTarget::Close
    );
    let (close_margin_x, close_margin_y) =
        normalized((geometry.close.x - 4).max(0), (geometry.close.y - 4).max(0));
    assert_eq!(
        layout.app_launcher_target(close_margin_x, close_margin_y, 12),
        AppLauncherTarget::Close
    );

    let app_window = layout.desktop_app_window_geometry(190, 160, 600, 620, false);
    let editor_state = ui::system_layout::DesktopAppWindowState::new(190, 160, 600, 620);
    let command_state = ui::system_layout::DesktopAppWindowState::new(260, 230, 520, 500);
    assert_ne!(
        layout
            .desktop_app_window_geometry(
                editor_state.x,
                editor_state.y,
                editor_state.width,
                editor_state.height,
                editor_state.maximized,
            )
            .window,
        layout
            .desktop_app_window_geometry(
                command_state.x,
                command_state.y,
                command_state.width,
                command_state.height,
                command_state.maximized,
            )
            .window
    );
    for (target, control) in [
        (DesktopAppWindowTarget::Minimize, app_window.minimize),
        (DesktopAppWindowTarget::Maximize, app_window.maximize),
        (DesktopAppWindowTarget::Close, app_window.close),
    ] {
        let (control_x, control_y) = normalized(
            control.x + control.width as i32 / 2,
            control.y + control.height as i32 / 2,
        );
        assert_eq!(
            layout
                .desktop_app_window_target(control_x, control_y, 190, 160, 600, 620, false, true,),
            target
        );
    }
    let toolbar = layout.desktop_toolbar_geometry(app_window);
    let toolbar_targets = [
        DesktopAppWindowTarget::NewDocument,
        DesktopAppWindowTarget::OpenDocument,
        DesktopAppWindowTarget::SaveDocument,
        DesktopAppWindowTarget::SaveAsDocument,
        DesktopAppWindowTarget::DeleteDocument,
    ];
    for (index, action) in toolbar.actions.iter().enumerate() {
        assert_eq!(action.width, toolbar.actions[0].width);
        assert_eq!(action.height, toolbar.actions[0].height);
        assert_eq!(
            action.height as usize,
            ui::system_layout::UI_TOOLBAR_ACTION_HEIGHT * layout.scale()
        );
        assert!(app_window.toolbar.contains(Point {
            x: action.x,
            y: action.y,
        }));
        assert!(action.right() <= toolbar.status.x);
        let (action_x, action_y) = normalized(
            action.x + action.width as i32 / 2,
            action.y + action.height as i32 / 2,
        );
        assert_eq!(
            layout.desktop_app_window_target(action_x, action_y, 190, 160, 600, 620, false, true,),
            toolbar_targets[index]
        );
    }
    assert!(toolbar.status.right() <= app_window.toolbar.right());

    let sheet_width = 420;
    let sheet_height = 220;
    let sheet_left =
        app_window.content.x + (app_window.content.width as i32 - sheet_width as i32) / 2;
    let sheet_top =
        app_window.content.y + (app_window.content.height as i32 - sheet_height as i32) / 2;
    let sheet_button_top = sheet_top + sheet_height as i32 - 60;
    let sheet_button_width = (sheet_width - 60) / 2;
    for (target, center_x) in [
        (
            EditorDialogTarget::Cancel,
            sheet_left + 24 + sheet_button_width as i32 / 2,
        ),
        (
            EditorDialogTarget::Accept,
            sheet_left
                + 24
                + ui::system_layout::UI_CONTROL_GAP as i32
                + sheet_button_width as i32
                + sheet_button_width as i32 / 2,
        ),
    ] {
        let (action_x, action_y) = normalized(
            center_x,
            sheet_button_top + ui::system_layout::UI_COMPACT_ACTION_HEIGHT as i32 / 2,
        );
        assert_eq!(
            layout.desktop_editor_dialog_target(
                action_x, action_y, 190, 160, 600, 620, false, false, 0,
            ),
            Some(target)
        );
    }

    let mut document = TextDocument::new();
    assert!(document.is_saved());
    assert!(document.insert(b'I'));
    assert!(document.insert(b'\n'));
    assert!(!document.is_saved());
    assert_eq!(document.bytes(), b"I\n");
    document.save();
    assert!(document.is_saved());
    assert!(document.backspace());
    assert!(!document.is_saved());
    document.clear();
    assert!(document.is_saved());
    assert!(document.bytes().is_empty());
    assert!(document.open(b"persisted\ntext"));
    assert_eq!(document.bytes(), b"persisted\ntext");
    assert!(document.is_saved());
    assert!(document.move_cursor_to_line_edge(false));
    assert!(document.insert(b'X'));
    assert_eq!(document.bytes(), b"persisted\nXtext");
    assert!(document.move_cursor(-1));
    assert!(document.delete());
    assert_eq!(document.bytes(), b"persisted\ntext");
    assert!(document.move_cursor_vertical(true));

    let mut field = [0u8; 12];
    field[..4].copy_from_slice(b"acde");
    let mut field_length = 4usize;
    let mut caret = 1usize;
    assert!(ui::text_input::insert_ascii(
        &mut field,
        &mut field_length,
        &mut caret,
        b'b'
    ));
    assert_eq!(&field[..field_length], b"abcde");
    assert!(ui::text_input::move_caret(&mut caret, field_length, 1));
    assert!(ui::text_input::backspace(
        &mut field,
        &mut field_length,
        &mut caret
    ));
    assert_eq!(&field[..field_length], b"abde");
    assert!(ui::text_input::delete(
        &mut field,
        &mut field_length,
        &mut caret
    ));
    assert_eq!(&field[..field_length], b"abe");
    assert_eq!(ui::text_input::caret_from_x(18, 9, field_length), 2);

    ui::text_input::set_presentation(true, true, 2, 7, true);
    assert_eq!(ui::text_input::caret(7), Some((true, 2)));
    assert!(ui::text_input::pointer_is_text());
    let visible_presentation = ui::text_input::presentation_hash();
    ui::text_input::set_presentation(true, false, 2, 7, false);
    assert_eq!(ui::text_input::caret(7), Some((false, 2)));
    assert!(!ui::text_input::pointer_is_text());
    assert_ne!(visible_presentation, ui::text_input::presentation_hash());

    let wrapped = b"abcdef\nghijkl";
    assert_eq!(visual_line_count(wrapped, 3), 6);
    assert_eq!(visual_line_start(wrapped, 3, 3), 7);
    let scroll = layout.desktop_editor_scroll_geometry(190, 160, 600, 620, false, 80, 8);
    assert!(scroll.maximum_scroll > 0);
    let (scroll_x, scroll_y) = normalized(
        scroll.thumb.x + scroll.thumb.width as i32 / 2,
        scroll.thumb.y + scroll.thumb.height as i32 / 2,
    );
    assert_eq!(
        layout.desktop_editor_scroll_target(scroll_x, scroll_y, scroll),
        Some(EditorScrollTarget::Thumb)
    );
    assert_eq!(
        layout.desktop_editor_scroll_offset_for_thumb(1000, scroll, 0),
        scroll.maximum_scroll
    );

    let mut path = [0u8; ui::text_editor::DOCUMENT_PATH_CAPACITY];
    let path_length = document_path(b"design-notes", &mut path).unwrap();
    assert_eq!(path_length, ui::text_editor::DOCUMENT_NAMESPACE.len() + 12);
    assert!(document_path(b"invalid/name", &mut path).is_none());
    assert_eq!(
        layout.desktop_editor_dialog_target(500, 500, 190, 160, 600, 620, false, false, 0),
        Some(EditorDialogTarget::NameField)
    );
}

// ------------------------=
// FUNC: drag_path_test
// DESC: Exercises captured horizontal, vertical, diagonal, and rapid motion while retaining content and bounded events.
// ------------------=
fn drag_path_test() {
    let mut server = WindowServer::new();
    let surface = SurfaceId(77);
    let window = server
        .create(
            ContextId(4),
            surface,
            Rect {
                x: 20,
                y: 40,
                width: 320,
                height: 220,
            },
            ZOrderClass::Normal,
        )
        .unwrap();
    server.capture_pointer(ContextId(4), window).unwrap();
    assert_eq!(
        server.pointer_target(Point { x: 1200, y: 700 }),
        Some(window)
    );
    let work_area = Rect {
        x: 0,
        y: 30,
        width: 1280,
        height: 690,
    };
    for index in 0..64 {
        let requested = match index % 4 {
            0 => Point {
                x: 20 + index * 5,
                y: 40,
            },
            1 => Point {
                x: 20,
                y: 40 + index * 3,
            },
            2 => Point {
                x: 20 + index * 4,
                y: 40 + index * 2,
            },
            _ => Point {
                x: 900 - index * 3,
                y: 440 - index * 2,
            },
        };
        let before = server.inspect(window).unwrap().bounds;
        let after = server
            .move_window(ContextId(4), window, requested, work_area)
            .unwrap();
        assert_eq!(server.inspect(window).unwrap().surface, surface);
        assert_eq!(server.transition_damage(window), Some((before, after)));
    }
    assert!(server.dropped_event_count() > 0);
    let mut retained_events = 0;
    while server.next_event().is_some() {
        retained_events += 1;
    }
    assert!(retained_events <= MAX_WINDOW_EVENTS);
    server.context_failed(ContextId(4));
    assert_eq!(server.pointer_target(Point { x: 1200, y: 700 }), None);
}

// ------------------------=
// FUNC: context_recovery_test
// DESC: Verifies one failed UI context loses its windows and surfaces while unrelated retained state survives.
// ------------------=
fn context_recovery_test() {
    let mut runtime = ui::InfinityUiRuntime::new();
    let first_surface = runtime
        .surfaces
        .create(
            ContextId(1),
            Size {
                width: 64,
                height: 64,
            },
            PixelFormat::Xrgb8888,
            SurfaceSecurityClass::Application,
            false,
        )
        .unwrap();
    let second_surface = runtime
        .surfaces
        .create(
            ContextId(2),
            Size {
                width: 64,
                height: 64,
            },
            PixelFormat::Xrgb8888,
            SurfaceSecurityClass::Application,
            false,
        )
        .unwrap();
    runtime
        .windows
        .create(
            ContextId(1),
            first_surface,
            Rect {
                x: 0,
                y: 0,
                width: 64,
                height: 64,
            },
            ZOrderClass::Normal,
        )
        .unwrap();
    let survivor = runtime
        .windows
        .create(
            ContextId(2),
            second_surface,
            Rect {
                x: 80,
                y: 0,
                width: 64,
                height: 64,
            },
            ZOrderClass::Normal,
        )
        .unwrap();
    assert_eq!(runtime.context_failed(ContextId(1)), (1, 1));
    assert_eq!(runtime.windows.count(), 1);
    assert_eq!(runtime.surfaces.count(), 1);
    assert_eq!(
        runtime.windows.inspect(survivor).unwrap().surface,
        second_surface
    );
}

// ------------------------=
// FUNC: surface_and_compositor_test
// DESC: Proves surface budget and ownership enforcement plus atomic damage-limited front-buffer presentation.
// ------------------=
fn surface_and_compositor_test() {
    let mut surfaces = SurfaceRegistry::new(64);
    let surface = surfaces
        .create(
            ContextId(7),
            Size {
                width: 4,
                height: 4,
            },
            PixelFormat::Xrgb8888,
            SurfaceSecurityClass::Application,
            false,
        )
        .unwrap();
    assert_eq!(surfaces.bytes_reserved(), 64);
    assert_eq!(
        surfaces.create(
            ContextId(7),
            Size {
                width: 1,
                height: 1
            },
            PixelFormat::Xrgb8888,
            SurfaceSecurityClass::Application,
            false,
        ),
        Err(SurfaceError::BudgetExceeded)
    );
    assert_eq!(
        surfaces.publish(ContextId(8), surface, 1),
        Err(SurfaceError::AccessDenied)
    );
    assert_eq!(
        surfaces.mark_damage(
            ContextId(8),
            surface,
            Rect {
                x: 0,
                y: 0,
                width: 1,
                height: 1
            }
        ),
        Err(SurfaceError::AccessDenied)
    );
    assert_eq!(
        surfaces.mark_damage(
            ContextId(7),
            surface,
            Rect {
                x: 3,
                y: 3,
                width: 4,
                height: 4
            }
        ),
        Ok(Rect {
            x: 3,
            y: 3,
            width: 1,
            height: 1
        })
    );
    assert_eq!(
        surfaces.inspect(surface).unwrap().pending_damage,
        Some(Rect {
            x: 3,
            y: 3,
            width: 1,
            height: 1
        })
    );
    assert_eq!(surfaces.publish(ContextId(7), surface, 1), Ok(()));
    assert_eq!(surfaces.inspect(surface).unwrap().pending_damage, None);
    assert_eq!(
        surfaces.publish(ContextId(7), surface, 1),
        Err(SurfaceError::StaleGeneration)
    );
    assert_eq!(
        surfaces.resize(
            ContextId(8),
            surface,
            Size {
                width: 2,
                height: 2
            }
        ),
        Err(SurfaceError::AccessDenied)
    );
    assert_eq!(
        surfaces.resize(
            ContextId(7),
            surface,
            Size {
                width: 0,
                height: 2
            }
        ),
        Err(SurfaceError::InvalidGeometry)
    );
    assert_eq!(
        surfaces
            .resize(
                ContextId(7),
                surface,
                Size {
                    width: 2,
                    height: 2
                }
            )
            .unwrap()
            .byte_length,
        16
    );
    assert_eq!(surfaces.bytes_reserved(), 16);
    surfaces
        .set_visibility(ContextId(7), surface, false)
        .unwrap();
    surfaces.set_opacity(ContextId(7), surface, 192).unwrap();
    surfaces
        .associate_window(ContextId(7), surface, Some(ui::window::WindowId(9)))
        .unwrap();
    assert!(!surfaces.inspect(surface).unwrap().visible);
    assert_eq!(surfaces.inspect(surface).unwrap().opacity, 192);
    assert_eq!(
        surfaces.inspect(surface).unwrap().associated_window,
        Some(ui::window::WindowId(9))
    );

    let descriptor = *surfaces.inspect(surface).unwrap();
    let pixels = [0xff44_88cc; 16];
    let layer = SurfaceFrame {
        descriptor,
        pixels: &pixels,
        bounds: Rect {
            x: 2,
            y: 1,
            width: 4,
            height: 4,
        },
        opacity: 255,
        z_class: ZOrderClass::Normal,
        visible: true,
    };
    let mut damage = DamageTracker::new();
    damage.add_semantic(
        Rect {
            x: 2,
            y: 1,
            width: 2,
            height: 2,
        },
        DamageClass::Content,
        surface.0,
        120,
    );
    let mut back = [0u32; 32];
    let mut front = [0x1122_3344u32; 32];
    let mut compositor = SoftwareCompositor::new(
        Size {
            width: 8,
            height: 4,
        },
        8,
        0xff00_0000,
    );
    compositor.compose(&mut back, &[layer], &damage).unwrap();
    assert!(front.iter().all(|pixel| *pixel == 0x1122_3344));
    compositor.present(&mut front, &back, &damage).unwrap();
    assert_eq!(front[1 * 8 + 2], 0xff44_88cc);
    assert_eq!(front[2 * 8 + 3], 0xff44_88cc);
    assert_eq!(front[0], 0x1122_3344);
    assert_eq!(compositor.metrics().presented_pixels, 4);

    let mut priority_damage = DamageTracker::new();
    priority_damage.add_semantic(
        Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        },
        DamageClass::Cursor,
        1,
        250,
    );
    priority_damage.add_semantic(
        Rect {
            x: 7,
            y: 3,
            width: 1,
            height: 1,
        },
        DamageClass::Content,
        2,
        10,
    );
    let mut priority_back = [0u32; 32];
    let mut priority_front = [0x1122_3344u32; 32];
    let mut priority_compositor = SoftwareCompositor::new(
        Size {
            width: 8,
            height: 4,
        },
        8,
        0xff00_0000,
    );
    priority_compositor
        .compose(&mut priority_back, &[], &priority_damage)
        .unwrap();
    let empty_record = DamageRecord {
        rect: Rect::default(),
        class: DamageClass::Content,
        source_id: 0,
        priority: 0,
    };
    let mut deferred = [empty_record; MAX_DAMAGE_REGIONS];
    assert_eq!(
        priority_compositor
            .present_prioritized(
                &mut priority_front,
                &priority_back,
                &priority_damage,
                1,
                &mut deferred,
            )
            .unwrap(),
        1
    );
    assert_eq!(priority_front[0], 0xff00_0000);
    assert_eq!(priority_front[31], 0x1122_3344);
    priority_compositor
        .present(&mut priority_front, &priority_back, &priority_damage)
        .unwrap();
    assert_eq!(priority_front[31], 0xff00_0000);
    assert_eq!(priority_compositor.metrics().deferred_regions, 1);

    let spoofed = SurfaceFrame {
        z_class: ZOrderClass::Trusted,
        ..layer
    };
    assert_eq!(
        compositor.compose(&mut back, &[spoofed], &damage),
        Err(CompositorError::PrivilegedZOrderDenied)
    );
    assert_eq!(
        surfaces.destroy(ContextId(8), surface),
        Err(SurfaceError::AccessDenied)
    );
    assert_eq!(surfaces.destroy(ContextId(7), surface), Ok(()));
    assert_eq!(surfaces.bytes_reserved(), 0);
}

// ------------------------=
// FUNC: window_server_restart_test
// DESC: Proves Window Server metadata reconstruction preserves ordinary surfaces while dropping capture and trusted overlays.
// ------------------=
fn window_server_restart_test() {
    let mut trusted = ui::trusted::TrustedUiManager::new();
    let lease = trusted
        .acquire_secure_input(true, 9, TrustedSurface::Authentication, 500)
        .unwrap();
    let token = trusted.authorize_trusted_window(lease).unwrap();
    let mut server = WindowServer::new();
    let ordinary = server
        .create(
            ContextId(8),
            SurfaceId(81),
            Rect {
                x: 90,
                y: 70,
                width: 420,
                height: 280,
            },
            ZOrderClass::Normal,
        )
        .unwrap();
    server.capture_pointer(ContextId(8), ordinary).unwrap();
    server
        .create_trusted(
            token,
            ContextId(9),
            SurfaceId(91),
            Rect {
                x: 200,
                y: 160,
                width: 360,
                height: 220,
            },
        )
        .unwrap();
    let checkpoint = server.checkpoint();
    let recovered = WindowServer::recover(checkpoint);
    assert_eq!(recovered.count(), 1);
    assert_eq!(recovered.inspect(ordinary).unwrap().surface, SurfaceId(81));
    assert_eq!(
        recovered.inspect(ordinary).unwrap().bounds,
        Rect {
            x: 90,
            y: 70,
            width: 420,
            height: 280
        }
    );
    assert_eq!(recovered.pointer_target(Point { x: 1200, y: 800 }), None);
}

// ------------------------=
// FUNC: semantic_damage_storm_test
// DESC: Verifies excessive independent damage remains bounded and collapses to one safe union without allocation.
// ------------------=
fn semantic_damage_storm_test() {
    let mut damage = DamageTracker::new();
    for index in 0..=MAX_DAMAGE_REGIONS {
        damage.add_semantic(
            Rect {
                x: (index * 4) as i32,
                y: 0,
                width: 1,
                height: 1,
            },
            DamageClass::Cursor,
            index as u32,
            240,
        );
    }
    assert!(damage.collapsed());
    assert_eq!(damage.regions().len(), 1);
    assert_eq!(damage.records()[0].class, DamageClass::Geometry);
    assert_eq!(damage.diagnostics().collapsed, 1);
    damage.clip_to(Rect {
        x: 8,
        y: 0,
        width: 16,
        height: 1,
    });
    assert_eq!(
        damage.regions()[0],
        Rect {
            x: 8,
            y: 0,
            width: 16,
            height: 1
        }
    );
}

// ------------------------=
// FUNC: installed_system_hit_geometry_test
// DESC: Proves visible installed-system controls retain exact pointer targets across wide, square, and HiDPI modes.
// ------------------=
fn installed_system_hit_geometry_test() {
    let wide = SystemLayout::new(1920, 1080);
    assert_eq!(wide.top_bar_height(), 38);
    assert_eq!(
        wide.onboarding_target(0, 200, 790),
        Some(OnboardingTarget::Primary)
    );
    assert_eq!(
        wide.onboarding_target(2, 80, 790),
        Some(OnboardingTarget::Back)
    );
    assert_eq!(
        wide.onboarding_target(2, 210, 790),
        Some(OnboardingTarget::Primary)
    );
    assert_eq!(
        wide.onboarding_target(2, 200, 490),
        Some(OnboardingTarget::Input)
    );

    let square = SystemLayout::new(1600, 1600);
    assert_eq!(square.top_bar_height(), 38);
    assert_eq!(
        square.onboarding_target(0, 200, 790),
        Some(OnboardingTarget::Primary)
    );
    assert_eq!(
        square.onboarding_target(3, 200, 490),
        Some(OnboardingTarget::Input)
    );
    assert_eq!(square.authentication_target(200, 420), Some(0));
    assert_eq!(square.authentication_target(200, 500), Some(1));
    assert_eq!(square.authentication_target(200, 560), Some(2));
    assert_eq!(
        square.desktop_target(30, 20, 30, 500, true, false),
        Some(DesktopTarget::InfinityMenu)
    );
    assert_eq!(square.desktop_target(200, 15, 30, 500, true, false), None);
    assert_eq!(
        square.desktop_target(400, 15, 30, 500, true, false),
        Some(DesktopTarget::TopMenu(5))
    );
    assert_eq!(
        square.desktop_target(805, 15, 30, 500, true, false),
        Some(DesktopTarget::Status(0))
    );
    assert_eq!(
        square.desktop_target(970, 15, 30, 500, true, false),
        Some(DesktopTarget::Status(6))
    );
    assert_eq!(
        square.desktop_target(100, 510, 30, 500, true, false),
        Some(DesktopTarget::HomeTitle)
    );
    assert_eq!(
        square.desktop_target(445, 510, 30, 500, true, false),
        Some(DesktopTarget::HomeControl(2))
    );
    assert_eq!(
        square.desktop_target(60, 570, 30, 500, true, false),
        Some(DesktopTarget::HomeSidebar(0))
    );
    assert_eq!(
        square.desktop_target(170, 560, 30, 500, true, false),
        Some(DesktopTarget::HomeItem(0))
    );
    assert_eq!(
        square.desktop_target(260, 970, 30, 500, true, false),
        Some(DesktopTarget::Dock(0))
    );
    assert_eq!(square.desktop_target(100, 510, 30, 500, false, false), None);
    assert_eq!(
        square.desktop_target_sized(629, 699, 30, 200, 600, 500, true, false),
        Some(DesktopTarget::HomeResize(3))
    );
    assert_eq!(
        square.desktop_target_sized(330, 700, 30, 200, 600, 500, true, false),
        Some(DesktopTarget::HomeResize(4))
    );
    let context = square.file_navigator_context_geometry(950, 950);
    assert!(context.x + context.width as i32 <= 1600);
    assert!(context.y + context.height as i32 <= 1600);
    let context_row_x = ((context.x + 20) * 1000 / 1600) as i32;
    let context_row_y = ((context.y + 6 + 28 * 3 + 10) * 1000 / 1600) as i32;
    assert_eq!(
        square.file_navigator_context_action(950, 950, context_row_x, context_row_y),
        Some(3)
    );
    assert_eq!(
        resize_home_window(100, 200, 430, 380, 3, 700, 800),
        (100, 200, 600, 600)
    );
    assert_eq!(
        resize_home_window(100, 200, 430, 380, 0, 350, 450),
        (230, 320, 300, 260)
    );
    assert_eq!(
        square.system_menu_target(0, 50, 50),
        SystemMenuTarget::Item(0)
    );
    assert_eq!(
        square.system_menu_target(0, 500, 500),
        SystemMenuTarget::Dismiss
    );
    let (menu_x, menu_y, menu_w, menu_h, _) = square.system_menu_geometry(0);
    let (damage_x, damage_y, damage_w, damage_h) = square.system_menu_damage_geometry(0);
    assert_eq!((damage_x, damage_y), (menu_x, menu_y));
    assert!(damage_w > menu_w && damage_h > menu_h);
    let settings = SettingsWindowState {
        x: 160,
        y: 210,
        width: 680,
        height: 620,
        maximized: false,
        expanded_row: None,
        scroll_offset: 0,
        row_count: 8,
    };
    let settings_geometry = square.settings_window_geometry(settings);
    let normalized_center = |rect: Rect| {
        (
            (rect.x + rect.width as i32 / 2) * 1000 / 1600,
            (rect.y + rect.height as i32 / 2) * 1000 / 1600,
        )
    };
    let section_point = (
        (settings_geometry.navigation.x + 20) * 1000 / 1600,
        (settings_geometry.navigation.y + 25) * 1000 / 1600,
    );
    assert_eq!(
        square.settings_target(section_point.0, section_point.1, settings),
        Some(SettingsTarget::Section(0))
    );
    let row_point = normalized_center(square.settings_row_geometry(settings, 0).summary);
    assert_eq!(
        square.settings_target(row_point.0, row_point.1, settings),
        Some(SettingsTarget::ContentRow(0))
    );
    let actionable_settings = SettingsWindowState {
        expanded_row: Some(0),
        ..settings
    };
    let action_detail = square.settings_row_geometry(actionable_settings, 0).detail;
    let action_point = (
        (action_detail.x + (ui::system_layout::UI_GUTTER + 60) as i32 * square.scale() as i32)
            * 1000
            / 1600,
        (action_detail.bottom()
            - ((ui::system_layout::UI_COMPACT_ACTION_HEIGHT / 2 + 10) * square.scale()) as i32)
            * 1000
            / 1600,
    );
    assert_eq!(
        square.settings_target(action_point.0, action_point.1, actionable_settings),
        Some(SettingsTarget::ExpandedAction)
    );
    let resize_point = (
        (settings_geometry.window.right() - 4) * 1000 / 1600,
        (settings_geometry.window.bottom() - 4) * 1000 / 1600,
    );
    assert_eq!(
        square.settings_target(resize_point.0, resize_point.1, settings),
        Some(SettingsTarget::Resize(3))
    );
    let settings_bottom = (
        (settings_geometry.window.x + settings_geometry.window.width as i32 / 2) * 1000 / 1600,
        settings_geometry.window.bottom() * 1000 / 1600,
    );
    assert_eq!(
        square.settings_target(settings_bottom.0, settings_bottom.1, settings),
        Some(SettingsTarget::Resize(4))
    );
    let expanded = SettingsWindowState {
        expanded_row: Some(2),
        height: 420,
        ..settings
    };
    let compact = SystemLayout::new(1600, 900);
    let expanded_geometry = compact.settings_window_geometry(expanded);
    let primary_row = square.settings_row_geometry(expanded, 2);
    assert_eq!(
        primary_row.detail.y,
        primary_row.summary.bottom() + 4,
        "expanded configuration must remain attached to its owning row"
    );
    assert!(expanded_geometry.maximum_scroll > 0);
    let thumb_point = (
        (expanded_geometry.scrollbar_thumb.x + expanded_geometry.scrollbar_thumb.width as i32 / 2)
            * 1000
            / 1600,
        (expanded_geometry.scrollbar_thumb.y + expanded_geometry.scrollbar_thumb.height as i32 / 2)
            * 1000
            / 900,
    );
    assert_eq!(
        compact.settings_target(thumb_point.0, thumb_point.1, expanded),
        Some(SettingsTarget::ScrollThumb)
    );
    let dragged_offset = compact.settings_scroll_offset_for_thumb(900, expanded, 0);
    assert_eq!(dragged_offset, expanded_geometry.maximum_scroll);
    let picker = square.settings_primary_geometry(expanded);
    let picker_point = |rect: Rect| {
        (
            (rect.x + rect.width as i32 / 2) * 1000 / 1600,
            (rect.y + rect.height as i32 / 2) * 1000 / 1600,
        )
    };
    let (spectrum_x, spectrum_y) = picker_point(picker.spectrum);
    assert!(matches!(
        square.settings_primary_target(spectrum_x, spectrum_y, expanded),
        Some(SettingsAccentTarget::Spectrum { saturation, value })
            if (120..=135).contains(&saturation) && (120..=135).contains(&value)
    ));
    let (hue_x, hue_y) = picker_point(picker.hue);
    assert!(matches!(
        square.settings_primary_target(hue_x, hue_y, expanded),
        Some(SettingsAccentTarget::Hue(hue)) if (165..=185).contains(&hue)
    ));
    let accent_expanded = SettingsWindowState {
        expanded_row: Some(3),
        ..expanded
    };
    let accent_picker = square.settings_accent_geometry(accent_expanded);
    let (accent_x, accent_y) = picker_point(accent_picker.spectrum);
    assert!(matches!(
        square.settings_accent_target(accent_x, accent_y, accent_expanded),
        Some(SettingsAccentTarget::Spectrum { .. })
    ));
    let opacity_expanded = SettingsWindowState {
        expanded_row: Some(4),
        scroll_offset: 0,
        height: 620,
        ..settings
    };
    let slider = square.settings_effect_slider_geometry(opacity_expanded, 4, 8, 15);
    let (slider_x, slider_y) = normalized_center(slider.thumb);
    assert_eq!(
        square.settings_effect_slider_target(slider_x, slider_y, opacity_expanded, 4, 15),
        Some(8)
    );
    assert_eq!(
        square.settings_effect_slider_drag_value(0, opacity_expanded, 4, 15),
        0
    );
    assert_eq!(
        square.settings_effect_slider_drag_value(1000, opacity_expanded, 4, 15),
        15
    );
    let timeout_expanded = SettingsWindowState {
        expanded_row: Some(3),
        ..settings
    };
    let timeout_slider = square.settings_effect_slider_geometry(timeout_expanded, 3, 44, 119);
    let timeout_x =
        (timeout_slider.thumb.x + (timeout_slider.thumb.width / 2) as i32) * 1000 / 1600;
    let timeout_y =
        (timeout_slider.thumb.y + (timeout_slider.thumb.height / 2) as i32) * 1000 / 1600;
    assert_eq!(
        square.settings_slider_target(timeout_x, timeout_y, timeout_expanded, 3, 119),
        Some(44)
    );
    assert_eq!(
        square.settings_slider_drag_value(0, timeout_expanded, 3, 119),
        0
    );
    assert_eq!(
        square.settings_slider_drag_value(1000, timeout_expanded, 3, 119),
        119
    );
    let blur_expanded = SettingsWindowState {
        expanded_row: Some(5),
        ..opacity_expanded
    };
    let blur_slider = square.settings_effect_slider_geometry(blur_expanded, 5, 4, 8);
    let (blur_x, blur_y) = normalized_center(blur_slider.thumb);
    assert_eq!(
        square.settings_effect_slider_target(blur_x, blur_y, blur_expanded, 5, 8),
        Some(4)
    );
    let standard_hidpi = SettingsWindowState {
        x: 160,
        y: 210,
        width: 680,
        height: 620,
        maximized: false,
        expanded_row: None,
        scroll_offset: 0,
        row_count: 6,
    };
    let hidpi_layout = SystemLayout::new(2560, 1440);
    let hidpi_window = hidpi_layout.settings_window_geometry(standard_hidpi);
    for color_row in [2usize, 3] {
        let row = hidpi_layout.settings_row_geometry(standard_hidpi, color_row);
        assert!(row.summary.y >= hidpi_window.viewport.y);
        assert!(row.summary.bottom() <= hidpi_window.viewport.bottom());
    }
    assert_eq!(
        resize_native_window(160, 210, 680, 620, 3, 920, 900, 600, 420),
        (160, 210, 760, 690)
    );
    assert_eq!(
        resize_native_window(160, 210, 680, 620, 4, 500, 900, 600, 420),
        (160, 210, 680, 690)
    );
    assert_eq!(
        square.desktop_app_window_target(699, 749, 210, 260, 490, 490, false, false),
        DesktopAppWindowTarget::Resize(3)
    );
    assert_eq!(
        square.desktop_app_window_target(455, 750, 210, 260, 490, 490, false, false),
        DesktopAppWindowTarget::Resize(4)
    );

    let hidpi = SystemLayout::new(2560, 1440);
    assert_eq!(hidpi.top_bar_height(), 76);
    assert_eq!(
        hidpi.onboarding_target(0, 220, 810),
        Some(OnboardingTarget::Primary)
    );
    assert_eq!(
        hidpi.onboarding_target(4, 220, 490),
        Some(OnboardingTarget::Input)
    );
    assert!(ui::redraw::desktop_menu_change_requires_bounded_redraw(
        2, 3, 0, 0, true
    ));
    assert!(ui::redraw::desktop_menu_change_requires_bounded_redraw(
        3, 3, 0, 0, true
    ));
    assert!(ui::redraw::desktop_menu_change_requires_bounded_redraw(
        3, 2, 0, 0, true
    ));
    assert!(!ui::redraw::desktop_menu_change_requires_bounded_redraw(
        2, 4, 0, 0, true
    ));
    assert!(ui::redraw::desktop_window_move_requires_structural_redraw(
        2, true, true, false
    ));
    assert!(ui::redraw::desktop_window_move_requires_structural_redraw(
        2, true, true, true
    ));
    assert!(ui::redraw::desktop_app_content_requires_bounded_redraw(
        8, true
    ));
    assert!(ui::redraw::desktop_app_content_requires_bounded_redraw(
        9, true
    ));
    assert!(!ui::redraw::desktop_app_content_requires_bounded_redraw(
        2, true
    ));
    assert!(ui::redraw::appearance_change_requires_structural_redraw(
        0x20bfff, 0x4da3ff
    ));
    assert!(!ui::redraw::appearance_change_requires_structural_redraw(
        0x4da3ff, 0x4da3ff
    ));
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
    let cover = fit_cover(
        Size {
            width: 1920,
            height: 1080,
        },
        Rect {
            x: 0,
            y: 0,
            width: 1536,
            height: 1024,
        },
    );
    assert_eq!(cover.height, 1024);
    assert!(cover.width >= 1536);
    let mut rows = [Rect::default(); 4];
    assert_eq!(
        linear_layout(
            Rect {
                x: 0,
                y: 0,
                width: 400,
                height: 100
            },
            Insets::default(),
            Axis::Horizontal,
            4,
            8,
            &mut rows
        ),
        4
    );
    assert_eq!(rows[1].x - rows[0].right(), 8);
}

// ------------------------=
// FUNC: skin_test
// DESC: Verifies package encoding, corruption rejection, alternate activation, rollback, and SafeSkin fallback.
// ------------------=
fn skin_test() {
    let dark = ui::skin::default_dark_skin();
    assert_eq!(dark.tokens.color(ColorRole::Accent).rgb24(), 0x4da3ff);
    let mut header = [0u8; ui::skin::SKIN_HEADER_BYTES];
    encode_header(&dark, &mut header);
    assert_eq!(
        decode_header(&header, dark.tokens).unwrap().id.as_bytes(),
        b"infinity.default.dark"
    );
    let mut corrupt = header;
    corrupt[0] = b'X';
    assert!(matches!(
        decode_header(&corrupt, dark.tokens),
        Err(SkinError::InvalidMagic)
    ));
    let mut registry = SkinRegistry::new();
    registry
        .activate(
            SkinId::from_bytes(b"infinity.diagnostic.light"),
            AppearanceScope::User,
        )
        .unwrap();
    assert_eq!(
        registry.active().id.as_bytes(),
        b"infinity.diagnostic.light"
    );
    registry.rollback();
    assert_eq!(registry.active().id.as_bytes(), b"infinity.default.dark");
    registry.enter_safe_mode();
    assert_eq!(registry.active().id.as_bytes(), b"infinity.safe");
    assert_eq!(
        registry.register(diagnostic_light_skin()),
        Err(SkinError::Duplicate)
    );
    let before = [
        registry.accent_surface(AccentSurface::WindowOutline),
        registry.accent_surface(AccentSurface::Header),
        registry.accent_surface(AccentSurface::TopBar),
        registry.accent_surface(AccentSurface::Dock),
        registry.accent_surface(AccentSurface::Widget),
        registry.accent_surface(AccentSurface::Focus),
        registry.accent_surface(AccentSurface::Selection),
    ];
    let generation = registry.generation();
    registry
        .set_accent(0xd45cff, AppearanceScope::User)
        .unwrap();
    assert!(registry.generation() > generation);
    assert_eq!(registry.accent_rgb(), 0xd45cff);
    let after = [
        registry.accent_surface(AccentSurface::WindowOutline),
        registry.accent_surface(AccentSurface::Header),
        registry.accent_surface(AccentSurface::TopBar),
        registry.accent_surface(AccentSurface::Dock),
        registry.accent_surface(AccentSurface::Widget),
        registry.accent_surface(AccentSurface::Focus),
        registry.accent_surface(AccentSurface::Selection),
    ];
    for index in [0usize, 5, 6] {
        assert_ne!(before[index], after[index]);
    }
    for index in 1..=4usize {
        assert_eq!(
            before[index], after[index],
            "frosted chrome must not be flooded by the selected accent"
        );
    }
    let accent_only = after;
    registry
        .set_primary(0x35233d, AppearanceScope::Machine)
        .unwrap();
    assert_eq!(registry.primary_rgb(), 0x35233d);
    let primary_after = [
        registry.accent_surface(AccentSurface::WindowOutline),
        registry.accent_surface(AccentSurface::Header),
        registry.accent_surface(AccentSurface::TopBar),
        registry.accent_surface(AccentSurface::Dock),
        registry.accent_surface(AccentSurface::Widget),
        registry.accent_surface(AccentSurface::Focus),
        registry.accent_surface(AccentSurface::Selection),
    ];
    for index in 1..=4usize {
        assert_ne!(accent_only[index], primary_after[index]);
    }
    for index in [0usize, 5, 6] {
        assert_eq!(accent_only[index], primary_after[index]);
    }
    let outline_before_effects = registry.accent_surface(AccentSurface::WindowOutline);
    registry
        .set_background_effects(64, 7, AppearanceScope::Machine)
        .unwrap();
    assert_eq!(registry.background_effects(), (64, 7));
    assert_eq!(registry.background_alpha(200), 128);
    assert_eq!(
        registry.accent_surface(AccentSurface::WindowOutline),
        outline_before_effects,
        "background effects must not alter outline/control color tokens"
    );
    assert_eq!(
        registry.set_background_effects(39, 4, AppearanceScope::Machine),
        Err(SkinError::InvalidAccent)
    );
    assert_eq!(hsv_to_rgb(0, 255, 255), 0xff0000);
    assert_eq!(hsv_to_rgb(120, 255, 255), 0x00ff00);
    assert_eq!(hsv_to_rgb(240, 255, 255), 0x0000ff);
    assert_eq!(rgb_to_hsv(0x00ff00), (120, 255, 255));
    assert_eq!(
        registry.set_accent(0, AppearanceScope::User),
        Err(SkinError::InvalidAccent)
    );
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
    runtime
        .scene
        .insert(element(
            1,
            None,
            Rect {
                x: 0,
                y: 0,
                width: 800,
                height: 600,
            },
            0,
            0,
        ))
        .unwrap();
    runtime
        .scene
        .insert(element(
            2,
            Some(ElementId(1)),
            Rect {
                x: 50,
                y: 50,
                width: 200,
                height: 60,
            },
            1,
            2,
        ))
        .unwrap();
    assert_eq!(
        runtime.scene.hit_test(Point { x: 70, y: 70 }),
        Some(ElementId(2))
    );
    runtime.begin_frame();
    runtime.scene.get_mut(ElementId(2)).unwrap().bounds.x = 80;
    let damage = runtime.commit_frame();
    assert!(!damage.is_empty());
    assert!(damage
        .iter()
        .all(|rect| rect.width < 800 || rect.height < 600));
}

// ------------------------=
// FUNC: element
// DESC: Constructs a compact semantic test element with accessibility metadata.
// ------------------=
fn element(
    id: u32,
    parent: Option<ElementId>,
    bounds: Rect,
    actions: u16,
    z_order: i16,
) -> SemanticElement {
    SemanticElement {
        id: ElementId(id),
        parent,
        kind: if id == 1 {
            ElementKind::Root
        } else {
            ElementKind::Button
        },
        accessibility_role: if id == 1 {
            AccessibilityRole::Application
        } else {
            AccessibilityRole::Button
        },
        bounds,
        previous_bounds: bounds,
        state: ElementState(ElementState::ENABLED),
        label_id: id,
        action_mask: actions,
        z_order,
        visible: true,
    }
}

// ------------------------=
// FUNC: window_isolation_test
// DESC: Proves cross-context mutation and pointer capture denial plus crashed-owner cleanup.
// ------------------=
fn window_isolation_test() {
    let mut server = WindowServer::new();
    let first = server
        .create(
            ContextId(1),
            SurfaceId(11),
            Rect {
                x: 0,
                y: 0,
                width: 300,
                height: 200,
            },
            ZOrderClass::Normal,
        )
        .unwrap();
    let second = server
        .create(
            ContextId(2),
            SurfaceId(12),
            Rect {
                x: 20,
                y: 20,
                width: 300,
                height: 200,
            },
            ZOrderClass::Floating,
        )
        .unwrap();
    assert!(matches!(
        server.mutate(ContextId(2), first),
        Err(WindowError::AccessDenied)
    ));
    assert_eq!(
        server.capture_pointer(ContextId(2), first),
        Err(WindowError::AccessDenied)
    );
    assert_eq!(server.hit_test(Point { x: 30, y: 30 }), Some(second));
    let work_area = Rect {
        x: 0,
        y: 40,
        width: 1280,
        height: 680,
    };
    assert_eq!(server.capture_pointer(ContextId(1), first), Ok(()));
    assert_eq!(
        server
            .move_window(ContextId(1), first, Point { x: 1200, y: 700 }, work_area)
            .unwrap(),
        Rect {
            x: 980,
            y: 520,
            width: 300,
            height: 200
        }
    );
    assert_eq!(
        server
            .resize_window(ContextId(1), first, 500, 300, work_area)
            .unwrap(),
        Rect {
            x: 980,
            y: 520,
            width: 300,
            height: 200
        }
    );
    assert_eq!(server.release_pointer(ContextId(1)), Ok(()));
    assert_eq!(
        server.move_window(ContextId(2), first, Point { x: 10, y: 40 }, work_area),
        Err(WindowError::AccessDenied)
    );
    assert_eq!(server.context_failed(ContextId(2)), 1);
    assert_eq!(server.hit_test(Point { x: 1000, y: 540 }), Some(first));
    assert_eq!(
        server.create(
            ContextId(1),
            SurfaceId(13),
            Rect {
                x: 0,
                y: 0,
                width: 300,
                height: 200
            },
            ZOrderClass::Trusted
        ),
        Err(WindowError::PrivilegedZOrder)
    );
    let transition = server
        .set_state(
            ContextId(1),
            first,
            WindowState::Maximized,
            work_area,
            Rect {
                x: 0,
                y: 0,
                width: 1280,
                height: 720,
            },
        )
        .unwrap();
    assert_eq!(transition.new_bounds, work_area);
    assert_eq!(
        server
            .set_state(
                ContextId(1),
                first,
                WindowState::Normal,
                work_area,
                work_area
            )
            .unwrap()
            .new_bounds,
        Rect {
            x: 980,
            y: 520,
            width: 300,
            height: 200
        }
    );
    assert_eq!(server.count(), 1);
    assert_eq!(
        server.close(ContextId(2), first),
        Err(WindowError::AccessDenied)
    );
    assert_eq!(server.close(ContextId(1), first).unwrap().id, first);
    assert_eq!(server.count(), 0);
    let mut previous_sequence = 0;
    let mut saw_move = false;
    while let Some(event) = server.next_event() {
        assert!(event.sequence > previous_sequence);
        previous_sequence = event.sequence;
        saw_move |= event.kind == WindowEventKind::Moved;
    }
    assert!(saw_move);
    assert_eq!(server.dropped_event_count(), 0);
}

// ------------------------=
// FUNC: service_foundation_test
// DESC: Verifies secure input, bounded clipboard, async cancellation, frame pacing, cache limits, localization, and vectors.
// ------------------=
fn service_foundation_test() {
    let mut runtime = ui::InfinityUiRuntime::new();
    assert_eq!(
        runtime
            .clipboard
            .write(false, 7, ClipboardKind::Utf8Text, b"private"),
        Err(ClipboardError::AccessDenied)
    );
    runtime
        .clipboard
        .write(true, 7, ClipboardKind::Utf8Text, b"typed text")
        .unwrap();
    assert_eq!(
        &runtime.clipboard.read(true).unwrap().bytes[..10],
        b"typed text"
    );

    assert_eq!(
        runtime
            .trusted
            .acquire_secure_input(false, 7, TrustedSurface::Authentication, 100),
        Err(TrustedUiError::AccessDenied)
    );
    let lease = runtime
        .trusted
        .acquire_secure_input(true, 7, TrustedSurface::Authentication, 100)
        .unwrap();
    let trusted_token = runtime.trusted.authorize_trusted_window(lease).unwrap();
    let trusted_window = runtime
        .windows
        .create_trusted(
            trusted_token,
            ContextId(7),
            SurfaceId(40),
            Rect {
                x: 10,
                y: 10,
                width: 400,
                height: 240,
            },
        )
        .unwrap();
    assert_eq!(
        runtime.windows.inspect(trusted_window).unwrap().z_class,
        ZOrderClass::Trusted
    );
    assert_eq!(
        InputRouter::route_pointer(
            &runtime.windows,
            &runtime.trusted,
            Point { x: 30, y: 30 },
            50
        ),
        Some(InputDestination::SecureContext(ContextId(7)))
    );
    assert_eq!(
        InputRouter::route_keyboard(&runtime.windows, &runtime.trusted, 50),
        Some(InputDestination::SecureContext(ContextId(7)))
    );
    assert_eq!(
        runtime
            .trusted
            .acquire_secure_input(true, 8, TrustedSurface::Lock, 100),
        Err(TrustedUiError::Busy)
    );
    assert!(runtime.trusted.expire(100));
    assert_eq!(
        runtime.trusted.release_secure_input(lease),
        Err(TrustedUiError::AccessDenied)
    );

    let token = runtime.async_tasks.begin(ElementId(9), 1, 100).unwrap();
    let stale = ui::async_model::TaskToken {
        id: token.id,
        generation: token.generation + 1,
    };
    assert_eq!(
        runtime.async_tasks.complete(stale, 2, None),
        Err(AsyncError::Stale)
    );
    runtime.async_tasks.complete(token, 3, None).unwrap();
    let pending = runtime.async_tasks.begin(ElementId(10), 4, 100).unwrap();
    assert_eq!(runtime.async_tasks.cancel_owner(ElementId(10)), 1);
    assert!(runtime.async_tasks.complete(pending, 5, None).is_ok());

    let mut clock = FrameClock::new(1);
    assert_eq!(clock.refresh_hz, 30);
    clock.advance(1, 1_000);
    clock.advance(100, 1_000);
    assert!(clock.dropped_frames > 0);
    let mut budget = CacheBudget {
        total_bytes: 128,
        used_bytes: 0,
        evictions: 0,
    };
    assert!(budget.reserve(96));
    assert!(!budget.reserve(64));
    assert_eq!(budget.evictions, 1);
    budget.release(96);

    let mut quality = AdaptiveQualityController::new();
    for _ in 0..3 {
        quality.observe_frame(true, false);
    }
    assert_eq!(quality.policy().level, QualityLevel::Balanced);
    for _ in 0..120 {
        quality.observe_frame(false, false);
    }
    assert_eq!(quality.policy().level, QualityLevel::Full);

    assert_eq!(resolve(EN_US, StringId::Welcome), b"Welcome to InfinityOS");
    assert_eq!(semantic_name(IconId::Eye), b"eye");
    let mut commands = [None; MAX_VECTOR_COMMANDS];
    commands[0] = Some(VectorCommand::Move(Point { x: 0, y: 0 }));
    commands[1] = Some(VectorCommand::Line(Point { x: 24, y: 24 }));
    let icon = VectorIcon {
        id: IconId::Infinity,
        view_box: Rect {
            x: 0,
            y: 0,
            width: 24,
            height: 24,
        },
        commands,
        command_count: 2,
        stroke_width: 2,
        filled: false,
    };
    assert_eq!(validate(&icon), Ok(()));
    let empty = VectorIcon {
        command_count: 0,
        ..icon
    };
    assert_eq!(validate(&empty), Err(VectorError::Empty));
    let _ = AsyncState::Cancelled;
}
