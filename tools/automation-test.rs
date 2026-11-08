#[path="../kernel/runtime/automation.rs"] mod automation;
use automation::*;
const OWNER:[u8;16]=[7;16];
// ------------------------=
// FUNC: apply
// DESC: Applies actual parsed commands through the production authority for state-based tests.
// ------------------=
fn apply(catalog:&mut Catalog,input:&[u8],now:u64)->Result<Option<u32>,Error>{catalog.apply(&parse(input)?,OWNER,true,now)}
// ------------------------=
// FUNC: create
// DESC: Creates one daemon using the same parser used by CLI and Task Manager clients.
// ------------------=
fn create(catalog:&mut Catalog)->u32{apply(catalog,b"service create nightly /home/default/documents/nightly.py start=boot delay=5 restart=on-failure",100).unwrap().unwrap()}
// ------------------------=
// FUNC: lifecycle
// DESC: Exercises creation, delayed start, restart backoff, explicit stop, kill, edit and deletion through observable states.
// ------------------=
fn lifecycle(){
    let mut c=Catalog::new();let id=create(&mut c);assert_eq!(c.count(Kind::Daemon),1);assert_eq!(c.due(104,4),None);assert_eq!(c.due(105,5).unwrap().id,id);
    c.started(id,10,105).unwrap();assert_eq!(c.due(106,6),None);assert_eq!(c.get(id).unwrap().process_id,10);assert_eq!(c.get(id).unwrap().runs,1);
    assert_eq!(apply(&mut c,b"service edit nightly enabled=false",106),Err(Error::Busy));
    c.finished(id,1,110).unwrap();assert_eq!(c.get(id).unwrap().state,State::Queued);assert_eq!(c.due(114,14),None);assert_eq!(c.due(115,15).unwrap().id,id);
    c.started(id,11,115).unwrap();apply(&mut c,b"service restart nightly",120).unwrap();assert_eq!(c.get(id).unwrap().state,State::Stopping);c.finished(id,-130,120).unwrap();assert_eq!(c.get(id).unwrap().state,State::Queued);
    c.started(id,12,120).unwrap();apply(&mut c,b"service stop nightly",121).unwrap();c.finished(id,-130,121).unwrap();assert_eq!(c.get(id).unwrap().state,State::Stopped);assert_eq!(c.due(200,200),None);
    apply(&mut c,b"service start nightly",200).unwrap();c.started(id,13,200).unwrap();apply(&mut c,b"service kill nightly",201).unwrap();assert_eq!(c.get(id).unwrap().state,State::Killing);assert_eq!(apply(&mut c,b"service delete nightly",201),Err(Error::Busy));c.finished(id,-9,201).unwrap();assert_eq!(c.due(999,999),None);
    apply(&mut c,b"service edit nightly /home/default/documents/updated.py start=manual restart=never enabled=false",202).unwrap();let e=c.get(id).unwrap();assert!(!e.enabled);assert_eq!(e.path.bytes(),b"/home/default/documents/updated.py");assert_eq!(e.trigger,Trigger::Manual);
    apply(&mut c,b"service delete nightly",203).unwrap();assert_eq!(c.get(id),None);
}
// ------------------------=
// FUNC: schedule
// DESC: Verifies interval coalescing, no overlap, one-shot completion, disabled execution and explicit manual runs.
// ------------------=
fn schedule(){
    let mut c=Catalog::new();let id=apply(&mut c,b"schedule create report /report.py every=60",100).unwrap().unwrap();assert_eq!(c.due(159,500),None);assert_eq!(c.due(160,500).unwrap().id,id);c.started(id,5,500).unwrap();assert_eq!(c.get(id).unwrap().next_run,560);assert_eq!(c.due(900,900),None);c.finished(id,0,900).unwrap();assert_eq!(c.get(id).unwrap().next_run,960);assert_eq!(c.due(959,959),None);
    apply(&mut c,b"schedule disable report",959).unwrap();assert_eq!(c.due(9999,9999),None);apply(&mut c,b"schedule run report",10000).unwrap();assert_eq!(c.due(10000,10000).unwrap().id,id);c.started(id,6,10000).unwrap();c.finished(id,0,10001).unwrap();assert_eq!(c.due(20000,20000),None);
    let once=apply(&mut c,b"schedule create once /once.py at=10010",10001).unwrap().unwrap();assert_eq!(c.due(10009,10009),None);c.started(once,7,10010).unwrap();c.finished(once,0,10011).unwrap();assert_eq!(c.due(20000,20000),None);
    let mut bytes=[0;STATE_BYTES];c.encode(&mut bytes).unwrap();let restored=Catalog::decode(&bytes).unwrap();assert_eq!(restored.due(20000,20000),None);assert_eq!(restored.get(once).unwrap().last_exit,Some(0));
}
// ------------------------=
// FUNC: persistence
// DESC: Simulates cold reboot with binary artifact roundtrips, interrupted processes and corruption rejection.
// ------------------=
fn persistence(){
    let mut c=Catalog::new();let id=create(&mut c);c.started(id,42,105).unwrap();let mut bytes=[0;STATE_BYTES];assert_eq!(c.encode(&mut bytes),Ok(STATE_BYTES));let restored=Catalog::decode(&bytes).unwrap();let e=restored.get(id).unwrap();assert_eq!(e.state,State::Failed);assert_eq!(e.process_id,0);assert_eq!(e.last_exit,Some(-130));assert_eq!(e.runs,1);assert_eq!(restored.due(999,4),None);assert_eq!(restored.due(999,5).unwrap().id,id);assert_eq!(e.owner,OWNER);
    bytes[100]^=1;assert_eq!(Catalog::decode(&bytes),Err(Error::Corrupt));assert_eq!(Catalog::decode(&bytes[..100]),Err(Error::Corrupt));
    let mut once=Catalog::new();let id=apply(&mut once,b"schedule create once /once.py at=123",100).unwrap().unwrap();once.started(id,43,123).unwrap();once.encode(&mut bytes).unwrap();let restored=Catalog::decode(&bytes).unwrap();assert_eq!(restored.due(999,999),None);assert_eq!(restored.get(id).unwrap().last_exit,Some(-130));
}
// ------------------------=
// FUNC: validation
// DESC: Rejects unauthorized, malformed, overflowing and capacity-exceeding operations without altering committed state.
// ------------------=
fn validation(){
    let mut c=Catalog::new();let command=parse(b"service create worker /worker.py").unwrap();assert_eq!(c.apply(&command,OWNER,false,1),Err(Error::Denied));assert_eq!(c.apply(&command,[0;16],true,1),Err(Error::Denied));assert_eq!(c.count(Kind::Daemon),0);
    for input in [b"service create a /../evil.py".as_slice(),b"schedule create a /a.py every=0",b"schedule create a /a.py at=0",b"service create a /a.py every=4",b"schedule create a /a.py restart=always",b"service create a /a.py start=manual delay=1",b"service create a /a.py enabled=true enabled=false",b"service delete a ignored",b"service create a '/missing.py",b"schedule create a /a.py every=18446744073709551616",b"service list unexpected"]{assert_eq!(parse(input),Err(Error::Invalid),"{input:?}");}
    let command=parse(b"schedule create quoted '/home/default/documents/with spaces.py' at=123").unwrap();assert_eq!(command.path.unwrap().bytes(),b"/home/default/documents/with spaces.py");
    for index in 0..CAPACITY{let text=format!("service create d{index} /d.py");apply(&mut c,text.as_bytes(),100).unwrap();}assert_eq!(apply(&mut c,b"service create overflow /o.py",100),Err(Error::Full));assert_eq!(apply(&mut c,b"service create d0 /d.py",100),Err(Error::Duplicate));
}
// ------------------------=
// FUNC: failed_admission
// DESC: Records failed source admission once and bounds automatic daemon retries.
// ------------------=
fn failed_admission(){let mut c=Catalog::new();let id=create(&mut c);c.finished(id,-13,100).unwrap();assert_eq!(c.get(id).unwrap().runs,1);assert_eq!(c.due(104,99),None);assert_eq!(c.due(105,99).unwrap().id,id);let once=apply(&mut c,b"schedule create missing /missing.py at=101",100).unwrap().unwrap();c.finished(once,-2,101).unwrap();assert_eq!(c.get(once).unwrap().state,State::Failed);assert_eq!(c.get(once).unwrap().runs,1);}
// ------------------------=
// FUNC: main
// DESC: Runs native catalog behavioral acceptance with no source-text or display-string oracles.
// ------------------=
fn main(){lifecycle();schedule();persistence();validation();failed_admission();fair_admission();reschedule_once();disable_retry();definition_guards();eligibility();utc_grammar();}

// ------------------------=
// FUNC: fair_admission
// DESC: Verifies admission rotates past a repeatedly failing daemon and handles reused catalog slots.
// ------------------=
fn fair_admission(){let mut c=Catalog::new();let a=apply(&mut c,b"service create first /first.py start=boot restart=always",1).unwrap().unwrap();let b=apply(&mut c,b"service create second /second.py start=boot",1).unwrap().unwrap();c.finished(a,-2,1).unwrap();assert_eq!(c.due_after(99,99,a).unwrap().id,b);apply(&mut c,b"service delete first",2).unwrap();let d=apply(&mut c,b"service create third /third.py start=boot",2).unwrap().unwrap();assert_eq!(c.due_after(99,99,d).unwrap().id,b);assert_eq!(c.due_after(99,99,b).unwrap().id,d);}

// ------------------------=
// FUNC: reschedule_once
// DESC: Rearms an edited one-time trigger while preserving lifetime run counts and durable no-replay state.
// ------------------=
fn reschedule_once(){let mut c=Catalog::new();let id=apply(&mut c,b"schedule create once /once.py at=10",1).unwrap().unwrap();c.started(id,1,10).unwrap();c.finished(id,0,11).unwrap();assert_eq!(c.due(99,99),None);apply(&mut c,b"schedule edit once at=120",100).unwrap();assert_eq!(c.due(119,119),None);assert_eq!(c.due(120,120).unwrap().id,id);assert_eq!(c.get(id).unwrap().runs,1);let mut bytes=[0;STATE_BYTES];c.encode(&mut bytes).unwrap();let mut c=Catalog::decode(&bytes).unwrap();assert_eq!(c.due(120,120).unwrap().id,id);c.started(id,2,120).unwrap();c.finished(id,0,121).unwrap();assert_eq!(c.get(id).unwrap().runs,2);c.encode(&mut bytes).unwrap();assert_eq!(Catalog::decode(&bytes).unwrap().due(999,999),None);}

// ------------------------=
// FUNC: disable_retry
// DESC: Prevents disabled automatic retries while preserving explicit manual execution of a disabled daemon.
// ------------------=
fn disable_retry(){let mut c=Catalog::new();let id=create(&mut c);c.started(id,1,105).unwrap();c.finished(id,1,106).unwrap();apply(&mut c,b"service disable nightly",107).unwrap();assert_eq!(c.due(999,999),None);assert_eq!(c.get(id).unwrap().state,State::Stopped);apply(&mut c,b"service start nightly",109).unwrap();assert_eq!(c.due(109,109).unwrap().id,id);c.started(id,2,109).unwrap();c.finished(id,1,110).unwrap();assert_eq!(c.due(999,999),None);}

// ------------------------=
// FUNC: definition_guards
// DESC: Allows lifecycle and unrelated changes while rejecting edited or replaced targets without modifying state.
// ------------------=
fn definition_guards(){
    let mut c=Catalog::new();let id=create(&mut c);let expected=c.get(id).unwrap().definition();
    apply(&mut c,b"service create other /other.py",100).unwrap();c.started(id,50,105).unwrap();c.finished(id,0,106).unwrap();
    let edit=parse(b"service edit nightly /new.py").unwrap();assert_eq!(c.apply_expected(&edit,OWNER,true,110,Some(expected)),Ok(Some(id)));
    let before=c;assert_eq!(c.apply_expected(&edit,OWNER,true,110,Some(expected)),Err(Error::Conflict));assert_eq!(c,before);
    let original=c.get(id).unwrap().definition();apply(&mut c,b"service delete nightly",111).unwrap();let replacement=create(&mut c);assert_ne!(replacement,id);
    let before=c;assert_eq!(c.apply_expected(&parse(b"service delete nightly").unwrap(),OWNER,true,112,Some(original)),Err(Error::Conflict));assert_eq!(c,before);
}

// ------------------------=
// FUNC: eligibility
// DESC: Checks displayed next-run classifications against real execution and durable one-shot transitions.
// ------------------=
fn eligibility(){
    let mut c=Catalog::new();let id=create(&mut c);assert_eq!(c.get(id).unwrap().next_status(Some(101),1),NextRun::BootDelay{seconds:4});assert_eq!(c.get(id).unwrap().next_status(None,5),NextRun::Ready);assert_eq!(c.due(0,5).unwrap().id,id);
    assert_eq!(c.get(id).unwrap().next_status(Some(105),5),NextRun::Ready);c.started(id,4,105).unwrap();assert_eq!(c.get(id).unwrap().next_status(Some(105),5),NextRun::Running);
    c.finished(id,0,106).unwrap();assert_eq!(c.get(id).unwrap().next_status(Some(999),999),NextRun::NextBoot);
    apply(&mut c,b"service stop nightly",107).unwrap();assert_eq!(c.get(id).unwrap().next_status(Some(999),999),NextRun::Paused);
    apply(&mut c,b"service disable nightly",108).unwrap();assert_eq!(c.get(id).unwrap().next_status(Some(999),999),NextRun::Disabled);
    apply(&mut c,b"service start nightly",109).unwrap();assert_eq!(c.get(id).unwrap().next_status(Some(109),109),NextRun::Ready);
    c.started(id,5,109).unwrap();apply(&mut c,b"service kill nightly",110).unwrap();assert_eq!(c.get(id).unwrap().next_status(Some(110),110),NextRun::Stopping);
    let id=apply(&mut c,b"schedule create once /once.py at=120",111).unwrap().unwrap();assert_eq!(c.get(id).unwrap().next_status(Some(111),111),NextRun::Scheduled{unix_seconds:120});c.started(id,6,120).unwrap();c.finished(id,0,121).unwrap();assert_eq!(c.get(id).unwrap().next_status(Some(999),999),NextRun::Finished);
    let id=apply(&mut c,b"service create manual /m.py",122).unwrap().unwrap();assert_eq!(c.get(id).unwrap().next_status(None,122),NextRun::Manual);
}

// ------------------------=
// FUNC: utc_grammar
// DESC: Verifies calendar parsing, quoted option values, whitespace and date roundtrips as structured values.
// ------------------=
fn utc_grammar(){
    let command=parse(b" \tschedule\tcreate once '/path with spaces.py' at=\"2028-02-29 23:59:59\"\n").unwrap();assert_eq!(command.trigger,Some(Trigger::Once{unix_seconds:1835481599}));assert_eq!(command.path.unwrap().bytes(),b"/path with spaces.py");
    for value in [b"2028-02-29T23:59:59Z".as_slice(),b"2028-02-29 23:59:59",b"1835481599"]{assert_eq!(parse_utc(value),Ok(1835481599));}
    for bad in [b"2100-02-29 00:00".as_slice(),b"2028-02-30 00:00",b"2026-11-09 24:00",b"2026-11-09 00:60",b"2026-11-09 00:00:60",b"2200-01-01 00:00",b"2026-13-01 00:00",b"2026-11-00 00:00"]{assert_eq!(parse_utc(bad),Err(Error::Invalid));}
    for seconds in [0,86400,951782400,1835481599,7258118399]{let formatted=format_utc(seconds).unwrap();assert_eq!(parse_utc(&formatted.bytes()[..19]),Ok(seconds));}assert_eq!(format_utc(7258118400),None);
    assert_eq!(Access::evaluate(true,false,true).reason,Some(Error::SignInRequired));assert!(!Access::evaluate(true,false,true).can_manage);assert!(Access::evaluate(true,true,false).can_read);assert!(!Access::evaluate(true,true,false).can_manage);assert!(Access::evaluate(true,true,true).can_manage);assert_eq!(Access::evaluate(false,true,true).reason,Some(Error::Storage));
}
