//! Task Manager controls use the Command Window's authorized automation transaction.
use super::*;
use crate::ui::task_management::{self as model,Feedback,Mode,Tab,Target};
use crate::runtime::automation::{Kind,Restart,State,Trigger};

impl ConsoleRuntime {
    // ------------------------=
    // FUNC: refresh_task_automation
    // DESC: Refreshes typed catalog rows while preserving any unsaved inline editor draft.
    // ------------------=
    pub(super) fn refresh_task_automation(&mut self) {
        let mut view=model::current();
        if view.owner!=self.current_user.0||view.session!=self.current_session.0 {
            view=model::View::new();view.owner=self.current_user.0;view.session=self.current_session.0;
        }
        let kind=if view.state.tab==Tab::Scheduled{Kind::Schedule}else{Kind::Daemon};
        view.count=0;view.available=false;
        crate::runtime::with_runtime(|runtime| {
            view.available=runtime.automation_ready;
            for i in 0..runtime.automation.count(kind).min(model::CAPACITY) {
                let Some(e)=runtime.automation.nth(kind,i)else{continue;};
                let (trigger,timing)=match e.trigger{Trigger::Manual=>(0,0),Trigger::Boot{delay_seconds}=>(1,u64::from(delay_seconds)),Trigger::Interval{seconds}=>(2,u64::from(seconds)),Trigger::Once{unix_seconds}=>(3,unix_seconds)};
                view.entries[view.count]=model::Entry{id:e.id,name:model::Text::from_bytes(e.name.bytes()),path:model::Text::from_bytes(e.path.bytes()),enabled:e.enabled,trigger,timing,restart:match e.restart{Restart::Never=>0,Restart::OnFailure=>1,Restart::Always=>2},state:match e.state{State::Stopped=>0,State::Queued=>1,State::Running=>2,State::Stopping=>3,State::Killing=>4,State::Completed=>5,State::Failed=>6},next:e.next_run,timing_text:task_automation_date(timing),next_text:task_automation_date(e.next_run),exit:e.last_exit,runs:e.runs};view.count+=1;
            }
        });
        view.state.selected=view.state.selected.min(view.count.saturating_sub(1));view.state.first=view.state.first.min(view.state.selected);
        model::publish(view);
    }
    // ------------------------=
    // FUNC: task_automation_geometry
    // DESC: Resolves the same content and toolbar rectangles used by the production painter.
    // ------------------=
    fn task_automation_geometry(&self)->(crate::ui::system_layout::DesktopAppWindowGeometry,usize) {
        let layout=SystemLayout::new(self.system.framebuffer_width,self.system.framebuffer_height);
        (layout.desktop_app_window_geometry(self.app_window_x,self.app_window_y,self.app_window_width,self.app_window_height,self.app_window_maximized),layout.scale().max(1))
    }
    // ------------------------=
    // FUNC: input_task_management
    // DESC: Routes native keyboard editing and activation to the same Task Manager controls as pointer input.
    // ------------------=
    pub(super) fn input_task_management(&mut self,key:ConsoleKey)->bool {
        let mut view=model::current();let (geometry,s)=self.task_automation_geometry();let layout=model::layout(geometry.content,geometry.toolbar,&view,s);
        if matches!(key,ConsoleKey::Tab(_)){if let ConsoleKey::Tab(reverse)=key{layout.tab(&mut view.state,reverse);}model::publish(view);return true;}
        if let ConsoleKey::Enter=key {if view.state.focus!=Target::None {self.task_automation_activate(view.state.focus);return true;}}
        if view.state.tab==Tab::Processes{return false;}
        if view.state.choice!=0 {
            match key {ConsoleKey::Escape=>{view.state.choice=0;view.state.focus=Target::Trigger;},ConsoleKey::Up|ConsoleKey::Down=>layout.tab(&mut view.state,matches!(key,ConsoleKey::Up)),_=>{}}
            model::publish(view);return true;
        }
        if matches!(view.state.mode,Mode::Create|Mode::Edit) {
            if let Target::Field(index)=view.state.focus {if index<3&&!(index==0&&view.state.mode==Mode::Edit){if let Some(edit)=text_edit_key(key){view.state.fields[index].edit(edit);view.state.feedback=Feedback::None;model::publish(view);return true;}}}
        }
        let target=match key {
            ConsoleKey::Escape if view.state.mode!=Mode::Browse=>Some(Target::Cancel),
            ConsoleKey::Escape=>return false,
            ConsoleKey::Up if view.state.mode==Mode::Browse=>{view.state.select(view.state.selected.saturating_sub(1),view.count,layout.rows);model::publish(view);None},
            ConsoleKey::Down if view.state.mode==Mode::Browse=>{view.state.select(view.state.selected+1,view.count,layout.rows);model::publish(view);None},
            ConsoleKey::Character(b'n'|b'N') if view.state.mode==Mode::Browse=>Some(Target::New),
            ConsoleKey::Character(b'e'|b'E') if view.state.mode==Mode::Browse=>Some(Target::Edit),
            ConsoleKey::Delete if view.state.mode==Mode::Browse=>Some(Target::Delete),
            _=>None,
        };
        if let Some(target)=target{self.task_automation_activate(target);}true
    }
    // ------------------------=
    // FUNC: activate_task_management_pointer
    // DESC: Dispatches only enabled controls and consumes management content before process-row selection.
    // ------------------=
    pub(super) fn activate_task_management_pointer(&mut self,geometry:crate::ui::system_layout::DesktopAppWindowGeometry,point:crate::ui::geometry::Point,scale:usize)->bool {
        let view=model::current();let layout=model::layout(geometry.content,geometry.toolbar,&view,scale);
        if let Some(target)=layout.hit(point){self.task_automation_activate(target);return true;}
        view.state.tab!=Tab::Processes&&(geometry.content.contains(point)||geometry.toolbar.contains(point))
    }
    // ------------------------=
    // FUNC: task_automation_activate
    // DESC: Applies presentation changes locally and sends all service mutations through the shared CLI transaction.
    // ------------------=
    fn task_automation_activate(&mut self,target:Target) {
        let mut view=model::current();let (geometry,s)=self.task_automation_geometry();let layout=model::layout(geometry.content,geometry.toolbar,&view,s);
        if !layout.controls[..layout.count].iter().any(|c|c.target==target&&c.enabled){return;}
        if !matches!(target,Target::Trigger|Target::RestartPolicy|Target::TriggerOption(_)|Target::RestartOption(_)){view.state.choice=0;}
        match target {
            Target::Tab(tab)=>{view.state.switch(tab);self.shell_menu=0;},
            Target::Row(index)=>view.state.select(index,view.count,layout.rows),
            Target::New=>view.state.begin(Mode::Create,None),
            Target::Edit=>{let selected=view.selected();view.state.begin(Mode::Edit,selected);},
            Target::Delete=>{let selected=view.selected();view.state.begin(Mode::Delete,selected);},
            Target::Cancel=>view.state.cancel(),
            Target::Field(index)=>view.state.focus=Target::Field(index),
            Target::Previous=>view.state.select(view.state.first.saturating_sub(layout.rows),view.count,layout.rows),
            Target::Next=>view.state.select(view.state.first+layout.rows,view.count,layout.rows),
            Target::Trigger=>{view.state.choice=1;view.state.focus=Target::TriggerOption(view.state.trigger);},
            Target::TriggerOption(option)=>{
                let previous=view.state.trigger;
                if option==3&&previous!=3 {
                    let now=crate::runtime::ai::clock::SYSTEM.utc_nanoseconds(crate::runtime::ai::qwen::workers::clock_ns()).map(|n|n/1_000_000_000);
                    view.state.fields[2]=now.and_then(|n|n.checked_add(86400)).map(task_automation_date).unwrap_or(model::Text::new());
                }else if previous==3&&option!=3{view.state.fields[2]=model::number(if option==2{60}else{0});}
                view.state.trigger=option;view.state.choice=0;view.state.focus=Target::Trigger;
            },
            Target::RestartPolicy=>{view.state.choice=2;view.state.focus=Target::RestartOption(view.state.restart);},
            Target::RestartOption(option)=>{view.state.restart=option;view.state.choice=0;view.state.focus=Target::RestartPolicy;},
            Target::Enabled if view.state.mode!=Mode::Browse=>{view.state.enabled=!view.state.enabled;view.state.focus=target;},
            Target::Save|Target::Confirm|Target::Start|Target::Stop|Target::Restart|Target::Kill|Target::Enabled=>{
                if let Some(command)=task_automation_command(&view,target) {
                    let success=self.task_automation_execute(command.bytes());
                    view.state.feedback=if success{if target==Target::Confirm{Feedback::Deleted}else{Feedback::Saved}}else{Feedback::Failed};
                    if success{view.state.mode=Mode::Browse;view.state.focus=Target::New;}
                }else{view.state.feedback=Feedback::Invalid;}
            },
            Target::None=>{},
        }
        model::publish(view);self.refresh_task_automation();
    }
}

// ------------------------=
// FUNC: task_automation_command
// DESC: Serializes a GUI intent into the exact strict command grammar accepted by the Command Window.
// ------------------=
fn task_automation_command(view:&model::View,target:Target)->Option<crate::runtime::console_execution::Bytes<512>> {
    use core::fmt::Write;
    let mut out=crate::runtime::console_execution::Bytes::<512>::new();
    let kind=if view.state.tab==Tab::Services{"service"}else{"schedule"};
    if target==Target::Save {
        let name=core::str::from_utf8(view.state.fields[0].bytes()).ok()?;
        let path=core::str::from_utf8(view.state.fields[1].bytes()).ok()?;
        let timing=if view.state.trigger==3{task_automation_parse_date(view.state.fields[2].bytes())?}else{model::decimal(&view.state.fields[2])?};
        let quote=if !path.contains('\''){'\''}else if !path.contains('"'){'"'}else{return None;};
        write!(out,"{kind} {} {name} {quote}{path}{quote} enabled={}",if view.state.mode==Mode::Create{"create"}else{"edit"},view.state.enabled).ok()?;
        match view.state.trigger {0=>write!(out," start=manual"),1=>write!(out," start=boot delay={timing}"),2=>write!(out," every={timing}"),_=>write!(out," at={timing}")}.ok()?;
        if view.state.tab==Tab::Services{write!(out," restart={}",match view.state.restart{1=>"on-failure",2=>"always",_=>"never"}).ok()?;}
    }else{
        let selected=view.selected()?;let name=core::str::from_utf8(selected.name.bytes()).ok()?;
        let action=match target{Target::Confirm=>"delete",Target::Start=>"start",Target::Stop=>"stop",Target::Restart=>"restart",Target::Kill=>"kill",Target::Enabled=>if selected.enabled{"disable"}else{"enable"},_=>return None};
        write!(out,"{kind} {action} {name}").ok()?;
    }
    crate::runtime::automation::parse(out.bytes()).ok()?;Some(out)
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
