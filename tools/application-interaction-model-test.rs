//! Real pinned Hermes decision followed by an actual registry-mediated application state transition.
#[path="../kernel/runtime/mod.rs"] mod runtime;
#[path="storage.rs"] mod storage;
#[path="../kernel/ui/mod.rs"] mod ui;
mod console {
    #[derive(Clone,Copy)]
    pub enum ConsoleKey {Character(u8),Backspace,Delete,Left,Right,Home,End,Enter}
}
// ------------------------=
// FUNC: output_text
// DESC: Supplies the host diagnostic sink without treating diagnostic prose as test evidence.
// ------------------=
fn output_text(_: &[u8]) {}
use runtime::ai::{application,application_host,qwen,AiRuntime};
use application::*;
// ------------------------=
// FUNC: main
// DESC: Verifies the real shared inference service selects and executes a legal semantic move; this is host evidence, not guest proof.
// ------------------=
fn main() {
    source_preview();
    repair_boundaries();
    if std::env::args().any(|value|value=="--preview-only") {return;}
    let bytes=Box::leak(std::fs::read("model-cache/Hermes-3-Llama-3.2-3B.Q4_K_M.gguf").unwrap().into_boxed_slice());
    let arena=Box::leak(vec![0u128;1280*1024*1024/16].into_boxed_slice());
    let arena=unsafe{std::slice::from_raw_parts_mut(arena.as_mut_ptr().cast::<u8>(),arena.len()*16)};
    let mut ai=Box::new(AiRuntime::new());
    assert!(ai.load_hermes(bytes,arena));
    ai.bind_chat_owner([1;16]);
    let workers:Vec<_>=(0..4).map(|id|std::thread::spawn(move||unsafe{qwen::workers::run_host_worker(id)})).collect();
    let start=std::time::Instant::now();
    while qwen::workers::online()!=4 {assert!(start.elapsed().as_secs()<10);std::thread::yield_now();}
    if std::env::args().any(|value|value=="--repair") {
        compiler_repair_probe(&mut ai);
        unsafe {qwen::workers::stop_host_workers();}
        for worker in workers {worker.join().unwrap();}
        return;
    }
    if std::env::args().any(|value|value=="--generate") {
        generation_probe(&mut ai);
        unsafe {qwen::workers::stop_host_workers();}
        for worker in workers {worker.join().unwrap();}
        return;
    }
    prefill_progress(&mut ai);
    let id=application_host::attach([1;16],[2;16]).unwrap().instance;
    if std::env::args().any(|value|value=="--sensitive") {
        sensitive_action(&mut ai,id);
        application_host::detach();unsafe {qwen::workers::stop_host_workers();}
        for worker in workers {worker.join().unwrap();}
        return;
    }
    application_host::with_registry(|registry| {
    registry.begin(id,[1;16],[2;16],1,b"Tic tac toe. You are X. Row 1: X X empty. Row 2: O O empty. Row 3: empty empty empty. It is your turn. Completing row 1 wins.",true,false).unwrap();
    for square in [2usize,5,6,7,8] {
        let mut choice=Choice{id:square as u32+1,action:[0;48],label:[0;96],arguments:[square as i64,1,0,0],risk:Risk::Reversible};
        choice.action[..15].copy_from_slice(b"board.placeMark");
        let label=format!("Place X at row {}, column {}",square/3+1,square%3+1);
        choice.label[..label.len()].copy_from_slice(label.as_bytes());
        registry.offer(id,[1;16],[2;16],choice).unwrap();
    }
    registry.commit(id,[1;16],[2;16]).unwrap();
    });
    ai.start_application_agent([1;16],[2;16],b"Play against me. Choose the winning move.",true).unwrap();
    let started=std::time::Instant::now();
    let request=loop {
        assert!(started.elapsed().as_secs()<120,"model decision deadline");
        ai.poll_qwen();
        assert_agent_alive();
        if let Some(request)=application_host::with_registry(|r|r.poll(id,[1;16],[2;16],1)).unwrap(){break request;}
    };
    let mut board=[1,1,0,2,2,0,0,0,0];
    let square=request.choice.arguments[0] as usize;
    assert_eq!(board[square],0);board[square]=1;
    assert_eq!(&board[..3],&[1,1,1],"Hermes must choose the immediate win");
    application_host::with_registry(|registry| {
        registry.begin(id,[1;16],[2;16],2,b"X won",false,true).unwrap();
        registry.commit(id,[1;16],[2;16]).unwrap();
        let receipt=registry.finish(id,[1;16],[2;16],request.token,Outcome::Applied,3).unwrap();
        assert_eq!(receipt.outcome,Outcome::Applied);
        assert!(registry.observe(id,[1;16],[2;16]).unwrap().terminal);
    });
    ai.poll_qwen();ai.poll_qwen();
    assert!(matches!(application_host::observe([1;16]),Err(Error::Denied)));
    // A fresh decision can be interrupted and the SAME model immediately reused.
    application_host::with_registry(|r|{r.begin(id,[1;16],[2;16],3,b"Agent turn",true,false).unwrap();r.commit(id,[1;16],[2;16]).unwrap();});
    ai.start_application_agent([1;16],[2;16],b"Choose a move",true).unwrap();
    ai.poll_qwen();
    ai.stop_application_agent();
    assert!(matches!(application_host::observe([1;16]),Err(Error::Denied)));
    assert!(ai.submit_app_turn(1,b"Say hello.").is_ok());
    ai.cancel_app_turn();
    typed_document(&mut ai,id);
    sensitive_action(&mut ai,id);
    application_host::detach();
    unsafe {qwen::workers::stop_host_workers();}
    for worker in workers {worker.join().unwrap();}
}

// ------------------------=
// FUNC: repair_boundaries
// DESC: Verifies repair budgets, revision and request fencing, completion gating, and exact source preservation on oversized context.
// ------------------=
fn repair_boundaries() {
    use ui::app_assistant::{Panel,GenerationStatus,Action};
    let request=b"Create a simple graphical calculator app";
    let mut panel=Panel::new();
    panel.request[..request.len()].copy_from_slice(request);panel.request_len=request.len();
    panel.generation=GenerationStatus::Complete;
    let commented=b"// function documentation\n/* more documentation */\nint main(void){return 0;}";
    assert!(panel.stage_generated_source(commented));assert_eq!(&panel.argument[..panel.argument_len],commented);
    assert!(!panel.stage_generated_source(b"// incomplete comment"));
    assert!(!panel.stage_generated_source(b"/* incomplete comment"));
    assert!(!panel.stage_generated_source(b"Here is code:\nint main(void){return 0;}"));
    assert!(panel.accept_generated(commented,true,true));
    assert_eq!(&panel.argument[..panel.argument_len],commented);
    let initial=panel;
    assert!(!panel.queue_compiler_repair(request,4,5,b"diagnostic"));assert!(panel==initial);
    assert!(!panel.queue_compiler_repair(b"different request",4,4,b"diagnostic"));assert!(panel==initial);
    assert!(!panel.queue_compiler_repair(request,4,4,b""));assert!(panel==initial);
    for attempt in 1..=2 {
        assert!(panel.queue_compiler_repair(request,4,4,b"diagnostic"));
        assert_eq!(panel.repair_attempts,attempt);assert_eq!(panel.generation,GenerationStatus::Queued);
        assert_eq!(panel.pending,Action::None);assert!(!panel.apply_generated_compile_requested());
        let mut short=[0;16];assert_eq!(panel.compiler_repair_prompt(b"int main(void){return 0;}",&mut short),None);
        assert_eq!(short,[0;16]);
        assert!(!panel.queue_compiler_repair(request,4,4,b"diagnostic"));
        panel.generation=GenerationStatus::Complete;
    }
    assert!(!panel.queue_compiler_repair(request,4,4,b"diagnostic"));
    panel.generation=GenerationStatus::Cancelled;
    assert!(!panel.queue_compiler_repair(request,4,4,b"diagnostic"));
}

// ------------------------=
// FUNC: compiler_repair_probe
// DESC: Feeds a real Clang failure to Hermes through the production repair prompt, compiles its correction, and checks executable behavior rather than prose.
// ------------------=
fn compiler_repair_probe(ai:&mut AiRuntime) {
    use ui::app_assistant::{Panel,GenerationStatus};
    let compiler=std::env::var("CC").unwrap_or_else(|_|"/opt/homebrew/opt/llvm/bin/clang".to_owned());
    let path="build/application-repair-source.c";
    let executable="build/application-repair-executable";
    let source=b"// ------------------------=\n// FUNC: main\n// DESC: Returns the requested exit code.\n// ------------------=\nint main(void) { return 42 }\n";
    std::fs::write(path,source).unwrap();
    let failed=std::process::Command::new(&compiler).args(["--target=aarch64-none-elf","-nostdinc","-Isdk/c/include","-ffreestanding","-std=c17","-Wall","-Wextra","-Werror","-c",path,"-o","build/application-repair-aarch64.o"]).output().unwrap();
    assert!(!failed.status.success());
    let request=b"Create an exit status 42 command line app";
    let mut panel=Panel::new();panel.request[..request.len()].copy_from_slice(request);panel.request_len=request.len();
    panel.generation=GenerationStatus::Complete;
    assert!(panel.queue_compiler_repair(request,7,7,&failed.stderr));
    let mut prompt=[0;6144];let length=panel.compiler_repair_prompt(source,&mut prompt).unwrap();
    panel.generation=GenerationStatus::Running;ui::app_assistant::write(2,panel);
    ai.submit_app_turn(2,&prompt[..length]).unwrap();
    let started=std::time::Instant::now();
    let mut last=Vec::new();
    while ai.app_turn_owner().is_some() && started.elapsed().as_secs()<300 {ai.poll_qwen();if !ai.acceptance_native_output().is_empty(){last=ai.acceptance_native_output().to_vec();}}
    assert!(ai.app_turn_owner().is_none());
    std::fs::write("build/application-repair-model-output.txt",&last).unwrap();
    let panel=ui::app_assistant::read(2);assert_eq!(panel.generation,GenerationStatus::Complete,"{}",String::from_utf8_lossy(&panel.response[..panel.response_len]));
    assert!(panel.apply_generated_compile_requested());assert_eq!(panel.repair_attempts,1);
    std::fs::write(path,&panel.argument[..panel.argument_len]).unwrap();
    for target in ["aarch64-none-elf","x86_64-none-elf"] {
        let output=format!("build/application-repair-{target}.o");
        let compiled=std::process::Command::new(&compiler).args(["-target",target,"-nostdinc","-Isdk/c/include","-ffreestanding","-std=c17","-Wall","-Wextra","-Werror","-c",path,"-o",&output]).output().unwrap();
        assert!(compiled.status.success(),"{}",String::from_utf8_lossy(&compiled.stderr));
    }
    let compiled=std::process::Command::new(&compiler).args(["-nostdinc","-Isdk/c/include","-std=c17","-Wall","-Wextra","-Werror",path,"-o",executable]).output().unwrap();
    assert!(compiled.status.success(),"{}",String::from_utf8_lossy(&compiled.stderr));
    let result=std::process::Command::new(executable).output().unwrap();
    assert_eq!(result.status.code(),Some(42));assert!(result.stdout.is_empty());assert!(result.stderr.is_empty());
    assert!(ai.submit_app_turn(1,b"Say hello.").is_ok());ai.cancel_app_turn();
}

// ------------------------=
// FUNC: source_preview
// DESC: Verifies raw source streams only for explicit creation and never grants compile authority before completion.
// ------------------=
fn source_preview() {
    let request=b"create a simple graphical calculator app";
    let mut panel=ui::app_assistant::Panel::new();
    panel.request[..request.len()].copy_from_slice(request);panel.request_len=request.len();
    panel.generation=ui::app_assistant::GenerationStatus::Running;
    let first=b"#include <stdio.h>\nint main(void) {";
    assert!(panel.stage_generated_source(first));
    assert_eq!(&panel.argument[..panel.argument_len],first);
    assert_eq!(panel.pending,ui::app_assistant::Action::None);
    assert!(!panel.apply_generated_compile_requested());
    let complete=b"#include <stdio.h>\nint main(void) { return 0; }";
    assert!(panel.stage_generated_source(complete));
    assert_eq!(&panel.argument[..panel.argument_len],complete);
    assert!(!panel.accept_for_owner(2,complete,false));
    panel.request_len=0;
    assert!(!panel.stage_generated_source(first));
}

// ------------------------=
// FUNC: prefill_progress
// DESC: Proves real Hermes prompt work advances before output and cancellation releases the app lease.
// ------------------=
fn prefill_progress(ai:&mut AiRuntime) {
    ai.submit_app_turn(1,b"Read this context before answering: a document editor exposes typed insertion actions with an object reference, a position, and bounded UTF-8 text. Explain the purpose in one short sentence.").unwrap();
    let initial=ai.acceptance_native_progress();
    let started=std::time::Instant::now();
    loop {
        assert!(started.elapsed().as_secs()<30,"prefill work deadline");
        ai.poll_qwen();
        if ai.acceptance_native_progress()>initial {
            assert!(ai.acceptance_native_output().is_empty());
            assert_eq!(ai.app_turn_owner(),Some(1));
            break;
        }
    }
    ai.cancel_app_turn();
    assert_eq!(ai.app_turn_owner(),None);
}

// ------------------------=
// FUNC: generation_probe
// DESC: Exercises the actual editor prompt and records only this fixture's generated source for compiler diagnostics.
// ------------------=
fn generation_probe(ai:&mut AiRuntime) {
    let request=b"Build a simple UI C based calculator application";
    let mut panel=ui::app_assistant::Panel::new();
    panel.input[..request.len()].copy_from_slice(request);panel.length=request.len();
    panel.request[..request.len()].copy_from_slice(request);panel.request_len=request.len();
    panel.generation=ui::app_assistant::GenerationStatus::Running;
    let mut prompt=[0;6144];
    let length=panel.generation_prompt(true,b"",&mut prompt).unwrap();
    ui::app_assistant::write(2,panel);
    ai.submit_app_turn(2,&prompt[..length]).unwrap();
    let started=std::time::Instant::now();
    let mut last=Vec::new();
    while ai.app_turn_owner().is_some() && started.elapsed().as_secs()<300 {
        ai.poll_qwen();
        if !ai.acceptance_native_output().is_empty() {last=ai.acceptance_native_output().to_vec();}
    }
    let panel=ui::app_assistant::read(2);
    std::fs::write("build/application-generation-model-output.txt",&last).unwrap();
    std::fs::write("build/application-generation-model-source.c",&panel.argument[..panel.argument_len]).unwrap();
    assert!(ai.app_turn_owner().is_none(),"generation probe deadline");
    assert_eq!(panel.generation,ui::app_assistant::GenerationStatus::Complete);
    assert!(panel.argument_len>0);
}

// ------------------------=
// FUNC: sensitive_action
// DESC: Demonstrates Hermes cannot execute a sensitive document operation before exact trusted UI approval.
// ------------------=
fn sensitive_action(ai:&mut AiRuntime,id:u64) {
    application_host::with_registry(|r| {
        r.begin(id,[1;16],[2;16],6,b"Document contains HelloOK. Clear removes its contents and requires user review.",true,false).unwrap();
        let mut choice=Choice{id:1,action:[0;48],label:[0;96],arguments:[42,0,0,0],risk:Risk::Sensitive};
        choice.action[..14].copy_from_slice(b"document.clear");choice.label[..14].copy_from_slice(b"Clear document");
        r.offer(id,[1;16],[2;16],choice).unwrap();r.commit(id,[1;16],[2;16]).unwrap();
    });
    ai.start_application_agent([1;16],[2;16],b"Clear the document.",false).unwrap();
    let started=std::time::Instant::now();
    let token=loop {
        assert!(started.elapsed().as_secs()<120,"review decision deadline");
        ai.poll_qwen();
        assert_agent_alive();
        assert_eq!(application_host::with_registry(|r|r.poll(id,[1;16],[2;16],1)).unwrap(),None);
        let panel=ui::app_assistant::read(1);
        if panel.pending==ui::app_assistant::Action::NativeApproval {
            assert_eq!(panel.argument_len,8);break u64::from_le_bytes(panel.argument[..8].try_into().unwrap());
        }
    };
    assert_eq!(ai.approve_application_action([3;16],[2;16],token),Err(Error::Denied));
    assert_eq!(ai.approve_application_action([1;16],[2;16],token+1),Err(Error::Denied));
    ai.approve_application_action([1;16],[2;16],token).unwrap();
    assert_eq!(ai.approve_application_action([1;16],[2;16],token),Err(Error::Denied));
    let request=application_host::with_registry(|r|r.poll(id,[1;16],[2;16],1)).unwrap().unwrap();
    assert_eq!(request.token,token);
    let mut document=String::from("HelloOK");document.clear();assert!(document.is_empty());
    application_host::with_registry(|r| {
        r.begin(id,[1;16],[2;16],7,b"Document empty",false,false).unwrap();r.commit(id,[1;16],[2;16]).unwrap();
        r.finish(id,[1;16],[2;16],token,Outcome::Applied,3).unwrap();
    });
    ai.poll_qwen();
    assert!(matches!(application_host::observe([1;16]),Err(Error::Denied)));
}

// ------------------------=
// FUNC: typed_document
// DESC: Uses the real Hermes agent and AIKit discovery to apply a bounded arbitrary-text edit to a document fixture.
// ------------------=
fn typed_document(ai:&mut AiRuntime,id:u64) {
    application_host::with_registry(|r| {
        r.begin(id,[1;16],[2;16],4,b"Document 42 contains Hello. Length 5 bytes. Insert text at a zero-based byte position. The user requests an edit.",true,false).unwrap();
        let mut action=Choice{id:9,action:[0;48],label:[0;96],arguments:[42,5,0,0],risk:Risk::Reversible};
        action.action[..15].copy_from_slice(b"document.insert");action.label[..11].copy_from_slice(b"Insert text");
        let mut parameters=[Parameter{name:[0;24],kind:ParameterKind::Fixed,minimum:0,maximum:0};4];
        for (index,(name,kind,minimum,maximum)) in [
            (b"document".as_slice(),ParameterKind::Reference,42,42),
            (b"position",ParameterKind::Integer,0,5), (b"reservedA",ParameterKind::Fixed,0,0), (b"reservedB",ParameterKind::Fixed,0,0)
        ].iter().enumerate() {
            parameters[index].name[..name.len()].copy_from_slice(name);
            parameters[index].kind=*kind;parameters[index].minimum=*minimum;parameters[index].maximum=*maximum;
        }
        let mut text_name=[0;24];text_name[..7].copy_from_slice(b"content");
        r.offer_schema(id,[1;16],[2;16],action,Schema{choice:9,parameters,text_name,text_limit:32}).unwrap();
        r.commit(id,[1;16],[2;16]).unwrap();
    });
    ai.start_application_agent([1;16],[2;16],b"Append exactly the two characters OK to the document. Do not add spaces, quotes or newlines.",false).unwrap();
    let context=application_host::context().unwrap();
    let descriptor=runtime::ai::kit::CapabilityDirectory::application_descriptor(context,0).unwrap();
    assert_eq!(descriptor.instance,id);assert!(descriptor.available);assert_eq!(descriptor.schema.unwrap().text_limit,32);
    let started=std::time::Instant::now();
    let request=loop {
        assert!(started.elapsed().as_secs()<120,"typed decision deadline");
        ai.poll_qwen();
        assert_agent_alive();
        if let Some(request)=application_host::with_registry(|r|r.poll(id,[1;16],[2;16],1)).unwrap(){break request;}
    };
    assert!(request.typed);assert_eq!(request.choice.arguments,[42,5,0,0]);
    let mut document=String::from("Hello");
    document.insert_str(request.choice.arguments[1] as usize,core::str::from_utf8(&request.text[..request.text_length]).unwrap());
    assert_eq!(document,"HelloOK");
    application_host::with_registry(|r| {
        r.begin(id,[1;16],[2;16],5,document.as_bytes(),false,false).unwrap();r.commit(id,[1;16],[2;16]).unwrap();
        r.finish(id,[1;16],[2;16],request.token,Outcome::Applied,3).unwrap();
    });
    ai.poll_qwen();
    assert!(matches!(runtime::ai::kit::CapabilityDirectory::application_descriptor(context,0),Err(Error::Denied)));
}

// ------------------------=
// FUNC: assert_agent_alive
// DESC: Fails immediately on revoked authority; panel text is diagnostic only, never the acceptance oracle.
// ------------------=
fn assert_agent_alive() {
    let panel=ui::app_assistant::read(1);
    assert!(application_host::observe([1;16]).is_ok(),"agent stopped: {}",String::from_utf8_lossy(&panel.response[..panel.response_len]));
}
