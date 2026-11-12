#![allow(dead_code)]
#[path = "../kernel/runtime/mod.rs"] mod runtime;
#[path = "../kernel/ui/mod.rs"] mod ui;
#[path = "../kernel/storage/spatial_path.rs"] mod spatial_path;
mod storage;
#[path = "../kernel/storage/aikit_memory.rs"] mod aikit_memory;
use runtime::ai::persistent_memory::{self as memory,Memory,Record,Category,Error};
use std::{fs::{File, OpenOptions}, io::{Read, Seek, SeekFrom, Write}};
use storage::{BlockDevice, object::{ObjectStore, ObjectError, STORE_RELATIVE_LBA}};

// ------------------------=
// FUNC: output_text
// DESC: Supplies the runtime diagnostic sink without using prose as an acceptance oracle.
// ------------------=
fn output_text(_: &[u8]) {}

struct Disk(File, u64);
impl BlockDevice for Disk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Exposes the actual file-backed test disk geometry.
    // ------------------=
    fn block_count(&self) -> u64 { self.1 }
    // ------------------------=
    // FUNC: read_sector
    // DESC: Reads one bounded disk sector across independent mounts.
    // ------------------=
    fn read_sector(&mut self, lba: u64, out: &mut [u8; 512]) -> bool {
        lba < self.1 && self.0.seek(SeekFrom::Start(lba * 512)).is_ok() && self.0.read_exact(out).is_ok()
    }
    // ------------------------=
    // FUNC: write_sector
    // DESC: Writes one actual native Object Store sector.
    // ------------------=
    fn write_sector(&mut self, lba: u64, bytes: &[u8; 512]) -> bool {
        lba < self.1 && self.0.seek(SeekFrom::Start(lba * 512)).is_ok() && self.0.write_all(bytes).is_ok()
    }
    // ------------------------=
    // FUNC: flush
    // DESC: Requires durable disk synchronization before a test checkpoint is acknowledged.
    // ------------------=
    fn flush(&mut self) -> bool { self.0.sync_all().is_ok() }
}

// ------------------------=
// FUNC: main
// DESC: Exercises native durable owner checkpoints, cold remount, corrections, deletion, corruption and stable IDs.
// ------------------=
fn main() {
    assert!(runtime::ai::mapping::DEFAULT.len()<=storage::object::MAX_CONTENT);
    runtime::ai::mapping::validate(runtime::ai::mapping::DEFAULT).unwrap();
    personal_recall_route();
    browser_operations();
    real_recall();
    search_tests();
    api_tests();
    std::fs::create_dir_all("build/tests").unwrap();
    let path = format!("build/tests/aikit-memory-{}.img",std::process::id());
    let sectors=STORE_RELATIVE_LBA+32768;
    let file=OpenOptions::new().create_new(true).read(true).write(true).open(&path).unwrap();
    file.set_len(sectors*512).unwrap();
    let mut store=ObjectStore::format(Disk(file,sectors),0,sectors,[0x64;16]).unwrap();
    let owner=[0x19;16];let other=[0x23;16];let mut bytes=[0;memory::MAX_BYTES];
    let mut m=Memory::new(owner);
    let id=m.store(owner,Record::new(owner,Category::Semantic,b"My dog is Ollie",b"explicit user request",10).unwrap()).unwrap();
    assert_eq!(m.store(owner,Record::new(owner,Category::Semantic,b"MY DOG IS OLLIE",b"explicit user request",11).unwrap()),Ok(id));
    assert_eq!(m.inspect(other,id).err(),Some(Error::Unauthorized));
    let n=aikit_memory::checkpoint(&mut store,owner,Some(&m),&mut bytes).unwrap();
    for end in 0..n {assert!(Memory::decode(&bytes[..end],owner).is_err());}
    let mut broken=bytes;broken[45]^=0x55;assert_eq!(Memory::decode(&broken[..n],owner).err(),Some(Error::Corrupt));
    drop(store);
    interrupted_writes(&path,sectors,owner,id);
    let mut store=ObjectStore::mount(Disk(OpenOptions::new().read(true).write(true).open(&path).unwrap(),sectors),0).unwrap();
    let n=aikit_memory::checkpoint(&mut store,owner,None,&mut bytes).unwrap();
    let mut m=Memory::decode(&bytes[..n],owner).unwrap();assert_eq!(m.inspect(owner,id).unwrap().content(),b"My dog is Ollie");
    m.update(owner,id,Record::new(owner,Category::Semantic,b"My dog is Oliver",b"explicit correction",20).unwrap()).unwrap();
    assert_eq!(m.inspect(owner,id).unwrap().created_at,10);
    aikit_memory::checkpoint(&mut store,owner,Some(&m),&mut bytes).unwrap();
    assert!(aikit_memory::checkpoint(&mut store,other,Some(&m),&mut bytes).is_err());
    m.delete(owner,id).unwrap();aikit_memory::checkpoint(&mut store,owner,Some(&m),&mut bytes).unwrap();
    drop(store);
    let mut store=ObjectStore::mount(Disk(OpenOptions::new().read(true).write(true).open(&path).unwrap(),sectors),0).unwrap();
    let n=aikit_memory::checkpoint(&mut store,owner,None,&mut bytes).unwrap();
    let mut m=Memory::decode(&bytes[..n],owner).unwrap();assert_eq!(m.inspect(owner,id).err(),Some(Error::NotFound));
    let next=m.store(owner,Record::new(owner,Category::Procedural,b"Verified fix: rebuild the app",b"verified devkit outcome",30).unwrap()).unwrap();assert!(next>id);
    drop(store);std::fs::remove_file(path).unwrap();
    println!("AIKit native checkpoint persistence, update, deletion and isolation: PASS (cold remount, not OS reboot)");
}

// ------------------------=
// FUNC: search_tests
// DESC: Verifies exact vector ordering, lexical Unicode matching, limits, context budget, and provider failure.
// ------------------=
fn search_tests(){
    use runtime::ai::memory_search::{self as search,EmbeddingProvider};
    let owner=[7;16];let mut m=Memory::new(owner);
    let mut a=Record::new(owner,Category::Semantic,"Ollie likes CAFÉ walks".as_bytes(),b"user",1).unwrap();
    a.set_embedding(b"fixture-model",&[1.0,0.0],2).unwrap();let a=m.store(owner,a).unwrap();
    let mut b=Record::new(owner,Category::Procedural,b"Ollie walks every day",b"verified",2).unwrap();
    b.set_embedding(b"fixture-model",&[0.6,0.8],2).unwrap();let b=m.store(owner,b).unwrap();
    let mut foreign=Record::new(owner,Category::Semantic,b"Unrelated",b"user",2).unwrap();foreign.set_embedding(b"other-model",&[1.0,0.0],2).unwrap();m.store(owner,foreign).unwrap();
    let hits=search::semantic(&m,owner,b"fixture-model",&[1.0,0.0],2,0.0).unwrap();assert_eq!(hits.len,2);assert_eq!(hits.hits[0].id,a);assert_eq!(hits.hits[1].id,b);assert_eq!(hits.incompatible,1);assert!((hits.hits[1].score-0.6).abs()<0.00001);
    assert_eq!(search::semantic(&m,owner,b"fixture-model",&[1.0,0.0],8,0.9).unwrap().len,1);
    assert_eq!(search::semantic(&m,owner,b"fixture-model",&[1.0],8,0.0).unwrap().incompatible,3);
    assert!(search::cosine(&[1.0],&[1.0,0.0]).is_err());assert!(search::cosine(&[f32::NAN],&[1.0]).is_err());assert!(search::cosine(&[0.0],&[1.0]).is_err());
    assert!(search::semantic(&m,[8;16],b"fixture-model",&[1.0,0.0],2,0.0).is_err());
    let hits=search::lexical(&m,owner,"café OLLIE ollie".as_bytes(),1).unwrap();assert_eq!(hits.len,1);assert_eq!(hits.hits[0].id,a);assert_eq!(hits.hits[0].score,2.0);
    let mut out=[0;1024];let n=search::context(&m,owner,b"Ollie",3,200,&mut out).unwrap();assert!(n>0 && n<=200);assert_eq!(search::context(&m,owner,b"absent",3,200,&mut out),Ok(0));assert_eq!(search::context(&m,owner,b"Ollie",3,10,&mut out),Ok(0));
    let mut provider=search::Unavailable;assert_eq!(provider.model(),Err(Error::Unavailable));assert_eq!(provider.embed(b"anything",&mut[0.0;2]),Err(Error::Unavailable));
    m.delete(owner,a).unwrap();assert!(search::lexical(&m,owner,b"Ollie",10).unwrap().hits[..1].iter().all(|h|h.id!=a));
    println!("AIKit lexical, cosine, owner isolation and bounded context: PASS");
}

struct Backend {bytes:Vec<u8>,allowed:bool,fail:bool,reads:usize,writes:usize}
impl runtime::ai::memory_api::Backend for Backend {
    // ------------------------=
    // FUNC: authorize
    // DESC: Models a revoked or valid IdentitySystem session for deterministic native API tests.
    // ------------------=
    fn authorize(&mut self,owner:[u8;16],session:[u8;16])->bool{self.allowed && owner==[7;16] && session==[9;16]}
    // ------------------------=
    // FUNC: load
    // DESC: Supplies committed bytes independently of the engine cache.
    // ------------------=
    fn load(&mut self,_:[u8;16],_:[u8;16],out:&mut[u8])->Result<usize,Error>{self.reads+=1;out[..self.bytes.len()].copy_from_slice(&self.bytes);Ok(self.bytes.len())}
    // ------------------------=
    // FUNC: save
    // DESC: Injects failed durability without publishing candidate records.
    // ------------------=
    fn save(&mut self,_:[u8;16],_:[u8;16],m:&Memory)->Result<(),Error>{if self.fail{return Err(Error::Storage);}let mut b=[0;memory::MAX_BYTES];let n=m.encode(&mut b)?;self.bytes=b[..n].to_vec();self.writes+=1;Ok(())}
    // ------------------------=
    // FUNC: now
    // DESC: Supplies a deterministic clock for cache tests; real retrieval timing is measured separately.
    // ------------------=
    fn now(&self)->u64{0}
}
// ------------------------=
// FUNC: api_tests
// DESC: Verifies explicit requests, failure receipts, revocation, cache reuse, forgetting and model-independent reload.
// ------------------=
fn api_tests(){
    use runtime::ai::{memory_api::{Engine,Operation},memory_request::{self,Command},hermes_memory_request::Outcome};
    let owner=[7;16];let session=[9;16];let mut bytes=[0;memory::MAX_BYTES];let n=Memory::new(owner).encode(&mut bytes).unwrap();
    let mut backend=Backend{bytes:bytes[..n].to_vec(),allowed:true,fail:false,reads:0,writes:0};let mut engine=Engine::new();let mut out=[0;8192];
    let request=memory_request::natural(b"Infinity, remember that Ollie likes walks").unwrap();assert_eq!(request,Command::Remember(b"Ollie likes walks"));
    let receipt=memory_request::respond(&mut engine,&mut backend,owner,session,request,1,&mut out);assert_eq!(receipt.outcome,Outcome::Saved);
    let hits=engine.search(&mut backend,owner,session,b"Ollie",4).unwrap();assert_eq!(hits.len,1);let id=hits.hits[0].id;
    let before=backend.reads;for _ in 0..20{engine.context(&mut backend,owner,session,b"Ollie",256,&mut out).unwrap();}assert_eq!(backend.reads,before);
    let duplicate=memory_request::respond(&mut engine,&mut backend,owner,session,request,2,&mut out);assert_eq!(duplicate.outcome,Outcome::Saved);assert_eq!(backend.writes,1);
    let mut samples=Vec::new();
    for _ in 0..1000 {let start=std::time::Instant::now();engine.context(&mut backend,owner,session,b"Ollie",256,&mut out).unwrap();samples.push(start.elapsed().as_nanos());}
    samples.sort();println!("Host cached retrieval p50={}ns p95={}ns; not native-VM latency",samples[500],samples[950]);
    backend.allowed=false;assert_eq!(engine.search(&mut backend,owner,session,b"Ollie",4).err(),Some(Error::Unauthorized));backend.allowed=true;
    backend.fail=true;let receipt=memory_request::respond(&mut engine,&mut backend,owner,session,Command::Update(id,b"changed"),3,&mut out);assert_eq!(receipt.outcome,Outcome::Failed);backend.fail=false;
    assert_eq!(engine.inspect(&mut backend,owner,session,id).unwrap().content(),b"Ollie likes walks");
    assert!(engine.execute(&mut backend,owner,session,Operation::Store(Category::Semantic,b"my password is secret",b"user"),4).is_err());
    let mut switched_model_engine=Engine::new();assert_eq!(switched_model_engine.search(&mut backend,owner,session,b"Ollie",4).unwrap().len,1);
    let receipt=memory_request::respond(&mut engine,&mut backend,owner,session,Command::ForgetId(id),5,&mut out);assert_eq!(receipt.outcome,Outcome::Forgotten);
    let mut reloaded=Engine::new();assert_eq!(reloaded.search(&mut backend,owner,session,b"Ollie",4).unwrap().len,0);
    assert!(memory_request::natural(b"do not remember that secret").is_none());
    assert_eq!(memory_request::natural(b"remember that I do not eat meat"),Some(Command::Remember(b"I do not eat meat")));
    assert_eq!(memory_request::natural(b"remember that my nickname is \"Moon\""),Some(Command::Remember(b"my nickname is \"Moon\"")));
    let mut ai=Box::new(runtime::ai::AiRuntime::new());ai.bind_chat_owner(owner);
    assert_eq!(ai.route_application_request(2,b"remember that create a calculator app is just an example"),Ok(runtime::ai::kit::RequestRoute::Inference(runtime::ai::kit::InferenceMode::Conversation)));
    assert!(!ai.control_pending());

    use runtime::ai::{control,kit::{CapabilityDirectory,ExecutionBroker,ExecutionRequest,RequestOrigin,PolicyContext,PolicyDenial}};
    assert!(CapabilityDirectory::contains(b"OS_MEMORY:{argument}"));assert!(control::Command::decode(b"OS_MEMORY:remember Ollie likes walks").is_some());
    let command=control::Command::decode(b"OS_MEMORY:remember Ollie likes walks").unwrap();
    for origin in [RequestOrigin::Model,RequestOrigin::DesktopChat,RequestOrigin::Voice] {
        let mut broker=ExecutionBroker::new();
        broker.stage(ExecutionRequest{command,owner,origin,correlation_id:1}).unwrap();
        let result=broker.take(PolicyContext{requester:owner,owner,chat_enabled:true,desktop_allowed:true});
        if origin==RequestOrigin::Model {assert_eq!(result,Err(PolicyDenial::PermissionDenied));}
        else {assert_eq!(result.unwrap().unwrap().command,command);}
    }
    use runtime::ai::{memory_api::{candidate,Evidence},intelligence::software::{Execution,Event}};
    assert!(candidate(owner,b"I prefer Rust",Evidence::Generated,true,1).is_err());
    assert!(candidate(owner,b"I prefer Rust",Evidence::UserPreference,false,1).is_err());
    assert_eq!(candidate(owner,b"I prefer Rust",Evidence::UserPreference,true,1).unwrap().category,Category::Semantic);
    let mut e=Execution::new(1,owner);
    assert!(candidate(owner,b"calculator",Evidence::Development(&e),true,1).is_err());
    for event in [Event::SourceSaved{object:[1;16],revision:1},Event::Compiled{artifact:[2;16]},Event::Launched{context:1},Event::InteractionReady{context:1,revision:1}]{assert!(e.record(1,owner,event));}
    assert_eq!(candidate(owner,b"calculator",Evidence::Development(&e),true,1).unwrap().category,Category::Episodic);
    assert!(e.record(1,owner,Event::RuntimeFailed{source:[1;16],revision:1,context:1,code:1}));
    assert!(candidate(owner,b"calculator",Evidence::Development(&e),true,1).is_err());
    println!("AIKit memory API, cache, durable receipts, explicit operations and revocation: PASS");
}

struct FaultDisk { disk:Disk, remaining:usize }
impl BlockDevice for FaultDisk {
    // ------------------------=
    // FUNC: block_count
    // DESC: Preserves the native fixture geometry during interruption injection.
    // ------------------=
    fn block_count(&self)->u64{self.disk.block_count()}
    // ------------------------=
    // FUNC: read_sector
    // DESC: Allows recovery reads even after the simulated power loss.
    // ------------------=
    fn read_sector(&mut self,lba:u64,out:&mut[u8;512])->bool{self.disk.read_sector(lba,out)}
    // ------------------------=
    // FUNC: write_sector
    // DESC: Fails every subsequent write after the selected durable transaction boundary.
    // ------------------=
    fn write_sector(&mut self,lba:u64,data:&[u8;512])->bool{if self.remaining==0{return false;}self.remaining-=1;self.disk.write_sector(lba,data)}
    // ------------------------=
    // FUNC: flush
    // DESC: Injects failure at flush boundaries as well as sector writes.
    // ------------------=
    fn flush(&mut self)->bool{if self.remaining==0{return false;}self.remaining-=1;self.disk.flush()}
}
// ------------------------=
// FUNC: interrupted_writes
// DESC: Cold-remounts actual ObjectStore transactions interrupted at content, metadata and root commit stages.
// ------------------=
fn interrupted_writes(path:&str,sectors:u64,owner:[u8;16],id:u64){
    for boundary in [0,1,4,32,128,512,1024] {
        let copy=format!("{path}-fault");std::fs::copy(path,&copy).unwrap();
        let disk=Disk(OpenOptions::new().read(true).write(true).open(&copy).unwrap(),sectors);
        let mut store=ObjectStore::mount(FaultDisk{disk,remaining:boundary},0).unwrap();
        let mut bytes=[0;memory::MAX_BYTES];let n=aikit_memory::checkpoint(&mut store,owner,None,&mut bytes).unwrap();
        let mut m=Memory::decode(&bytes[..n],owner).unwrap();
        m.update(owner,id,Record::new(owner,Category::Semantic,b"Replacement",b"user",2).unwrap()).unwrap();
        let success=aikit_memory::checkpoint(&mut store,owner,Some(&m),&mut bytes).is_ok();drop(store);
        let mut store=ObjectStore::mount(Disk(OpenOptions::new().read(true).write(true).open(&copy).unwrap(),sectors),0).unwrap();
        let n=aikit_memory::checkpoint(&mut store,owner,None,&mut bytes).unwrap();let restored=Memory::decode(&bytes[..n],owner).unwrap();
        let text=restored.inspect(owner,id).unwrap().content();
        assert!(text==b"My dog is Ollie" || text==b"Replacement");if success{assert_eq!(text,b"Replacement");}
        drop(store);std::fs::remove_file(copy).unwrap();
    }
    println!("AIKit interrupted native checkpoint recovery: PASS");
}

// ------------------------=
// FUNC: personal_recall_route
// DESC: Exercises the actual voice and desktop router and verifies recall cannot authorize a model settings proposal.
// ------------------=
fn personal_recall_route() {
    use runtime::ai::{kit::*,mapping::ConfiguredMap};
    let mut map=ConfiguredMap::new();
    for origin in [RequestOrigin::Voice,RequestOrigin::DesktopChat] {
        for input in [b"What's the name of my cat?".as_slice(),b"What is my cat called",b"Tell me about my cat", "What’s my dog's name?".as_bytes()] {
            let request=RequestApi::new().accept(input,[7;16],origin);
            let route=IntentRouter::route(&mut map,&request);
            assert_eq!(route,RequestRoute::Inference(InferenceMode::Conversation));
            assert!(InferenceRequest::admit(&request,route).is_none());
        }
        let request=RequestApi::new().accept(b"Change my computer name to Shade",[7;16],origin);
        assert!(matches!(IntentRouter::route(&mut map,&request),RequestRoute::Control(_)));
    }
}

// ------------------------=
// FUNC: real_recall
// DESC: Exercises the shipped pretrained model, independent sessions and a cold Hermes conversation over the production submission boundary.
// ------------------=
fn real_recall() {
    use runtime::ai::{memory_api::{Engine,Operation}, memory_browser, qwen};
    use runtime::console_execution::Bytes;
    let owner=[7;16]; let session=[9;16];
    let mut serialized=[0;memory::MAX_BYTES];
    let n=Memory::new(owner).encode(&mut serialized).unwrap();
    let mut backend=Backend{bytes:serialized[..n].to_vec(),allowed:true,fail:false,reads:0,writes:0};
    let mut engine=Engine::new();
    for content in [b"Install the missing compiler dependency to fix the build".as_slice(),b"The orchard grows peaches and apples",b"The diagnostic project code is ORBIT-7429."] {
        engine.execute(&mut backend,owner,session,Operation::Store(Category::Semantic,content,b"test user"),1).unwrap();
    }
    let mut page=Bytes::new();
    memory_browser::request(&mut engine,&mut backend,owner,session,2,0,0,b"Compilation failed because a library was absent",2,&mut page).unwrap();
    assert_eq!(page.bytes().split(|b|*b==b'\t').next(),Some(b"1".as_slice()),"paraphrase ranking: {:?}",page.bytes());
    let mut literal=Bytes::new();
    memory_browser::request(&mut engine,&mut backend,owner,session,2,0,0,b"ORBIT",2,&mut literal).unwrap();
    assert_eq!(literal.bytes().split(|b|*b==b'\t').next(),Some(b"3".as_slice()));
    let stats=engine.execute(&mut backend,owner,session,Operation::Status,2).unwrap();
    assert_eq!(stats.embedded,3);
    let question=b"What is the diagnostic project code?";
    let mut context=[0x7f;256];
    backend.allowed=false;
    assert_eq!(engine.context(&mut backend,[8;16],session,question,256,&mut context),Err(Error::Unauthorized));
    assert_eq!(context,[0;256]);
    backend.allowed=true;
    let n=engine.context(&mut backend,owner,session,question,256,&mut context).unwrap();
    assert!(context[..n].windows(10).any(|s|s==b"ORBIT-7429"));
    assert!(!context[..n].windows(7).any(|s|s==b"orchard"));
    let mut samples=Vec::with_capacity(256);
    for _ in 0..256 {
        let started=std::time::Instant::now();
        assert_eq!(engine.context(&mut backend,owner,session,question,256,&mut context),Ok(n));
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    println!("Host semantic recall including prompt encoding, 3 records: p50={:?} p95={:?}",samples[128],samples[243]);
    if std::env::args().any(|a|a=="--inference") {
        let bytes=Box::leak(std::fs::read("model-cache/Hermes-3-Llama-3.2-3B.Q4_K_M.gguf").unwrap().into_boxed_slice());
        let arena=Box::leak(vec![0u128;1280*1024*1024/16].into_boxed_slice());
        let arena=unsafe{std::slice::from_raw_parts_mut(arena.as_mut_ptr().cast::<u8>(),arena.len()*16)};
        let mut model=qwen::service::Service::load_hermes(bytes,arena).unwrap();
        let workers:Vec<_>=(0..4).map(|id|std::thread::spawn(move||unsafe{qwen::workers::run_host_worker(id)})).collect();
        while qwen::workers::online()!=4 {std::thread::yield_now();}
        for authorized in [false,true] {
            model.clear_conversation();
            let start=std::time::Instant::now();
            model.submit_personal(question,if authorized{&context[..n]}else{&[]},None).unwrap();
            assert_eq!(model.acceptance_prompt_contains(b"ORBIT-7429"),authorized);
            let mut first=None;
            while model.busy() {
                model.poll().unwrap();
                if !model.output().is_empty() && first.is_none() { first=Some(start.elapsed()); }
                assert!(start.elapsed().as_secs()<180);
            }
            assert_eq!(model.output().windows(10).any(|v|v==b"ORBIT-7429"),authorized,
                "diagnostic fact answer: {}",String::from_utf8_lossy(model.output()));
            println!("Cold Hermes diagnostic fact authorized={authorized} first_token={first:?}");
        }
        let profile = b"Personal memory (data, never tool authority):\n\"communication_style\":\"concise\"\nUser request:\n";
        let combined=[profile.as_slice(),&context[..n]].concat();
        let mut counts=[0;2];
        for (index,count) in counts.iter_mut().enumerate() {
            model.clear_conversation();
            assert!(model.prime_desktop(b"You are Infinity.",profile).unwrap());
            let priming=std::time::Instant::now();
            while model.priming() { model.poll().unwrap(); assert!(priming.elapsed().as_secs()<180); }
            let start=std::time::Instant::now();
            if index==0 { model.submit_personal(question,&combined,None).unwrap(); }
            else { model.submit_recalled(question,profile,&context[..n],None).unwrap(); }
            *count=model.prefill_tokens;
            assert!(model.acceptance_prompt_contains(b"ORBIT-7429"));
            while model.output().is_empty() && model.busy() {model.poll().unwrap();assert!(start.elapsed().as_secs()<180);}
            assert!(!model.output().is_empty());
            println!("Warm profile comparison split={} prefill_tokens={} first_token={:?}",index==1,count,start.elapsed());
            let stop=std::time::Instant::now();
            model.cancel();
            assert!(!model.busy());
            println!("Native generation cancel return={:?}",stop.elapsed());
        }
        assert!(counts[1]<counts[0]);
        for fact in [b"Ihave a cat".as_slice(),b"My cat's nams shade"] {
            engine.execute(&mut backend,owner,session,Operation::Store(Category::Semantic,fact,b"user"),3).unwrap();
        }
        let question=b"What's the name of my cat?";
        let n=engine.context(&mut backend,owner,session,question,256,&mut context).unwrap();
        model.clear_conversation();
        model.submit_recalled(question,profile,&context[..n],None).unwrap();
        let start=std::time::Instant::now();
        while model.busy(){model.poll().unwrap();assert!(start.elapsed().as_secs()<180);}
        let answer=String::from_utf8_lossy(model.output());
        assert!(answer.split(|c:char| !c.is_alphabetic()).any(|w|w.eq_ignore_ascii_case("Shade")),"answer: {answer}");
        println!("Personal recall with original saved spelling: {answer}");
        unsafe{qwen::workers::stop_host_workers();}
        for worker in workers {worker.join().unwrap();}
    }
}

// ------------------------=
// FUNC: browser_operations
// DESC: Exercises the UI service and conversation against the same catalog, including filter pages, revocation and removal.
// ------------------=
fn browser_operations() {
    use runtime::ai::{memory_api::{Engine,Operation}, memory_request::{self,Command},memory_browser};
    use runtime::console_execution::Bytes;
    let (owner,session)=([7;16],[9;16]);
    let mut bytes=[0;memory::MAX_BYTES];
    let n=Memory::new(owner).encode(&mut bytes).unwrap();
    let mut backend=Backend{bytes:bytes[..n].to_vec(),allowed:true,fail:false,reads:0,writes:0};
    let mut engine=Engine::new();
    let mut output=Bytes::new();
    memory_browser::request(&mut engine,&mut backend,owner,session,4,0,3,b"Resolve compiler errors by installing the missing package",100,&mut output).unwrap();
    let id=std::str::from_utf8(output.bytes()).unwrap().parse::<u64>().unwrap();
    let saved=engine.inspect(&mut backend,owner,session,id).unwrap();
    assert_eq!((saved.created_at,saved.updated_at,saved.category,saved.source()),(100,100,Category::Procedural,b"Infinity Memory".as_slice()));
    let original=saved.embedding().to_vec();
    let mut duplicate=Bytes::new();
    memory_browser::request(&mut engine,&mut backend,owner,session,7,0,3,saved.content(),101,&mut duplicate).unwrap();
    let receipt:Vec<u64>=std::str::from_utf8(duplicate.bytes()).unwrap().split_whitespace().map(|v|v.parse().unwrap()).collect();
    assert_eq!(receipt,[id,0]);
    let mut ctx=[0;256];
    assert!(engine.context(&mut backend,owner,session,b"missing library breaks compilation",256,&mut ctx).unwrap()>0);
    let mut out=[0;8192];
    let r=memory_request::respond(&mut engine,&mut backend,owner,session,Command::Remember(b"Project architecture uses a distributed queue"),101,&mut out);
    assert_eq!(r.outcome,runtime::ai::hermes_memory_request::Outcome::Saved);
    let mut page=Bytes::new();
    memory_browser::request(&mut engine,&mut backend,owner,session,1,0,2,b"distributed",102,&mut page).unwrap();
    assert_eq!(page.bytes().split(|b|*b==b'\t').next(),Some(b"2".as_slice()));
    memory_browser::request(&mut engine,&mut backend,owner,session,5,id,1,b"The orchard grows pears",103,&mut Bytes::new()).unwrap();
    let updated=engine.inspect(&mut backend,owner,session,id).unwrap();
    assert_eq!((updated.created_at,updated.updated_at,updated.category),(100,103,Category::Episodic));
    assert_ne!(original,updated.embedding());
    assert_eq!(engine.context(&mut backend,owner,session,b"missing library breaks compilation",256,&mut ctx),Ok(0));
    let mut denied=Bytes::new();
    for action in 0..=7 {
        assert_eq!(memory_browser::request(&mut engine,&mut backend,[8;16],session,action,id,1,b"private",104,&mut denied),Err(Error::Unauthorized));
        assert!(denied.bytes().is_empty());
    }
    memory_browser::request(&mut engine,&mut backend,owner,session,6,id,0,b"",105,&mut Bytes::new()).unwrap();
    assert_eq!(engine.inspect(&mut backend,owner,session,id).err(),Some(Error::NotFound));
    assert_eq!(engine.context(&mut backend,owner,session,b"orchard pears",256,&mut ctx),Ok(0));
    let unicode=engine.execute(&mut backend,owner,session,Operation::Store(Category::Semantic,"Café Montréal".as_bytes(),b"test"),106).unwrap();
    let mut fallback=Bytes::new();
    memory_browser::request(&mut engine,&mut backend,owner,session,2,0,0,"Montréal".as_bytes(),106,&mut fallback).unwrap();
    let first_id=std::str::from_utf8(fallback.bytes().split(|b|*b==b'\t').next().unwrap()).unwrap().parse::<u64>().unwrap();
    assert_eq!(first_id,unicode.id);
    for i in 0..8 {
        engine.execute(&mut backend,owner,session,Operation::Store(Category::Semantic,format!("Notebook {i}").as_bytes(),b"test"),i).unwrap();
    }
    for (page_index,expected) in [(0,6),(1,2)] {
        let mut page=Bytes::new();
        memory_browser::request(&mut engine,&mut backend,owner,session,1,page_index,2,b"Notebook",106,&mut page).unwrap();
        assert_eq!(page.bytes().iter().filter(|b|**b==b'\n').count(),expected+1);
    }
}
