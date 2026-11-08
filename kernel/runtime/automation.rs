//! Shared, persistent daemon and scheduled-task authority. Times are UTC seconds.
pub const CAPACITY:usize=16;
pub const STATE_BYTES:usize=32+CAPACITY*256;
pub const PATH:&[u8]=b"/system/automation/state";
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Text<const N:usize>{bytes:[u8;N],length:usize}
impl<const N:usize> Text<N>{
    // ------------------------=
    // FUNC: new
    // DESC: Validates bounded UTF-8 input without silently truncating a command or path.
    // ------------------=
    pub fn new(bytes:&[u8])->Result<Self,Error>{if bytes.len()>N||bytes.contains(&0)||core::str::from_utf8(bytes).is_err(){return Err(Error::Invalid);}let mut out=Self::empty();out.bytes[..bytes.len()].copy_from_slice(bytes);out.length=bytes.len();Ok(out)}
    // ------------------------=
    // FUNC: empty
    // DESC: Creates an empty bounded field for static catalog initialization.
    // ------------------=
    pub const fn empty()->Self{Self{bytes:[0;N],length:0}}
    // ------------------------=
    // FUNC: bytes
    // DESC: Exposes the initialized field bytes to both command and graphical clients.
    // ------------------=
    pub fn bytes(&self)->&[u8]{&self.bytes[..self.length]}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Kind{Daemon,Schedule}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Trigger{Manual,Boot{delay_seconds:u32},Interval{seconds:u32},Once{unix_seconds:u64},Cron{schedule:Cron}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Cron{pub minutes:u64,pub hours:u32,pub days:u32,pub months:u16,pub weekdays:u8,pub dom_any:bool,pub dow_any:bool}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Restart{Never,OnFailure,Always}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum State{Stopped,Queued,Running,Stopping,Killing,Completed,Failed}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Error{Invalid,Unknown,Duplicate,Full,Denied,Busy,Storage,Corrupt,Unavailable,SignInRequired,Protected,ScriptUnavailable,ClockUnavailable,Conflict}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Access{pub available:bool,pub can_read:bool,pub can_manage:bool,pub reason:Option<Error>}
impl Access{
    // ------------------------=
    // FUNC: evaluate
    // DESC: Applies the same signed-in inspection and administrator mutation policy to every authorized client.
    // ------------------=
    pub fn evaluate(available:bool,signed_in:bool,administrator:bool)->Self{Self{available,can_read:available&&signed_in,can_manage:available&&signed_in&&administrator,reason:if !signed_in{Some(Error::SignInRequired)}else if !available{Some(Error::Storage)}else if !administrator{Some(Error::Denied)}else{None}}}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Receipt{pub id:Option<u32>,pub action:Action}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Definition{pub id:u32,pub kind:Kind,pub name:Text<48>,pub path:Text<96>,pub owner:[u8;16],pub enabled:bool,pub trigger:Trigger,pub restart:Restart}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum NextRun{Disabled,Manual,Paused,NextBoot,BootDelay{seconds:u64},Scheduled{unix_seconds:u64},Ready,Running,Stopping,Finished,ClockUnavailable}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Entry{
    pub id:u32,pub name:Text<48>,pub path:Text<96>,pub owner:[u8;16],pub kind:Kind,
    pub enabled:bool,pub trigger:Trigger,pub restart:Restart,pub state:State,
    pub next_run:u64,pub last_exit:Option<i64>,pub runs:u64,pub process_id:u32,
    restart_requested:bool,boot_fired:bool,manual_stop:bool,once_fired:bool,manual_queue:bool,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Action{List,Inspect,Create,Edit,Delete,Start,Stop,Kill,Restart,Enable,Disable,Logs}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Command{pub kind:Kind,pub action:Action,pub name:Text<48>,pub path:Option<Text<96>>,pub enabled:Option<bool>,pub trigger:Option<Trigger>,pub restart:Option<Restart>}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Catalog{entries:[Option<Entry>;CAPACITY],next_id:u32,pub revision:u64}
impl Entry{
    // ------------------------=
    // FUNC: definition
    // DESC: Captures editable configuration independently of changing execution receipts and catalog revision.
    // ------------------=
    pub fn definition(&self)->Definition{Definition{id:self.id,kind:self.kind,name:self.name,path:self.path,owner:self.owner,enabled:self.enabled,trigger:self.trigger,restart:self.restart}}
    // ------------------------=
    // FUNC: next_status
    // DESC: Explains actual admission eligibility without displaying stale timestamps for manual or completed tasks.
    // ------------------=
    pub fn next_status(&self,now:Option<u64>,boot_seconds:u64)->NextRun{
        if self.state==State::Running{return NextRun::Running;}
        if matches!(self.state,State::Stopping|State::Killing){return NextRun::Stopping;}
        if self.state==State::Queued&&(self.enabled||self.manual_queue){return match now{None=>NextRun::ClockUnavailable,Some(now)if now>=self.next_run=>NextRun::Ready,Some(_)=>NextRun::Scheduled{unix_seconds:self.next_run}};}
        if !self.enabled{return NextRun::Disabled;}
        if self.manual_stop{return NextRun::Paused;}
        match self.trigger{
            Trigger::Manual=>NextRun::Manual,
            Trigger::Boot{..}if self.boot_fired=>NextRun::NextBoot,
            Trigger::Boot{delay_seconds}if boot_seconds<u64::from(delay_seconds)=>NextRun::BootDelay{seconds:u64::from(delay_seconds)-boot_seconds},
            Trigger::Once{..}if self.once_fired=>NextRun::Finished,
            Trigger::Cron{..}if self.next_run==u64::MAX=>NextRun::Finished,
            Trigger::Cron{schedule}=>match now{None=>NextRun::ClockUnavailable,Some(now)if now>=self.next_run&&schedule.matches_utc(now)=>NextRun::Ready,Some(now)=>match schedule.next_after(now.max(self.next_run.saturating_sub(60))){Some(unix_seconds)=>NextRun::Scheduled{unix_seconds},None=>NextRun::Finished}},
            Trigger::Boot{..}=>NextRun::Ready,
            _=>match now{None=>NextRun::ClockUnavailable,Some(now)if now>=self.next_run=>NextRun::Ready,Some(_)=>NextRun::Scheduled{unix_seconds:self.next_run}},
        }
    }
}
impl Catalog{
    // ------------------------=
    // FUNC: new
    // DESC: Creates a catalog with no implicit executable authority.
    // ------------------=
    pub const fn new()->Self{Self{entries:[None;CAPACITY],next_id:1,revision:0}}
    // ------------------------=
    // FUNC: count
    // DESC: Counts configured records of the requested management tab.
    // ------------------=
    pub fn count(&self,kind:Kind)->usize{self.entries.iter().flatten().filter(|e|e.kind==kind).count()}
    // ------------------------=
    // FUNC: nth
    // DESC: Supplies one immutable authoritative row to both CLI and GUI.
    // ------------------=
    pub fn nth(&self,kind:Kind,index:usize)->Option<Entry>{self.entries.iter().flatten().filter(|e|e.kind==kind).nth(index).copied()}
    // ------------------------=
    // FUNC: get
    // DESC: Looks up an immutable record by stable identifier.
    // ------------------=
    pub fn get(&self,id:u32)->Option<Entry>{self.entries.iter().flatten().find(|e|e.id==id).copied()}
    // ------------------------=
    // FUNC: find
    // DESC: Resolves exact names within a tab without accepting ambiguous prefixes.
    // ------------------=
    pub fn find(&self,kind:Kind,name:&[u8])->Option<Entry>{self.entries.iter().flatten().find(|e|e.kind==kind&&e.name.bytes()==name).copied()}
    // ------------------------=
    // FUNC: apply_expected
    // DESC: Rejects stale configuration or recreated identifiers while allowing unrelated jobs and execution receipts to advance.
    // ------------------=
    pub fn apply_expected(&mut self,command:&Command,owner:[u8;16],administrator:bool,now:u64,expected:Option<Definition>)->Result<Option<u32>,Error>{
        if let Some(expected)=expected{
            if expected.kind!=command.kind||expected.name!=command.name||self.get(expected.id).map(|e|e.definition())!=Some(expected){return Err(Error::Conflict);}
        }
        self.apply(command,owner,administrator,now)
    }
    // ------------------------=
    // FUNC: apply
    // DESC: Applies administrator configuration or lifecycle intent to a staged catalog before its durable commit.
    // ------------------=
    pub fn apply(&mut self,command:&Command,owner:[u8;16],administrator:bool,now:u64)->Result<Option<u32>,Error>{
        if owner==[0;16]{return Err(Error::Denied);}
        if command.action==Action::List{return Ok(None);}
        if matches!(command.action,Action::Inspect|Action::Logs){return self.find(command.kind,command.name.bytes()).map(|e|Some(e.id)).ok_or(Error::Unknown);}
        if !administrator{return Err(Error::Denied);}
        if command.action==Action::Create{
            if self.find(command.kind,command.name.bytes()).is_some(){return Err(Error::Duplicate);}
            let path=command.path.ok_or(Error::Invalid)?;validate_path(path.bytes())?;validate_name(command.name.bytes())?;
            let trigger=command.trigger.unwrap_or(if command.kind==Kind::Daemon{Trigger::Manual}else{Trigger::Cron{schedule:Cron::hourly()}});validate_trigger(command.kind,trigger)?;
            let restart=command.restart.unwrap_or(Restart::Never);if command.kind==Kind::Schedule&&restart!=Restart::Never{return Err(Error::Invalid);}
            let index=self.entries.iter().position(Option::is_none).ok_or(Error::Full)?;
            let id=self.next_id;self.next_id=id.checked_add(1).ok_or(Error::Full)?;
            self.entries[index]=Some(Entry{id,name:command.name,path,owner,kind:command.kind,enabled:command.enabled.unwrap_or(true),trigger,restart,state:State::Stopped,next_run:initial_due(trigger,now),last_exit:None,runs:0,process_id:0,restart_requested:false,boot_fired:false,manual_stop:false,once_fired:false,manual_queue:false});
            self.revision=self.revision.wrapping_add(1);return Ok(Some(id));
        }
        let index=self.entries.iter().position(|e|e.is_some_and(|e|e.kind==command.kind&&e.name==command.name)).ok_or(Error::Unknown)?;
        let mut entry=self.entries[index].unwrap();let active=matches!(entry.state,State::Running|State::Stopping|State::Killing);
        match command.action{
            Action::Delete=>{if active{return Err(Error::Busy);}self.entries[index]=None;self.revision=self.revision.wrapping_add(1);return Ok(Some(entry.id));},
            Action::Edit=>{
                if active||entry.state==State::Queued{return Err(Error::Busy);}
                if let Some(path)=command.path{validate_path(path.bytes())?;entry.path=path;}
                if let Some(trigger)=command.trigger{validate_trigger(entry.kind,trigger)?;entry.trigger=trigger;entry.next_run=initial_due(trigger,now);entry.boot_fired=false;entry.once_fired=false;}
                if let Some(restart)=command.restart{if entry.kind==Kind::Schedule&&restart!=Restart::Never{return Err(Error::Invalid);}entry.restart=restart;}
                if let Some(enabled)=command.enabled{entry.enabled=enabled;}
                entry.manual_stop=false;
            },
            Action::Start=>{if active||entry.state==State::Queued{return Err(Error::Busy);}entry.state=State::Queued;entry.manual_queue=true;entry.manual_stop=false;entry.next_run=now;},
            Action::Stop|Action::Kill=>{entry.manual_stop=true;entry.restart_requested=false;entry.state=if active{if command.action==Action::Kill{State::Killing}else{State::Stopping}}else{State::Stopped};},
            Action::Restart=>{entry.manual_stop=false;entry.restart_requested=active;entry.manual_queue=true;entry.state=if active{State::Stopping}else{State::Queued};entry.next_run=now;},
            Action::Enable=>{entry.enabled=true;entry.manual_stop=false;entry.boot_fired=false;entry.next_run=initial_due(entry.trigger,now);},
            Action::Disable=>{entry.enabled=false;if entry.state==State::Queued&&!entry.manual_queue{entry.state=State::Stopped;}},
            _=>return Err(Error::Invalid),
        }
        self.entries[index]=Some(entry);self.revision=self.revision.wrapping_add(1);Ok(Some(entry.id))
    }
    // ------------------------=
    // FUNC: due
    // DESC: Returns one due nonoverlapping task; missed intervals coalesce into a single execution.
    // ------------------=
    pub fn due(&self,now:u64,boot_seconds:u64)->Option<Entry>{self.due_after(now,boot_seconds,0)}
    // ------------------------=
    // FUNC: due_after
    // DESC: Rotates admission by stable identifier so a failing daemon cannot starve other due tasks.
    // ------------------=
    pub fn due_after(&self,now:u64,boot_seconds:u64,previous:u32)->Option<Entry>{
        let due=|e:&&Entry|{
            if e.state==State::Queued{return (e.enabled||e.manual_queue)&&now>=e.next_run;}
            if !e.enabled||e.manual_stop||matches!(e.state,State::Running|State::Stopping|State::Killing){return false;}
            match e.trigger{Trigger::Manual=>false,Trigger::Boot{delay_seconds}=>!e.boot_fired&&boot_seconds>=u64::from(delay_seconds),Trigger::Interval{..}=>now>=e.next_run,Trigger::Cron{schedule}=>e.next_run!=u64::MAX&&now>=e.next_run&&schedule.matches_utc(now),Trigger::Once{..}=>!e.once_fired&&now>=e.next_run}
        };
        self.entries.iter().flatten().filter(due).filter(|e|e.id>previous).min_by_key(|e|e.id)
            .or_else(||self.entries.iter().flatten().filter(due).min_by_key(|e|e.id)).copied()
    }
    // ------------------------=
    // FUNC: started
    // DESC: Records a successful native process allocation and fences repeat dispatch until completion.
    // ------------------=
    pub fn started(&mut self,id:u32,pid:u32,now:u64)->Result<(),Error>{
        let entry=self.entries.iter_mut().flatten().find(|e|e.id==id).ok_or(Error::Unknown)?;
        if pid==0||matches!(entry.state,State::Running|State::Stopping|State::Killing){return Err(Error::Busy);}
        entry.process_id=pid;entry.state=State::Running;entry.runs=entry.runs.saturating_add(1);entry.boot_fired=true;entry.once_fired|=matches!(entry.trigger,Trigger::Once{..});entry.last_exit=None;
        if let Trigger::Interval{seconds}=entry.trigger{entry.next_run=now.saturating_add(u64::from(seconds));}
        if let Trigger::Cron{schedule}=entry.trigger{entry.next_run=schedule.next_after(now).unwrap_or(u64::MAX);}
        self.revision=self.revision.wrapping_add(1);Ok(())
    }
    // ------------------------=
    // FUNC: finished
    // DESC: Records exit status and bounded restart backoff without automatically resurrecting an explicit stop.
    // ------------------=
    pub fn finished(&mut self,id:u32,exit:i64,now:u64)->Result<(),Error>{
        let entry=self.entries.iter_mut().flatten().find(|e|e.id==id).ok_or(Error::Unknown)?;
        let stopped=matches!(entry.state,State::Stopping|State::Killing);entry.process_id=0;entry.last_exit=Some(exit);entry.boot_fired=true;entry.once_fired|=matches!(entry.trigger,Trigger::Once{..});
        if entry.runs==0{entry.runs=1;}
        if entry.restart_requested{entry.state=State::Queued;entry.manual_queue=true;entry.next_run=now;entry.restart_requested=false;}
        else if !stopped&&!entry.manual_stop&&entry.enabled&&entry.kind==Kind::Daemon&&matches!(entry.restart,Restart::Always|Restart::OnFailure) && (entry.restart==Restart::Always||exit!=0){entry.state=State::Queued;entry.manual_queue=false;entry.next_run=now.saturating_add(5);}
        else{entry.state=if stopped{State::Stopped}else if exit==0{State::Completed}else{State::Failed};}
        if let Trigger::Interval{seconds}=entry.trigger{if entry.next_run<=now{entry.next_run=now.saturating_add(u64::from(seconds));}}
        if let Trigger::Cron{schedule}=entry.trigger{if entry.state!=State::Queued&&entry.next_run<=now{entry.next_run=schedule.next_after(now).unwrap_or(u64::MAX);}}
        self.revision=self.revision.wrapping_add(1);Ok(())
    }
    // ------------------------=
    // FUNC: encode
    // DESC: Encodes a bounded versioned binary checkpoint independent of structure padding and host architecture.
    // ------------------=
    pub fn encode(&self,out:&mut[u8])->Result<usize,Error>{if out.len()<STATE_BYTES{return Err(Error::Full);}out[..STATE_BYTES].fill(0);out[..8].copy_from_slice(b"IAUTO001");out[8..12].copy_from_slice(&self.next_id.to_le_bytes());out[16..24].copy_from_slice(&self.revision.to_le_bytes());
        for(index,entry)in self.entries.iter().enumerate(){let Some(e)=entry else{continue;};let p=&mut out[32+index*256..32+(index+1)*256];p[0]=1;p[1]=if e.kind==Kind::Daemon{0}else{1};p[2]=u8::from(e.enabled);p[3]=match e.restart{Restart::Never=>0,Restart::OnFailure=>1,Restart::Always=>2};p[4..8].copy_from_slice(&e.id.to_le_bytes());p[8..24].copy_from_slice(&e.owner);p[24]=e.name.length as u8;p[25]=e.path.length as u8;p[26]=state_code(e.state);p[27]=u8::from(e.manual_stop);p[28]=u8::from(e.last_exit.is_some());p[29]=u8::from(e.once_fired);p[32..80].copy_from_slice(&e.name.bytes);p[80..176].copy_from_slice(&e.path.bytes);let(tag,time)=match e.trigger{Trigger::Manual=>(0,0),Trigger::Boot{delay_seconds}=>(1,u64::from(delay_seconds)),Trigger::Interval{seconds}=>(2,u64::from(seconds)),Trigger::Once{unix_seconds}=>(3,unix_seconds),Trigger::Cron{schedule}=>{p[216..224].copy_from_slice(&schedule.minutes.to_le_bytes());p[224..228].copy_from_slice(&schedule.hours.to_le_bytes());p[228..232].copy_from_slice(&schedule.days.to_le_bytes());p[232..234].copy_from_slice(&schedule.months.to_le_bytes());p[234]=schedule.weekdays;p[235]=u8::from(schedule.dom_any)|u8::from(schedule.dow_any)<<1;(4,0)}};p[176]=tag;p[184..192].copy_from_slice(&time.to_le_bytes());p[192..200].copy_from_slice(&e.next_run.to_le_bytes());p[200..208].copy_from_slice(&e.last_exit.unwrap_or(0).to_le_bytes());p[208..216].copy_from_slice(&e.runs.to_le_bytes());}
        let checksum=checksum(&out[8..STATE_BYTES]);out[24..32].copy_from_slice(&checksum.to_le_bytes());Ok(STATE_BYTES)
    }
    // ------------------------=
    // FUNC: decode
    // DESC: Rejects corrupt or duplicate records and restores definitions without replaying transient process handles.
    // ------------------=
    pub fn decode(bytes:&[u8])->Result<Self,Error>{
        if bytes.len()!=STATE_BYTES||&bytes[..8]!=b"IAUTO001"||read64(bytes,24)!=checksum(&bytes[8..]){return Err(Error::Corrupt);}
        let mut out=Self::new();out.next_id=read32(bytes,8);out.revision=read64(bytes,16);if out.next_id==0{return Err(Error::Corrupt);}
        for index in 0..CAPACITY{let p=&bytes[32+index*256..32+(index+1)*256];if p[0]==0{continue;}if p[0]!=1||p[1]>1||p[2]>1||p[3]>2||p[24]>48||p[25]>96||p[26]>6||p[27]>1||p[28]>1||p[29]>1{return Err(Error::Corrupt);}
            let kind=if p[1]==0{Kind::Daemon}else{Kind::Schedule};let name=Text::new(&p[32..32+p[24]as usize])?;let path=Text::new(&p[80..80+p[25]as usize])?;validate_name(name.bytes())?;validate_path(path.bytes())?;
            let time=read64(p,184);let trigger=match p[176]{0=>Trigger::Manual,1 if time<=u32::MAX as u64=>Trigger::Boot{delay_seconds:time as u32},2 if time<=u32::MAX as u64=>Trigger::Interval{seconds:time as u32},3=>Trigger::Once{unix_seconds:time},4 if p[235]<=3=>Trigger::Cron{schedule:Cron{minutes:read64(p,216),hours:read32(p,224),days:read32(p,228),months:u16::from_le_bytes([p[232],p[233]]),weekdays:p[234],dom_any:p[235]&1!=0,dow_any:p[235]&2!=0}},_=>return Err(Error::Corrupt)};validate_trigger(kind,trigger)?;
            let id=read32(p,4);let mut owner=[0;16];owner.copy_from_slice(&p[8..24]);if owner==[0;16]||id==0||id>=out.next_id||out.get(id).is_some()||out.find(kind,name.bytes()).is_some(){return Err(Error::Corrupt);}
            let was_active=matches!(p[26],1|2|3|4);let restart=match p[3]{0=>Restart::Never,1=>Restart::OnFailure,_=>Restart::Always};if kind==Kind::Schedule&&restart!=Restart::Never{return Err(Error::Corrupt);}
            out.entries[index]=Some(Entry{id,name,path,owner,kind,enabled:p[2]!=0,trigger,restart,state:if was_active{State::Failed}else{match p[26]{5=>State::Completed,6=>State::Failed,_=>State::Stopped}},next_run:read64(p,192),last_exit:if was_active{Some(-130)}else if p[28]!=0{Some(read64(p,200)as i64)}else{None},runs:read64(p,208),process_id:0,restart_requested:false,boot_fired:false,manual_stop:false,once_fired:p[29]!=0,manual_queue:false});
        }Ok(out)
    }
}
// ------------------------=
// FUNC: initial_due
// DESC: Establishes a UTC due time; boot and manual triggers use separate admission rules.
// ------------------=
fn initial_due(trigger:Trigger,now:u64)->u64{match trigger{Trigger::Interval{seconds}=>now.saturating_add(u64::from(seconds)),Trigger::Once{unix_seconds}=>unix_seconds,Trigger::Cron{schedule}=>schedule.next_after(now).unwrap_or(u64::MAX),_=>now}}
// ------------------------=
// FUNC: validate_name
// DESC: Restricts names to unambiguous command tokens shared by both clients.
// ------------------=
fn validate_name(name:&[u8])->Result<(),Error>{if name.is_empty()||!name.iter().all(|c|c.is_ascii_alphanumeric()||b"-_.".contains(c)){Err(Error::Invalid)}else{Ok(())}}
// ------------------------=
// FUNC: validate_path
// DESC: Requires canonical absolute Python source paths without traversal or empty components.
// ------------------=
pub fn validate_path(path:&[u8])->Result<(),Error>{if !path.starts_with(b"/")||!path.ends_with(b".py")||path.len()>95||path.iter().any(|b|b.is_ascii_control())||path[1..].split(|b|*b==b'/').any(|p|p.is_empty()||p==b"."||p==b".."){Err(Error::Invalid)}else{Ok(())}}
// ------------------------=
// FUNC: validate_trigger
// DESC: Rejects zero periods and combinations that cannot be represented by the selected task type.
// ------------------=
fn validate_trigger(kind:Kind,trigger:Trigger)->Result<(),Error>{match(kind,trigger){(_,Trigger::Interval{seconds:0})|(_,Trigger::Once{unix_seconds:0})|(Kind::Daemon,Trigger::Interval{..}|Trigger::Once{..}|Trigger::Cron{..})=>Err(Error::Invalid),(_,Trigger::Cron{schedule})=>schedule.validate(),_=>Ok(())}}
// ------------------------=
// FUNC: state_code
// DESC: Maps lifecycle values to stable checkpoint discriminants.
// ------------------=
fn state_code(state:State)->u8{match state{State::Stopped=>0,State::Queued=>1,State::Running=>2,State::Stopping=>3,State::Killing=>4,State::Completed=>5,State::Failed=>6}}
// ------------------------=
// FUNC: checksum
// DESC: Detects damaged checkpoint contents while excluding the checksum field itself.
// ------------------=
fn checksum(bytes:&[u8])->u64{let mut sum=0xcbf29ce484222325u64;for(index,byte)in bytes.iter().enumerate(){if !(16..24).contains(&index){sum=(sum^u64::from(*byte)).wrapping_mul(0x100000001b3);}}sum}
// ------------------------=
// FUNC: read32
// DESC: Decodes a validated little-endian integer without structure casts.
// ------------------=
fn read32(bytes:&[u8],at:usize)->u32{u32::from_le_bytes(bytes[at..at+4].try_into().unwrap())}
// ------------------------=
// FUNC: read64
// DESC: Decodes a validated little-endian checkpoint counter.
// ------------------=
fn read64(bytes:&[u8],at:usize)->u64{u64::from_le_bytes(bytes[at..at+8].try_into().unwrap())}
// ------------------------=
// FUNC: number
// DESC: Parses decimal command values with checked overflow.
// ------------------=
fn number(bytes:&[u8])->Result<u64,Error>{if bytes.is_empty(){return Err(Error::Invalid);}bytes.iter().try_fold(0u64,|n,b|if b.is_ascii_digit(){n.checked_mul(10).and_then(|n|n.checked_add(u64::from(*b-b'0'))).ok_or(Error::Invalid)}else{Err(Error::Invalid)})}
// ------------------------=
// FUNC: days_in_month
// DESC: Supplies Gregorian month lengths for validated UTC calendar conversion.
// ------------------=
fn days_in_month(year:u64,month:u64)->u64{match month{4|6|9|11=>30,2=>if year%4==0&&(year%100!=0||year%400==0){29}else{28},_=>31}}
// ------------------------=
// FUNC: parse_utc
// DESC: Accepts explicit UTC dates or existing epoch seconds without local timezone ambiguity.
// ------------------=
pub fn parse_utc(input:&[u8])->Result<u64,Error>{
    if input.iter().all(u8::is_ascii_digit){return number(input);}
    let bytes=input.strip_suffix(b"Z").unwrap_or(input);
    if !matches!(bytes.len(),16|19)||bytes[4]!=b'-'||bytes[7]!=b'-'||!matches!(bytes[10],b'T'|b' ')||bytes[13]!=b':'||bytes.len()==19&&bytes[16]!=b':'{return Err(Error::Invalid);}
    let(year,month,day,hour,minute)=(number(&bytes[..4])?,number(&bytes[5..7])?,number(&bytes[8..10])?,number(&bytes[11..13])?,number(&bytes[14..16])?);
    let second=if bytes.len()==19{number(&bytes[17..19])?}else{0};
    if !(1970..=2199).contains(&year)||!(1..=12).contains(&month)||day==0||day>days_in_month(year,month)||hour>23||minute>59||second>59{return Err(Error::Invalid);}
    let mut days=0;for y in 1970..year{days+=365+u64::from(days_in_month(y,2)==29);}for m in 1..month{days+=days_in_month(year,m);}
    Ok((days+day-1)*86400+hour*3600+minute*60+second)
}
// ------------------------=
// FUNC: format_utc
// DESC: Formats bounded human-readable UTC dates while retaining epoch compatibility outside the supported calendar range.
// ------------------=
pub fn format_utc(seconds:u64)->Option<Text<32>>{
    if seconds>=7_258_118_400{return None;}
    let mut days=seconds/86400;let mut year=1970;while days>=365+u64::from(days_in_month(year,2)==29){days-=365+u64::from(days_in_month(year,2)==29);year+=1;}
    let mut month=1;while days>=days_in_month(year,month){days-=days_in_month(year,month);month+=1;}
    let mut bytes=*b"0000-00-00 00:00:00 UTC";
    for(start,width,mut value)in[(0,4,year),(5,2,month),(8,2,days+1),(11,2,seconds%86400/3600),(14,2,seconds%3600/60),(17,2,seconds%60)]{for index in (start..start+width).rev(){bytes[index]=b'0'+(value%10)as u8;value/=10;}}
    Text::new(&bytes).ok()
}
// ------------------------=
// FUNC: parse
// DESC: Parses one explicit service or schedule command with quoted paths and strict option validation.
// ------------------=
pub fn parse(input:&[u8])->Result<Command,Error>{
    let mut storage=[[0u8;320];16];let mut lengths=[0;16];let mut count=0;let mut at=0;
    while at<input.len(){
        while input.get(at).is_some_and(u8::is_ascii_whitespace){at+=1;}if at==input.len(){break;}if count==storage.len(){return Err(Error::Invalid);}
        let mut quote=None;
        while at<input.len(){let byte=input[at];if quote.is_none()&&byte.is_ascii_whitespace(){break;}at+=1;
            if quote==Some(byte){quote=None;continue;}if quote.is_none()&&matches!(byte,b'\''|b'"'){quote=Some(byte);continue;}
            if lengths[count]==storage[count].len(){return Err(Error::Invalid);}storage[count][lengths[count]]=byte;lengths[count]+=1;
        }
        if quote.is_some(){return Err(Error::Invalid);}count+=1;
    }
    let tokens:[&[u8];16]=core::array::from_fn(|i|&storage[i][..lengths[i]]);
    let kind=match tokens[0]{b"service"=>Kind::Daemon,b"taskscheduler"|b"schedule"=>Kind::Schedule,_=>return Err(Error::Invalid)};
    let action=match tokens[1]{b""|b"list"=>Action::List,b"inspect"|b"show"=>Action::Inspect,b"create"|b"add"=>Action::Create,b"edit"|b"update"=>Action::Edit,b"delete"|b"remove"=>Action::Delete,b"start"|b"run"=>Action::Start,b"stop"=>Action::Stop,b"kill"=>Action::Kill,b"restart"=>Action::Restart,b"enable"=>Action::Enable,b"disable"=>Action::Disable,b"logs"=>Action::Logs,_=>return Err(Error::Invalid)};
    if action==Action::List&&count>2||action!=Action::List&&count<3{return Err(Error::Invalid);}
    let name=Text::new(tokens[2])?;if action!=Action::List{validate_name(name.bytes())?;}
    let mut command=Command{kind,action,name,path:None,enabled:None,trigger:None,restart:None};let mut delay=None;let mut mode=None;
    let mut options=3;
    if kind==Kind::Schedule&&matches!(action,Action::Create|Action::Edit)&&count>3&&!tokens[3].starts_with(b"/")&&!tokens[3].contains(&b'='){
        if tokens[3].iter().any(u8::is_ascii_whitespace){command.trigger=Some(Trigger::Cron{schedule:parse_cron(tokens[3])?});options=4;}
        else{if count<8{return Err(Error::Invalid);}let mut expression=Text::empty();for token in &tokens[3..8]{if expression.length!=0{cron_append(&mut expression,b" ")?;}cron_append(&mut expression,token)?;}command.trigger=Some(Trigger::Cron{schedule:parse_cron(expression.bytes())?});options=8;}
    }
    for token in tokens.iter().take(count).skip(options){
        if !matches!(action,Action::Create|Action::Edit){return Err(Error::Invalid);}
        if token.starts_with(b"/"){if command.path.is_some(){return Err(Error::Invalid);}validate_path(token)?;command.path=Some(Text::new(token)?);continue;}
        let Some(eq)=token.iter().position(|b|*b==b'=') else{return Err(Error::Invalid);};let(key,value)=(&token[..eq],&token[eq+1..]);
        match key{
            b"enabled" if command.enabled.is_none()=>command.enabled=Some(match value{b"true"|b"yes"=>true,b"false"|b"no"=>false,_=>return Err(Error::Invalid)}),
            b"start" if mode.is_none()&&command.trigger.is_none()=>{mode=Some(match value{b"boot"=>true,b"manual"=>false,_=>return Err(Error::Invalid)});},
            b"delay" if delay.is_none()=>delay=Some(u32::try_from(number(value)?).map_err(|_|Error::Invalid)?),
            b"every" if command.trigger.is_none()&&mode.is_none()=>command.trigger=Some(Trigger::Interval{seconds:u32::try_from(number(value)?).map_err(|_|Error::Invalid)?}),
            b"cron" if command.trigger.is_none()&&mode.is_none()=>command.trigger=Some(Trigger::Cron{schedule:parse_cron(value)?}),
            b"at" if command.trigger.is_none()&&mode.is_none()=>command.trigger=Some(Trigger::Once{unix_seconds:parse_utc(value)?}),
            b"restart" if command.restart.is_none()=>command.restart=Some(match value{b"never"=>Restart::Never,b"on-failure"=>Restart::OnFailure,b"always"=>Restart::Always,_=>return Err(Error::Invalid)}),
            _=>return Err(Error::Invalid),
        }
    }
    if let Some(boot)=mode{if !boot&&delay.is_some(){return Err(Error::Invalid);}command.trigger=Some(if boot{Trigger::Boot{delay_seconds:delay.unwrap_or(0)}}else{Trigger::Manual});}else if delay.is_some(){return Err(Error::Invalid);}
    if action==Action::Create&&command.path.is_none(){return Err(Error::Invalid);}
    if let Some(trigger)=command.trigger{validate_trigger(kind,trigger)?;}if kind==Kind::Schedule&&command.restart.is_some_and(|r|r!=Restart::Never){return Err(Error::Invalid);}
    Ok(command)
}

impl Cron {
    // ------------------------=
    // FUNC: hourly
    // DESC: Supplies the canonical default cron trigger at minute zero of every hour in UTC.
    // ------------------=
    pub const fn hourly()->Self{Self{minutes:1,hours:0x00ff_ffff,days:0xffff_fffe,months:0x1ffe,weekdays:0x7f,dom_any:true,dow_any:true}}
    // ------------------------=
    // FUNC: validate
    // DESC: Rejects empty or out-of-domain masks, impossible calendar days and inconsistent wildcard flags.
    // ------------------=
    pub fn validate(&self)->Result<(),Error>{
        if self.minutes==0||self.minutes>>60!=0||self.hours==0||self.hours>>24!=0||self.days==0||self.days&1!=0||self.months==0||self.months&!0x1ffe!=0||self.weekdays==0||self.weekdays&!0x7f!=0||self.dom_any&&self.days&2==0||self.dow_any&&self.weekdays&1==0{return Err(Error::Invalid);}
        if !self.dom_any&&!self.dow_any{return Ok(());}
        if !(1..=12).any(|month|self.months&(1<<month)!=0&&(1..=days_in_month(2000,month)).any(|day|self.days&(1<<day)!=0)){return Err(Error::Invalid);}
        Ok(())
    }
    // ------------------------=
    // FUNC: matches_day
    // DESC: Applies traditional cron OR matching for two restricted day fields and AND matching when either starts with a wildcard.
    // ------------------=
    fn matches_day(&self,month:u64,day:u64,epoch_day:u64)->bool{
        let dom=self.days&(1<<day)!=0;let dow=self.weekdays&(1<<((epoch_day+4)%7))!=0;
        self.months&(1<<month)!=0&&if self.dom_any||self.dow_any{dom&&dow}else{dom||dow}
    }
    // ------------------------=
    // FUNC: matches_utc
    // DESC: Evaluates the five cron fields against a supported UTC timestamp without a local-time or DST dependency.
    // ------------------=
    pub fn matches_utc(&self,seconds:u64)->bool{
        let Some((_,month,day))=utc_date(seconds)else{return false;};
        self.validate().is_ok()&&self.minutes&(1<<(seconds/60%60))!=0&&self.hours&(1<<(seconds/3600%24))!=0&&self.matches_day(month,day,seconds/86400)
    }
    // ------------------------=
    // FUNC: next_after
    // DESC: Finds the next matching minute strictly after a timestamp with bounded day-level calendar traversal through year 2199.
    // ------------------=
    pub fn next_after(&self,after:u64)->Option<u64>{
        self.validate().ok()?;let first=after.checked_div(60)?.checked_add(1)?.checked_mul(60)?;
        let(mut year,mut month,mut day)=utc_date(first)?;let mut epoch_day=first/86400;
        while year<2200{
            if self.matches_day(month,day,epoch_day){
                for hour in 0..24{if self.hours&(1<<hour)==0{continue;}for minute in 0..60{if self.minutes&(1<<minute)==0{continue;}let candidate=epoch_day*86400+hour*3600+minute*60;if candidate>=first{return Some(candidate);}}}
            }
            epoch_day+=1;day+=1;if day>days_in_month(year,month){day=1;month+=1;if month>12{month=1;year+=1;}}
        }
        None
    }
}
// ------------------------=
// FUNC: utc_date
// DESC: Converts supported Unix seconds to a Gregorian calendar date for bounded cron matching.
// ------------------=
fn utc_date(seconds:u64)->Option<(u64,u64,u64)>{
    if seconds>=7_258_118_400{return None;}let mut days=seconds/86400;let mut year=1970;
    while days>=365+u64::from(days_in_month(year,2)==29){days-=365+u64::from(days_in_month(year,2)==29);year+=1;}
    let mut month=1;while days>=days_in_month(year,month){days-=days_in_month(year,month);month+=1;}Some((year,month,days+1))
}
// ------------------------=
// FUNC: cron_value
// DESC: Reads numeric cron values and standard three-letter month or weekday names case-insensitively.
// ------------------=
fn cron_value(bytes:&[u8],field:usize)->Result<u64,Error>{
    if bytes.len()==3{
        let names:&[&[u8]]=match field{3=>&[b"jan",b"feb",b"mar",b"apr",b"may",b"jun",b"jul",b"aug",b"sep",b"oct",b"nov",b"dec"],4=>&[b"sun",b"mon",b"tue",b"wed",b"thu",b"fri",b"sat"],_=>&[]};
        if let Some(index)=names.iter().position(|name|bytes.eq_ignore_ascii_case(name)){return Ok(index as u64+u64::from(field==3));}
    }
    number(bytes)
}
// ------------------------=
// FUNC: cron_field
// DESC: Compiles comma lists, inclusive ranges and wildcard or range steps into one bounded field mask.
// ------------------=
fn cron_field(bytes:&[u8],field:usize,min:u64,max:u64)->Result<u64,Error>{
    let mut mask=0;
    for item in bytes.split(|byte|*byte==b','){
        if item.is_empty(){return Err(Error::Invalid);}let mut parts=item.split(|byte|*byte==b'/');let range=parts.next().unwrap();let step=parts.next().map(number).transpose()?.unwrap_or(1);
        if step==0||parts.next().is_some(){return Err(Error::Invalid);}
        let(start,end)=if range==b"*"{(min,max)}else if let Some(dash)=range.iter().position(|byte|*byte==b'-'){(cron_value(&range[..dash],field)?,cron_value(&range[dash+1..],field)?)}else{if item.contains(&b'/'){return Err(Error::Invalid);}let value=cron_value(range,field)?;(value,value)};
        if start<min||end>max||start>end{return Err(Error::Invalid);}let mut value=start;
        loop{mask|=1<<if field==4&&value==7{0}else{value};if step>end-value{break;}value+=step;}
    }
    if mask==0{Err(Error::Invalid)}else{Ok(mask)}
}
// ------------------------=
// FUNC: parse_cron
// DESC: Parses exactly five traditional cron fields in UTC with Sunday aliases and preserved day wildcard semantics.
// ------------------=
 pub fn parse_cron(bytes:&[u8])->Result<Cron,Error>{
    if bytes.len()>256{return Err(Error::Invalid);}let mut fields=bytes.split(u8::is_ascii_whitespace).filter(|field|!field.is_empty());let mut text=[b"".as_slice();5];
    for field in &mut text{*field=fields.next().ok_or(Error::Invalid)?;}if fields.next().is_some(){return Err(Error::Invalid);}
    let schedule=Cron{minutes:cron_field(text[0],0,0,59)?,hours:cron_field(text[1],1,0,23)?as u32,days:cron_field(text[2],2,1,31)?as u32,months:cron_field(text[3],3,1,12)?as u16,weekdays:cron_field(text[4],4,0,7)?as u8,dom_any:text[2].starts_with(b"*"),dow_any:text[4].starts_with(b"*")};schedule.validate()?;Ok(schedule)
}
// ------------------------=
// FUNC: cron_append
// DESC: Appends canonical cron bytes with an explicit bound rather than truncating an editable expression.
// ------------------=
fn cron_append(out:&mut Text<256>,bytes:&[u8])->Result<(),Error>{if out.length+bytes.len()>256{return Err(Error::Full);}out.bytes[out.length..out.length+bytes.len()].copy_from_slice(bytes);out.length+=bytes.len();Ok(())}
// ------------------------=
// FUNC: cron_number
// DESC: Writes the at-most-two-digit cron domain values without allocation.
// ------------------=
fn cron_number(out:&mut Text<256>,value:u64)->Result<(),Error>{let digits=[b'0'+(value/10)as u8,b'0'+(value%10)as u8];cron_append(out,if value<10{&digits[1..]}else{&digits})}
// ------------------------=
// FUNC: cron_format_field
// DESC: Compresses mask runs while preserving whether a day field originally starts with a wildcard.
// ------------------=
fn cron_format_field(out:&mut Text<256>,mut mask:u64,min:u64,max:u64,wildcard:bool)->Result<(),Error>{
    let full=((1u64<<(max+1))-1)&!((1u64<<min)-1);
    if wildcard&&mask==full{return cron_append(out,b"*");}
    if wildcard{cron_append(out,b"*/")?;cron_number(out,max-min+1)?;mask&=!(1<<min);if mask==0{return Ok(());}cron_append(out,b",")?;}
    let mut first=true;let mut at=min;
    while at<=max{if mask&(1<<at)==0{at+=1;continue;}let start=at;while at<max&&mask&(1<<(at+1))!=0{at+=1;}if !first{cron_append(out,b",")?;}cron_number(out,start)?;if at>start{cron_append(out,b"-")?;cron_number(out,at)?;}first=false;at+=1;}
    Ok(())
}
// ------------------------=
// FUNC: format_cron
// DESC: Produces a lossless editable five-field expression including the day wildcard flags used for standard OR semantics.
// ------------------=
 pub fn format_cron(schedule:Cron)->Result<Text<256>,Error>{
    schedule.validate()?;let mut out=Text::empty();
    for(index,(mask,min,max,wildcard))in[(schedule.minutes,0,59,schedule.minutes==(1u64<<60)-1),(u64::from(schedule.hours),0,23,schedule.hours==0x00ff_ffff),(u64::from(schedule.days),1,31,schedule.dom_any),(u64::from(schedule.months),1,12,schedule.months==0x1ffe),(u64::from(schedule.weekdays),0,6,schedule.dow_any)].into_iter().enumerate(){if index>0{cron_append(&mut out,b" ")?;}cron_format_field(&mut out,mask,min,max,wildcard)?;}
    Ok(out)
}
