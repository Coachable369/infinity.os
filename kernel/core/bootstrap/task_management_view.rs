//! Native Task Manager automation workspace using the approved Precision kit.
use super::DisplayDevice;
use crate::ui::{geometry::Rect,panel_recipe::{Material,Surface},task_management::{self as model,Control,Feedback,Mode,Tab,Target,View}};

// ------------------------=
// FUNC: trigger_label
// DESC: Describes a typed trigger choice without implying a job has already run.
// ------------------=
fn trigger_label(trigger:u8)->&'static[u8] {match trigger{1=>b"At boot (delay seconds)",2=>b"Every interval (seconds)",3=>b"Once at a UTC date/time",_=>b"Manual start"}}
// ------------------------=
// FUNC: state_label
// DESC: Maps catalog state to concise observable lifecycle feedback.
// ------------------=
fn state_label(state:u8)->&'static[u8] {match state{1=>b"Queued",2=>b"Running",3=>b"Stopping",4=>b"Killing",5=>b"Completed",6=>b"Failed",_=>b"Stopped"}}
// ------------------------=
// FUNC: restart_label
// DESC: Labels the exact restart policy currently selected in the draft.
// ------------------=
fn restart_label(policy:u8)->&'static[u8] {match policy{1=>b"On failure",2=>b"Always restart",_=>b"Never restart"}}
// ------------------------=
// FUNC: inset
// DESC: Insets a control label without changing its authoritative hit region.
// ------------------=
fn inset(r:Rect,s:usize)->Rect {Rect{x:r.x+(10*s)as i32,width:r.width.saturating_sub((20*s)as u32),..r}}
impl DisplayDevice {
    // ------------------------=
    // FUNC: render_task_automation_tabs
    // DESC: Replaces the telemetry caption with the shared three-tab workspace navigation.
    // ------------------=
    pub(super) fn render_task_automation_tabs(&mut self,content:Rect,toolbar:Rect,scale:usize,view:&View) {
        let layout=model::layout(content,toolbar,view,scale);
        for control in &layout.controls[..layout.count] {if control.target==Target::New {self.task_automation_button(*control,b"New",true,view.state.focus==Target::New,scale);}
        if let Target::Tab(tab)=control.target {
            self.task_automation_button(*control,match tab{Tab::Processes=>b"Processes",Tab::Services=>b"Services",Tab::Scheduled=>b"Scheduled Tasks"},tab==view.state.tab,view.state.focus==control.target,scale);
        }}
    }
    // ------------------------=
    // FUNC: render_task_automation
    // DESC: Paints actual daemon and schedule snapshots with the production image-backed controls.
    // ------------------=
    pub(super) fn render_task_automation(&mut self,content:Rect,toolbar:Rect,scale:usize,view:&View)->bool {
        if view.state.tab==Tab::Processes{return false;}
        let s=scale.max(1);let layout=model::layout(content,toolbar,view,s);
        let old=self.render_clip;
        self.intersect_render_clip(content.x.max(0)as usize,content.y.max(0)as usize,content.width as usize,content.height as usize);
        self.precision_surface(content,Surface::WindowFill,Material{fill:(9,15,38),edge:(134,199,255),opacity:245,edge_opacity:0,focus:0,pressed:false,scale:s});
        self.app_label(Rect{x:layout.list.x,y:content.y+(16*s)as i32,width:layout.list.width,height:(28*s)as u32},if view.state.tab==Tab::Services{b"Daemon services"}else{b"Scheduled Python"},(255,255,255),true,s);
        self.precision_surface(layout.detail,Surface::Panel,Material{fill:(16,27,58),edge:(134,199,255),opacity:245,edge_opacity:170,focus:0,pressed:false,scale:s});
        let detail=inset(layout.detail,s);
        let heading=Rect{y:detail.y+(8*s)as i32,height:(32*s)as u32,..detail};
        let selected_name=view.selected().map(|e|e.name);
        self.app_label(heading,match view.state.mode{Mode::Create=>b"New definition",Mode::Edit=>b"Edit definition",Mode::Delete=>b"Delete definition?",Mode::Browse=>selected_name.as_ref().map(|n|n.bytes()).unwrap_or(b"Automation")},(255,255,255),true,s);
        if view.count==0 {self.app_label(Rect{height:(72*s)as u32,..layout.list},if view.available{b"No definitions yet"}else{b"Service unavailable"},(174,197,221),false,s);}
        if view.state.mode==Mode::Browse {
            if let Some(entry)=view.selected() {
                self.task_automation_detail_line(detail,44,b"Python script",entry.path.bytes(),s);
                self.task_automation_detail_line(detail,78,b"State",state_label(entry.state),s);
                let timing=model::number(entry.timing);let mut policy=[0u8;64];
                let prefix:&[u8]=match entry.trigger{1=>b"Boot + ",2=>b"Every ",3=>b"At UTC ",_=>b"Manual"};
                policy[..prefix.len()].copy_from_slice(prefix);let mut len=prefix.len();
                if entry.trigger!=0{policy[len..len+timing.len].copy_from_slice(timing.bytes());len+=timing.len;if entry.trigger<3{policy[len..len+2].copy_from_slice(b" s");len+=2;}}
                self.task_automation_detail_line(detail,104,if entry.trigger==3{b"Once (UTC)"}else{b"Start policy"},if entry.trigger==3{entry.timing_text.bytes()}else{&policy[..len]},s);
                if view.state.tab==Tab::Services{self.task_automation_detail_line(detail,130,b"Restart policy",restart_label(entry.restart),s);}else{self.task_automation_detail_line(detail,130,b"Next (UTC)",if entry.next==0{b"Not queued"}else{entry.next_text.bytes()},s);}
                let runs=model::number(entry.runs);let mut summary=[0u8;56];summary[..runs.len].copy_from_slice(runs.bytes());let mut length=runs.len;
                if let Some(exit)=entry.exit {
                    let code=model::number(exit.unsigned_abs());let separator=if exit<0{b" / -".as_slice()}else{b" / "};
                    summary[length..length+separator.len()].copy_from_slice(separator);length+=separator.len();
                    summary[length..length+code.len].copy_from_slice(code.bytes());length+=code.len;
                }
                self.task_automation_detail_line(detail,156,if entry.exit.is_some(){b"Runs / exit"}else{b"Runs"},&summary[..length],s);
            } else {self.app_label(Rect{y:detail.y+(52*s)as i32,height:(52*s)as u32,..detail},b"Create a Python definition to get started.",(174,197,221),false,s);}
        } else if view.state.mode==Mode::Delete {
            self.app_label(Rect{y:detail.y+(52*s)as i32,height:(60*s)as u32,..detail},b"Remove this definition? Stop a running task first.",(232,191,193),false,s);
        }
        let mut choice_panel=false;
        for c in &layout.controls[..layout.count] {
            if matches!(c.target,Target::TriggerOption(_)|Target::RestartOption(_))&&!choice_panel {
                choice_panel=true;
                let mut bounds=c.bounds;for option in &layout.controls[..layout.count]{if matches!(option.target,Target::TriggerOption(_)|Target::RestartOption(_)){bounds=bounds.union(option.bounds);}}
                self.precision_surface(Rect{x:bounds.x-(4*s)as i32,y:bounds.y-(4*s)as i32,width:bounds.width+(8*s)as u32,height:bounds.height+(8*s)as u32},Surface::Panel,Material{fill:(9,15,38),edge:(134,199,255),opacity:255,edge_opacity:255,focus:0,pressed:false,scale:s});
            }
            let focused=view.state.focus==c.target;
            match c.target {
                Target::Tab(_)|Target::New=>{},
                Target::Row(index)=>{
                    let e=view.entries[index];let active=index==view.state.selected;
                    self.precision_surface(c.bounds,Surface::Secondary,Material{fill:(16,27,58),edge:(134,199,255),opacity:if active{255}else{130},edge_opacity:if active{240}else{0},focus:if active||focused{220}else{0},pressed:false,scale:s});
                    let r=inset(c.bounds,s);self.app_label(Rect{height:(24*s)as u32,..r},e.name.bytes(),(240,246,255),true,s);
                    self.app_label(Rect{y:r.y+(23*s)as i32,height:(18*s)as u32,..r},if !e.enabled&&e.state==2{b"Running / disabled"}else if !e.enabled{b"Disabled"}else{state_label(e.state)},(174,197,221),false,s);
                },
                Target::Field(index)=>{
                    self.app_label(Rect{y:c.bounds.y-(24*s)as i32,height:(24*s)as u32,..c.bounds},match index{0=>if c.enabled{b"Name"}else{b"Name (fixed identifier)"},1=>b"Python script path",_=>match view.state.trigger{3=>b"UTC date / time: YYYY-MM-DD HH:MM[:SS]",2=>b"Interval seconds",1=>b"Delay seconds",_=>b"Timing (unused for manual)"}},(174,197,221),false,s);
                    self.precision_surface(c.bounds,Surface::Field,Material{fill:(9,15,38),edge:(134,199,255),opacity:if c.enabled{255}else{130},edge_opacity:if c.enabled{255}else{130},focus:if focused&&c.enabled{255}else{0},pressed:false,scale:s});
                    let field=view.state.fields[index];let r=inset(c.bounds,s);let available=(r.width as usize/(8*s)).max(1);let first=field.caret.saturating_sub(available.saturating_sub(1));
                    self.app_label(r,&field.bytes()[first..],(240,246,255),false,s);
                    if focused {let prefix=&field.bytes()[first..field.caret];let x=r.x.max(0)as usize+self.app_text_width(prefix,false,s);self.fill_rect_alpha(x,(r.y+8*s as i32).max(0)as usize,s,16*s,255,255,255,255);}
                },
                target=>{
                    let label:&[u8]=match target {
                        Target::Edit=>b"Edit",Target::Delete=>b"Delete",Target::Start=>if view.state.tab==Tab::Services{b"Start"}else{b"Run now"},Target::Stop=>b"Stop",Target::Restart=>b"Restart",Target::Kill=>b"Kill",Target::Save=>b"Save",Target::Cancel=>b"Cancel",Target::Confirm=>b"Delete",Target::Previous=>b"Previous",Target::Next=>b"Next",Target::Trigger=>trigger_label(view.state.trigger),Target::RestartPolicy=>restart_label(view.state.restart),Target::TriggerOption(option)=>trigger_label(option),Target::RestartOption(option)=>restart_label(option),Target::Enabled=>if if view.state.mode==Mode::Browse{view.selected().is_some_and(|e|e.enabled)}else{view.state.enabled}{b"Enabled"}else{b"Disabled"},_=>b""};
                    self.task_automation_button(*c,label,matches!(target,Target::Save),focused,s);
                }
            }
        }
        let feedback:&[u8]=match view.state.feedback{Feedback::None=>if !view.available{b"Automation storage is unavailable."}else if matches!(view.state.mode,Mode::Create|Mode::Edit){b"Tab moves focus. Enter changes a choice or saves."}else{b"Definitions persist across restarts."},Feedback::Saved=>b"Change saved.",Feedback::Deleted=>b"Definition deleted.",Feedback::Failed=>b"Change failed. Check access, script path, and task state.",Feedback::Invalid=>if view.state.trigger==3{b"Check name, Python path, and UTC date/time."}else{b"Check name, Python path, and numeric timing."},Feedback::SaveOrCancel=>b"Save or Cancel this draft before leaving."};
        self.app_label(layout.status,feedback,if matches!(view.state.feedback,Feedback::Failed|Feedback::Invalid|Feedback::SaveOrCancel){(255,170,170)}else{(174,197,221)},false,s);
        self.render_clip=old;true
    }
    // ------------------------=
    // FUNC: task_automation_button
    // DESC: Composes reusable Primary and Secondary controls with native disabled and focused states.
    // ------------------=
    fn task_automation_button(&mut self,c:Control,label:&[u8],primary:bool,focused:bool,s:usize) {
        self.precision_surface(c.bounds,if primary{Surface::Primary}else{Surface::Secondary},Material{fill:if primary{(62,111,255)}else{(16,27,58)},edge:(134,199,255),opacity:if c.enabled{255}else{110},edge_opacity:if c.enabled{255}else{100},focus:if focused&&c.enabled{255}else{0},pressed:false,scale:s});
        let mut label_rect=inset(c.bounds,s);
        if matches!(c.target,Target::Trigger|Target::RestartPolicy){label_rect.width=label_rect.width.saturating_sub((24*s)as u32);self.app_disclosure(Rect{x:c.bounds.right()-(30*s)as i32,width:(24*s)as u32,..c.bounds},true,(174,197,221),s);}
        self.app_label(label_rect,label,if c.enabled{(240,246,255)}else{(112,130,155)},true,s);
    }
    // ------------------------=
    // FUNC: task_automation_detail_line
    // DESC: Fits a named value into a clipped detail row using the shared type hierarchy.
    // ------------------=
    fn task_automation_detail_line(&mut self,r:Rect,y:usize,label:&[u8],value:&[u8],s:usize) {
        let label_width=if y==44{0}else{r.width*45/100};
        if label_width>0{self.app_label(Rect{y:r.y+(y*s)as i32,width:label_width,height:(24*s)as u32,..r},label,(145,171,202),false,s);}
        self.app_label(Rect{x:r.x+label_width as i32,y:r.y+(y*s)as i32,width:r.width-label_width,height:(24*s)as u32},value,(230,240,253),false,s);
    }
}
