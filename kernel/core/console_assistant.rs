//! Per-window local assistance and explicit, fenced app action execution.
use super::*;
use crate::ui::app_assistant::{self as assistant, Action, Target};
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: reset_app_assistant_session
    // DESC: Clears transient prompts, pending app actions and clipboard at an authenticated session boundary.
    // ------------------=
    pub(super) fn reset_app_assistant_session(&mut self) {
        if self.assistant_session != self.current_session {
            crate::runtime::ai::with_ai_runtime(|ai| ai.cancel_app_turn());
            assistant::reset();
            self.command_history=infinity_enterprise_core::History::new();
            crate::ui::clipboard::with_shared(|c|c.session(self.current_session.0,false));
            self.assistant_session = self.current_session;
        }
    }
    // ------------------------=
    // FUNC: assistant_owner
    // DESC: Resolves the currently focused native app instance, excluding login, lock, installer, and modal editors.
    // ------------------=
    pub(super) fn assistant_owner(&self) -> Option<(usize, crate::ui::geometry::Rect)> {
        if !matches!(self.mode, ConsoleMode::Desktop | ConsoleMode::Settings)
            || self.editor_dialog != EditorDialog::None
        {
            return None;
        }
        let l = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        if self.mode == ConsoleMode::Settings {
            return Some((4, l.settings_window_geometry(self.settings_window).window));
        }
        if self.desktop_app==DesktopAppKind::Browser {
            let s=self.browser_window_state();
            return Some((0,l.desktop_app_window_geometry(s.x,s.y,s.width,s.height,s.maximized).window));
        }
        let id = match self.desktop_app {
            DesktopAppKind::TextEditor => 2,
            DesktopAppKind::CommandWindow => 1,
            DesktopAppKind::TaskManager => 3,
            DesktopAppKind::Browser => return None,
            DesktopAppKind::None => 0,
        };
        if id == 0 {
            if !self.home_window_visible {
                return None;
            }
            let index = crate::runtime::with_runtime(|r| r.file_navigators.active_index())
                .flatten()
                .unwrap_or(0);
            let (x, y, w, h) = l.home_window_geometry_sized(
                self.home_window_x,
                self.home_window_y,
                self.home_window_width,
                self.home_window_height,
                self.home_window_maximized,
            );
            Some((
                5 + index,
                crate::ui::geometry::Rect {
                    x: x as i32,
                    y: y as i32,
                    width: w as u32,
                    height: h as u32,
                },
            ))
        } else {
            Some((
                id,
                l.desktop_app_window_geometry(
                    self.app_window_x,
                    self.app_window_y,
                    self.app_window_width,
                    self.app_window_height,
                    self.app_window_maximized,
                )
                .window,
            ))
        }
    }
    // ------------------------=
    // FUNC: assistant_revision
    // DESC: Binds document proposals to the current object identity, content and selection.
    // ------------------=
    fn assistant_revision(&self) -> u64 {
        assistant::fingerprint(
            self.editor_document.bytes(),
            self.editor_document.cursor(),
            self.editor_document.selection(),
        ) ^ assistant::fingerprint(
            &self.editor_document_path[..self.editor_document_path_length],
            0,
            None,
        )
    }
    // ------------------------=
    // FUNC: input_window_assistant
    // DESC: Consumes text only when this window's expanded composer owns focus.
    // ------------------=
    pub(super) fn input_window_assistant(&mut self, key: ConsoleKey) -> bool {
        let Some((id, _)) = self.assistant_owner() else {
            return false;
        };
        let mut panel = assistant::read(id);
        if matches!(key, ConsoleKey::Shortcut(b'i')) {
            panel.expanded = !panel.expanded;
            panel.focused = panel.expanded;
            assistant::write(id, panel);
            return true;
        }
        if !panel.expanded || !panel.focused {
            return false;
        }
        match key {
            ConsoleKey::Shortcut(b'c'|b'x') => {
                if self.copy_workplace_text(&panel.input[..panel.length]) && matches!(key,ConsoleKey::Shortcut(b'x')) {
                    panel.input.fill(0);panel.length=0;
                }
            }
            ConsoleKey::Shortcut(b'v') => {
                let mut bytes=[0;crate::ui::clipboard::MAX_CLIPBOARD_BYTES];
                if let Ok(n)=self.read_workplace_text(&mut bytes) {
                    let mut caret=panel.length;
                    let _=infinity_enterprise_core::paste_line(&mut panel.input,&mut panel.length,&mut caret,&bytes[..n]);
                }
            }
            ConsoleKey::Character(c) if (32..=126).contains(&c) => {
                if panel.length < panel.input.len() {
                    panel.input[panel.length] = c;
                    panel.length += 1;
                }
            }
            ConsoleKey::Backspace => panel.length = panel.length.saturating_sub(1),
            ConsoleKey::Escape => panel.focused = false,
            ConsoleKey::Enter => {
                assistant::write(id, panel);
                self.submit_window_assistant(id);
                return true;
            }
            _ => {}
        }
        assistant::write(id, panel);
        true
    }
    // ------------------------=
    // FUNC: pointer_window_assistant
    // DESC: Gives the shared panel hit priority and clears its composer focus when returning to the app.
    // ------------------=
    pub(super) fn pointer_window_assistant(&mut self, clicked: bool) -> bool {
        let Some((id, window)) = self.assistant_owner() else {
            if assistant::set_hovered(None) {
                self.redraw();
            }
            return false;
        };
        let mut panel = assistant::read(id);
        let scale = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        )
        .scale();
        let p = crate::ui::geometry::Point {
            x: self.system.framebuffer_width as i32 * self.pointer_x / 1000,
            y: self.system.framebuffer_height as i32 * self.pointer_y / 1000,
        };
        let geometry = assistant::geometry_in_viewport(
            window,
            self.system.framebuffer_width,
            scale,
            panel.expanded,
        );
        let hovered = geometry.toggle.contains(p);
        let hover_changed = panel.hovered != hovered;
        let hover_owner_changed = assistant::set_hovered(if hovered { Some(id) } else { None });
        panel.hovered = hovered;
        let Some(target) = assistant::hit(geometry, panel.expanded, p) else {
            if clicked {
                panel.focused = false;
            }
            assistant::write(id, panel);
            if hover_changed || hover_owner_changed {
                self.redraw();
            }
            return false;
        };
        if !clicked {
            assistant::write(id, panel);
            if hover_changed || hover_owner_changed {
                self.redraw();
            }
            return panel.expanded;
        }
        self.ai_chat_focus = 0;
        match target {
            Target::Toggle => {
                panel.expanded = !panel.expanded;
                panel.focused = panel.expanded;
            }
            Target::Composer => panel.focused = true,
            Target::Dismiss => {
                panel.pending = Action::None;
                panel.reply(b"Proposal dismissed. The app is unchanged.");
            }
            Target::Send => {
                assistant::write(id, panel);
                self.submit_window_assistant(id);
                return true;
            }
            Target::Apply => {
                assistant::write(id, panel);
                self.apply_window_assistant(id);
                return true;
            }
            Target::Body => panel.focused = false,
        }
        assistant::write(id, panel);
        true
    }
    // ------------------------=
    // FUNC: submit_window_assistant
    // DESC: Offers supported contextual actions or an honestly scoped local response; never sends app data to a remote provider.
    // ------------------=
    fn submit_window_assistant(&mut self, id: usize) {
        let mut panel = assistant::read(id);
        if panel.length == 0 {
            return;
        }
        if !self.ai_chat_allowed() {
            panel.pending = Action::None;
            panel.reply(b"AI is disabled in Settings.");
            assistant::write(id, panel);
            return;
        }
        crate::runtime::ai::with_ai_runtime(|ai| {
            if ai.app_turn_owner() == Some(id) { ai.cancel_app_turn(); }
        });
        panel.request = panel.input;
        panel.request_len = panel.length;
        let direct = panel.propose(id == 2, self.assistant_revision());
        if !direct {
            // Unsupported intents still retain only the prompt supplied to this app.
            let text = &panel.input[..panel.length];
            if text.eq_ignore_ascii_case(b"help") || text.eq_ignore_ascii_case(b"what can you do") {
                panel.reply(if id==2 {b"Ask for text or code, then Apply the generated edit. Commands: insert TEXT, clear text, undo, redo, select all, find TEXT, save file. Save uses the normal filename dialog when needed."}
                    else if id==0 {b"Browser assistance: refresh, maximize, restore, or minimize this window with Apply. Other prompts use local dialogue; website contents are not automatically shared."}
                    else if id >= 5 {b"This File Navigator supports open folder /absolute/path, maximize, restore, minimize, and refresh. Actions target this window only and require Apply."}
                    else {b"Local window actions: maximize, restore, minimize, refresh. Every action requires Apply. No files or settings are changed by chat."});
            } else if id == 2 && text.eq_ignore_ascii_case(b"describe document") {
                let language = self.editor_tools.language.name();
                let mut response = [0; 512];
                let mut n = 0;
                for part in [b"Current syntax: ".as_slice(),language,if self.editor_document.is_saved(){b". Saved document. ".as_slice()}else{b". Unsaved changes. ".as_slice()},b"Local text and code generation is available for this document."]{response[n..n+part.len()].copy_from_slice(part);n+=part.len();}
                panel.reply(&response[..n]);
            } else {
                let mut prompt = [0; 6144];
                let context = if id == 2 {
                    if let Some((a,b)) = self.editor_document.selection() { &self.editor_document.bytes()[a..b] }
                    else { self.editor_document.bytes() }
                } else { b"" };
                panel.document_revision = self.assistant_revision();
                if let Some(n) = panel.generation_prompt(id == 2, context, &mut prompt) {
                    match crate::runtime::ai::with_ai_runtime(|ai| ai.submit_app_turn(id, &prompt[..n])) {
                        Ok(()) => {
                            panel.generation = assistant::GenerationStatus::Running;
                            panel.reply(b"Generating locally...");
                        }
                        Err(message) => {
                            panel.generation = assistant::GenerationStatus::Failed;
                            panel.reply(message);
                        }
                    }
                } else {
                    panel.reply(b"The document is too large for this request. Select a smaller passage and retry.");
                }
            }
        }
        panel.length = 0;
        assistant::write(id, panel);
        if direct && id == 2 && !matches!(panel.pending, Action::Maximize | Action::Minimize | Action::Restore | Action::Refresh) {
            self.apply_window_assistant(id);
        }
    }
    // ------------------------=
    // FUNC: apply_window_assistant
    // DESC: Executes only the focused window's consumed, explicitly approved capability through normal application workflows.
    // ------------------=
    fn apply_window_assistant(&mut self, id: usize) {
        if !self.ai_chat_allowed() {
            let mut panel = assistant::read(id);
            panel.pending = Action::None;
            panel.reply(b"AI is disabled in Settings.");
            assistant::write(id, panel);
            return;
        }
        if self.assistant_owner().map(|v| v.0) != Some(id) {
            return;
        }
        let mut panel = assistant::read(id);
        let action = panel.take_action(self.assistant_revision());
        if action == Action::None {
            assistant::write(id, panel);
            return;
        }
        let mut success = true;
        match action {
            Action::Maximize | Action::Restore => {
                let value = action == Action::Maximize;
                let current = if id == 4 {
                    self.settings_window.maximized
                } else if id >= 5 {
                    self.home_window_maximized
                } else {
                    self.app_window_maximized
                };
                if current != value {
                    self.toggle_window_maximized(if id==0 {5}else if id >= 5 { 0 } else { id });
                }
                let _ = self.checkpoint_desktop_layout();
            }
            Action::Minimize => {
                panel.focused = false;
                if id == 4 {
                    crate::ui::app_launcher::minimized_shelf::set(
                        crate::ui::app_launcher::minimized_shelf::SETTINGS, true);
                    self.settings_open = false;
                    self.mode = ConsoleMode::Desktop;
                } else if id >= 5 {
                    self.checkpoint_active_file_navigator();
                    let _ = crate::runtime::with_runtime(|runtime| {
                        runtime.file_navigators.minimize_active()
                    });
                    self.home_window_visible = false;
                } else {
                    self.minimize_desktop_app();
                }
                let _ = self.checkpoint_desktop_layout();
            }
            Action::Find if id == 2 => {
                success = self
                    .editor_document
                    .find(&panel.argument[..panel.argument_len], false)
            }
            Action::Insert | Action::Replace | Action::Clear if id == 2 => {
                success = panel.edit_document(action, &mut self.editor_document);
            }
            Action::Undo if id == 2 => success = self.editor_document.undo(),
            Action::Redo if id == 2 => success = self.editor_document.redo(),
            Action::SelectAll if id == 2 => self
                .editor_document
                .select(0, self.editor_document.bytes().len()),
            Action::Save if id == 2 => {
                success = self.save_editor_document();
                panel.reply(if success { b"File saved." }
                    else if self.editor_dialog == EditorDialog::SaveAs { b"Choose a filename and location in Save As to finish saving." }
                    else { b"Save failed. Your document remains in memory." });
                assistant::write(id, panel);
                return;
            }
            Action::Refresh => {
                #[cfg(feature="native-browser")]
                if id==0 {browser_controller::key(self,ConsoleKey::Shortcut(b'r'));}
                if id == 3 {
                    self.refresh_task_manager_output();
                } else if id >= 5 {
                    self.refresh_desktop_items();
                }
            }
            Action::Navigate if id >= 5 => {
                let path = &panel.argument[..panel.argument_len];
                success = crate::storage::object_inspect_path(path).ok()
                    .map(|(metadata, _)| metadata.kind == crate::storage::object::ObjectType::NamespaceNode)
                    .unwrap_or(false);
                if success {
                    success = crate::runtime::with_runtime(|runtime| {
                        runtime.file_navigator.as_mut().map(|navigator| navigator.navigate(path).is_ok())
                    }).flatten().unwrap_or(false);
                    self.home_selected_item = None;
                    self.checkpoint_active_file_navigator();
                }
            }
            _ => success = false,
        }
        if id == 2 {
            self.reveal_editor_cursor();
        }
        panel.reply(if success{b"Action applied through the app. Document edits remain undoable; Save may require a filename."}else{b"The app could not apply this action (no match, no history, save error, or document capacity)."});
        assistant::write(id, panel);
    }
}
