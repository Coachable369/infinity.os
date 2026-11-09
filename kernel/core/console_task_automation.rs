//! Task Manager controls use the Command Window's authorized automation transaction.
use super::*;
use crate::ui::task_management::{self as model,Feedback,Mode,Tab,Target};
use crate::runtime::automation::{self,Definition,Error,Kind,Restart,State,Trigger};

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: refresh_task_automation
    // DESC: Refreshes authorized rows by stable identity and preserves drafts unless the owning session changes.
    // ------------------=
    pub(super) fn refresh_task_automation(&mut self) {
        let mut view=model::current();
        if view.owner!=self.current_user.0||view.session!=self.current_session.0 {
            view=model::View::new();view.owner=self.current_user.0;view.session=self.current_session.0;
        }
        let access=self.automation_access();view.count=0;view.total=0;view.available=access.available;view.can_manage=access.can_manage;
        view.access_reason=access.reason.map(task_automation_error).unwrap_or(Feedback::None);
        let kind=if view.state.tab==Tab::Scheduled{Kind::Schedule}else{Kind::Daemon};
        let monotonic=crate::runtime::ai::qwen::workers::clock_ns();
        let now=crate::runtime::ai::clock::SYSTEM.utc_nanoseconds(monotonic).map(|n|n/1_000_000_000);
        if access.can_read {crate::runtime::with_runtime(|runtime| {
            view.total=runtime.automation.count(Kind::Daemon)+runtime.automation.count(Kind::Schedule);
            for i in 0..runtime.automation.count(kind).min(model::CAPACITY) {
                let Some(e)=runtime.automation.nth(kind,i)else{continue;};
                let (trigger,timing)=match e.trigger{Trigger::Manual=>(0,0),Trigger::Boot{delay_seconds}=>(1,u64::from(delay_seconds)),Trigger::Interval{seconds}=>(2,u64::from(seconds)),Trigger::Once{unix_seconds}=>(3,unix_seconds),Trigger::Cron{..}=>(4,0)};
                let next=e.next_status(now,monotonic/1_000_000_000);
                view.entries[view.count]=model::Entry{id:e.id,owner:e.owner,can_output:access.can_manage||e.owner==self.current_user.0,next_status:task_automation_next(next),name:model::Text::from_bytes(e.name.bytes()),path:model::Text::from_bytes(e.path.bytes()),enabled:e.enabled,trigger,timing,restart:match e.restart{Restart::Never=>0,Restart::OnFailure=>1,Restart::Always=>2},state:match e.state{State::Stopped=>0,State::Queued=>1,State::Running=>2,State::Stopping=>3,State::Killing=>4,State::Completed=>5,State::Failed=>6},next:e.next_run,timing_text:match e.trigger{Trigger::Cron{schedule}=>automation::format_cron(schedule).map(|text|model::Field::from_bytes(text.bytes())).unwrap_or(model::Field::new()),_=>model::Field::from_bytes(task_automation_date(timing).bytes())},next_text:task_automation_date(e.next_run),exit:e.last_exit,runs:e.runs};view.count+=1;
            }
        });}
        let (geometry,s)=self.task_automation_geometry();let rows=model::layout(geometry.content,geometry.toolbar,&view,s).rows;
        view.reconcile(rows);model::publish(view);
    }
    // ------------------------=
    // FUNC: task_automation_geometry
    // DESC: Resolves Task Manager's own content and toolbar, including when another application is focused.
    // ------------------=
    fn task_automation_geometry(&self)->(crate::ui::system_layout::DesktopAppWindowGeometry,usize) {
        let layout=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height);
        let window=self.desktop_app_windows().2;
        (layout.desktop_app_window_geometry(window.x,window.y,window.width,window.height,window.maximized),layout.scale().max(1))
    }
    // ------------------------=
    // FUNC: input_task_management
    // DESC: Routes native keyboard editing, clipboard and activation through the same authorized controls as pointer input.
    // ------------------=
    pub(super) fn input_task_management(&mut self,key:ConsoleKey)->bool {
        self.refresh_task_automation();
        let mut view=model::current();let (geometry,s)=self.task_automation_geometry();let layout=model::layout(geometry.content,geometry.toolbar,&view,s);
        if let ConsoleKey::Tab(reverse)=key {layout.tab(&mut view.state,reverse);model::publish(view);return true;}
        if let ConsoleKey::Enter=key {if view.state.focus!=Target::None {self.task_automation_activate(view.state.focus);return true;}}
        if view.state.tab==Tab::Processes{return false;}
        if view.state.choice!=0 {
            match key {ConsoleKey::Escape=>{view.state.focus=if view.state.choice==2{Target::RestartPolicy}else{Target::Trigger};view.state.choice=0;},ConsoleKey::Up|ConsoleKey::Down=>layout.tab(&mut view.state,matches!(key,ConsoleKey::Up)),_=>{}}
            model::publish(view);return true;
        }
        if matches!(view.state.mode,Mode::Create|Mode::Edit) {
            if let Target::Field(index)=view.state.focus {
                if index<3&&layout.controls[..layout.count].iter().any(|c|c.target==Target::Field(index)&&c.enabled) {
                    let field=&mut view.state.fields[index];let limit=if index==0{48}else if index==1{95}else if view.state.trigger==4{256}else{20};
                    match key {
                        ConsoleKey::Shortcut(b'a')=>{field.selected=true;field.caret=field.len;},
                        ConsoleKey::Shortcut(b'c'|b'x')=>{if field.selected&&field.len>0 {if self.copy_workplace_text(field.bytes()){if matches!(key,ConsoleKey::Shortcut(b'x')){*field=model::Field::new();}}else{view.state.feedback=Feedback::Clipboard;}}},
                        ConsoleKey::Shortcut(b'v')=>{
                            let mut bytes=[0u8;256];
                            view.state.feedback=match self.read_workplace_text(&mut bytes){Ok(n)=>if field.paste(&bytes[..n],limit){Feedback::None}else{Feedback::TooLong},Err(crate::ui::clipboard::ClipboardError::TooLarge)=>Feedback::TooLong,Err(_)=>Feedback::Clipboard};bytes.fill(0);
                        },
                        _=>if let Some(edit)=text_edit_key(key){
                            view.state.feedback=if let ConsoleKey::Character(byte)=key{if field.paste(&[byte],limit){Feedback::None}else{Feedback::TooLong}}else{field.edit(edit);Feedback::None};
                        }else{if !matches!(key,ConsoleKey::Escape){return true;}}
                    }
                    if !matches!(key,ConsoleKey::Escape){model::publish(view);return true;}
                }
            }
        }
        let target=match key {
            ConsoleKey::Escape if view.state.mode!=Mode::Browse=>Some(Target::Cancel),
            ConsoleKey::Escape=>return false,
            ConsoleKey::Up if view.state.mode==Mode::Browse=>{view.select(view.state.selected.saturating_sub(1),layout.rows);model::publish(view);None},
            ConsoleKey::Down if view.state.mode==Mode::Browse=>{view.select(view.state.selected+1,layout.rows);model::publish(view);None},
            ConsoleKey::Character(b'n'|b'N') if view.state.mode==Mode::Browse=>Some(Target::New),
            ConsoleKey::Character(b'e'|b'E') if view.state.mode==Mode::Browse=>Some(Target::Edit),
            ConsoleKey::Delete if view.state.mode==Mode::Browse=>Some(Target::Delete),_=>None,
        };
        if let Some(target)=target{self.task_automation_activate(target);}true
    }
    // ------------------------=
    // FUNC: activate_task_management_pointer
    // DESC: Places a measured field caret and dispatches only currently enabled production controls.
    // ------------------=
    pub(super) fn activate_task_management_pointer(&mut self,geometry:crate::ui::system_layout::DesktopAppWindowGeometry,point:crate::ui::geometry::Point,scale:usize)->bool {
        self.refresh_task_automation();
        let mut view=model::current();let layout=model::layout(geometry.content,geometry.toolbar,&view,scale);
        if let Some(target)=layout.hit(point){
            if let Target::Field(index)=target {
                let c=layout.controls[..layout.count].iter().find(|c|c.target==target).unwrap();let s=scale.max(1);
                let field=&mut view.state.fields[index];field.caret=model::field_caret(field,c.bounds.width.saturating_sub((20*s)as u32)as usize,point.x-c.bounds.x-(10*s)as i32,s);field.selected=false;
                view.state.focus=target;view.state.choice=0;model::publish(view);
            }else{self.task_automation_activate(target);}return true;
        }
        view.state.tab!=Tab::Processes&&(geometry.content.contains(point)||geometry.toolbar.contains(point))
    }
    // ------------------------=
    // FUNC: task_automation_activate
    // DESC: Preserves drafts on typed failures and commits preconditioned mutations through the shared native authority.
    // ------------------=
    fn task_automation_activate(&mut self,target:Target) {
        self.refresh_task_automation();
        let mut view=model::current();let (geometry,s)=self.task_automation_geometry();let layout=model::layout(geometry.content,geometry.toolbar,&view,s);
        if !layout.controls[..layout.count].iter().any(|c|c.target==target&&c.enabled){return;}
        if !matches!(target,Target::Trigger|Target::RestartPolicy|Target::TriggerOption(_)|Target::RestartOption(_)){view.state.choice=0;}
        match target {
            Target::Tab(tab)=>{view.state.switch(tab);self.shell_menu=0;},
            Target::Row(index)=>view.select(index,layout.rows),
            Target::New=>view.state.begin(Mode::Create,None),
            Target::Edit=>{let selected=view.selected();view.state.begin(Mode::Edit,selected);view.state.focus=Target::Field(1);},
            Target::Delete=>{let selected=view.selected();view.state.begin(Mode::Delete,selected);},
            Target::Cancel=>view.state.cancel(),Target::Field(index)=>view.state.focus=Target::Field(index),
            Target::Previous=>view.select(view.state.first.saturating_sub(layout.rows),layout.rows),
            Target::Next=>view.select(view.state.first+layout.rows,layout.rows),
            Target::Trigger=>{view.state.choice=1;view.state.focus=Target::TriggerOption(view.state.trigger);},
            Target::TriggerOption(option)=>{
                let previous=view.state.trigger;
                if option==4&&previous!=4 {view.state.fields[2]=model::Field::from_bytes(b"*/15 * * * *");}
                else if option==3&&previous!=3 {
                    let now=crate::runtime::ai::clock::SYSTEM.utc_nanoseconds(crate::runtime::ai::qwen::workers::clock_ns()).map(|n|n/1_000_000_000);
                    view.state.fields[2]=now.and_then(|n|n.checked_add(86400)).map(|date|model::Field::from_bytes(task_automation_date(date).bytes())).unwrap_or(model::Field::new());
                }else if previous>=3&&option<3{view.state.fields[2]=model::Field::from_bytes(model::number(if option==2{60}else{0}).bytes());}
                view.state.trigger=option;view.state.choice=0;view.state.focus=Target::Trigger;
            },
            Target::RestartPolicy=>{view.state.choice=2;view.state.focus=Target::RestartOption(view.state.restart);},
            Target::RestartOption(option)=>{view.state.restart=option;view.state.choice=0;view.state.focus=Target::RestartPolicy;},
            Target::Enabled if view.state.mode!=Mode::Browse=>{view.state.enabled=!view.state.enabled;view.state.focus=target;},
            Target::Output=>{view.state.feedback=self.task_automation_output(&view).err().unwrap_or(Feedback::None);},
            Target::Save|Target::Confirm|Target::Start|Target::Stop|Target::Restart|Target::Kill|Target::Enabled=>{
                let expected=if matches!(target,Target::Save|Target::Confirm)&&view.state.mode!=Mode::Create{view.state.original.map(|e|task_automation_definition(e,view.state.tab))}else{None};
                let result=task_automation_command(&view,target).and_then(|command|self.task_automation_execute_receipt(command.bytes(),expected).map_err(task_automation_error));
                match result {
                    Ok(receipt)=>{
                        view.state.cancel();view.state.feedback=if target==Target::Confirm{Feedback::Deleted}else if matches!(target,Target::Start|Target::Stop|Target::Kill|Target::Restart){Feedback::Requested}else{Feedback::Saved};
                        if let Some(id)=receipt.id {view.state.selected_id=id;}
                    },
                    Err(error)=>{view.state.feedback=error;view.state.focus=match error{Feedback::InvalidName|Feedback::Duplicate|Feedback::Protected=>Target::Field(0),Feedback::InvalidPath|Feedback::Script=>Target::Field(1),Feedback::InvalidTiming=>Target::Field(2),_=>view.state.focus};}
                }
            },Target::None=>{},
        }
        model::publish(view);self.refresh_task_automation();
    }
    // ------------------------=
    // FUNC: task_automation_output
    // DESC: Opens authorized retained logs without displacing an interactive terminal or an unsaved command or editor.
    // ------------------=
    fn task_automation_output(&mut self,view:&model::View)->Result<(),Feedback> {
        if self.ssh_terminal.active||self.command_length>0||crate::ui::terminal_edit::inspect(|s|s.active){return Err(Feedback::TerminalBusy);}
        let selected=view.selected().ok_or(Feedback::Conflict)?;
        let command=task_automation_command(view,Target::Output)?;
        self.open_command_window();if self.desktop_app!=DesktopAppKind::CommandWindow{return Err(Feedback::Unavailable);}
        self.publish_command_terminal();
        crate::ui::command_terminal::mutate(|s|{s.monitor=false;s.help=false;s.disk.active=false;s.search=false;s.menu=crate::ui::command_terminal::Menu::None;s.changed();});
        let _capture=crate::ui::command_terminal::Capture::new(true);
        if let Err(error)=self.task_automation_execute_receipt(command.bytes(),Some(task_automation_definition(selected,view.state.tab))){self.automation_error(error,Some(automation::Action::Logs));return Err(task_automation_error(error));}
        Ok(())
    }
}

// ------------------------=
// FUNC: task_automation_error
// DESC: Preserves typed admission failures for actionable inline feedback without parsing diagnostic text.
// ------------------=
fn task_automation_error(error:Error)->Feedback {match error{Error::Invalid=>Feedback::Invalid,Error::Unknown|Error::Conflict=>Feedback::Conflict,Error::Duplicate=>Feedback::Duplicate,Error::Full=>Feedback::Full,Error::Denied=>Feedback::ReadOnly,Error::Busy=>Feedback::Busy,Error::Storage|Error::Corrupt=>Feedback::Storage,Error::Unavailable=>Feedback::Unavailable,Error::SignInRequired=>Feedback::SignIn,Error::Protected=>Feedback::Protected,Error::ScriptUnavailable=>Feedback::Script,Error::ClockUnavailable=>Feedback::Clock}}
// ------------------------=
// FUNC: task_automation_next
// DESC: Converts backend execution eligibility into the runtime-independent presentation model.
// ------------------=
fn task_automation_next(next:automation::NextRun)->model::NextRun {use automation::NextRun as N;match next{N::Disabled=>model::NextRun::Disabled,N::Manual=>model::NextRun::Manual,N::Paused=>model::NextRun::Paused,N::NextBoot=>model::NextRun::NextBoot,N::BootDelay{seconds}=>model::NextRun::BootDelay(seconds),N::Scheduled{..}=>model::NextRun::Scheduled,N::Ready=>model::NextRun::Ready,N::Running=>model::NextRun::Running,N::Stopping=>model::NextRun::Stopping,N::Finished=>model::NextRun::Finished,N::ClockUnavailable=>model::NextRun::ClockUnavailable}}
// ------------------------=
// FUNC: task_automation_definition
// DESC: Reconstructs the original durable identity and configuration for an atomic stale-edit guard.
// ------------------=
fn task_automation_definition(entry:model::Entry,tab:Tab)->Definition {
    Definition{id:entry.id,kind:if tab==Tab::Services{Kind::Daemon}else{Kind::Schedule},name:automation::Text::new(entry.name.bytes()).unwrap(),path:automation::Text::new(entry.path.bytes()).unwrap(),owner:entry.owner,enabled:entry.enabled,trigger:match entry.trigger{1=>Trigger::Boot{delay_seconds:entry.timing as u32},2=>Trigger::Interval{seconds:entry.timing as u32},3=>Trigger::Once{unix_seconds:entry.timing},4=>Trigger::Cron{schedule:automation::parse_cron(entry.timing_text.bytes()).unwrap()},_=>Trigger::Manual},restart:match entry.restart{1=>Restart::OnFailure,2=>Restart::Always,_=>Restart::Never}}
}
// ------------------------=
// FUNC: task_automation_command
// DESC: Validates individual fields before serializing the exact strict Command Window grammar.
// ------------------=
fn task_automation_command(view:&model::View,target:Target)->Result<crate::runtime::console_execution::Bytes<512>,Feedback> {
    use core::fmt::Write;
    let mut out=crate::runtime::console_execution::Bytes::<512>::new();
    let kind=if view.state.tab==Tab::Services{"service"}else{"taskscheduler"};
    if target==Target::Save {
        let name=view.state.fields[0].bytes();
        if name.is_empty()||name.len()>48||!name.iter().all(|c|c.is_ascii_alphanumeric()||b"-_.".contains(c)){return Err(Feedback::InvalidName);}
        let name=core::str::from_utf8(name).map_err(|_|Feedback::InvalidName)?;
        let path=view.state.fields[1].bytes();automation::validate_path(path).map_err(|_|Feedback::InvalidPath)?;
        let timing=match view.state.trigger{0=>0,4=>{automation::parse_cron(view.state.fields[2].bytes()).map_err(|_|Feedback::InvalidTiming)?;0},3=>task_automation_parse_date(view.state.fields[2].bytes()).filter(|n|*n>0).ok_or(Feedback::InvalidTiming)?,_=>model::decimal(&view.state.fields[2]).filter(|n|*n<=u32::MAX as u64&&(view.state.trigger!=2||*n>0)).ok_or(Feedback::InvalidTiming)?};
        write!(out,"{kind} {} {name} ",if view.state.mode==Mode::Create{"create"}else{"edit"}).map_err(|_|Feedback::TooLong)?;
        out.write_char('"').map_err(|_|Feedback::TooLong)?;
        for character in core::str::from_utf8(path).map_err(|_|Feedback::InvalidPath)?.chars() {if character=='"'{out.write_str("\"'\"'\"")}else{out.write_char(character)}.map_err(|_|Feedback::TooLong)?;}
        write!(out,"\" enabled={}",view.state.enabled).map_err(|_|Feedback::TooLong)?;
        match view.state.trigger {0=>write!(out," start=manual"),1=>write!(out," start=boot delay={timing}"),2=>write!(out," every={timing}"),4=>write!(out," cron=\"{}\"",core::str::from_utf8(view.state.fields[2].bytes()).map_err(|_|Feedback::InvalidTiming)?),_=>write!(out," at={timing}")}.map_err(|_|Feedback::TooLong)?;
        if view.state.tab==Tab::Services{write!(out," restart={}",match view.state.restart{1=>"on-failure",2=>"always",_=>"never"}).map_err(|_|Feedback::TooLong)?;}
    }else{
        let selected=if target==Target::Confirm{view.state.original}else{view.selected()}.ok_or(Feedback::Conflict)?;
        let name=core::str::from_utf8(selected.name.bytes()).map_err(|_|Feedback::InvalidName)?;
        let action=match target{Target::Confirm=>"delete",Target::Start=>"start",Target::Stop=>"stop",Target::Restart=>"restart",Target::Kill=>"kill",Target::Output=>"logs",Target::Enabled=>if selected.enabled{"disable"}else{"enable"},_=>return Err(Feedback::Invalid)};
        write!(out,"{kind} {action} {name}").map_err(|_|Feedback::TooLong)?;
    }
    automation::parse(out.bytes()).map_err(task_automation_error)?;Ok(out)
}

// ------------------------=
// FUNC: task_automation_date
// DESC: Formats native UTC calendar fields for display without exposing epoch counters in the graphical interface.
// ------------------=
pub(super) fn task_automation_date(seconds:u64)->model::Text {
    let Some(date)=crate::runtime::ai::clock::local_time(seconds,0)else{return model::Text::new();};
    let mut text=model::Text::from_bytes(b"0000-00-00 00:00:00");
    text.bytes[0]=b'0'+(date.year/1000)as u8;text.bytes[1]=b'0'+(date.year/100%10)as u8;
    text.bytes[2]=b'0'+(date.year/10%10)as u8;text.bytes[3]=b'0'+(date.year%10)as u8;
    for(at,value)in[(5,date.month),(8,date.day),(11,date.hour),(14,date.minute),(17,date.second)]{text.bytes[at]=b'0'+value/10;text.bytes[at+1]=b'0'+value%10;}
    text
}
// ------------------------=
// FUNC: task_automation_parse_date
// DESC: Validates a human-readable UTC date using the shared Gregorian clock conversion before scheduling.
// ------------------=
pub(super) fn task_automation_parse_date(bytes:&[u8])->Option<u64> {
    if !matches!(bytes.len(),16|19)||bytes[4]!=b'-'||bytes[7]!=b'-'||!matches!(bytes[10],b' '|b'T')||bytes[13]!=b':'||(bytes.len()==19&&bytes[16]!=b':'){return None;}
    let year=model::decimal(&model::Text::from_bytes(&bytes[..4]))?;
    if !(1970..=2199).contains(&year){return None;}
    let mut parts=[0u8;5];
    for(index,at)in[5usize,8,11,14,17].into_iter().enumerate(){if at>=bytes.len(){break;}parts[index]=u8::try_from(model::decimal(&model::Text::from_bytes(&bytes[at..at+2]))?).ok()?;}
    infinity_http::clock::unix_seconds(year as u16,parts[0],parts[1],parts[2],parts[3],parts[4],0)
}
