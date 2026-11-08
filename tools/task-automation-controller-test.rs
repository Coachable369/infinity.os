#![allow(dead_code)]
//! Uses production keyboard/pointer controller with the real typed catalog authority.
#[path="../kernel/runtime/mod.rs"]mod runtime;
#[path="../kernel/ui/mod.rs"]mod ui;
use ui::system_layout::SystemLayout;
#[derive(Clone,Copy)]
pub enum ConsoleKey{Shortcut(u8),SelectMove(i8),Character(u8),Backspace,Enter,Escape,Tab(bool),Up,Down,Left,Right,Home,End,Delete,Help}
mod console {pub use super::ConsoleKey;}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum DesktopAppKind{TaskManager,CommandWindow}
struct SshTerminal{active:bool}
struct System{framebuffer_width:usize,framebuffer_height:usize}
struct ConsoleRuntime{current_user:runtime::identity::StableId,current_session:runtime::identity::StableId,system:System,app_window_x:i32,app_window_y:i32,app_window_width:i32,app_window_height:i32,app_window_maximized:bool,shell_menu:u8,allowed:bool,commits:usize,fail:Option<runtime::automation::Error>,ssh_terminal:SshTerminal,command_length:usize,desktop_app:DesktopAppKind,logs:usize}
// ------------------------=
// FUNC: output_text
// DESC: Supplies an inert diagnostic transport without using text as an assertion oracle.
// ------------------=
fn output_text(_:&[u8]){}
// ------------------------=
// FUNC: text_edit_key
// DESC: Maps the same console key events into the production field editing primitives.
// ------------------=
fn text_edit_key(k:ConsoleKey)->Option<ui::text_input::TextEditKey>{use ui::text_input::TextEditKey as T;Some(match k{ConsoleKey::Character(c)=>T::Character(c),ConsoleKey::Backspace=>T::Backspace,ConsoleKey::Delete=>T::Delete,ConsoleKey::Left=>T::Left,ConsoleKey::Right=>T::Right,ConsoleKey::Home=>T::Home,ConsoleKey::End=>T::End,_=>return None})}
#[path="../kernel/core/console_task_automation.rs"]mod controller;
impl ConsoleRuntime{
    // ------------------------=
    // FUNC: automation_error
    // DESC: Leaves diagnostic prose outside the behavioral oracle while retaining typed receipt failures.
    // ------------------=
    fn automation_error(&mut self,_:runtime::automation::Error,_:Option<runtime::automation::Action>){}
    // ------------------------=
    // FUNC: automation_access
    // DESC: Supplies explicit admission fixtures; native identity policy is tested by the shared adapter harness.
    // ------------------=
    fn automation_access(&self)->runtime::automation::Access {use runtime::automation::{Access,Error};let available=runtime::with_runtime(|r|r.automation_ready).unwrap_or(false);Access{available,can_read:available,can_manage:available&&self.allowed,reason:if !available{Some(Error::Storage)}else if !self.allowed{Some(Error::Denied)}else{None}}}
    // ------------------------=
    // FUNC: task_automation_execute_receipt
    // DESC: Drives the real parser and guarded catalog transaction with injectable persistence failures.
    // ------------------=
    fn task_automation_execute_receipt(&mut self,bytes:&[u8],expected:Option<runtime::automation::Definition>)->Result<runtime::automation::Receipt,runtime::automation::Error>{
        use runtime::automation::{self as a,Action,Error,Receipt};
        if let Some(error)=self.fail{return Err(error);}
        let command=a::parse(bytes)?;
        let id=runtime::with_runtime(|r|r.automation.apply_expected(&command,self.current_user.0,self.allowed,100,expected)).ok_or(Error::Unavailable)??;
        if command.action==Action::Logs {self.logs+=1;ui::command_terminal::output(b"fixture");}else{self.commits+=1;}Ok(Receipt{id,action:command.action})
    }
    // ------------------------=
    // FUNC: copy_workplace_text
    // DESC: Uses the real bounded session clipboard for controller selection and atomic paste tests.
    // ------------------=
    fn copy_workplace_text(&self,bytes:&[u8])->bool{ui::clipboard::with_shared(|c|c.write(self.current_session.0,ui::clipboard::ClipboardKind::Utf8Text,bytes,1)).is_ok()}
    // ------------------------=
    // FUNC: read_workplace_text
    // DESC: Reads the same native clipboard authority as the application transport.
    // ------------------=
    fn read_workplace_text(&self,out:&mut[u8])->Result<usize,ui::clipboard::ClipboardError>{ui::clipboard::with_shared(|c|c.read(self.current_session.0,ui::clipboard::ClipboardKind::Utf8Text,out,1))}
    // ------------------------=
    // FUNC: open_command_window
    // DESC: Records the visible destination without involving a framebuffer in this controller fixture.
    // ------------------=
    fn open_command_window(&mut self){self.desktop_app=DesktopAppKind::CommandWindow;}
    // ------------------------=
    // FUNC: publish_command_terminal
    // DESC: Establishes the actual session-private terminal capture used by View Output.
    // ------------------=
    fn publish_command_terminal(&self){ui::command_terminal::mutate(|s|{s.session(self.current_session.0,true);});}

}
// ------------------------=
// FUNC: click
// DESC: Sends a physical point at an enabled production target through the actual pointer controller.
// ------------------=
fn click(c:&mut ConsoleRuntime,target:ui::task_management::Target){let l=SystemLayout::new(c.system.framebuffer_width,c.system.framebuffer_height);let g=l.desktop_app_window_geometry(c.app_window_x,c.app_window_y,c.app_window_width,c.app_window_height,c.app_window_maximized);let view=ui::task_management::current();let layout=ui::task_management::layout(g.content,g.toolbar,&view,l.scale());let bounds=layout.controls[..layout.count].iter().find(|v|v.target==target&&v.enabled).unwrap().bounds;assert!(c.activate_task_management_pointer(g,ui::geometry::Point{x:bounds.x+bounds.width as i32/2,y:bounds.y+bounds.height as i32/2},l.scale()));}
// ------------------------=
// FUNC: attempt_disabled
// DESC: Sends a real pointer event to disabled geometry and verifies that the UI cannot activate it.
// ------------------=
fn attempt_disabled(c:&mut ConsoleRuntime,target:ui::task_management::Target){let l=SystemLayout::new(c.system.framebuffer_width,c.system.framebuffer_height);let g=l.desktop_app_window_geometry(c.app_window_x,c.app_window_y,c.app_window_width,c.app_window_height,c.app_window_maximized);let view=ui::task_management::current();let layout=ui::task_management::layout(g.content,g.toolbar,&view,l.scale());let control=layout.controls[..layout.count].iter().find(|v|v.target==target).unwrap();assert!(!control.enabled);let bounds=control.bounds;let commits=c.commits;assert!(c.activate_task_management_pointer(g,ui::geometry::Point{x:bounds.x+bounds.width as i32/2,y:bounds.y+bounds.height as i32/2},l.scale()));assert_eq!(c.commits,commits);}
// ------------------------=
// FUNC: type_field
// DESC: Replaces one editable field through real key input, preserving normal caret semantics.
// ------------------=
fn type_field(c:&mut ConsoleRuntime,index:usize,value:&[u8]){click(c,ui::task_management::Target::Field(index));c.input_task_management(ConsoleKey::End);for _ in 0..96{c.input_task_management(ConsoleKey::Backspace);}for byte in value{assert!(c.input_task_management(ConsoleKey::Character(*byte)));}}
// ------------------------=
// FUNC: main
// DESC: Exercises service/schedule creation, configuration, lifecycle intent, denied mutation, and inline deletion through the production GUI controller.
// ------------------=
fn main(){
    use ui::task_management::{self as m,Tab,Target,Mode,Feedback};use runtime::automation::{Kind,State,Trigger,Restart};
    runtime::initialize(false);runtime::with_runtime(|r|r.automation_ready=true);let mut c=ConsoleRuntime{current_user:runtime::identity::StableId([1;16]),current_session:runtime::identity::StableId([10;16]),system:System{framebuffer_width:1280,framebuffer_height:800},app_window_x:80,app_window_y:100,app_window_width:760,app_window_height:650,app_window_maximized:false,shell_menu:0,allowed:true,commits:0,fail:None,ssh_terminal:SshTerminal{active:false},command_length:0,desktop_app:DesktopAppKind::TaskManager,logs:0};
    ui::clipboard::with_shared(|clip|clip.session(c.current_session.0,true));
    c.refresh_task_automation();click(&mut c,Target::Tab(Tab::Services));click(&mut c,Target::New);
    type_field(&mut c,0,b"worker");type_field(&mut c,1,b"/home/default/my worker.py");click(&mut c,Target::Trigger);assert_eq!(m::current().state.trigger,0);c.input_task_management(ConsoleKey::Escape);assert_eq!(m::current().state.trigger,0);click(&mut c,Target::Trigger);c.input_task_management(ConsoleKey::Down);assert_eq!(m::current().state.trigger,0);c.input_task_management(ConsoleKey::Enter);assert_eq!(m::current().state.trigger,1);type_field(&mut c,2,b"15");click(&mut c,Target::RestartPolicy);click(&mut c,Target::RestartOption(1));click(&mut c,Target::Save);
    let e=runtime::with_runtime(|r|r.automation.find(Kind::Daemon,b"worker").unwrap()).unwrap();assert_eq!(e.trigger,Trigger::Boot{delay_seconds:15});assert_eq!(e.restart,Restart::OnFailure);assert_eq!(m::current().state.feedback,Feedback::Saved);
    click(&mut c,Target::Start);assert_eq!(runtime::with_runtime(|r|r.automation.get(e.id).unwrap().state),Some(State::Queued));
    click(&mut c,Target::Stop);assert_eq!(runtime::with_runtime(|r|r.automation.get(e.id).unwrap().state),Some(State::Stopped));
    click(&mut c,Target::Edit);type_field(&mut c,1,b"/home/default/updated.py");click(&mut c,Target::Tab(Tab::Scheduled));assert_eq!(m::current().state.tab,Tab::Services);assert_eq!(m::current().state.feedback,Feedback::SaveOrCancel);click(&mut c,Target::Save);
    assert_eq!(runtime::with_runtime(|r|r.automation.get(e.id).unwrap().path.bytes()==b"/home/default/updated.py"),Some(true));
    c.allowed=false;let commits=c.commits;c.refresh_task_automation();assert!(!m::current().can_manage);assert_eq!(m::current().access_reason,Feedback::ReadOnly);attempt_disabled(&mut c,Target::Enabled);c.input_task_management(ConsoleKey::Character(b'n'));assert_eq!(m::current().state.mode,Mode::Browse);assert_eq!(c.commits,commits);assert!(runtime::with_runtime(|r|r.automation.get(e.id).unwrap().enabled).unwrap());c.allowed=true;c.refresh_task_automation();
    click(&mut c,Target::Tab(Tab::Scheduled));click(&mut c,Target::New);type_field(&mut c,0,b"nightly");type_field(&mut c,1,b"/home/default/nightly.py");type_field(&mut c,2,b"3600");click(&mut c,Target::Save);
    let job=runtime::with_runtime(|r|r.automation.find(Kind::Schedule,b"nightly").unwrap()).unwrap();assert_eq!(job.trigger,Trigger::Interval{seconds:3600});
    click(&mut c,Target::Edit);type_field(&mut c,2,b"0");click(&mut c,Target::Save);assert_eq!(m::current().state.mode,Mode::Edit);assert_eq!(m::current().state.feedback,Feedback::InvalidTiming);assert_eq!(m::current().state.focus,Target::Field(2));click(&mut c,Target::Cancel);
    click(&mut c,Target::Delete);assert!(runtime::with_runtime(|r|r.automation.get(job.id).is_some()).unwrap());click(&mut c,Target::Cancel);assert_eq!(m::current().state.mode,Mode::Browse);click(&mut c,Target::Delete);click(&mut c,Target::Confirm);assert!(runtime::with_runtime(|r|r.automation.get(job.id).is_none()).unwrap());
    assert_eq!(controller::task_automation_parse_date(b"2000-02-29 00:00"),Some(951782400));
    assert_eq!(controller::task_automation_parse_date(b"2100-02-29 00:00:00"),None);
    assert_eq!(controller::task_automation_parse_date(b"2028-02-29 23:59:59"),Some(1835481599));
    for invalid in [b"2026-11-09 24:00".as_slice(),b"2026-13-01 00:00",b"2026-11-31 00:00",b"2026-11-09 12:60",b"2026-11-09 12:01:60",b"1794182400"]{assert_eq!(controller::task_automation_parse_date(invalid),None);}
    for utc in [86400,951782400,1835481599,7258118399]{let displayed=controller::task_automation_date(utc);assert_eq!(controller::task_automation_parse_date(displayed.bytes()),Some(utc));}
    click(&mut c,Target::New);type_field(&mut c,0,b"once");type_field(&mut c,1,b"/home/default/once.py");click(&mut c,Target::Trigger);click(&mut c,Target::TriggerOption(3));type_field(&mut c,2,b"2028-02-29 23:59:59");click(&mut c,Target::Save);
    assert_eq!(runtime::with_runtime(|r|r.automation.find(Kind::Schedule,b"once").unwrap().trigger),Some(Trigger::Once{unix_seconds:1835481599}));
    // Created rows are selected by ID even when a freed catalog slot is reused.
    let once=runtime::with_runtime(|r|r.automation.find(Kind::Schedule,b"once").unwrap()).unwrap();assert_eq!(m::current().selected().unwrap().id,once.id);
    click(&mut c,Target::Edit);let draft=m::current().state.fields;
    for (error,feedback) in [(runtime::automation::Error::Storage,Feedback::Storage),(runtime::automation::Error::ClockUnavailable,Feedback::Clock),(runtime::automation::Error::ScriptUnavailable,Feedback::Script),(runtime::automation::Error::Busy,Feedback::Busy)]{
        c.fail=Some(error);click(&mut c,Target::Save);assert_eq!(m::current().state.feedback,feedback);assert_eq!(m::current().state.mode,Mode::Edit);assert_eq!(m::current().state.fields,draft);
    }c.fail=None;click(&mut c,Target::Cancel);
    // Output cannot displace interactive work, then uses native authorized terminal capture.
    for condition in 0..3 {c.command_length=if condition==0{1}else{0};c.ssh_terminal.active=condition==1;ui::terminal_edit::mutate(|s|s.active=condition==2);click(&mut c,Target::Output);assert_eq!(m::current().state.feedback,Feedback::TerminalBusy);assert_eq!(c.logs,0);assert_eq!(c.desktop_app,DesktopAppKind::TaskManager);}
    c.command_length=0;c.ssh_terminal.active=false;ui::terminal_edit::mutate(|s|s.active=false);click(&mut c,Target::Output);assert_eq!(c.logs,1);assert_eq!(c.desktop_app,DesktopAppKind::CommandWindow);assert!(ui::command_terminal::inspect(|s|s.count>0));c.desktop_app=DesktopAppKind::TaskManager;
    click(&mut c,Target::Tab(Tab::Services));
    // Manual timing is disabled and ignored even when a previous policy left a nonnumeric draft.
    click(&mut c,Target::New);type_field(&mut c,0,b"manual");type_field(&mut c,1,b"/home/default/manual.py");let mut v=m::current();v.state.fields[2]=m::Text::new();m::publish(v);click(&mut c,Target::Save);
    let manual=runtime::with_runtime(|r|r.automation.find(Kind::Daemon,b"manual").unwrap()).unwrap();assert_eq!(manual.trigger,Trigger::Manual);assert_eq!(m::current().selected().unwrap().id,manual.id);
    // Shared selection and clipboard edit a real field, atomically rejecting an oversized paste.
    click(&mut c,Target::Edit);click(&mut c,Target::Field(1));c.input_task_management(ConsoleKey::Shortcut(b'a'));assert!(m::current().state.fields[1].selected);c.input_task_management(ConsoleKey::Shortcut(b'c'));
    c.input_task_management(ConsoleKey::Shortcut(b'x'));assert_eq!(m::current().state.fields[1].len,0);c.input_task_management(ConsoleKey::Shortcut(b'v'));assert_eq!(m::current().state.fields[1].bytes(),manual.path.bytes());
    c.input_task_management(ConsoleKey::Shortcut(b'a'));assert!(c.copy_workplace_text(&[b'x';97]));c.input_task_management(ConsoleKey::Shortcut(b'v'));assert_eq!(m::current().state.fields[1].bytes(),manual.path.bytes());assert_eq!(m::current().state.feedback,Feedback::TooLong);
    c.input_task_management(ConsoleKey::Character(b'z'));assert_eq!(m::current().state.fields[1].bytes(),b"z");click(&mut c,Target::Save);assert_eq!(m::current().state.feedback,Feedback::InvalidPath);assert_eq!(m::current().state.focus,Target::Field(1));
    type_field(&mut c,1,b"/home/default/both'\"quotes.py");click(&mut c,Target::Save);assert_eq!(runtime::with_runtime(|r|r.automation.get(manual.id).unwrap().path.bytes()==b"/home/default/both'\"quotes.py"),Some(true));
    // A sibling deletion changes row indices but must not retarget an open confirmation.
    click(&mut c,Target::Delete);let command=runtime::automation::parse(b"service delete worker").unwrap();runtime::with_runtime(|r|r.automation.apply(&command,[1;16],true,100).unwrap());c.refresh_task_automation();assert_eq!(m::current().selected().unwrap().id,manual.id);assert_eq!(m::current().state.original.unwrap().id,manual.id);assert!(!m::current().state.conflict);click(&mut c,Target::Confirm);assert!(runtime::with_runtime(|r|r.automation.get(manual.id).is_none()).unwrap());
    // Recreated names have new identities; an old draft may not overwrite or delete them.
    for mode in [Mode::Edit,Mode::Delete] {
        let create=runtime::automation::parse(b"service create fence /home/default/fence.py").unwrap();runtime::with_runtime(|r|r.automation.apply(&create,[1;16],true,100).unwrap());c.refresh_task_automation();
        let id=runtime::with_runtime(|r|r.automation.find(Kind::Daemon,b"fence").unwrap().id).unwrap();let mut v=m::current();v.state.selected_id=id;m::publish(v);c.refresh_task_automation();click(&mut c,if mode==Mode::Edit{Target::Edit}else{Target::Delete});
        if mode==Mode::Edit {type_field(&mut c,1,b"/home/default/unsaved.py");}
        let delete=runtime::automation::parse(b"service delete fence").unwrap();runtime::with_runtime(|r|{r.automation.apply(&delete,[1;16],true,100).unwrap();r.automation.apply(&create,[1;16],true,100).unwrap();});c.refresh_task_automation();
        let stale=m::current();assert!(stale.state.conflict);assert_eq!(stale.state.original.unwrap().id,id);assert_eq!(stale.state.mode,mode);assert!(stale.selected().is_none());attempt_disabled(&mut c,if mode==Mode::Edit{Target::Save}else{Target::Confirm});if mode==Mode::Edit {assert_eq!(m::current().state.fields[1].bytes(),b"/home/default/unsaved.py");}click(&mut c,Target::Cancel);runtime::with_runtime(|r|r.automation.apply(&delete,[1;16],true,100).unwrap());c.refresh_task_automation();
    }
    // Lifecycle-only telemetry is not a configuration conflict; busy save remains recoverable.
    let create=runtime::automation::parse(b"service create lifecycle /home/default/lifecycle.py").unwrap();runtime::with_runtime(|r|r.automation.apply(&create,[1;16],true,100).unwrap());c.refresh_task_automation();click(&mut c,Target::Edit);
    let start=runtime::automation::parse(b"service start lifecycle").unwrap();runtime::with_runtime(|r|r.automation.apply(&start,[1;16],true,100).unwrap());c.refresh_task_automation();assert!(!m::current().state.conflict);click(&mut c,Target::Save);assert_eq!(m::current().state.feedback,Feedback::Busy);assert_eq!(m::current().state.mode,Mode::Edit);
    let stop=runtime::automation::parse(b"service stop lifecycle").unwrap();runtime::with_runtime(|r|r.automation.apply(&stop,[1;16],true,100).unwrap());click(&mut c,Target::Save);assert_eq!(m::current().state.mode,Mode::Browse);
    // Saving an unchanged preexisting UTF-8 path preserves its exact bytes.
    let unicode=runtime::automation::parse("service create unicode /home/default/café.py".as_bytes()).unwrap();let unicode_id=runtime::with_runtime(|r|r.automation.apply(&unicode,[1;16],true,100).unwrap().unwrap()).unwrap();c.refresh_task_automation();let mut v=m::current();v.state.selected_id=unicode_id;m::publish(v);c.refresh_task_automation();click(&mut c,Target::Edit);click(&mut c,Target::Save);assert_eq!(runtime::with_runtime(|r|r.automation.get(unicode_id).unwrap().path.bytes()=="/home/default/café.py".as_bytes()),Some(true));
    click(&mut c,Target::New);type_field(&mut c,0,b"private-unsaved-draft");
    c.current_user=runtime::identity::StableId([2;16]);c.current_session=runtime::identity::StableId([20;16]);c.refresh_task_automation();
    let switched=m::current();assert_eq!(switched.owner,[2;16]);assert_eq!(switched.session,[20;16]);assert_eq!(switched.state.mode,Mode::Browse);assert_eq!(switched.state.tab,Tab::Processes);assert!(switched.state.fields.iter().all(|f|f.len==0));
    click(&mut c,Target::Tab(Tab::Scheduled));click(&mut c,Target::New);type_field(&mut c,0,b"same-user-new-session");
    c.current_session=runtime::identity::StableId([21;16]);c.refresh_task_automation();assert_eq!(m::current().state.mode,Mode::Browse);assert!(m::current().state.fields.iter().all(|f|f.len==0));
    println!("Task Manager GUI controller: typed create/edit/lifecycle/denial/delete and draft preservation passed");
}
