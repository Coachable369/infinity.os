#![allow(dead_code)]
//! Uses production keyboard/pointer controller with the real typed catalog authority.
#[path="../kernel/runtime/mod.rs"]mod runtime;
#[path="../kernel/ui/mod.rs"]mod ui;
use ui::system_layout::SystemLayout;
#[derive(Clone,Copy)]
pub enum ConsoleKey{Shortcut(u8),SelectMove(i8),Character(u8),Backspace,Enter,Escape,Tab(bool),Up,Down,Left,Right,Home,End,Delete,Help}
mod console {pub use super::ConsoleKey;}
struct System{framebuffer_width:usize,framebuffer_height:usize}
struct ConsoleRuntime{current_user:runtime::identity::StableId,current_session:runtime::identity::StableId,system:System,app_window_x:i32,app_window_y:i32,app_window_width:i32,app_window_height:i32,app_window_maximized:bool,shell_menu:u8,allowed:bool,commits:usize}
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
    // FUNC: task_automation_execute
    // DESC: Drives the actual shared parser and catalog; storage/worker acceptance belongs to their own tests.
    // ------------------=
    fn task_automation_execute(&mut self,bytes:&[u8])->bool{let Ok(command)=runtime::automation::parse(bytes)else{return false;};let ok=runtime::with_runtime(|r|r.automation.apply(&command,self.current_user.0,self.allowed,100)).is_some_and(|v|v.is_ok());if ok{self.commits+=1;}ok}
}
// ------------------------=
// FUNC: click
// DESC: Sends a physical point at an enabled production target through the actual pointer controller.
// ------------------=
fn click(c:&mut ConsoleRuntime,target:ui::task_management::Target){let l=SystemLayout::new(c.system.framebuffer_width,c.system.framebuffer_height);let g=l.desktop_app_window_geometry(c.app_window_x,c.app_window_y,c.app_window_width,c.app_window_height,c.app_window_maximized);let view=ui::task_management::current();let layout=ui::task_management::layout(g.content,g.toolbar,&view,l.scale());let bounds=layout.controls[..layout.count].iter().find(|v|v.target==target&&v.enabled).unwrap().bounds;assert!(c.activate_task_management_pointer(g,ui::geometry::Point{x:bounds.x+bounds.width as i32/2,y:bounds.y+bounds.height as i32/2},l.scale()));}
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
    runtime::initialize(false);runtime::with_runtime(|r|r.automation_ready=true);let mut c=ConsoleRuntime{current_user:runtime::identity::StableId([1;16]),current_session:runtime::identity::StableId([10;16]),system:System{framebuffer_width:1280,framebuffer_height:800},app_window_x:80,app_window_y:100,app_window_width:760,app_window_height:650,app_window_maximized:false,shell_menu:0,allowed:true,commits:0};
    c.refresh_task_automation();click(&mut c,Target::Tab(Tab::Services));click(&mut c,Target::New);
    type_field(&mut c,0,b"worker");type_field(&mut c,1,b"/home/default/my worker.py");type_field(&mut c,2,b"15");click(&mut c,Target::Trigger);assert_eq!(m::current().state.trigger,0);c.input_task_management(ConsoleKey::Escape);assert_eq!(m::current().state.trigger,0);click(&mut c,Target::Trigger);c.input_task_management(ConsoleKey::Down);assert_eq!(m::current().state.trigger,0);c.input_task_management(ConsoleKey::Enter);assert_eq!(m::current().state.trigger,1);click(&mut c,Target::RestartPolicy);click(&mut c,Target::RestartOption(1));click(&mut c,Target::Save);
    let e=runtime::with_runtime(|r|r.automation.find(Kind::Daemon,b"worker").unwrap()).unwrap();assert_eq!(e.trigger,Trigger::Boot{delay_seconds:15});assert_eq!(e.restart,Restart::OnFailure);assert_eq!(m::current().state.feedback,Feedback::Saved);
    click(&mut c,Target::Start);assert_eq!(runtime::with_runtime(|r|r.automation.get(e.id).unwrap().state),Some(State::Queued));
    click(&mut c,Target::Stop);assert_eq!(runtime::with_runtime(|r|r.automation.get(e.id).unwrap().state),Some(State::Stopped));
    click(&mut c,Target::Edit);type_field(&mut c,1,b"/home/default/updated.py");click(&mut c,Target::Tab(Tab::Scheduled));assert_eq!(m::current().state.tab,Tab::Services);assert_eq!(m::current().state.feedback,Feedback::SaveOrCancel);click(&mut c,Target::Save);
    assert_eq!(runtime::with_runtime(|r|r.automation.get(e.id).unwrap().path.bytes()==b"/home/default/updated.py"),Some(true));
    c.allowed=false;let commits=c.commits;click(&mut c,Target::Enabled);assert_eq!(c.commits,commits);assert_eq!(m::current().state.feedback,Feedback::Failed);assert!(runtime::with_runtime(|r|r.automation.get(e.id).unwrap().enabled).unwrap());c.allowed=true;
    click(&mut c,Target::Tab(Tab::Scheduled));click(&mut c,Target::New);type_field(&mut c,0,b"nightly");type_field(&mut c,1,b"/home/default/nightly.py");type_field(&mut c,2,b"3600");click(&mut c,Target::Save);
    let job=runtime::with_runtime(|r|r.automation.find(Kind::Schedule,b"nightly").unwrap()).unwrap();assert_eq!(job.trigger,Trigger::Interval{seconds:3600});
    click(&mut c,Target::Edit);type_field(&mut c,2,b"0");click(&mut c,Target::Save);assert_eq!(m::current().state.mode,Mode::Edit);assert_eq!(m::current().state.feedback,Feedback::Invalid);click(&mut c,Target::Cancel);
    click(&mut c,Target::Delete);assert!(runtime::with_runtime(|r|r.automation.get(job.id).is_some()).unwrap());click(&mut c,Target::Cancel);assert_eq!(m::current().state.mode,Mode::Browse);click(&mut c,Target::Delete);click(&mut c,Target::Confirm);assert!(runtime::with_runtime(|r|r.automation.get(job.id).is_none()).unwrap());
    assert_eq!(controller::task_automation_parse_date(b"2000-02-29 00:00"),Some(951782400));
    assert_eq!(controller::task_automation_parse_date(b"2100-02-29 00:00:00"),None);
    assert_eq!(controller::task_automation_parse_date(b"2028-02-29 23:59:59"),Some(1835481599));
    for invalid in [b"2026-11-09 24:00".as_slice(),b"2026-13-01 00:00",b"2026-11-31 00:00",b"2026-11-09 12:60",b"2026-11-09 12:01:60",b"1794182400"]{assert_eq!(controller::task_automation_parse_date(invalid),None);}
    for utc in [86400,951782400,1835481599,7258118399]{let displayed=controller::task_automation_date(utc);assert_eq!(controller::task_automation_parse_date(displayed.bytes()),Some(utc));}
    click(&mut c,Target::New);type_field(&mut c,0,b"once");type_field(&mut c,1,b"/home/default/once.py");click(&mut c,Target::Trigger);click(&mut c,Target::TriggerOption(3));type_field(&mut c,2,b"2028-02-29 23:59:59");click(&mut c,Target::Save);
    assert_eq!(runtime::with_runtime(|r|r.automation.find(Kind::Schedule,b"once").unwrap().trigger),Some(Trigger::Once{unix_seconds:1835481599}));
    click(&mut c,Target::New);type_field(&mut c,0,b"private-unsaved-draft");
    c.current_user=runtime::identity::StableId([2;16]);c.current_session=runtime::identity::StableId([20;16]);c.refresh_task_automation();
    let switched=m::current();assert_eq!(switched.owner,[2;16]);assert_eq!(switched.session,[20;16]);assert_eq!(switched.state.mode,Mode::Browse);assert_eq!(switched.state.tab,Tab::Processes);assert!(switched.state.fields.iter().all(|f|f.len==0));
    click(&mut c,Target::Tab(Tab::Scheduled));click(&mut c,Target::New);type_field(&mut c,0,b"same-user-new-session");
    c.current_session=runtime::identity::StableId([21;16]);c.refresh_task_automation();assert_eq!(m::current().state.mode,Mode::Browse);assert!(m::current().state.fields.iter().all(|f|f.len==0));
    println!("Task Manager GUI controller: typed create/edit/lifecycle/denial/delete and draft preservation passed");
}
