//! Per-window local assistance and explicit, fenced app action execution.
use super::*;
use crate::ui::app_assistant::{self as assistant, Action, Target};
impl ConsoleRuntime {
    // ------------------------=
    // FUNC: assistant_owner_alive
    // DESC: Keeps background app generation attached to its original live window rather than the current foreground app.
    // ------------------=
    pub(super) fn assistant_owner_alive(&self,id:usize)->bool {
        if !matches!(self.mode,ConsoleMode::Desktop|ConsoleMode::Settings|ConsoleMode::SystemMenu|ConsoleMode::AppLauncher) {return false;}
        use crate::ui::app_launcher::minimized_shelf as shelf;
        let mask=shelf::current().mask;
        match id {
            0=>self.browser_window.visible || mask&(1<<shelf::BROWSER)!=0,
            1=>self.command_window.visible || mask&(1<<shelf::COMMAND)!=0,
            2=>self.editor_window.visible || mask&(1<<shelf::EDITOR)!=0,
            3=>self.task_manager_window.visible || mask&(1<<shelf::TASKS)!=0,
            4=>self.settings_open || mask&(1<<shelf::SETTINGS)!=0,
            11=>self.mode==ConsoleMode::AppLauncher,
            12=>self.spatial.open,
            5..=10=>crate::runtime::with_runtime(|r|r.file_navigators.window(id-5).is_some()).unwrap_or(false),
            _=>false,
        }
    }
    // ------------------------=
    // FUNC: scroll_window_assistant
    // DESC: Consumes panel wheel events before the underlying application can scroll.
    // ------------------=
    pub(super) fn scroll_window_assistant(&mut self, vertical:i8)->bool {
        let Some((id,window))=self.assistant_owner() else{return false;};
        if !assistant::expanded(id) {return false;}
        let scale=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height).scale();
        let g=assistant::geometry_in_viewport(window,self.system.framebuffer_width,scale,true);
        let p=crate::ui::geometry::Point{x:self.system.framebuffer_width as i32*self.pointer_x/1000,y:self.system.framebuffer_height as i32*self.pointer_y/1000};
        if !g.panel.contains(p) {return false;}
        let mut panel=assistant::read(id);
        let delta=crate::ui::input_preferences::current().wheel(vertical)*24*scale as i32;
        panel.scroll=(panel.scroll as i64+delta as i64).clamp(0,panel.scroll_max as i64) as u32;
        assistant::write(id,panel);true
    }
    // ------------------------=
    // FUNC: dispatch_context_request
    // DESC: Uses the attached-panel action resolver for authenticated desktop text and voice commands.
    // ------------------=
    pub(super) fn dispatch_context_request(&mut self, input:&[u8])->bool {
        if !self.ai_chat_allowed() {return false;}
        let Some((id,_))=self.assistant_owner() else{return false;};
        let mut panel=assistant::read(id);
        if input.len()>panel.input.len() {return false;}
        panel.input[..input.len()].copy_from_slice(input);panel.length=input.len();panel.caret=input.len();
        if !panel.propose_contextual(id) && !panel.propose(id==2,self.assistant_revision()) {return false;}
        panel.request=panel.input;panel.request_len=panel.length;panel.length=0;panel.caret=0;
        assistant::write(id,panel);
        self.apply_window_assistant(id)
    }
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
        if !matches!(self.mode, ConsoleMode::Desktop | ConsoleMode::Settings | ConsoleMode::AppLauncher)
            || self.editor_dialog != EditorDialog::None
        {
            return None;
        }
        let l = SystemLayout::new(
            self.system.framebuffer_width,
            self.system.framebuffer_height,
        );
        if self.spatial.open {
            return Some((12,assistant::spatial_window(self.system.framebuffer_width,self.system.framebuffer_height)));
        }
        if self.mode==ConsoleMode::AppLauncher {return Some((11,l.app_launcher_geometry().panel));}
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
                    panel.input.fill(0);panel.length=0;panel.caret=0;
                }
            }
            ConsoleKey::Shortcut(b'v') => {
                let mut bytes=[0;crate::ui::clipboard::MAX_CLIPBOARD_BYTES];
                if let Ok(n)=self.read_workplace_text(&mut bytes) {
                    let _=infinity_enterprise_core::paste_line(&mut panel.input,&mut panel.length,&mut panel.caret,&bytes[..n]);
                }
            }
            ConsoleKey::Character(c) if (32..=126).contains(&c) => {
                crate::ui::text_input::insert_ascii(&mut panel.input,&mut panel.length,&mut panel.caret,c);
            }
            ConsoleKey::Backspace => {crate::ui::text_input::backspace(&mut panel.input,&mut panel.length,&mut panel.caret);},
            ConsoleKey::Delete => {crate::ui::text_input::delete(&mut panel.input,&mut panel.length,&mut panel.caret);},
            ConsoleKey::Left => panel.caret=panel.caret.saturating_sub(1),
            ConsoleKey::Right => panel.caret=(panel.caret+1).min(panel.length),
            ConsoleKey::Home => panel.caret=0,
            ConsoleKey::End => panel.caret=panel.length,
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
        let expanded = assistant::expanded(id);
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
            expanded,
        );
        let hovered = geometry.toggle.contains(p);
        let hover_owner_changed = assistant::set_hovered(if hovered { Some(id) } else { None });
        let target = assistant::hit(geometry, expanded, p);
        if expanded && assistant::dragging(id) {
            let mut panel=assistant::read(id);
            {
                if !self.pointer_pressed {panel.scroll_drag=false;} else {
                    let thumb=assistant::scroll_thumb(geometry,panel.scroll,panel.scroll_max);
                    let travel=geometry.scrollbar.height.saturating_sub(thumb.height).max(1);
                    panel.scroll=((p.y-geometry.scrollbar.y-panel.scroll_grab as i32).max(0) as u64*panel.scroll_max as u64/travel as u64).min(panel.scroll_max as u64) as u32;
                }
                assistant::write(id,panel);self.redraw();return true;
            }
        }
        if !clicked {
            if hover_owner_changed { self.redraw(); }
            return expanded && target.is_some();
        }
        let mut panel = assistant::read(id);
        let Some(target) = target else {
            if clicked {
                panel.focused = false;
            }
            assistant::write(id, panel);
            if hover_owner_changed {
                self.redraw();
            }
            return false;
        };
        self.ai_chat_focus = 0;
        match target {
            Target::Close => {panel.expanded=false;panel.focused=false;panel.scroll_drag=false;},
            Target::Scrollbar => {
                panel.scroll_drag=panel.scroll_max>0;
                let thumb=assistant::scroll_thumb(geometry,panel.scroll,panel.scroll_max);
                panel.scroll_grab=if thumb.contains(p) {(p.y-thumb.y).max(0) as u32}else{thumb.height/2};
                let travel=geometry.scrollbar.height.saturating_sub(thumb.height).max(1);
                panel.scroll=((p.y-geometry.scrollbar.y-panel.scroll_grab as i32).max(0) as u64*panel.scroll_max as u64/travel as u64).min(panel.scroll_max as u64) as u32;
            },
            Target::Toggle => {
                panel.expanded = !panel.expanded;
                panel.focused = panel.expanded;
            }
            Target::Composer => {panel.focused = true;panel.caret=panel.length;},
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
        if let Some(command)=crate::runtime::ai::control::Command::explicit(&panel.input[..panel.length]) {
            if !matches!(command,crate::runtime::ai::control::Command::Context(_)) {
                let result=self.dispatch_ai_control(command);
                panel.length=0;panel.caret=0;
                panel.reply(if let Some(success)=result {command.result(success)}else{b"The app needs your decision in its save dialog."});
                assistant::write(id,panel);return;
            }
        }
        if panel.propose_contextual(id) {
            panel.length = 0;
            panel.caret = 0;
            assistant::write(id, panel);
            self.apply_window_assistant(id);
            return;
        }
        let direct = panel.propose(id == 2, self.assistant_revision());
        if !direct {
            // Unsupported intents still retain only the prompt supplied to this app.
            let text = &panel.input[..panel.length];
            if text.eq_ignore_ascii_case(b"help") || text.eq_ignore_ascii_case(b"what can you do") {
                panel.reply(assistant::help(id));
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
                } else { assistant::help(id) };
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
        panel.caret = 0;
        assistant::write(id, panel);
        if direct && id == 2 && !matches!(panel.pending, Action::Maximize | Action::Minimize | Action::Restore | Action::Refresh) {
            self.apply_window_assistant(id);
        }
    }
    // ------------------------=
    // FUNC: apply_window_assistant
    // DESC: Executes only the focused window's consumed, explicitly approved capability through normal application workflows.
    // ------------------=
    fn apply_window_assistant(&mut self, id: usize) -> bool {
        if !self.ai_chat_allowed() {
            let mut panel = assistant::read(id);
            panel.pending = Action::None;
            panel.reply(b"AI is disabled in Settings.");
            assistant::write(id, panel);
            return false;
        }
        if self.assistant_owner().map(|v| v.0) != Some(id) {
            return false;
        }
        let mut panel = assistant::read(id);
        let action = panel.take_action(self.assistant_revision());
        if !assistant::supports(id,action) {
            assistant::write(id, panel);
            return false;
        }
        let mut success = true;
        match action {
            Action::NavigateWeb if id==0 => {
                #[cfg(feature="native-browser")]
                browser_controller::request_access(self,&panel.argument[..panel.argument_len]);
                #[cfg(not(feature="native-browser"))]
                {success=false;}
            },
            Action::Back | Action::Forward if (5..11).contains(&id) => {
                success=crate::runtime::with_runtime(|r|r.file_navigator.as_mut().map(|n|
                    if action==Action::Back {n.back().is_ok()}else{n.forward().is_ok()})).flatten().unwrap_or(false);
                self.checkpoint_active_file_navigator();
            },
            Action::FilterApps if id==11 => {
                if panel.argument_len>self.command.len(){success=false;}else{
                    self.reset_input();self.command[..panel.argument_len].copy_from_slice(&panel.argument[..panel.argument_len]);
                    self.command_length=panel.argument_len;self.command_cursor=panel.argument_len;self.system_focus=0;
                    crate::ui::app_launcher::launcher_scroll_by(i32::MIN,0);
                    panel.focused=false;
                }
            },
            Action::NextItem | Action::PreviousItem | Action::ActivateItem | Action::AddIdea | Action::NewCategory | Action::ZoomIn | Action::ZoomOut if id==12 => {
                success=self.spatial_assistant_action(action);panel.focused=false;
            },
            Action::Close if id==11 => {self.close_app_launcher();return true;},
            Action::Close if id==12 => {self.spatial_close();return true;},
            Action::DraftCommand if id==1 => {
                if panel.argument_len>self.command.len() {success=false;}else{
                    self.reset_input();self.command[..panel.argument_len].copy_from_slice(&panel.argument[..panel.argument_len]);
                    self.command_length=panel.argument_len;self.command_cursor=panel.argument_len;
                    panel.focused=false;panel.reply(b"Command prepared, not executed. Review it in Command Window and press Enter to run.");
                    assistant::write(id,panel);return true;
                }
            },
            Action::NextTask | Action::PreviousTask if id==3 => {
                let before=self.task_manager_selected;
                self.input_task_manager(if action==Action::NextTask {ConsoleKey::Down}else{ConsoleKey::Up});
                success=self.task_manager_selected!=before;
            },
            Action::CycleTheme | Action::CycleIcons | Action::CycleModel if id==4 => {
                self.open_settings(if action==Action::CycleModel {3}else{1});
                self.activate_settings_content_row(match action {Action::CycleTheme=>0,Action::CycleIcons=>1,_=>2});
            },
            Action::Close => {
                use crate::runtime::ai::control::App;
                let app=match id {0=>App::Browser,1=>App::Terminal,2=>App::TextEditor,3=>App::TaskManager,4=>App::Settings,_=>App::FileNavigator};
                return self.dispatch_ai_window(app,true)==Some(true);
            },
            Action::Back | Action::Forward | Action::NewTab | Action::CloseTab | Action::ZoomIn | Action::ZoomOut if id==0 => {
                #[cfg(feature="native-browser")]
                browser_controller::key(self,ConsoleKey::Shortcut(match action {Action::Back=>b'[',Action::Forward=>b']',Action::NewTab=>b't',Action::CloseTab=>b'w',Action::ZoomIn=>b'+',_=>b'-'}));
                #[cfg(not(feature="native-browser"))]
                {success=false;}
            },
            Action::ListView | Action::GridView if (5..11).contains(&id) => {
                success=crate::runtime::with_runtime(|r|r.file_navigator.as_mut().map(|n| {
                    n.view_mode=if action==Action::ListView {crate::runtime::object_navigation::ViewMode::List}else{crate::runtime::object_navigation::ViewMode::Grid};
                })).flatten().is_some();
                self.checkpoint_active_file_navigator();
            },
            Action::SearchWeb if id == 0 => {
                #[cfg(feature="native-browser")]
                {
                    browser_controller::search_text(self, &panel.argument[..panel.argument_len]);
                    panel.reply(b"Google search requested in this browser. Network policy still applies.");
                }
                #[cfg(not(feature="native-browser"))]
                panel.reply(b"The native browser is unavailable in this build.");
                assistant::write(id, panel);
                return cfg!(feature="native-browser");
            }
            Action::FindFile if (5..11).contains(&id) => {
                success = self.assistant_find_file(&panel.argument[..panel.argument_len]);
                panel.reply(if success { b"Found and selected the file in this Navigator." }
                    else { b"No matching filename found in this folder tree. Open a broader folder and try again." });
                assistant::write(id, panel);
                return success;
            }
            Action::Maximize | Action::Restore if id<11 => {
                let value = action == Action::Maximize;
                let current = if id == 0 {
                    self.browser_window_state().maximized
                } else if id == 4 {
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
            Action::Minimize if id<11 => {
                panel.focused = false;
                if id == 4 {
                    crate::ui::app_launcher::minimized_shelf::set(
                        crate::ui::app_launcher::minimized_shelf::SETTINGS, true);
                    self.settings_open = false;
                    self.mode = ConsoleMode::Desktop;
                } else if (5..11).contains(&id) {
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
                return success;
            }
            Action::Refresh => {
                success=matches!(id,0|3|5..=10);
                #[cfg(feature="native-browser")]
                if id==0 {browser_controller::key(self,ConsoleKey::Shortcut(b'r'));}
                if id == 3 {
                    self.refresh_task_manager_output();
                } else if (5..11).contains(&id) {
                    self.refresh_desktop_items();
                }
            }
            Action::Navigate if (5..11).contains(&id) => {
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
        success
    }

    // ------------------------=
    // FUNC: assistant_find_file
    // DESC: Finds an exact filename within the active namespace and reveals its row without opening or executing the file.
    // ------------------=
    fn assistant_find_file(&mut self, name: &[u8]) -> bool {
        let Some(state) = crate::runtime::with_runtime(|r| r.file_navigator).flatten() else { return false; };
        let root = state.active_namespace_ref.as_bytes();
        for index in 0..256 {
            let Ok(Some(entry)) = crate::storage::namespace_list_nth(root, index) else { break; };
            let path = &entry.path[..entry.path_len as usize];
            if !crate::runtime::object_navigation::namespace_basename(path).eq_ignore_ascii_case(name) { continue; }
            let Ok(parent) = crate::runtime::object_navigation::parent_path(path) else { continue; };
            // Listing prefixes must never permit a sibling namespace to escape the search root.
            if parent.as_bytes() != root && !(parent.as_bytes().starts_with(root)
                && (root == b"/" || parent.as_bytes().get(root.len()) == Some(&b'/'))) { continue; }
            let moved = crate::runtime::with_runtime(|r| r.file_navigator.as_mut()
                .is_some_and(|n| n.navigate(parent.as_bytes()).is_ok())).unwrap_or(false);
            if !moved { return false; }
            if let Ok(Some(index)) = crate::storage::namespace_child_sorted_index(parent.as_bytes(), path, state.sort_descending) {
                let row = index + crate::runtime::object_navigation::FILE_NAVIGATOR_NAVIGATION_ENTRY_COUNT;
                self.home_selected_item = Some(row);
                let scale = SystemLayout::new(self.system.framebuffer_width, self.system.framebuffer_height).scale();
                crate::runtime::with_runtime(|r| { if let Some(n) = r.file_navigator.as_mut() {
                    n.selected_index = row as u16;
                    n.view_mode = crate::runtime::object_navigation::ViewMode::List;
                    n.scroll_offset = row * 34 * scale;
                } });
                self.checkpoint_active_file_navigator();
                return true;
            }
        }
        false
    }
}
