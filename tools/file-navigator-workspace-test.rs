#![allow(dead_code)]

mod console {
    #[derive(Clone, Copy)]
    pub enum ConsoleKey {
        Character(u8),
        Backspace,
        Delete,
        Left,
        Right,
        Home,
        End,
        Enter,
    }
}

#[path = "../kernel/ui/text_input.rs"]
pub mod text_input_impl;

mod ui {
    pub use crate::text_input_impl as text_input;
}

mod iop {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum OperationId {
        NamespaceResolve,
        NamespaceList,
        ObjectInspect,
        ObjectHistory,
        ObjectRelationships,
        ObjectSearch,
        ApplicationAssociationResolve,
        NamespaceCreate,
        NamespaceDelete,
        NamespaceMove,
        ObjectCreate,
        ObjectCopy,
        ObjectDelete,
        ObjectDestroy,
        NamespaceAttach,
        NamespaceDetach,
        TrashMove,
        TrashList,
        TrashRestore,
        TrashDelete,
        TrashEmpty,
        ShellProfileList,
        ShellProfileInspect,
        ShellProfileCreate,
        ShellProfileClone,
        ShellProfileEnable,
        ShellProfileDisable,
        ShellProfileSetDefault,
        ShellProfileDelete,
        ShellAliasList,
        ShellAliasAdd,
        ShellAliasDelete,
        ShellAliasResolve,
        ApplicationLaunch,
    }
}

#[path = "../kernel/runtime/object_navigation.rs"]
mod object_navigation;

use object_navigation::{
    FileNavigatorAction, FileNavigatorDialog, FileNavigatorMenu, FileNavigatorWorkspace,
    PerformanceMode, ViewMode,
};

// ------------------------=
// FUNC: main
// DESC: Behaviorally verifies independent navigator state, geometry, layering, and close isolation.
// ------------------=
fn main() {
    let mut workspace = FileNavigatorWorkspace::new();
    let first = workspace
        .launch(b"/home/default", 41)
        .expect("first navigator");
    let mut first_window = workspace.window(first).expect("first window");
    use object_navigation::{ContextAction as A, OpenTarget};
    first_window.state.open_context_menu(500, 500, Some(4));
    first_window.state.context_target = OpenTarget::TextEditor;
    assert_eq!(first_window.state.selected_index, 4);
    assert_eq!(first_window.state.context_actions(), &[A::Open, A::OpenWith, A::Rename, A::Duplicate, A::Copy, A::Cut, A::Paste, A::Trash]);
    first_window.state.context_open_with = true;
    assert_eq!(first_window.state.context_actions(), &[A::TextEditor, A::Back]);
    first_window.state.context_target = OpenTarget::Unsupported;
    assert_eq!(first_window.state.context_actions(), &[A::Unavailable, A::Back]);
    first_window.state.open_context_menu(500, 500, None);
    assert!(!first_window.state.context_open_with);
    assert_eq!(first_window.state.context_actions(), &[A::NewFolder, A::Paste, A::List, A::Grid, A::Sort]);
    first_window
        .state
        .navigate(b"/home/default/documents")
        .expect("navigate first");
    first_window.state.view_mode = ViewMode::Grid;
    first_window.state.scroll_offset = 144;
    first_window.x = 72;
    first_window.width = 650;
    assert!(workspace.update_active(first_window));

    let second = workspace
        .launch(b"/home/default/pictures", 42)
        .expect("second navigator");
    let mut second_window = workspace.window(second).expect("second window");
    second_window.state.selected_index = 3;
    second_window.x = 246;
    assert!(workspace.update_active(second_window));

    assert_eq!(workspace.count(), 2);
    assert!(workspace.minimize_active().is_some());
    assert_eq!(workspace.count(), 2);
    assert!(!workspace.window(second).unwrap().visible);
    assert_eq!(workspace.restore_minimized(), Some(second));
    assert_eq!(workspace.window(second).unwrap().state.selected_index, 3);
    assert_eq!(workspace.window(second).unwrap().task_handle, 42);
    assert_eq!(workspace.restore_minimized(), None);
    workspace.minimize_active();
    workspace.minimize_active();
    assert_eq!(workspace.active_index(), None);
    let exact = workspace.restore(second).expect("restore exact minimized slot");
    assert_eq!(exact.task_handle,42);
    assert_eq!(exact.state.selected_index,3);
    assert!(!workspace.window(first).unwrap().visible);
    assert!(workspace.restore(usize::MAX).is_none());
    workspace.restore(first).expect("restore other slot");
    workspace.raise(second);
    assert_eq!(workspace.active_index(), Some(second));
    let raised_first = workspace.raise(first).expect("raise first");
    assert_eq!(
        raised_first.state.active_namespace_ref.as_bytes(),
        b"/home/default/documents"
    );
    assert_eq!(raised_first.state.view_mode, ViewMode::Grid);
    assert_eq!(raised_first.state.scroll_offset, 144);
    assert_eq!(raised_first.x, 72);
    assert_eq!(
        workspace
            .window(second)
            .expect("second retained")
            .state
            .selected_index,
        3
    );
    assert_eq!(workspace.window(second).expect("second retained").x, 246);
    assert_eq!(workspace.back_to_front(1).expect("front layer").0, first);
    assert_eq!(workspace.topmost_at(300, 300), Some(first));
    assert_eq!(workspace.topmost_at(90, 300), Some(first));
    assert_eq!(workspace.topmost_at(990, 990), None);

    let mut menu_window = workspace.window(first).expect("menu window");
    menu_window.state.open_menu(FileNavigatorMenu::View);
    assert_eq!(menu_window.state.menu_open, Some(FileNavigatorMenu::View));
    assert_eq!(
        FileNavigatorMenu::File.action(0),
        Some(FileNavigatorAction::NewWindow)
    );
    assert_eq!(
        FileNavigatorMenu::View.action(2),
        Some(FileNavigatorAction::TogglePreview)
    );
    assert_eq!(FileNavigatorMenu::Performance.index(), 2);
    assert_eq!(
        FileNavigatorMenu::Performance.action(0),
        Some(FileNavigatorAction::SetPerformance(
            PerformanceMode::Restricted
        ))
    );
    assert_eq!(
        FileNavigatorMenu::Performance.action(2),
        Some(FileNavigatorAction::SetPerformance(
            PerformanceMode::Expanded
        ))
    );
    assert_eq!(
        FileNavigatorMenu::Navigate.action(7),
        Some(FileNavigatorAction::Navigate(7))
    );
    assert_eq!(
        FileNavigatorMenu::Navigate.action(8),
        Some(FileNavigatorAction::CustomLocation)
    );
    assert_eq!(FileNavigatorMenu::Help.item_count(), 1);
    menu_window.state.open_dialog(FileNavigatorDialog::About);
    assert_eq!(menu_window.state.menu_open, None);
    assert_eq!(
        menu_window.state.dialog_open,
        Some(FileNavigatorDialog::About)
    );
    menu_window.state.inspector_open = true;
    assert!(workspace.update_active(menu_window));

    let remaining = workspace.close_active().expect("second remains");
    assert_eq!(workspace.count(), 1);
    assert_eq!(workspace.active_index(), Some(second));
    assert_eq!(
        remaining.state.active_namespace_ref.as_bytes(),
        b"/home/default/pictures"
    );
    println!("PASS File Navigator workspace: independent state, geometry, click targeting, z-order, and close isolation");
}
