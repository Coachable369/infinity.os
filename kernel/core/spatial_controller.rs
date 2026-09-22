//! Native spatial workflows, using the existing shell and object boundaries.
use super::*;
use crate::ui::app_launcher::motion::{DeferredSelection, Motion};
use crate::ui::spatial::{
    DropRequest, DropTarget, Item, Label, Preview, SpatialState, OVERVIEW_COUNT,
};

struct EditorSnapshot {
    document: TextDocument,
    path: [u8; 95],
    path_length: usize,
    name: [u8; 47],
    name_length: usize,
    scroll: usize,
    valid: bool,
}
const EMPTY_EDITOR: EditorSnapshot = EditorSnapshot {
    document: TextDocument::new(),
    path: [0; 95],
    path_length: 0,
    name: [0; 47],
    name_length: 0,
    scroll: 0,
    valid: false,
};
// BSP shell-owned storage, never copied or allocated in a paint/input-motion path.
static mut WORLD_EDITORS: [EditorSnapshot; 4] = [const { EMPTY_EDITOR }; 4];
type NavigatorSnapshot = (
    crate::runtime::object_navigation::FileNavigatorWorkspace,
    Option<crate::runtime::object_navigation::FileNavigatorState>,
);
static mut WORLD_NAVIGATORS: [Option<NavigatorSnapshot>; 4] = [None; 4];

pub(super) struct Controller {
    pub open: bool,
    state: SpatialState,
    owner: [u8; 16],
    tab: usize,
    focus: usize,
    notice: &'static [u8],
    motion: Motion,
    closing: bool,
    last_progress: i32,
    editing: u8,
    text: [u8; 192],
    length: usize,
    caret: usize,
    link: Option<usize>,
    writable: bool,
    drag: Option<(usize, i32, i32, SpatialState)>,
    damage: Option<(usize, usize, usize, usize)>,
    zoom: Motion,
    zoom_target: i32,
    last_zoom: i32,
    last_caret_phase: u64,
    selection: DeferredSelection,
    ghost: Option<(i32, i32)>,
    pub refresh: core::cell::Cell<bool>,
    previews: [Preview; OVERVIEW_COUNT],
    preview_count: usize,
    refreshed_at: u64,
    pub arriving: bool,
    arrival: Motion,
    pending_drop: Option<DropRequest>,
    settling: Option<(usize, usize, usize)>,
    settle: Motion,
}
impl Controller {
    // ------------------------=
    // FUNC: new
    // DESC: Initializes private, allocation-free interaction state.
    // ------------------=
    pub const fn new() -> Self {
        Self {
            open: false,
            state: SpatialState::new([0; 16]),
            owner: [0; 16],
            tab: 0,
            focus: 0,
            notice: b"Choose an open app to return to it.",
            motion: Motion::settled(0),
            closing: false,
            last_progress: -1,
            editing: 0,
            text: [0; 192],
            length: 0,
            caret: 0,
            link: None,
            writable: false,
            drag: None,
            damage: None,
            zoom: Motion::settled(0),
            zoom_target: 0,
            last_zoom: -1,
            last_caret_phase: 0,
            selection: DeferredSelection::new(),
            ghost: None,
            refresh: core::cell::Cell::new(false),
            previews: [Preview::EMPTY; OVERVIEW_COUNT],
            preview_count: 0,
            refreshed_at: 0,
            arriving: false,
            arrival: Motion::settled(255),
            pending_drop: None,
            settling: None,
            settle: Motion::settled(255),
        }
    }
}
// ------------------------=
// FUNC: now
// DESC: Converts the shared monotonic clock to animation milliseconds.
// ------------------=
fn now() -> u64 {
    crate::ui::performance::monotonic_ns().unwrap_or(0) / 1_000_000
}

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: spatial_confirm_drop
    // DESC: Applies an explicit drop only after confirmation, revalidating source identity and never moving originals.
    // ------------------=
    fn spatial_confirm_drop(&mut self) {
        let Some(request) = self.spatial.pending_drop.take() else {
            return;
        };
        let Some(item) = self.spatial.state.items[request.index] else {
            return;
        };
        self.spatial.focus = request.index;
        match request.target {
            DropTarget::Collection(group) => {
                let old = self.spatial.state;
                let (x, y, _, _) = crate::ui::spatial::item_card(request.index, &item);
                let target_x = (80 + usize::from(group) * 210).min(710) as u16;
                let target_y = (230 + request.index / 4 * 130) as u16;
                if self
                    .spatial
                    .state
                    .place(
                        self.current_user.0,
                        request.index,
                        group,
                        target_x,
                        target_y,
                    )
                    .is_ok()
                    && self.spatial_commit(old)
                {
                    self.spatial.settling = Some((request.index, x, y));
                    self.spatial.settle = Motion::settled(0);
                    self.spatial.settle.retarget(
                        255,
                        now(),
                        180,
                        self.spatial.state.reduced_motion
                            || crate::ui::performance::monotonic_ns().is_none(),
                    );
                }
            }
            DropTarget::Editor | DropTarget::Folder(_) => {
                if crate::storage::namespace_resolve(item.path.get())
                    .ok()
                    .map(|id| id.0)
                    != Some(item.object)
                {
                    self.spatial.notice =
                        b"Source moved or unavailable. Recollect it before dropping.";
                    return;
                }
                if matches!(request.target, DropTarget::Editor) {
                    if !self.editor_document.is_saved() {
                        self.spatial.notice =
                            b"Save the modified editor document before opening a file.";
                        return;
                    }
                    if !crate::storage::object_inspect_path(item.path.get()).is_ok_and(|(m, _)| {
                        m.content_type == crate::storage::object::ContentType::Utf8Text
                    }) {
                        self.spatial.notice =
                            b"The editor accepts text files. Source files are unchanged.";
                        return;
                    }
                    self.spatial_close();
                    self.open_text_editor_path(item.path.get());
                    self.redraw();
                    return;
                }
                let DropTarget::Folder(parent) = request.target else {
                    return;
                };
                let leaf = crate::runtime::object_navigation::namespace_basename(item.path.get());
                for suffix in 0..100usize {
                    let mut name = [0u8; 64];
                    if leaf.len() > 48 {
                        break;
                    }
                    name[..leaf.len()].copy_from_slice(leaf);
                    let mut length = leaf.len();
                    if suffix != 0 {
                        name[length..length + 6].copy_from_slice(b" copy ");
                        length += 6;
                        length += write_decimal(&mut name[length..], suffix);
                    }
                    let Ok(destination) = crate::runtime::object_navigation::namespace_child_path(
                        parent.get(),
                        &name[..length],
                    ) else {
                        break;
                    };
                    match crate::storage::object_copy_path(item.path.get(), destination.as_bytes())
                    {
                        Ok(_) => {
                            self.spatial.notice =
                                b"Copy created in File Navigator. Original file unchanged.";
                            self.spatial.refresh.set(true);
                            return;
                        }
                        Err(crate::storage::object::ObjectError::NameConflict) => continue,
                        Err(_) => break,
                    }
                }
                self.spatial.notice = b"Copy failed or unsupported. No source file was moved.";
            }
        }
    }
    // ------------------------=
    // FUNC: spatial_previews
    // DESC: Projects independent navigator identities and application visibility outside the paint path.
    // ------------------=
    fn spatial_previews(&mut self) {
        let mut entries = [Preview::EMPTY; OVERVIEW_COUNT];
        let mut count = 0;
        crate::runtime::with_runtime(|r| {
            for index in 0..crate::runtime::object_navigation::MAX_FILE_NAVIGATOR_INSTANCES {
                if let Some(window) = r.file_navigators.window(index) {
                    if !window.visible {
                        continue;
                    }
                    let mut p = Preview::EMPTY;
                    p.slot = 6 + index;
                    p.navigator = Some(index);
                    p.visible = true;
                    p.label.set(window.state.active_namespace_ref.as_bytes());
                    entries[count] = p;
                    count += 1;
                }
            }
        });
        if count == 0 {
            entries[0].label.set(b"Files");
            entries[0].visible = self.home_window_visible;
            count = 1;
        }
        let (editor, command, tasks) = self.desktop_app_windows();
        for (app, visible, label) in [
            (1, command.visible, b"Command".as_slice()),
            (2, editor.visible, b"Text Editor"),
            (3, tasks.visible, b"Task Manager"),
            (4, self.settings_open, b"Settings"),
        ] {
            let mut p = Preview::EMPTY;
            p.app = app;
            p.slot = app as usize;
            p.visible = visible;
            p.label.set(label);
            entries[count] = p;
            count += 1;
        }
        self.spatial.previews = entries;
        self.spatial.preview_count = count;
        if self.spatial.tab == 0 {
            self.spatial.focus = self.spatial.focus.min(count - 1);
        }
    }
    // ------------------------=
    // FUNC: spatial_open
    // DESC: Opens the native overlay after loading only this authenticated user's checkpoint.
    // ------------------=
    pub(super) fn spatial_open(&mut self) {
        if self.current_session.is_zero() {
            return;
        }
        if self.mode == ConsoleMode::AppLauncher {
            self.close_app_launcher();
        }
        if self.mode == ConsoleMode::SystemMenu {
            self.close_shell_menu();
        }
        if self.spatial.owner != self.current_user.0 {
            unsafe {
                for snapshot in &mut *(&raw mut WORLD_EDITORS) {
                    *snapshot = EMPTY_EDITOR;
                }
                *(&raw mut WORLD_NAVIGATORS) = [None; 4];
            }
            self.spatial = Controller::new();
            self.spatial.owner = self.current_user.0;
            match crate::storage::spatial_state::load(self.current_user.0, self.current_session.0) {
                Ok(state) => {
                    self.spatial.state = state;
                    self.spatial.writable = true;
                }
                Err(
                    crate::storage::object::ObjectError::NotFound
                    | crate::storage::object::ObjectError::NamespaceNotFound,
                ) => {
                    self.spatial.state = SpatialState::new(self.current_user.0);
                    self.spatial.writable = true;
                }
                Err(_) => {
                    self.spatial.notice =
                        b"Checkpoint unavailable. Existing data will not be overwritten.";
                }
            }
        }
        self.checkpoint_active_file_navigator();
        self.spatial_previews();
        self.redraw();
        self.spatial.open = true;
        self.spatial.closing = false;
        self.spatial.last_progress = -1;
        self.spatial.motion = Motion::settled(0);
        self.spatial.motion.retarget(
            255,
            now(),
            220,
            self.spatial.state.reduced_motion || crate::ui::performance::monotonic_ns().is_none(),
        );
        self.spatial_present();
    }
    // ------------------------=
    // FUNC: spatial_close
    // DESC: Restores the desktop without discarding any running application or document.
    // ------------------=
    pub(super) fn spatial_close(&mut self) {
        if let Some((_, _, _, old)) = self.spatial.drag.take() {
            self.spatial.state = old;
        }
        self.spatial.open = false;
        self.spatial.selection.cancel();
        self.spatial.ghost = None;
        self.spatial.pending_drop = None;
        self.spatial.settling = None;
        self.spatial.closing = false;
        self.spatial.editing = 0;
        self.spatial.text.fill(0);
        self.spatial.length = 0;
        crate::bootstrap::spatial_close();
        self.redraw();
    }
    // ------------------------=
    // FUNC: spatial_present
    // DESC: Renders retained previews; no application query or layout work is performed in painting.
    // ------------------=
    pub(super) fn spatial_present(&self) {
        let mut painted = self.spatial.state;
        if let Some((index, x, y)) = self.spatial.settling {
            if let Some(item) = painted.items[index].as_mut() {
                let t = self.spatial.settle.value(now()).clamp(0, 255) as usize;
                item.x = ((x * (255 - t) + usize::from(item.x) * t) / 255) as u16;
                item.y = ((y * (255 - t) + usize::from(item.y) * t) / 255) as u16;
            }
        }
        crate::bootstrap::spatial_present(
            &painted,
            self.spatial.tab,
            self.spatial.focus,
            &self.spatial.previews[..self.spatial.preview_count],
            self.spatial.notice,
            self.spatial.motion.value(now()).clamp(0, 255) as u8,
            self.pointer_x,
            self.pointer_y,
            if self.spatial.editing != 0 {
                Some((
                    &self.spatial.text[..self.spatial.length],
                    self.spatial.caret,
                ))
            } else {
                None
            },
            self.spatial.damage,
            self.spatial.zoom.value(now()).clamp(0, 255) as u8,
            self.spatial.ghost.and(self.spatial.drag.map(|d| d.0)),
            self.spatial.pending_drop,
        );
    }
    // ------------------------=
    // FUNC: spatial_tick
    // DESC: Schedules only changing animation frames and settles without idle repaints.
    // ------------------=
    pub(super) fn spatial_tick(&mut self) -> bool {
        if self.spatial.settling.is_some() {
            if !self.spatial.settle.active(now()) {
                self.spatial.settling = None;
            }
            self.spatial.damage = Some((70, 220, 850, 530));
            self.spatial_present();
            self.spatial.damage = None;
            return true;
        }
        if self.spatial.refresh.get() && now().saturating_sub(self.spatial.refreshed_at) >= 100 {
            self.spatial.refresh.set(false);
            self.spatial.refreshed_at = now();
            crate::bootstrap::spatial_refresh_begin();
            self.redraw_scene();
            crate::bootstrap::spatial_refresh_end();
            self.spatial_previews();
            self.spatial_present();
            return true;
        }
        let progress = self.spatial.motion.value(now());
        if self.spatial.closing && progress == 0 {
            if let Some((tab, focus)) = self.spatial.selection.finish(progress) {
                self.spatial.closing = false;
                self.spatial.tab = tab;
                self.spatial.focus = focus;
                if tab == 1 {
                    crate::bootstrap::spatial_arrival_begin();
                }
                self.spatial_execute_action(0);
                if tab == 1 {
                    if self.spatial.open {
                        crate::bootstrap::spatial_arrival_cancel();
                    } else {
                        crate::bootstrap::spatial_arrival_capture();
                        self.spatial.arriving = true;
                        self.spatial.arrival = Motion::settled(0);
                        self.spatial.arrival.retarget(
                            255,
                            now(),
                            180,
                            self.spatial.state.reduced_motion
                                || crate::ui::performance::monotonic_ns().is_none(),
                        );
                        self.spatial_arrival_tick();
                    }
                }
                if self.spatial.open {
                    self.spatial.motion.retarget(
                        255,
                        now(),
                        220,
                        self.spatial.state.reduced_motion,
                    );
                }
                return true;
            }
            self.spatial_close();
            return true;
        }
        let zoom = self.spatial.zoom.value(now());
        let phase = now() / 500;
        if self.spatial.editing != 0 && phase != self.spatial.last_caret_phase {
            self.spatial.last_caret_phase = phase;
            self.spatial.damage = Some((95, 385, 810, 160));
            self.spatial_present();
            self.spatial.damage = None;
            return true;
        }
        if progress == self.spatial.last_progress && zoom == self.spatial.last_zoom {
            return false;
        }
        self.spatial.last_progress = progress;
        self.spatial.last_zoom = zoom;
        self.spatial_present();
        true
    }
    // ------------------------=
    // FUNC: spatial_arrival_tick
    // DESC: Presents the committed destination from a retained frame without rerunning app painters.
    // ------------------=
    pub(super) fn spatial_arrival_tick(&mut self) -> bool {
        let value = self.spatial.arrival.value(now()).clamp(0, 255) as u8;
        crate::bootstrap::spatial_arrival_present(value, self.pointer_x, self.pointer_y);
        if value == 255 {
            self.spatial.arriving = false;
        }
        true
    }
    // ------------------------=
    // FUNC: spatial_finish_arrival
    // DESC: Finishes presentation immediately before accepting new user input.
    // ------------------=
    pub(super) fn spatial_finish_arrival(&mut self) {
        if self.spatial.arriving {
            crate::bootstrap::spatial_arrival_present(255, self.pointer_x, self.pointer_y);
            self.spatial.arriving = false;
        }
    }
    // ------------------------=
    // FUNC: spatial_commit
    // DESC: Persists a complete checkpoint, rolling back visible metadata on storage failure.
    // ------------------=
    fn spatial_commit(&mut self, previous: SpatialState) -> bool {
        if !self.spatial.writable
            || crate::storage::spatial_state::commit(
                self.current_user.0,
                self.current_session.0,
                &self.spatial.state,
            )
            .is_err()
        {
            self.spatial.state = previous;
            self.spatial.notice = b"Could not save. No references or source files were changed.";
            return false;
        }
        self.spatial.notice = b"Saved to your installed workspace.";
        true
    }
    // ------------------------=
    // FUNC: spatial_input
    // DESC: Provides keyboard navigation, explicit reference removal, and normal bounded text editing.
    // ------------------=
    pub(super) fn spatial_input(&mut self, key: ConsoleKey) {
        if self.spatial.pending_drop.is_some() {
            match key {
                ConsoleKey::Enter => self.spatial_confirm_drop(),
                ConsoleKey::Escape => {
                    self.spatial.pending_drop = None;
                    self.spatial.notice = b"Drop cancelled. Source files are unchanged.";
                }
                _ => {}
            }
            if self.spatial.open {
                self.spatial_present();
            }
            return;
        }
        self.spatial.settling = None;
        if self.spatial.closing && self.spatial.selection.cancel() {
            self.spatial.closing = false;
            self.spatial
                .motion
                .retarget(255, now(), 180, self.spatial.state.reduced_motion);
            self.spatial.notice = b"Switch cancelled. Your workspace is unchanged.";
            self.spatial_present();
            return;
        }
        if let Some((_, _, _, old)) = self.spatial.drag.take() {
            self.spatial.state = old;
        }
        self.spatial.ghost = None;
        if self.spatial.editing != 0 {
            if matches!(key, ConsoleKey::Escape) {
                self.spatial.editing = 0;
            } else if matches!(key, ConsoleKey::Enter) {
                self.spatial_accept_text();
            } else {
                let s = &mut self.spatial;
                match key {
                    ConsoleKey::Character(c) => {
                        crate::ui::text_input::insert_ascii(
                            &mut s.text,
                            &mut s.length,
                            &mut s.caret,
                            c,
                        );
                    }
                    ConsoleKey::Backspace => {
                        crate::ui::text_input::backspace(&mut s.text, &mut s.length, &mut s.caret);
                    }
                    ConsoleKey::Delete => {
                        crate::ui::text_input::delete(&mut s.text, &mut s.length, &mut s.caret);
                    }
                    ConsoleKey::Left => {
                        crate::ui::text_input::move_caret(&mut s.caret, s.length, -1);
                    }
                    ConsoleKey::Right => {
                        crate::ui::text_input::move_caret(&mut s.caret, s.length, 1);
                    }
                    ConsoleKey::Home => {
                        s.caret = 0;
                    }
                    ConsoleKey::End => {
                        s.caret = s.length;
                    }
                    _ => {}
                }
            }
            self.spatial_present();
            return;
        }
        let count = if self.spatial.tab == 0 {
            self.spatial.preview_count.max(1)
        } else if self.spatial.tab == 1 {
            4
        } else {
            16
        };
        match key {
            ConsoleKey::Character(b'c') if self.spatial.tab >= 2 => self.spatial_action(0),
            ConsoleKey::Character(b's') if self.spatial.tab == 1 => self.spatial_action(1),
            ConsoleKey::Character(b'r') if self.spatial.tab == 1 => self.spatial_action(2),
            ConsoleKey::Character(b't') if self.spatial.tab == 3 => self.spatial_action(1),
            ConsoleKey::Character(b'l') if self.spatial.tab == 4 => self.spatial_action(1),
            ConsoleKey::Character(b'g') if self.spatial.tab == 2 => self.spatial_action(1),
            ConsoleKey::Character(b'm') if self.spatial.tab < 2 => self.spatial_action(3),
            ConsoleKey::Character(b'+' | b'=') if self.spatial.tab == 0 => {
                self.spatial_scroll(1);
            }
            ConsoleKey::Character(b'-') if self.spatial.tab == 0 => {
                self.spatial_scroll(-1);
            }
            ConsoleKey::Escape | ConsoleKey::Shortcut(b'K') => {
                self.spatial.closing = true;
                self.spatial.motion.retarget(
                    0,
                    now(),
                    160,
                    self.spatial.state.reduced_motion
                        || crate::ui::performance::monotonic_ns().is_none(),
                );
            }
            ConsoleKey::Tab(back) => {
                self.spatial.tab = (self.spatial.tab + if back { 4 } else { 1 }) % 5;
                self.spatial.focus = 0;
                self.spatial.link = None;
            }
            ConsoleKey::Left => self.spatial.focus = (self.spatial.focus + count - 1) % count,
            ConsoleKey::Right => self.spatial.focus = (self.spatial.focus + 1) % count,
            ConsoleKey::Up => {
                self.spatial.focus =
                    self.spatial
                        .focus
                        .saturating_sub(if self.spatial.tab == 0 && count <= 5 {
                            3
                        } else {
                            4
                        })
            }
            ConsoleKey::Down => {
                self.spatial.focus = (self.spatial.focus
                    + if self.spatial.tab == 0 && count <= 5 {
                        3
                    } else {
                        4
                    })
                .min(count - 1)
            }
            ConsoleKey::Enter => self.spatial_action(if self.spatial.tab < 2 { 0 } else { 3 }),
            ConsoleKey::Delete if self.spatial.tab >= 2 => self.spatial_action(2),
            _ => {}
        }
        if self.spatial.open {
            self.spatial_present();
        }
    }
    // ------------------------=
    // FUNC: spatial_pointer
    // DESC: Routes explicit clicks while keeping ordinary pointer motion cursor-only.
    // ------------------=
    pub(super) fn spatial_pointer(&mut self, clicked: bool, released: bool) {
        if self.spatial.pending_drop.is_some() {
            if clicked && (540..588).contains(&self.pointer_y) {
                if (540..830).contains(&self.pointer_x) {
                    self.spatial_input(ConsoleKey::Enter);
                } else if (170..460).contains(&self.pointer_x) {
                    self.spatial_input(ConsoleKey::Escape);
                }
            }
            crate::bootstrap::system_ui_cursor(self.pointer_x, self.pointer_y);
            return;
        }
        if clicked {
            self.spatial.settling = None;
        }
        if self.spatial.closing {
            if clicked {
                self.spatial_input(ConsoleKey::Escape);
            } else {
                crate::bootstrap::system_ui_cursor(self.pointer_x, self.pointer_y);
            }
            return;
        }
        crate::ui::text_input::set_pointer_shape(
            if self.spatial.editing != 0
                && (100..900).contains(&self.pointer_x)
                && (390..540).contains(&self.pointer_y)
            {
                crate::ui::text_input::PointerShape::Text
            } else {
                crate::ui::text_input::PointerShape::Default
            },
        );
        if let Some((index, x, y, old)) = self.spatial.drag {
            let moved = (self.pointer_x - x).abs() + (self.pointer_y - y).abs() > 6;
            if self.spatial.tab == 3 {
                if released {
                    self.spatial.drag = None;
                    self.spatial.ghost = None;
                    if moved
                        && self.pointer_y < 570
                        && self.mode == ConsoleMode::Desktop
                        && self.desktop_app == DesktopAppKind::TextEditor
                    {
                        let (editor, _, _) = self.desktop_app_windows();
                        let layout = SystemLayout::new(
                            self.system.framebuffer_width,
                            self.system.framebuffer_height,
                        );
                        let content = layout
                            .desktop_app_window_geometry(
                                editor.x,
                                editor.y,
                                editor.width,
                                editor.height,
                                editor.maximized,
                            )
                            .content;
                        let point = crate::ui::geometry::Point {
                            x: self.pointer_x * self.system.framebuffer_width as i32 / 1000,
                            y: self.pointer_y * self.system.framebuffer_height as i32 / 1000,
                        };
                        if editor.visible && content.contains(point) {
                            if self.spatial.state.items[index].is_some_and(|i| i.object != [0; 16])
                            {
                                self.spatial.pending_drop = DropRequest::new(
                                    &self.spatial.state,
                                    index,
                                    DropTarget::Editor,
                                );
                                self.spatial_present();
                            } else {
                                self.spatial_action(3);
                            }
                            return;
                        }
                    }
                    if moved
                        && self.pointer_y < 570
                        && self.mode == ConsoleMode::Desktop
                        && self.desktop_app == DesktopAppKind::None
                        && self.home_window_visible
                    {
                        let layout = SystemLayout::new(
                            self.system.framebuffer_width,
                            self.system.framebuffer_height,
                        );
                        let (x, y, w, h) = layout.home_window_geometry_sized(
                            self.home_window_x,
                            self.home_window_y,
                            self.home_window_width,
                            self.home_window_height,
                            self.home_window_maximized,
                        );
                        let px = self.pointer_x * self.system.framebuffer_width as i32 / 1000;
                        let py = self.pointer_y * self.system.framebuffer_height as i32 / 1000;
                        if px >= x as i32
                            && px < (x + w) as i32
                            && py >= (y + 100 * layout.scale()) as i32
                            && py < (y + h) as i32
                        {
                            if let Some(path) = crate::runtime::with_runtime(|r| {
                                r.file_navigator.map(|n| n.active_namespace_ref)
                            })
                            .flatten()
                            {
                                let mut target = Label::empty();
                                if target.set(path.as_bytes()) {
                                    self.spatial.pending_drop = DropRequest::new(
                                        &self.spatial.state,
                                        index,
                                        DropTarget::Folder(target),
                                    );
                                }
                            }
                        }
                    }
                    self.spatial_present();
                } else if moved && self.pointer_pressed {
                    let next = (self.pointer_x.clamp(45, 745), self.pointer_y.clamp(80, 760));
                    let previous = self.spatial.ghost.unwrap_or(next);
                    self.spatial.ghost = Some(next);
                    let left = previous.0.min(next.0).saturating_sub(8) as usize;
                    let top = previous.1.min(next.1).saturating_sub(8) as usize;
                    self.spatial.damage = Some((
                        left,
                        top,
                        (previous.0 - next.0).unsigned_abs() as usize + 206,
                        (previous.1 - next.1).unsigned_abs() as usize + 86,
                    ));
                    self.spatial_present();
                    self.spatial.damage = None;
                }
                crate::bootstrap::system_ui_cursor(self.pointer_x, self.pointer_y);
                return;
            }
            if released {
                self.spatial.drag = None;
                if self.spatial.state != old {
                    if self.spatial.tab == 2 && (750..785).contains(&self.pointer_y) {
                        for group in 0..4 {
                            if (80 + group * 210..270 + group * 210).contains(&self.pointer_x) {
                                self.spatial.state = old;
                                self.spatial.pending_drop = DropRequest::new(
                                    &old,
                                    index,
                                    DropTarget::Collection(group as u8),
                                );
                                self.spatial_present();
                                return;
                            }
                        }
                    }
                    self.spatial_commit(old);
                    self.spatial_present();
                }
            } else if moved && self.pointer_pressed {
                let original = old.items[index].unwrap();
                let (a, b, _, _) = crate::ui::spatial::item_card(index, &original);
                let previous =
                    crate::ui::spatial::item_card(index, &self.spatial.state.items[index].unwrap());
                if let Some(item) = self.spatial.state.items[index].as_mut() {
                    item.x = (a as i32 + self.pointer_x - x).clamp(80, 710) as u16;
                    item.y = (b as i32 + self.pointer_y - y).clamp(230, 620) as u16;
                }
                let next =
                    crate::ui::spatial::item_card(index, &self.spatial.state.items[index].unwrap());
                let left = previous.0.min(next.0).saturating_sub(8);
                let top = previous.1.min(next.1).saturating_sub(8);
                self.spatial.damage = Some(if self.spatial.tab == 4 && original.links != 0 {
                    (70, 220, 850, 530)
                } else {
                    (
                        left,
                        top,
                        previous.0.max(next.0) + 198 - left,
                        previous.1.max(next.1) + 118 - top,
                    )
                });
                self.spatial_present();
                self.spatial.damage = None;
                return;
            }
        }
        if !clicked {
            crate::bootstrap::system_ui_cursor(self.pointer_x, self.pointer_y);
            return;
        }
        let (x, y) = (
            self.pointer_x,
            self.pointer_y - (255 - self.spatial.motion.value(now()).clamp(0, 255)) * 35 / 255,
        );
        let shelf = self.spatial.tab == 3;
        let close_top = if shelf { 585 } else { 95 };
        if (897..941).contains(&x) && (close_top..close_top + 38).contains(&y) {
            self.spatial_input(ConsoleKey::Escape);
            return;
        }
        if self.spatial.editing != 0 {
            return;
        }
        let tabs_top = if shelf { 635 } else { 150 };
        if (tabs_top..tabs_top + 45).contains(&y) {
            for i in 0..5 {
                if (70 + i * 176..234 + i * 176).contains(&x) {
                    self.spatial.tab = i as usize;
                    self.spatial.focus = 0;
                    self.spatial.link = None;
                }
            }
        }
        let hit = if self.spatial.tab == 3 {
            (0..16).find(|i| {
                self.spatial.state.items[*i].is_some()
                    && crate::ui::spatial::shelf_card(*i, self.spatial.focus)
                        .map(|r| crate::ui::spatial::contains(r, x, y))
                        .unwrap_or(false)
            })
        } else if self.spatial.tab >= 2 {
            crate::ui::spatial::hit_item(&self.spatial.state, x, y)
        } else if self.spatial.tab == 1 {
            (0..4).find(|i| crate::ui::spatial::contains(crate::ui::spatial::world_card(*i), x, y))
        } else {
            let focus = self.spatial.focus;
            core::iter::once(focus)
                .chain((0..self.spatial.preview_count).filter(|i| *i != focus))
                .find(|i| {
                    crate::ui::spatial::contains(
                        crate::ui::spatial::overview_bounds(
                            *i,
                            focus,
                            self.spatial.zoom.value(now()).clamp(0, 255) as u8,
                            self.spatial.preview_count,
                        ),
                        x,
                        y,
                    )
                })
        };
        if let Some(i) = hit {
            self.spatial.focus = i;
            if self.spatial.tab >= 2 {
                self.spatial.drag = Some((i, x, y, self.spatial.state));
            }
        }
        if (805..853).contains(&y) {
            for i in 0..4 {
                if (80 + i * 210..270 + i * 210).contains(&x) {
                    self.spatial_action(i as usize);
                    break;
                }
            }
        }
        if self.spatial.open {
            self.spatial_present();
        }
    }
    // ------------------------=
    // FUNC: spatial_collect
    // DESC: Captures a real navigator selection as a reference; never moves or modifies the source.
    // ------------------=
    fn spatial_collect(&mut self) {
        if self.desktop_app == DesktopAppKind::TextEditor {
            if let Some((start, end)) = self.editor_document.selection() {
                let text = &self.editor_document.bytes()[start..end];
                let mut item = Item {
                    object: [0; 16],
                    path: Label::empty(),
                    name: Label::empty(),
                    text: Label::empty(),
                    collection: self.spatial.state.active_world,
                    x: 0,
                    y: 0,
                    links: 0,
                };
                if !item.text.set(text) || !item.name.set(&text[..text.len().min(32)]) {
                    self.spatial.notice =
                        b"Select up to 192 printable characters for a text clipping.";
                    return;
                }
                let old = self.spatial.state;
                match self.spatial.state.gather(self.current_user.0, item) {
                    Ok(i) => {
                        self.spatial.focus = i;
                        self.spatial_commit(old);
                    }
                    Err(_) => self.spatial.notice = b"Shelf is full.",
                }
                return;
            }
        }
        let navigator = crate::runtime::with_runtime(|r| r.file_navigator).flatten();
        let Some(n) = navigator else {
            self.spatial.notice = b"Select a file in File Navigator first.";
            return;
        };
        let Some(entry) =
            navigator_child_nth(n.active_namespace_ref.as_bytes(), n.selected_index as usize)
        else {
            self.spatial.notice = b"Select a file or folder, not . or .. .";
            return;
        };
        let path = &entry.path[..entry.path_len as usize];
        let name = path.rsplit(|b| *b == b'/').next().unwrap_or(path);
        let mut item = Item {
            object: entry.object.id.0,
            path: Label::empty(),
            name: Label::empty(),
            text: Label::empty(),
            collection: self.spatial.state.active_world,
            x: 0,
            y: 0,
            links: 0,
        };
        if !item.path.set(path) || !item.name.set(name) {
            self.spatial.notice = b"This reference exceeds the supported name or path length.";
            return;
        }
        let old = self.spatial.state;
        match self.spatial.state.gather(self.current_user.0, item) {
            Ok(i) => {
                self.spatial.focus = i;
                self.spatial_commit(old);
            }
            Err(_) => self.spatial.notice = b"Shelf is full. Remove a reference to make room.",
        }
    }
    // ------------------------=
    // FUNC: spatial_accept_text
    // DESC: Commits a renamed environment or text clipping without touching source documents.
    // ------------------=
    fn spatial_accept_text(&mut self) {
        let old = self.spatial.state;
        let s = &mut self.spatial;
        if s.length == 0 {
            s.notice = b"Enter a name or text before saving.";
            return;
        }
        if s.editing == 1 {
            if !s.state.worlds[s.focus].name.set(&s.text[..s.length]) {
                s.notice = b"World names may contain up to 24 characters.";
                return;
            }
        } else {
            let mut item = Item {
                object: [0; 16],
                path: Label::empty(),
                name: Label::empty(),
                text: Label::empty(),
                collection: s.state.active_world,
                x: 0,
                y: 0,
                links: 0,
            };
            item.text.set(&s.text[..s.length]);
            item.name.set(&s.text[..s.length.min(32)]);
            match s.state.gather(self.current_user.0, item) {
                Ok(i) => s.focus = i,
                Err(_) => {
                    s.notice = b"Shelf is full.";
                    return;
                }
            }
        }
        s.editing = 0;
        s.text.fill(0);
        s.length = 0;
        self.spatial_commit(old);
    }
    // ------------------------=
    // FUNC: spatial_action
    // DESC: Executes user-selected workspace actions through existing shell services.
    // ------------------=
    fn spatial_action(&mut self, action: usize) {
        if self.spatial.tab < 2 && action == 0 {
            self.spatial
                .selection
                .request(self.spatial.tab, self.spatial.focus);
            self.spatial.closing = true;
            self.spatial.notice = b"Switching workspace. Press any key to cancel.";
            self.spatial.motion.retarget(
                0,
                now(),
                280,
                self.spatial.state.reduced_motion
                    || crate::ui::performance::monotonic_ns().is_none(),
            );
            return;
        }
        self.spatial_execute_action(action);
    }
    // ------------------------=
    // FUNC: spatial_execute_action
    // DESC: Applies a selected action after its cancellable transition reaches the endpoint.
    // ------------------=
    fn spatial_execute_action(&mut self, action: usize) {
        let tab = self.spatial.tab;
        let index = self.spatial.focus;
        if tab < 2 && action == 3 {
            let old = self.spatial.state;
            self.spatial.state.reduced_motion = !old.reduced_motion;
            self.spatial_commit(old);
            return;
        }
        if tab == 0 && action == 0 {
            let Some(preview) = self.spatial.previews.get(index).copied() else {
                return;
            };
            self.spatial_close();
            self.mode = ConsoleMode::Desktop;
            match preview.app {
                0 => {
                    if let Some(navigator) = preview.navigator {
                        self.load_file_navigator_window(navigator);
                    } else if !self.home_window_visible {
                        self.open_file_navigator_window(b"/home/default");
                    } else {
                        self.focus_desktop_app(DesktopAppKind::None);
                    }
                }
                1 => self.open_command_window(),
                2 => self.open_text_editor(),
                3 => self.open_task_manager(),
                _ => self.open_settings(0),
            }
            self.redraw();
            return;
        }
        if tab == 1 {
            if action == 2 {
                self.spatial.editing = 1;
                self.spatial.length = 0;
                self.spatial.caret = 0;
                self.spatial.notice = b"World name (24 characters). Enter saves; Esc cancels.";
                return;
            }
            self.checkpoint_active_file_navigator();
            let old = self.spatial.state;
            let layout = self.capture_desktop_layout();
            let location =
                crate::runtime::with_runtime(|r| r.file_navigator.map(|n| n.active_namespace_ref))
                    .flatten();
            if action == 1 {
                self.spatial.state.worlds[index].layout = Some(layout);
                self.spatial.state.worlds[index]
                    .editor
                    .set(&self.editor_document_path[..self.editor_document_path_length]);
                if let Some(location) = location {
                    self.spatial.state.worlds[index]
                        .location
                        .set(location.as_bytes());
                }
                self.spatial_commit(old);
            }
            if action == 0 {
                let active = self.spatial.state.active_world as usize;
                self.spatial.state.worlds[active].layout = Some(layout);
                self.spatial.state.worlds[active]
                    .editor
                    .set(&self.editor_document_path[..self.editor_document_path_length]);
                if let Some(location) = location {
                    self.spatial.state.worlds[active]
                        .location
                        .set(location.as_bytes());
                }
                self.spatial.state.active_world = index as u8;
                if self.spatial_commit(old) {
                    if active != index {
                        let previous =
                            crate::runtime::with_runtime(|r| (r.file_navigators, r.file_navigator));
                        unsafe {
                            (*(&raw mut WORLD_NAVIGATORS))[active] = previous;
                            let snapshot = &mut (*(&raw mut WORLD_EDITORS))[active];
                            snapshot.document = self.editor_document;
                            snapshot.path = self.editor_document_path;
                            snapshot.path_length = self.editor_document_path_length;
                            snapshot.name = self.editor_document_name;
                            snapshot.name_length = self.editor_document_name_length;
                            snapshot.scroll = self.editor_scroll_row;
                            snapshot.valid = true;
                        }
                    }
                    let target = self.spatial.state.worlds[index].layout;
                    let location = self.spatial.state.worlds[index].location;
                    self.spatial_close();
                    self.mode = ConsoleMode::Desktop;
                    if active != index {
                        let destination = unsafe { (*(&raw const WORLD_NAVIGATORS))[index] };
                        crate::runtime::with_runtime(|r| {
                            let (windows, current) = destination.unwrap_or((
                                crate::runtime::object_navigation::FileNavigatorWorkspace::new(),
                                None,
                            ));
                            r.file_navigators = windows;
                            r.file_navigator = current;
                        });
                    }
                    let destination_layout = if let Some(layout) = target {
                        layout
                    } else {
                        let mut empty = layout;
                        empty.home.visible = false;
                        empty.editor.visible = false;
                        empty.command.visible = false;
                        empty.task_manager.visible = false;
                        empty.settings.visible = false;
                        empty.focused_surface = DesktopResumeSurface::Workspace;
                        empty
                    };
                    self.restore_desktop_layout(destination_layout);
                    if active != index {
                        let restored = unsafe {
                            let snapshot = &(*(&raw const WORLD_EDITORS))[index];
                            if snapshot.valid {
                                self.editor_document = snapshot.document;
                                self.editor_document_path = snapshot.path;
                                self.editor_document_path_length = snapshot.path_length;
                                self.editor_document_name = snapshot.name;
                                self.editor_document_name_length = snapshot.name_length;
                                self.editor_scroll_row = snapshot.scroll;
                                true
                            } else {
                                false
                            }
                        };
                        if !restored {
                            self.editor_document = TextDocument::new();
                            self.editor_document_path_length = 0;
                            self.editor_document_name_length = 0;
                            self.editor_scroll_row = 0;
                            let path = self.spatial.state.worlds[index].editor;
                            if !path.get().is_empty() {
                                self.open_text_editor_path(path.get());
                            }
                        }
                    }
                    if !location.get().is_empty() {
                        let _ = crate::runtime::with_runtime(|r| {
                            if r.file_navigator.is_none() {
                                r.file_navigator =
                                    crate::runtime::object_navigation::FileNavigatorState::new(
                                        location.get(),
                                    )
                                    .ok();
                            }
                            r.file_navigator
                                .as_mut()
                                .map(|n| n.navigate(location.get()))
                        });
                        self.checkpoint_active_file_navigator();
                    }
                    // Loading a saved document may focus Editor; the world's
                    // saved foreground surface remains authoritative.
                    self.restore_desktop_layout(destination_layout);
                    self.redraw();
                }
            }
            return;
        }
        if tab < 2 {
            return;
        }
        if action == 0 {
            self.spatial_collect();
            return;
        }
        if tab == 3 && action == 1 {
            self.spatial.editing = 2;
            self.spatial.length = 0;
            self.spatial.caret = 0;
            self.spatial.notice = b"Text clipping (192 characters). Enter saves; Esc cancels.";
            return;
        }
        let Some(item) = self.spatial.state.items[index] else {
            self.spatial.notice = b"Select a collected reference first.";
            return;
        };
        let old = self.spatial.state;
        match action {
            1 if tab == 2 => {
                self.spatial.pending_drop = DropRequest::new(
                    &self.spatial.state,
                    index,
                    DropTarget::Collection((item.collection + 1) % 4),
                );
            }
            1 if tab == 4 => {
                if let Some(source) = self.spatial.link.take() {
                    if self
                        .spatial
                        .state
                        .connect(self.current_user.0, source, index)
                        .is_ok()
                    {
                        self.spatial_commit(old);
                    } else {
                        self.spatial.notice = b"Select a different reference to connect.";
                    }
                } else {
                    self.spatial.link = Some(index);
                    self.spatial.notice = b"Select another reference, then Link / unlink.";
                }
            }
            2 => {
                let _ = self.spatial.state.remove(self.current_user.0, index);
                self.spatial.link = None;
                self.spatial_commit(old);
            }
            3 if tab == 3 => {
                if item.text.get().is_empty() {
                    self.spatial.pending_drop =
                        DropRequest::new(&self.spatial.state, index, DropTarget::Editor);
                    return;
                }
                if !self.editor_document.replace_selection(item.text.get()) {
                    self.spatial.notice =
                        b"The editor has no space for this clipping. Nothing was replaced.";
                    return;
                }
                self.spatial_close();
                self.open_text_editor();
                self.redraw();
            }
            3 => {
                if item.object == [0; 16] {
                    self.spatial.notice = b"Text clippings can be inserted from Matter Shelf.";
                    return;
                }
                if crate::storage::namespace_resolve(item.path.get())
                    .ok()
                    .map(|id| id.0)
                    != Some(item.object)
                {
                    self.spatial.notice =
                        b"Source moved or unavailable. Recollect it from File Navigator.";
                    return;
                }
                let metadata = crate::storage::object_inspect_path(item.path.get());
                if let Ok((m, _)) = metadata {
                    self.spatial_close();
                    if m.kind == crate::storage::object::ObjectType::NamespaceNode {
                        self.open_file_navigator_window(item.path.get());
                    } else {
                        self.open_text_editor_path(item.path.get());
                    }
                    self.redraw();
                } else {
                    self.spatial.notice =
                        b"Source unavailable; the reference grants no additional access.";
                }
            }
            _ => {}
        }
    }
    // ------------------------=
    // FUNC: spatial_scroll
    // DESC: Keeps wheel input inside the modal surface instead of scrolling hidden applications.
    // ------------------=
    pub(super) fn spatial_scroll(&mut self, vertical: i8) -> bool {
        if vertical == 0 {
            return false;
        }
        if self.spatial.tab == 0 {
            self.spatial.zoom_target =
                (self.spatial.zoom_target + i32::from(vertical) * 48).clamp(0, 255);
            self.spatial.zoom.retarget(
                self.spatial.zoom_target,
                now(),
                240,
                self.spatial.state.reduced_motion
                    || crate::ui::performance::monotonic_ns().is_none(),
            );
            self.spatial_present();
            return true;
        }
        self.spatial_input(if vertical > 0 {
            ConsoleKey::Up
        } else {
            ConsoleKey::Down
        });
        true
    }
}
