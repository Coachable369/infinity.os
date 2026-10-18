//! Real pinned Hermes decision followed by an actual registry-mediated application state transition.
#[path="../kernel/runtime/mod.rs"] mod runtime;
#[path="storage.rs"] mod storage;
#[path="../kernel/ui/mod.rs"] mod ui;
#[path="application-generation-probe.rs"] mod generation_probe;
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
    if std::env::args().any(|value|value=="--export-generation-prompt") {
        generation_probe::export_generation_prompt().unwrap();
        return;
    }
    source_preview();
    repair_boundaries();
    generation_probe::repair_api_boundaries();
    repair_diagnostic_priority();
    source_reserve_boundaries();
    if std::env::args().any(|value|value=="--preview-only") {return;}
    let arguments:Vec<_>=std::env::args().collect();
    let selected=arguments.iter().position(|value|value=="--model")
        .map(|index|arguments.get(index+1).expect("--model requires hermes or ministral").as_str()).unwrap_or("hermes");
    assert!(matches!(selected,"hermes"|"ministral"),"--model requires hermes or ministral");
    assert!(selected=="hermes" || arguments.iter().any(|value|value=="--generate"),"Alternate model comparison currently requires --generate");
    let path=if selected=="ministral" {"model-cache/Ministral-3-3B-Instruct-2512-Q4_K_M.gguf"}
        else {"model-cache/Hermes-3-Llama-3.2-3B.Q4_K_M.gguf"};
    let bytes=Box::leak(std::fs::read(path).unwrap().into_boxed_slice());
    let arena=Box::leak(vec![0u128;1280*1024*1024/16].into_boxed_slice());
    let arena=unsafe{std::slice::from_raw_parts_mut(arena.as_mut_ptr().cast::<u8>(),arena.len()*16)};
    let mut ai=Box::new(AiRuntime::new());
    let model=if selected=="ministral" {
        assert!(ai.load_ministral(bytes,arena));runtime::ai::chat::MINISTRAL_MODEL_ID
    } else {assert!(ai.load_hermes(bytes,arena));runtime::ai::chat::HERMES_MODEL_ID};
    ai.bind_chat_owner([1;16]);
    let index=runtime::ai::chat::CHAT_MODELS.iter().position(|entry|entry.id==model).unwrap();
    assert!(ai.chat.select_model_index(index));
    source_reserve_releases_lease(&mut ai);
    if arguments.iter().any(|value|value=="--admission-only") {return;}
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
        let result=generation_probe::run(&mut ai);
        unsafe {qwen::workers::stop_host_workers();}
        for worker in workers {worker.join().unwrap();}
        result.unwrap();
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
// FUNC: source_reserve_boundaries
// DESC: Checks typed application-source admission at exact context limits without changing ordinary conversational admission.
// ------------------=
fn source_reserve_boundaries() {
    use qwen::service::{submission_fits,SubmissionKind};
    for count in [0,1,1024,2559] {assert!(submission_fits(count,SubmissionKind::ApplicationSource));}
    for count in [2560,4095,4096,usize::MAX] {assert!(!submission_fits(count,SubmissionKind::ApplicationSource));}
    assert!(submission_fits(4094,SubmissionKind::Conversation));
    assert!(!submission_fits(4096,SubmissionKind::Conversation));
}

// ------------------------=
// FUNC: source_reserve_releases_lease
// DESC: Uses the actual model tokenizer to reject an oversized source turn and proves immediate reuse of the same app inference lease.
// ------------------=
fn source_reserve_releases_lease(ai:&mut AiRuntime) {
    ui::app_assistant::reset();
    assert!(ai.submit_app_source(2,&b"a ".repeat(3000)).is_err());
    assert_eq!(ai.app_turn_owner(),None);
    assert!(ai.submit_app_turn(1,b"Say hello.").is_ok());
    assert_eq!(ai.app_turn_owner(),Some(1));
    ai.cancel_app_turn();assert_eq!(ai.app_turn_owner(),None);
    let request=b"Create a calendar app";
    let mut panel=ui::app_assistant::Panel::new();
    panel.input[..request.len()].copy_from_slice(request);panel.length=request.len();
    panel.request[..request.len()].copy_from_slice(request);panel.request_len=request.len();
    let mut prompt=[0u8;6144];
    let length=panel.generation_prompt(true,b"",&mut prompt).unwrap();
    ui::app_assistant::write(2,panel);
    assert!(ai.submit_app_source(2,&prompt[..length]).is_ok());
    assert_eq!(ai.app_turn_owner(),Some(2));
    ai.cancel_app_turn();assert_eq!(ai.app_turn_owner(),None);
    ui::app_assistant::reset();
}

// ------------------------=
// FUNC: repair_diagnostic_priority
// DESC: Verifies warning-heavy compiler output cannot evict error records from bounded repair feedback and preserves unknown diagnostic bytes.
// ------------------=
fn repair_diagnostic_priority() {
    use ui::app_assistant::{compiler_diagnostics,Panel,GenerationStatus};
    let mut diagnostics=Vec::new();
    for _ in 0..5 {diagnostics.extend_from_slice(b"source.c:24:3: warning: ");diagnostics.extend_from_slice(&[b'w';300]);diagnostics.push(b'\n');}
    diagnostics.extend_from_slice(b"source.c:42:3: error: invalid argument count\n header.h:84:1: note: declared here\nsource.c:75:4: error: invalid operand types\n");
    let original=diagnostics.clone();
    let mut buffer=[0u8;1024];let selected=compiler_diagnostics::prioritize(&diagnostics,&mut buffer);
    assert_eq!(selected.errors,2);assert_eq!(selected.notes,1);assert!(selected.truncated);
    assert_eq!(diagnostics,original);assert!(selected.bytes<=buffer.len());
    let replay=compiler_diagnostics::prioritize(&buffer[..selected.bytes],&mut [0u8;1024]);
    assert_eq!(replay.errors,2);assert_eq!(replay.notes,1);
    let request=b"create a graphical calculator app";
    let mut panel=Panel::new();panel.request[..request.len()].copy_from_slice(request);panel.request_len=request.len();
    panel.generation=GenerationStatus::Complete;
    assert!(panel.queue_compiler_repair(request,7,7,&diagnostics));
    let report=compiler_diagnostics::prioritize(&panel.repair_diagnostics[..panel.repair_diagnostics_len],&mut buffer);
    assert_eq!(report.errors,2);assert_eq!(report.notes,1);
    let mut raw=[0;3];let unknown=compiler_diagnostics::prioritize(&[9,8,7,6],&mut raw);
    assert_eq!(raw,[9,8,7]);assert_eq!(unknown.bytes,3);assert!(unknown.truncated);
    assert_eq!(unknown.errors,0);
    let mut guard=[0xa5;4];let empty=compiler_diagnostics::prioritize(&diagnostics,&mut guard[..0]);
    assert_eq!(guard,[0xa5;4]);assert_eq!(empty.bytes,0);assert!(empty.truncated);
    let fatal=compiler_diagnostics::prioritize(b"ld.lld: error: unresolved symbol\nfatal error: invalid source\n",&mut buffer);
    assert_eq!(fatal.errors,2);
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
// FUNC: repair_compiler
// DESC: Keeps SDK headers isolated from host libc while explicitly retaining the selected compiler's builtin C headers.
// ------------------=
fn repair_compiler(compiler:&str, builtin_include:&std::path::Path)->std::process::Command {
    let mut command=std::process::Command::new(compiler);
    command.args(["-nostdinc","-Isdk/c/include","-isystem"]).arg(builtin_include);
    command
}

// ------------------------=
// FUNC: compiler_repair_probe
// DESC: Feeds a real Clang failure to Hermes through the production repair prompt, compiles its correction, and checks executable behavior rather than prose.
// ------------------=
fn compiler_repair_probe(ai:&mut AiRuntime) {
    use ui::app_assistant::{Panel,GenerationStatus};
    let compiler=std::env::var("CC").unwrap_or_else(|_|"/opt/homebrew/opt/llvm/bin/clang".to_owned());
    let resource=std::process::Command::new(&compiler).arg("-print-resource-dir").output().unwrap();
    assert!(resource.status.success());
    let builtin_include=std::path::PathBuf::from(std::str::from_utf8(&resource.stdout).unwrap().trim()).join("include");
    assert!(builtin_include.join("stddef.h").is_file());
    let path="build/application-repair-source.c";
    let executable="build/application-repair-executable";
    // Prove the toolchain can compile SDK and builtin headers before interpreting
    // a failing compile as a source error suitable for model repair.
    std::fs::write(path,b"#include <stdio.h>\n#include <stddef.h>\n#include <stdint.h>\n_Static_assert(sizeof(size_t)==8, \"64-bit target\");\nint main(void){return 42;}\n").unwrap();
    for target in ["aarch64-none-elf","x86_64-none-elf"] {
        let preflight=repair_compiler(&compiler,&builtin_include).args(["-target",target,"-ffreestanding","-std=c17","-Werror","-c",path,"-o","build/application-repair-preflight.o"]).output().unwrap();
        assert!(preflight.status.success(),"{}",String::from_utf8_lossy(&preflight.stderr));
    }
    let source=b"// ------------------------=\n// FUNC: main\n// DESC: Returns the requested exit code.\n// ------------------=\nint main(void) { return 42 }\n";
    std::fs::write(path,source).unwrap();
    let failed=repair_compiler(&compiler,&builtin_include).args(["--target=aarch64-none-elf","-ffreestanding","-std=c17","-Wall","-Wextra","-Werror","-c",path,"-o","build/application-repair-aarch64.o"]).output().unwrap();
    assert!(!failed.status.success());
    let request=b"Create an exit status 42 command line app";
    let mut panel=Panel::new();panel.request[..request.len()].copy_from_slice(request);panel.request_len=request.len();
    panel.generation=GenerationStatus::Complete;
    assert!(panel.queue_compiler_repair(request,7,7,&failed.stderr));
    let mut prompt=[0;6144];let length=panel.compiler_repair_prompt(source,&mut prompt).unwrap();
    panel.generation=GenerationStatus::Running;ui::app_assistant::write(2,panel);
    ai.submit_app_source(2,&prompt[..length]).unwrap();
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
        let compiled=repair_compiler(&compiler,&builtin_include).args(["-target",target,"-ffreestanding","-std=c17","-Wall","-Wextra","-Werror","-c",path,"-o",&output]).output().unwrap();
        assert!(compiled.status.success(),"{}",String::from_utf8_lossy(&compiled.stderr));
    }
    let compiled=repair_compiler(&compiler,&builtin_include).args(["-std=c17","-Wall","-Wextra","-Werror",path,"-o",executable]).output().unwrap();
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
    buffered_source_preview();
}

// ------------------------=
// FUNC: buffered_source_preview
// DESC: Drives real token-fragment framing into paced editor buffers, preserving exact source, completion authority and one undo checkpoint.
// ------------------=
fn buffered_source_preview() {
    use ui::app_assistant::{Panel,GenerationStatus};
    let request=b"Create a simple graphical calculator app";
    let source=b"#include <stdio.h>\n// function body\nint main(void) {\n    return 0;\n}";
    for (prefix,suffix) in [(b"REPLACE_COMPILE\n".as_slice(),b"\nEND_ACTION".as_slice()),
        (b"REPLACE_COMPILE\n```c\n".as_slice(),b"\n```\nEND_ACTION".as_slice()),
        (b"```c\n".as_slice(),b"\n```".as_slice()),(b"".as_slice(),b"\n   ".as_slice())] {
        let mut panel=Panel::new();panel.request[..request.len()].copy_from_slice(request);
        panel.request_len=request.len();panel.generation=GenerationStatus::Running;
        let mut document=ui::text_editor::TextDocument::new();
        assert!(document.replace_generation_preview(b"original",false));
        let mut response=prefix.to_vec();response.extend_from_slice(source);response.extend_from_slice(suffix);
        for end in 1..response.len() {
            panel.stage_generated_source(&response[..end]);
            let now=Some(end as u64*100_000_000);
            if let Some(range)=panel.stream.ready(&panel.argument[..panel.argument_len],now,false) {
                if range.start>0 {assert_eq!(document.bytes(),&panel.argument[..range.start]);}
                assert!(document.append_generation_preview(&panel.argument[range.clone()],range.start==0));
                assert!(panel.stream.committed(range.end,now));
                assert!(source.starts_with(document.bytes()));
            }
            assert!(!panel.apply_generated_compile_requested());
        }
        assert!(panel.accept_generated(&response,true,true));panel.generation=GenerationStatus::Complete;
        while let Some(range)=panel.stream.ready(&panel.argument[..panel.argument_len],None,true) {
            assert!(document.append_generation_preview(&panel.argument[range.clone()],range.start==0));
            assert!(panel.stream.committed(range.end,None));
        }
        assert_eq!(document.bytes(),source);
        assert!(panel.apply_generated_compile_requested());
        assert!(document.undo());assert_eq!(document.bytes(),b"original");
    }
    let mut panel=Panel::new();panel.request[..request.len()].copy_from_slice(request);
    panel.request_len=request.len();
    assert!(!panel.accept_generated(b"REPLACE_COMPILE\n```c\nint main(void){return 0;}\nEND_ACTION",true,true));
    assert!(!panel.apply_generated_compile_requested());
    // Ordinary literal Markdown edits retain their requested fences.
    let mut literal=Panel::new();
    assert!(literal.accept_generated(b"REPLACE\n```c\nint main(void){return 0;}\n```\nEND_ACTION",true,true));
    assert_eq!(&literal.argument[..literal.argument_len],b"```c\nint main(void){return 0;}\n```");
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
