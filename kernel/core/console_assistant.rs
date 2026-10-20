//! Per-window local assistance and explicit, fenced app action execution.
use super::*;
use crate::ui::app_assistant::{self as assistant, Action, Target};
#[no_mangle]
pub static mut INFINITY_APP_GENERATION_SNAPSHOT:[u64;14]=[0x494e4647454e3031,1,0,0,0,0,0,0,0,0,0,0,0,0];
#[no_mangle]
pub static mut INFINITY_SOURCE_WORKFLOW_SNAPSHOT:[u64;24]=[0x494e465352433031,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
#[no_mangle]
pub static mut INFINITY_SOURCE_WORKFLOW_PATH:[u8;95]=[0;95];
// ------------------------=
// FUNC: publish_source_workflow_snapshot
// DESC: Exposes owner-scoped goal state and actual source/version receipts for read-only installed validation without admitting actions or exposing source contents.
// ------------------=
fn publish_source_workflow_snapshot(console:&ConsoleRuntime) {
    use crate::runtime::ai::intelligence::{PlanState,TaskState};
    let panel=assistant::read(2);
    let mut values=[0u64;20];
    crate::runtime::ai::with_ai_runtime(|ai| {
        if let Some(execution)=ai.software_execution(console.current_user.0) {
            values[0]=execution.goal;
            if let Some(plan)=ai.intelligence_plan(execution.goal,console.current_user.0) {
                values[1]=match plan.state {PlanState::Proposed=>1,PlanState::AwaitingApproval=>2,PlanState::Executing=>3,
                    PlanState::Interrupted=>4,PlanState::Completed=>5,PlanState::Failed=>6,PlanState::Cancelled=>7};
                values[2]=plan.task_count as u64;
                values[3]=plan.tasks[..plan.task_count as usize].iter().filter(|task|task.as_ref().is_some_and(|task|task.state==TaskState::Succeeded)).count() as u64;
            }
            values[4]=u64::from_le_bytes(execution.source[..8].try_into().unwrap());
            values[5]=u64::from_le_bytes(execution.source[8..].try_into().unwrap());
            values[6]=execution.source_revision;
            values[7]=u64::from_le_bytes(execution.artifact[..8].try_into().unwrap());
            values[8]=u64::from_le_bytes(execution.artifact[8..].try_into().unwrap());
            values[9]=execution.context;
            values[19]=u64::from(ai.software_execution_id()==Some(execution.goal));
        }
    });
    if let Some((object,version))=panel.source_binding {
        values[10]=u64::from_le_bytes(object[..8].try_into().unwrap());
        values[11]=u64::from_le_bytes(object[8..].try_into().unwrap());values[12]=version as u64;
    }
    let path_length=console.editor_document_path_length.min(95);
    values[13]=panel.effective_language() as u64;
    values[14]=crate::storage::object::crc32(&console.editor_document_path[..path_length]) as u64;
    values[15]=path_length as u64;
    values[16]=assistant::fingerprint(&panel.request[..panel.request_len],0,None);
    values[17]=console.editor_document.bytes().len() as u64;
    values[18]=u64::from(console.editor_document.is_saved());
    unsafe {
        let target=core::ptr::addr_of_mut!(INFINITY_SOURCE_WORKFLOW_SNAPSHOT).cast::<u64>();
        let sequence=target.add(2).read_volatile().wrapping_add(2)&!1;
        target.add(2).write_volatile(sequence|1);
        let path=core::ptr::addr_of_mut!(INFINITY_SOURCE_WORKFLOW_PATH).cast::<u8>();
        for index in 0..95 {path.add(index).write_volatile(if index<path_length {console.editor_document_path[index]}else{0});}
        for (index,value) in values.into_iter().enumerate(){target.add(index+3).write_volatile(value);}
        core::sync::atomic::fence(core::sync::atomic::Ordering::Release);
        target.add(23).write_volatile(sequence);target.add(2).write_volatile(sequence);
    }
}
// ------------------------=
// FUNC: publish_generation_snapshot
// DESC: Publishes seqlocked inference and editor-delivery counts without exposing user source or granting execution authority.
// ------------------=
fn publish_generation_snapshot() {
    let panel=assistant::read(2);
    let progress=crate::runtime::ai::with_ai_runtime(|ai|ai.app_generation_progress());
    unsafe {
        let target=core::ptr::addr_of_mut!(INFINITY_APP_GENERATION_SNAPSHOT).cast::<u64>();
        let sequence=target.add(2).read_volatile().wrapping_add(2)&!1;
        target.add(2).write_volatile(sequence|1);
        let values=[panel.generation as u64,progress.map_or(0,|p|p.prompt_tokens as u64),
            progress.map_or(0,|p|p.processed_tokens as u64),progress.map_or(0,|p|p.generated_tokens as u64),
            progress.map_or(0,|p|p.output_bytes as u64),panel.argument_len as u64,panel.stream.delivered as u64,
            panel.stream.flushes as u64,panel.repair_attempts as u64,crate::runtime::ai::qwen::workers::clock_ns()];
        for (index,value) in values.into_iter().enumerate(){target.add(index+3).write_volatile(value);}
        core::sync::atomic::fence(core::sync::atomic::Ordering::Release);
        target.add(13).write_volatile(sequence);
        target.add(2).write_volatile(sequence);
    }
}
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
        let g=assistant::geometry_for_owner(id,window,self.system.framebuffer_width,scale,true);
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
        if let Some(update)=assistant::editor_compile_intent::update(input) {
            let Some(goal)=crate::runtime::ai::with_ai_runtime(|ai|ai.software_goal_for_request(input)) else{return false;};
            if self.editor_dialog!=EditorDialog::None {return false;}
            let mut panel=assistant::read(2);
            if matches!(panel.generation,assistant::GenerationStatus::Queued|assistant::GenerationStatus::Running) {return false;}
            let (language,object,version)=match self.resolve_assistant_update_source(update.target) {
                Ok(binding)=>binding,
                Err(message)=>{panel.reject_unstarted_generation();panel.reply(message);assistant::write(2,panel);return false;},
            };
            panel.expanded=true;panel.focused=false;self.ai_chat_focus=0;
            let queued=panel.queue_user_update(input,self.assistant_revision(),self.editor_document.bytes(),language,goal);
            if queued {panel.source_binding=Some((object,version));}
            panel.reply(if queued {b"Updating the resolved source document. Existing behavior will be preserved, then the file will be saved and validated."}
                else {b"This source cannot be updated safely in the current editor state. No file was changed."});
            assistant::write(2,panel);
            if queued {compiler_commands::generation_progress(self,compiler_commands::VoiceStep::Planning);
                compiler_commands::generation_progress(self,compiler_commands::VoiceStep::PlanReady);}
            return queued;
        }
        if assistant::editor_compile_intent::repair(input).is_some() {
            if self.editor_dialog!=EditorDialog::None {return false;}
            self.open_text_editor();self.ai_chat_focus=0;
            let mut panel=assistant::read(2);
            let mut diagnostics=[0u8;1024];
            #[cfg(all(feature="native-compiler",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
            let diagnostic_length=compiler_commands::repair_diagnostics(self,&mut diagnostics);
            #[cfg(not(all(feature="native-compiler",target_os="none",any(target_arch="aarch64",target_arch="x86_64"))))]
            let diagnostic_length=0;
            panel.expanded=true;panel.focused=false;
            panel.source_language=if self.editor_document_path[..self.editor_document_path_length].ends_with(b".py") {
                assistant::editor_compile_intent::SourceLanguage::Python
            }else{assistant::editor_compile_intent::SourceLanguage::C};
            let queued=panel.queue_user_repair(input,self.assistant_revision(),self.editor_document.bytes(),&diagnostics[..diagnostic_length]);
            panel.reply(if queued {b"Repairing the current source and checking it with the native compiler. No application will launch automatically."}
                else {b"Open a nonempty source document and finish or cancel any active generation before asking to fix its code."});
            assistant::write(2,panel);return queued;
        }
        if assistant::editor_compile_intent::create_application(input) {
            let Some(goal)=crate::runtime::ai::with_ai_runtime(|ai|ai.software_goal_for_request(input)) else{return false;};
            if input.len()>1024 {return false;}
            self.open_text_editor();
            self.ai_chat_focus=0;
            let mut panel=assistant::read(2);
            panel.expanded=true;panel.focused=false;
            if !self.editor_document.is_saved() && !self.editor_document.bytes().is_empty() {
                panel.reject_unstarted_generation();
                panel.reply(b"Save or discard your current document, then request the new application again. Your edits have not been changed.");
                assistant::write(2,panel);return false;
            }
            if matches!(panel.generation,assistant::GenerationStatus::Queued|assistant::GenerationStatus::Running) {
                panel.reply(b"An application is already being generated.");assistant::write(2,panel);return false;
            }
            // New normally revokes the current editor goal. This New belongs to
            // the exact goal already authorized above, so detach only its old
            // presentation binding while the document transaction is reset.
            let mut previous=assistant::read(2);previous.workflow_goal=None;
            assistant::write(2,previous);
            self.perform_editor_action(PendingDocumentAction::New);
            panel.reset_delivery();
            panel.pending=Action::None;panel.argument_len=0;panel.awaiting_compile_save=false;
            panel.stream=assistant::generation_stream::Stream::new();
            panel.repair_attempts=0;panel.repair_diagnostics_len=0;
            panel.input[..input.len()].copy_from_slice(input);panel.length=input.len();
            panel.request=panel.input;panel.request_len=input.len();
            panel.document_revision=self.assistant_revision();
            panel.workflow_goal=Some(goal);
            panel.source_language=assistant::editor_compile_intent::language(input);
            panel.source_binding=None;
            panel.generation=assistant::GenerationStatus::Queued;
            panel.reply(b"Creating your application. Source will be saved in Documents, validated by its native language toolchain, then launched.");
            assistant::write(2,panel);
            compiler_commands::generation_progress(self,compiler_commands::VoiceStep::Planning);
            compiler_commands::generation_progress(self,compiler_commands::VoiceStep::PlanReady);
            return true;
        }
        let Some((id,_))=self.assistant_owner() else{return false;};
        let mut panel=assistant::read(id);
        if input.len()>panel.input.len() {return false;}
        panel.input[..input.len()].copy_from_slice(input);panel.length=input.len();panel.caret=input.len();
        if !panel.propose_contextual(id) && !panel.propose(id==2,self.assistant_revision()) {
            return id==1 && self.dispatch_native_interaction(input);
        }
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
            crate::runtime::ai::with_ai_runtime(|ai| ai.stop_application_agent());
            crate::runtime::ai::with_ai_runtime(|ai| ai.cancel_app_turn());
            assistant::reset();
            self.command_history=infinity_enterprise_core::History::new();
            crate::ui::clipboard::with_shared(|c|c.session(self.current_session.0,false));
            self.assistant_session = self.current_session;
        }
    }
    // ------------------------=
    // FUNC: dispatch_native_interaction
    // DESC: Routes an explicitly app-directed request through the same authorized model session for typed and voice input.
    // ------------------=
    fn dispatch_native_interaction(&mut self,input:&[u8])->bool {
        if !self.ai_chat_allowed() || crate::runtime::ai::application_host::context().is_none(){return false;}
        if input.eq_ignore_ascii_case(b"stop") || input.eq_ignore_ascii_case(b"stop playing") || input.eq_ignore_ascii_case(b"pause") {
            crate::runtime::ai::with_ai_runtime(|ai|ai.stop_application_agent());
            let mut panel=assistant::read(1);panel.reply(b"AI control paused. You can continue using the app.");assistant::write(1,panel);return true;
        }
        // Questions remain conversation, never implicit permission to manipulate an app.
        let Ok(text)=core::str::from_utf8(input) else{return false;};
        let verb=text.split_ascii_whitespace().next().unwrap_or("");
        if !["play","move","place","select","set","add","remove","delete","insert","draw","undo","redo"].iter().any(|v|verb.eq_ignore_ascii_case(v))
            || text.split_ascii_whitespace().any(|s|["not","never","don't"].iter().any(|v|s.eq_ignore_ascii_case(v))) {return false;}
        crate::runtime::ai::with_ai_runtime(|ai|ai.start_application_agent(self.current_user.0,self.current_session.0,input,assistant::editor_compile_intent::requests_opponent(input))).is_ok()
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
    pub(super) fn assistant_revision(&self) -> u64 {
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
        let geometry = assistant::geometry_for_owner(
            id, window,
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
                if panel.pending==Action::NativeApproval {
                    crate::runtime::ai::with_ai_runtime(|ai|ai.stop_application_agent());
                }
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
        if id==1 {
            crate::runtime::ai::with_ai_runtime(|ai|ai.stop_application_agent());
            panel.pending=Action::None;panel.argument_len=0;
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
        panel.reset_delivery();
        panel.request = panel.input;
        panel.request_len = panel.length;
        panel.repair_attempts=0;panel.repair_diagnostics_len=0;
        panel.stream=assistant::generation_stream::Stream::new();
        panel.workflow_goal=None;
        panel.generation=assistant::GenerationStatus::Idle;
        panel.pending=Action::None;
        panel.awaiting_compile_save=false;
        match crate::runtime::ai::with_ai_runtime(|ai| ai.route_application_request(id, &panel.input[..panel.length])) {
            Ok(crate::runtime::ai::kit::RequestRoute::Control(_)) => {
                panel.length=0;panel.caret=0;
                panel.reply(b"OS action queued for this application.");
                assistant::write(id,panel);return;
            }
            Ok(crate::runtime::ai::kit::RequestRoute::Goal(goal)) => {
                panel.workflow_goal=Some(goal);
                panel.length=0;panel.caret=0;
                panel.reply(b"A bounded goal plan was created. Progress and approvals remain tied to this session.");
                assistant::write(id,panel);return;
            }
            Ok(crate::runtime::ai::kit::RequestRoute::Inference(_)) => {},
            Err(message) => {panel.reply(message);assistant::write(id,panel);return;}
        }
        let mut memory_reply = [0; 8192];
        if let Some(receipt) = crate::runtime::ai::with_ai_runtime(|ai|
            ai.personal_memory_reply(&panel.input[..panel.length], &mut memory_reply)) {
            panel.length = 0;
            panel.caret = 0;
            panel.generation = assistant::GenerationStatus::Complete;
            panel.reply(&memory_reply[..receipt.length]);
            assistant::write(id, panel);
            return;
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
            if id==1 && self.dispatch_native_interaction(&panel.input[..panel.length]) {return;}
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
                    assistant::write(id,panel);
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
    // FUNC: apply_completed_editor_compile
    // DESC: Puts explicitly requested completed C into its original editor and starts compilation once, retaining revision and capability checks.
    // ------------------=
    pub(super) fn apply_completed_editor_compile(&mut self) -> bool {
        publish_generation_snapshot();
        publish_source_workflow_snapshot(self);
        let mut panel=assistant::read(2);
        if panel.generation==assistant::GenerationStatus::Complete && panel.pending==Action::None
            && panel.stream.delivered>=panel.argument_len {return false;}
        if !crate::runtime::ai::with_ai_runtime(|ai|ai.generation_workflow_current(panel.workflow_goal)) {
            if panel.workflow_goal.is_some_and(|goal|panel.cancel_workflow(goal)) {
                panel.reply(b"This generation belongs to an interrupted goal. Partial source is preserved; no save or compile will run.");
                assistant::write(2,panel);
                return true;
            }
            return false;
        }
        if panel.generation==assistant::GenerationStatus::Queued {
            // Admission happens after the intent broker releases its control turn.
            if !self.ai_chat_allowed() || !self.assistant_owner_alive(2) || !panel.expanded
                || panel.document_revision!=self.assistant_revision() {
                panel.generation=assistant::GenerationStatus::Cancelled;
                panel.reply(b"Application creation cancelled because its editor or session changed.");
            } else {
                let mut prompt=[0u8;16*1024];
                let prompt_length=if panel.repair_diagnostics_len>0
                    || assistant::editor_compile_intent::repair(&panel.request[..panel.request_len]).is_some() {
                    panel.compiler_repair_prompt(self.editor_document.bytes(),&mut prompt)
                } else if assistant::editor_compile_intent::update(&panel.request[..panel.request_len]).is_some() {
                    panel.update_prompt(self.editor_document.bytes(),&mut prompt)
                } else {panel.generation_prompt(true,b"",&mut prompt)};
                if let Some(n)=prompt_length {
                    match crate::runtime::ai::with_ai_runtime(|ai|ai.submit_app_source(2,&prompt[..n])) {
                        Ok(())=>{panel.generation=assistant::GenerationStatus::Running;panel.reply(b"Generating source locally...");
                            compiler_commands::generation_progress(self,compiler_commands::VoiceStep::Generating);},
                        Err(message)=>{panel.generation=assistant::GenerationStatus::Failed;panel.reply(message);},
                    }
                } else {panel.generation=assistant::GenerationStatus::Failed;panel.reply(b"Application request is too large.");}
            }
            panel.length=0;panel.caret=0;
            assistant::write(2,panel);
            if matches!(panel.generation,assistant::GenerationStatus::Failed|assistant::GenerationStatus::Cancelled) {
                compiler_commands::generation_progress(self,if panel.generation==assistant::GenerationStatus::Failed {
                    compiler_commands::VoiceStep::GenerationFailed
                }else{compiler_commands::VoiceStep::GenerationCancelled});
            }
            return true;
        }
        if panel.cancel_source_on_revision(self.assistant_revision()) {
            crate::runtime::ai::with_ai_runtime(|ai|{if ai.app_turn_owner()==Some(2){ai.cancel_app_turn();}});
            panel.reply(b"Generation cancelled because you edited the document. Your edits and partial generated source are preserved; nothing will be saved or compiled automatically.");
            assistant::write(2,panel);
            compiler_commands::generation_progress(self,compiler_commands::VoiceStep::GenerationCancelled);
            if let Some(goal)=panel.workflow_goal {
                crate::runtime::ai::with_ai_runtime(|ai|ai.record_software_event(goal,self.current_user.0,
                    crate::runtime::ai::intelligence::software::Event::Failed{code:4}));
            }
            return true;
        }
        let previewed=self.apply_streamed_editor_source();
        let current=assistant::read(2);
        if matches!(current.generation,assistant::GenerationStatus::Failed|assistant::GenerationStatus::Cancelled) {
            compiler_commands::generation_progress(self,if current.generation==assistant::GenerationStatus::Failed {
                compiler_commands::VoiceStep::GenerationFailed
            }else{compiler_commands::VoiceStep::GenerationCancelled});
            crate::runtime::ai::with_ai_runtime(|ai|{if let Some(goal)=current.workflow_goal {
                ai.record_software_event(goal,self.current_user.0,crate::runtime::ai::intelligence::software::Event::Failed{code:1});
            }});
        }
        if current.generation==assistant::GenerationStatus::Complete && current.stream.delivered<current.argument_len
            && assistant::editor_compile_intent::source_request(&current.request[..current.request_len]) {return previewed;}
        if !self.assistant_owner_alive(2) || !assistant::read(2).apply_generated_compile_requested() { return previewed; }
        compiler_commands::generation_progress(self,compiler_commands::VoiceStep::SourceReady);
        self.apply_window_assistant(2) || previewed
    }
    // ------------------------=
    // FUNC: apply_streamed_editor_source
    // DESC: Mirrors authorized in-progress application source into the owning editor while final save and compilation remain completion-gated.
    // ------------------=
    fn apply_streamed_editor_source(&mut self)->bool {
        let mut panel=assistant::read(2);
        if !self.assistant_owner_alive(2)
            || !matches!(panel.generation,assistant::GenerationStatus::Running|assistant::GenerationStatus::Complete)
            || !crate::runtime::ai::with_ai_runtime(|ai|ai.generation_workflow_current(panel.workflow_goal))
            || !assistant::editor_compile_intent::source_request(&panel.request[..panel.request_len])
            || panel.document_revision!=self.assistant_revision() || panel.argument_len==0 {return false;}
        let now=crate::ui::performance::monotonic_ns();
        let Some(range)=panel.stream.ready(&panel.argument[..panel.argument_len],now,
            panel.generation==assistant::GenerationStatus::Complete) else{return false;};
        let follow_tail=self.editor_scroll_geometry().maximum_scroll.saturating_sub(self.editor_scroll_row)<=1;
        if !self.editor_document.append_generation_preview(&panel.argument[range.clone()],range.start==0) {return false;}
        panel.stream.committed(range.end,now);
        if follow_tail {self.editor_scroll_row=self.editor_scroll_geometry().maximum_scroll;}
        panel.document_revision=self.assistant_revision();
        assistant::write(2,panel);
        true
    }
    // ------------------------=
    // FUNC: apply_window_assistant
    // DESC: Executes only the focused window's consumed, explicitly approved capability through normal application workflows.
    // ------------------=
    pub(super) fn apply_window_assistant(&mut self, id: usize) -> bool {
        if !self.ai_chat_allowed() {
            let mut panel = assistant::read(id);
            panel.pending = Action::None;
            panel.reply(b"AI is disabled in Settings.");
            assistant::write(id, panel);
            return false;
        }
        if self.assistant_owner().map(|v| v.0) != Some(id)
            && !(id == 2 && self.assistant_owner_alive(2) && assistant::read(id).apply_generated_compile_requested()) {
            return false;
        }
        let mut panel = assistant::read(id);
        if !crate::runtime::ai::with_ai_runtime(|ai|ai.generation_workflow_current(panel.workflow_goal)) {
            if let Some(goal)=panel.workflow_goal {panel.cancel_workflow(goal);}
            assistant::write(id,panel);
            return false;
        }
        let action = panel.take_action(self.assistant_revision());
        if !assistant::supports(id,action) {
            assistant::write(id, panel);
            return false;
        }
        let mut success = true;
        match action {
            Action::NativeApproval if id==1 => {
                let result=if panel.argument_len==8 {
                    let token=u64::from_le_bytes(panel.argument[..8].try_into().unwrap());
                    crate::runtime::ai::with_ai_runtime(|ai|ai.approve_application_action(self.current_user.0,self.current_session.0,token)).is_ok()
                } else {false};
                panel.reply(if result {b"Approved action dispatched. Waiting for the app's result."}else{b"This approval is stale or no longer authorized. No action dispatched."});
                assistant::write(id,panel);return result;
            },
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
            Action::Compile | Action::ReplaceCompile | Action::SaveCompile if id == 2 => {
                if action == Action::SaveCompile && !self.save_editor_document() {
                    if self.editor_dialog == EditorDialog::SaveAs {
                        panel.defer_compile_until_saved();
                        panel.reply(b"Choose a filename and location in Save As. Compilation will start after the document is saved.");
                    } else {
                        panel.reply(b"Save failed. Nothing was compiled; your code remains in the editor.");
                    }
                    assistant::write(id, panel);
                    return false;
                }
                if action==Action::ReplaceCompile && self.editor_document.bytes()!=&panel.argument[..panel.argument_len]
                    && !panel.edit_document(action,&mut self.editor_document) {
                    panel.reply(b"The generated source could not be applied. Nothing was compiled.");
                    assistant::write(id,panel);return false;
                }
                if action == Action::ReplaceCompile {
                    if panel.stream.delivered == 0 {
                        self.editor_document.set_cursor(0);
                        self.editor_scroll_row = 0;
                    }
                    panel.focused = false;
                }
                #[cfg(all(feature="native-compiler",target_os="none",any(target_arch="aarch64",target_arch="x86_64")))]
                {
                    let launch=action==Action::ReplaceCompile && assistant::editor_compile_intent::requests_launch(&panel.request[..panel.request_len]);
                    if action==Action::ReplaceCompile && panel.effective_language()==assistant::editor_compile_intent::SourceLanguage::Python {
                        #[cfg(feature="native-python")]
                        let result=match compiler_commands::save_generated_source(self) {
                            Ok((path,length))=>python_commands::generated(self,&path[..length],panel.workflow_goal,launch),
                            Err(_)=>Err(b"Python source could not be saved. Nothing was launched; the document remains in the editor.".as_slice()),
                        };
                        #[cfg(not(feature="native-python"))]
                        let result:Result<(),&'static [u8]>=Err(b"Native Python is not included in this build. Your source remains in the editor.");
                        panel.source_binding=assistant::read(id).source_binding;
                        success=result.is_ok();
                        if let Err(message)=result {
                            panel.generation=assistant::GenerationStatus::Failed;panel.reply(message);
                            if let Some(goal)=panel.workflow_goal {crate::runtime::ai::with_ai_runtime(|ai|
                                ai.record_software_event(goal,self.current_user.0,crate::runtime::ai::intelligence::software::Event::Failed{code:5}));}
                        }
                        else {panel.reply(b"Python source saved. The native interpreter is checking its syntax before any authorized execution.");}
                        assistant::write(id,panel);return success;
                    }
                    let compile=if action==Action::ReplaceCompile {compiler_commands::save_generated_and_build(self,launch)} else {compiler_commands::start(self)};
                    panel.source_binding=assistant::read(id).source_binding;
                    success=compile.is_ok();
                    if action==Action::ReplaceCompile && !success {panel.generation=assistant::GenerationStatus::Failed;}
                    panel.reply(match compile {
                        Ok(())=>b"Compiling this source snapshot in isolated user mode. You can keep editing; the result will appear here.",
                        Err(compiler_commands::StartError::Busy)=>b"A native build is already active. Its result will appear here when it finishes.",
                        Err(compiler_commands::StartError::SessionUnavailable)=>b"Compilation requires the current authenticated session. Sign in again, then retry.",
                        Err(compiler_commands::StartError::AiDisabled)=>b"Compilation through Infinity AI is disabled in Settings.",
                        Err(compiler_commands::StartError::ClockUnavailable)=>b"Compilation is waiting for the system clock service. Retry after startup completes.",
                        Err(compiler_commands::StartError::CapabilityUnavailable)=>b"Compilation could not acquire a temporary output capability. Expired capabilities were reclaimed; retry once.",
                        Err(compiler_commands::StartError::CompilerUnavailable)=>b"The native compiler execution platform is unavailable on this boot.",
                        Err(compiler_commands::StartError::SourceInvalid)=>b"This document is empty or exceeds the native compiler source limit.",
                        Err(compiler_commands::StartError::SaveFailed)=>b"Source could not be saved at the selected location. Nothing was compiled or launched; your source remains in the editor.",
                    });
                }
                #[cfg(not(all(feature="native-compiler",target_os="none",any(target_arch="aarch64",target_arch="x86_64"))))]
                {success=false;panel.reply(b"The native compiler is not included in this build.");}
                assistant::write(id,panel);return success;
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
