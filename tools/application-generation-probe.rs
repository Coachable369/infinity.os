//! Actual Hermes generation with structured progress and native-target compilation.
//! Compilation here uses the host cross-toolchain, not the installed compiler.
use crate::{runtime::ai::AiRuntime, ui};
use std::{error::Error, fs, io::Write, path::{Path,PathBuf}, process::Command, time::{Duration,Instant,SystemTime,UNIX_EPOCH}};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct Options {
    request: String,
    model: String,
    directory: PathBuf,
    targets: Vec<&'static str>,
    deadline: Duration,
    repair_rounds: u8,
}

// ------------------------=
// FUNC: options
// DESC: Selects a real user objective and an isolated repository-local evidence directory.
// ------------------=
fn options() -> Result<Options> {
    let args:Vec<_>=std::env::args().skip(1).collect();
    let stamp=SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let mut result=Options {
        request:"Build a simple UI C based calculator application".into(),
        model:"hermes".into(),
        directory:PathBuf::from(format!("build/application-generation/{stamp}")),
        targets:vec!["aarch64"], deadline:Duration::from_secs(300), repair_rounds:2,
    };
    let mut index=0;
    while index<args.len() {
        match args[index].as_str() {
            "--export-generation-prompt" => (),
            "--generate" => {
                if args.get(index+1).is_some_and(|value| !value.starts_with("--")) {
                    index+=1; result.request=args[index].clone();
                }
            },
            "--output-dir" => {index+=1;result.directory=PathBuf::from(args.get(index).ok_or("Missing output directory")?);},
            "--model" => {
                index+=1;result.model=args.get(index).ok_or("Missing model")?.clone();
                if !matches!(result.model.as_str(),"hermes"|"ministral") {return Err("Unsupported installed model".into());}
            },
            "--target" => {
                index+=1;
                result.targets=match args.get(index).map(String::as_str) {
                    Some("aarch64")=>vec!["aarch64"], Some("x86_64")=>vec!["x86_64"],
                    Some("both")=>vec!["aarch64","x86_64"], _=>return Err("Unsupported generation compile target".into()),
                };
            },
            "--deadline-seconds" => {
                index+=1;
                let seconds:u64=args.get(index).ok_or("Missing deadline")?.parse()?;
                if seconds==0 || seconds>3600 {return Err("Generation deadline must be within 1..3600 seconds".into());}
                result.deadline=Duration::from_secs(seconds);
            },
            "--repair-rounds" => {
                index+=1;
                result.repair_rounds=args.get(index).ok_or("Missing repair round limit")?.parse()?;
                if result.repair_rounds>2 {return Err("Repair rounds must be within 0..2".into());}
            },
            _=>return Err(format!("Unknown generation probe option: {}",args[index]).into()),
        }
        index+=1;
    }
    let root=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize()?;
    if !result.directory.is_absolute() {result.directory=root.join(&result.directory);}
    if result.directory.components().any(|part|part==std::path::Component::ParentDir)
        || !result.directory.starts_with(root.join("build")) {
        return Err("Generation evidence must remain inside the repository build directory".into());
    }
    if result.directory.exists() {return Err("Evidence directory already exists; choose a new run directory".into());}
    fs::create_dir_all(&result.directory)?;
    if !result.directory.canonicalize()?.starts_with(root.join("build").canonicalize()?) {
        return Err("Evidence directory resolves outside the repository".into());
    }
    Ok(result)
}

// ------------------------=
// FUNC: quoted
// DESC: Escapes request and artifact paths for diagnostic JSON without interpreting their contents.
// ------------------=
fn quoted(value:&str)->String {
    let mut output=String::from("\"");
    for character in value.chars() {
        match character {'"'=>output.push_str("\\\""),'\\'=>output.push_str("\\\\"),'\n'=>output.push_str("\\n"),'\r'=>output.push_str("\\r"),'\t'=>output.push_str("\\t"),
            c if c<' '=>output.push_str(&format!("\\u{:04x}",c as u32)),c=>output.push(c)}
    }
    output.push('"');output
}

// ------------------------=
// FUNC: compile_generated
// DESC: Compiles only model-produced source, links the native CRT, and checks binary ELF machine and executable type.
// ------------------=
fn compile_generated(directory:&Path,target:&str)->Result<(bool,Vec<u8>)> {
    let compiler=std::env::var("CC").unwrap_or_else(|_|"/opt/homebrew/opt/llvm/bin/clang".into());
    let linker=std::env::var("INFINITY_LLD").unwrap_or_else(|_|"/opt/homebrew/opt/lld/bin/ld.lld".into());
    let resource=Command::new(&compiler).arg("-print-resource-dir").output()?;
    if !resource.status.success() {return Err("Cross-compiler resource lookup failed".into());}
    let includes=PathBuf::from(std::str::from_utf8(&resource.stdout)?.trim()).join("include");
    if !includes.join("stddef.h").is_file() {return Err("Cross-compiler builtin headers are unavailable".into());}
    let source=directory.join("source.c");let object=directory.join(format!("application-{target}.o"));
    let binary=directory.join(format!("application-{target}.elf"));
    let mut command=Command::new(&compiler);
    command.args(["-target",&format!("{target}-none-elf"),"-nostdinc","-Isdk/c/include","-isystem"])
        .arg(includes).args(["-ffreestanding","-fno-builtin","-fno-stack-protector","-fno-unwind-tables",
            "-fno-asynchronous-unwind-tables","-fPIE","-fvisibility=hidden","-std=c17","-Wall","-Wextra","-Os",
            if target=="aarch64" {"-mno-outline-atomics"} else {"-mno-red-zone"},"-c"])
        .arg(&source).arg("-o").arg(&object);
    let compiled=command.output()?;
    fs::write(directory.join(format!("compiler-{target}.stdout")),&compiled.stdout)?;
    fs::write(directory.join(format!("compiler-{target}.stderr")),&compiled.stderr)?;
    let mut linked_code=None;let mut artifact_valid=false;
    let mut diagnostics=compiled.stderr.clone();
    if compiled.status.success() {
        let runtime=PathBuf::from(format!("builds/toolchains/{target}/application-crt.o"));
        if !runtime.is_file() {return Err(format!("Native application CRT missing: {}",runtime.display()).into());}
        let linked=Command::new(linker).args(["-static","-pie","--no-dynamic-linker","--build-id=none",
            "-z","max-page-size=4096","-T","sdk/c/application.ld"]).arg(runtime).arg(&object).arg("-o").arg(&binary).output()?;
        fs::write(directory.join(format!("linker-{target}.stdout")),&linked.stdout)?;
        fs::write(directory.join(format!("linker-{target}.stderr")),&linked.stderr)?;
        linked_code=linked.status.code();
        diagnostics.extend_from_slice(&linked.stderr);
        if linked.status.success() {
            let bytes=fs::read(&binary)?;
            artifact_valid=bytes.len()>=64 && &bytes[..6]==b"\x7fELF\x02\x01"
                && u16::from_le_bytes([bytes[16],bytes[17]])==3
                && u16::from_le_bytes([bytes[18],bytes[19]])==if target=="aarch64" {183} else {62};
        }
    }
    let evidence=format!("{{\"target\":{},\"compiler_exit\":{},\"linker_exit\":{},\"elf_valid\":{},\"installed_compiler\":false}}\n",
        quoted(target),compiled.status.code().map_or("null".into(),|code|code.to_string()),
        linked_code.map_or("null".into(),|code|code.to_string()),artifact_valid);
    fs::write(directory.join(format!("compile-{target}.json")),&evidence)?;print!("{evidence}");
    Ok((compiled.status.success() && linked_code==Some(0) && artifact_valid,diagnostics))
}

// ------------------------=
// FUNC: prepare_prompt
// DESC: Builds and saves the real editor generation request without loading inference weights or duplicating prompt text.
// ------------------=
fn prepare_prompt(options:&Options)->Result<(ui::app_assistant::Panel,[u8;6144],usize)> {
    let request=options.request.as_bytes();
    let mut panel=ui::app_assistant::Panel::new();
    if request.is_empty() || request.len()>panel.input.len() {return Err("Request exceeds actual editor input capacity".into());}
    panel.input[..request.len()].copy_from_slice(request);panel.length=request.len();
    panel.request[..request.len()].copy_from_slice(request);panel.request_len=request.len();
    panel.generation=ui::app_assistant::GenerationStatus::Running;
    fs::write(options.directory.join("request.txt"),request)?;
    let mut prompt=[0;6144];
    let length=match panel.generation_prompt(true,b"",&mut prompt) {
        Some(length)=>length,
        None=>{fs::write(options.directory.join("result.json"),b"{\"prompt_admitted\":false,\"generation_complete\":false,\"compiled\":false,\"installed_acceptance\":false}\n")?;
            return Err("Production generation prompt exceeded its buffer; request and failure were preserved".into());}
    };
    fs::write(options.directory.join("prompt.txt"),&prompt[..length])?;
    Ok((panel,prompt,length))
}

// ------------------------=
// FUNC: export_generation_prompt
// DESC: Exports the production prompt only, explicitly distinguishing diagnostic preparation from generation or installed acceptance.
// ------------------=
pub fn export_generation_prompt()->Result<()> {
    let options=options()?;
    let (_,_,length)=prepare_prompt(&options)?;
    let report=format!("{{\"request\":{},\"model\":{},\"prompt_admitted\":true,\"prompt_bytes\":{},\"model_loaded\":false,\"generation_complete\":false,\"compiled\":false,\"installed_acceptance\":false}}\n",
        quoted(&options.request),quoted(&options.model),length);
    fs::write(options.directory.join("result.json"),&report)?;
    print!("{report}");
    println!("Prompt evidence: {}",options.directory.display());
    Ok(())
}

// ------------------------=
// FUNC: document_revision
// DESC: Uses the production editor fingerprint, including its unnamed object identity, for repair and streaming fences.
// ------------------=
fn document_revision(document:&ui::text_editor::TextDocument)->u64 {
    ui::app_assistant::fingerprint(document.bytes(),document.cursor(),document.selection())
        ^ ui::app_assistant::fingerprint(b"",0,None)
}

// ------------------------=
// FUNC: stream_document
// DESC: Delivers real model source through the same bounded stream and native document APIs used by the editor.
// ------------------=
fn stream_document(document:&mut ui::text_editor::TextDocument,now:u64)->Result<()> {
    let mut panel=ui::app_assistant::read(2);
    if panel.document_revision!=document_revision(document) {return Err("Document changed during model generation".into());}
    while let Some(range)=panel.stream.ready(&panel.argument[..panel.argument_len],Some(now),
        panel.generation==ui::app_assistant::GenerationStatus::Complete) {
        if !document.append_generation_preview(&panel.argument[range.clone()],range.start==0) {
            return Err("Native document rejected generated source preview".into());
        }
        panel.stream.committed(range.end,Some(now));
        panel.document_revision=document_revision(document);
    }
    ui::app_assistant::write(2,panel);
    Ok(())
}

struct Attempt {
    complete:bool,
    compiled:bool,
    diagnostics:Vec<u8>,
    compiled_revision:u64,
    repairable:bool,
}

// ------------------------=
// FUNC: generation_attempt
// DESC: Captures one actual inference and cross-compile attempt without substituting application code or hiding failure artifacts.
// ------------------=
fn generation_attempt(ai:&mut AiRuntime,options:&Options,directory:&Path,prompt:&[u8],
    document:&mut ui::text_editor::TextDocument)->Result<Attempt> {
    fs::write(directory.join("prompt.txt"),prompt)?;
    ai.submit_app_source(2,prompt).map_err(|error|format!("Generation admission: {}",String::from_utf8_lossy(error)))?;
    let mut panel=ui::app_assistant::read(2);panel.generation=ui::app_assistant::GenerationStatus::Running;
    ui::app_assistant::write(2,panel);
    let started=Instant::now();let mut next_report=Duration::ZERO;
    let mut progress=fs::File::create(directory.join("progress.jsonl"))?;
    let mut raw=Vec::new();let mut last_source=0;let mut snapshots=0;let mut first_output=None;
    let mut first_source=None;let mut token_work=0;let mut prompt_tokens=0;
    let mut processed_tokens=0;let mut generated_tokens=0;
    while ai.app_turn_owner().is_some() && started.elapsed()<options.deadline {
        ai.poll_qwen();token_work=token_work.max(ai.acceptance_native_progress());
        if let Some(inference)=ai.app_generation_progress() {
            prompt_tokens=prompt_tokens.max(inference.prompt_tokens);
            processed_tokens=processed_tokens.max(inference.processed_tokens);
            generated_tokens=generated_tokens.max(inference.generated_tokens);
        }
        let output=ai.acceptance_native_output();
        if !output.is_empty() {raw.clear();raw.extend_from_slice(output);first_output.get_or_insert(started.elapsed().as_millis());}
        if let Err(error)=stream_document(document,started.elapsed().as_nanos().min(u64::MAX as u128) as u64) {
            ai.cancel_app_turn();return Err(error);
        }
        let current=ui::app_assistant::read(2);
        if current.argument_len>0 {first_source.get_or_insert(started.elapsed().as_millis());}
        if started.elapsed()>=next_report || ai.app_turn_owner().is_none() {
            let record=format!("{{\"elapsed_ms\":{},\"token_work\":{},\"prompt_tokens\":{},\"processed_tokens\":{},\"generated_tokens\":{},\"output_bytes\":{},\"source_bytes\":{},\"generation_state\":{},\"lease_owned\":{}}}\n",
                started.elapsed().as_millis(),token_work,prompt_tokens,processed_tokens,generated_tokens,
                raw.len(),current.argument_len,current.generation as u8,ai.app_turn_owner().is_some());
            progress.write_all(record.as_bytes())?;progress.flush()?;print!("{record}");std::io::stdout().flush()?;
            fs::write(directory.join("model-output.txt"),&raw)?;
            fs::write(directory.join("source.c"),&current.argument[..current.argument_len])?;
            fs::write(directory.join("editor-source.c"),document.bytes())?;
            if current.argument_len>last_source && snapshots<128 {
                snapshots+=1;fs::write(directory.join(format!("source-{snapshots:03}.c")),&current.argument[..current.argument_len])?;
                last_source=current.argument_len;
            }
            next_report=started.elapsed()+Duration::from_secs(2);
        }
    }
    let timed_out=ai.app_turn_owner().is_some();
    let mut panel=ui::app_assistant::read(2);
    let proposed=!timed_out && panel.generation==ui::app_assistant::GenerationStatus::Complete
        && panel.argument_len>0 && panel.apply_generated_compile_requested();
    if proposed && panel.document_revision==document_revision(document)
        && document.bytes()!=&panel.argument[..panel.argument_len] {
        if !panel.edit_document(ui::app_assistant::Action::ReplaceCompile,document) {
            return Err("Native document rejected the completed source proposal".into());
        }
        panel.document_revision=document_revision(document);ui::app_assistant::write(2,panel);
    }
    fs::write(directory.join("model-output.txt"),&raw)?;
    fs::write(directory.join("source.c"),&panel.argument[..panel.argument_len])?;
    fs::write(directory.join("editor-source.c"),document.bytes())?;
    if timed_out {ai.cancel_app_turn();}
    let complete=proposed
        && document.bytes()==&panel.argument[..panel.argument_len];
    let mut compiled=complete;
    let compiled_revision=document_revision(document);
    let mut diagnostics=Vec::new();let mut repairable=complete;
    if complete {for target in &options.targets {
        match compile_generated(directory,target) {
            Ok((result,errors))=>{compiled&=result;if !result {diagnostics.extend_from_slice(&errors);}},
            Err(error)=>{compiled=false;repairable=false;fs::write(directory.join(format!("compile-{target}.error")),error.to_string())?;}
        }
    }}
    let report=format!("{{\"request\":{},\"model\":{},\"prompt_admitted\":true,\"prompt_bytes\":{},\"elapsed_ms\":{},\"token_work\":{},\"prompt_tokens\":{},\"processed_tokens\":{},\"generated_tokens\":{},\"first_output_ms\":{},\"first_source_ms\":{},\"source_bytes\":{},\"source_snapshots\":{},\"generation_state\":{},\"timed_out\":{},\"generation_complete\":{},\"compiled\":{},\"installed_acceptance\":false,\"domain_behavior_verified\":false}}\n",
        quoted(&options.request),quoted(&options.model),prompt.len(),started.elapsed().as_millis(),token_work,prompt_tokens,processed_tokens,generated_tokens,
        first_output.map_or("null".into(),|value|value.to_string()),first_source.map_or("null".into(),|value|value.to_string()),
        panel.argument_len,snapshots,panel.generation as u8,timed_out,complete,compiled);
    fs::write(directory.join("attempt.json"),&report)?;print!("{report}");
    Ok(Attempt {complete,compiled,diagnostics,compiled_revision,repairable})
}

// ------------------------=
// FUNC: queue_repair
// DESC: Admits only a bounded production repair against the actual unchanged native document and compiler diagnostics.
// ------------------=
fn queue_repair(request:&[u8],attempt:&Attempt,document:&ui::text_editor::TextDocument,
    prompt:&mut [u8])->Result<usize> {
    let mut panel=ui::app_assistant::read(2);
    if !attempt.repairable || !panel.queue_compiler_repair(request,attempt.compiled_revision,
        document_revision(document),&attempt.diagnostics) {
        return Err("Production compiler repair rejected the request, revision, diagnostics, or retry budget".into());
    }
    let length=panel.compiler_repair_prompt(document.bytes(),prompt)
        .ok_or("Complete repair context exceeds the bounded prompt buffer; source was not truncated")?;
    ui::app_assistant::write(2,panel);
    Ok(length)
}

// ------------------------=
// FUNC: run
// DESC: Executes at most two diagnostic-driven model repairs, preserving every source and binary attempt and reporting host evidence separately.
// ------------------=
pub fn run(ai:&mut AiRuntime)->Result<()> {
    let options=options()?;
    let (mut panel,initial,length)=prepare_prompt(&options)?;
    let mut document=Box::new(ui::text_editor::TextDocument::new());
    panel.document_revision=document_revision(&document);
    ui::app_assistant::write(2,panel);
    let mut prompt=vec![0;16*1024];prompt[..length].copy_from_slice(&initial[..length]);
    let mut prompt_length=length;let mut attempts=0;let mut compiled=false;let mut complete=false;
    let mut failure=None;let mut final_directory=options.directory.clone();
    for round in 0..=options.repair_rounds {
        let directory=if round==0 {options.directory.clone()} else {options.directory.join(format!("attempt-{round:02}"))};
        fs::create_dir_all(&directory)?;
        final_directory=directory.clone();attempts+=1;
        let attempt=match generation_attempt(ai,&options,&directory,&prompt[..prompt_length],&mut document) {
            Ok(attempt)=>attempt,
            Err(error)=>{ai.cancel_app_turn();
                let detail=error.to_string();
                fs::write(directory.join("attempt.json"),format!("{{\"generation_complete\":false,\"compiled\":false,\"failure\":{}}}\n",quoted(&detail)))?;
                failure=Some(detail);break;},
        };
        complete=attempt.complete;compiled=attempt.compiled;
        if compiled {break;}
        if !complete || round==options.repair_rounds {break;}
        match queue_repair(options.request.as_bytes(),&attempt,&document,&mut prompt) {
            Ok(length)=>prompt_length=length,
            Err(error)=>{failure=Some(error.to_string());break;},
        }
    }
    let report=format!("{{\"request\":{},\"model\":{},\"repair_limit\":{},\"attempts\":{},\"final_attempt\":{},\"generation_complete\":{},\"compiled\":{},\"failure\":{},\"installed_acceptance\":false,\"domain_behavior_verified\":false}}\n",
        quoted(&options.request),quoted(&options.model),options.repair_rounds,attempts,quoted(&final_directory.to_string_lossy()),
        complete,compiled,failure.as_deref().map_or("null".into(),quoted));
    fs::write(options.directory.join("result.json"),&report)?;print!("{report}");
    println!("Generation evidence: {}",options.directory.display());
    if !compiled {return Err(failure.unwrap_or_else(||"Actual model generation or native-target compilation failed after bounded repair".into()).into());}
    Ok(())
}

// ------------------------=
// FUNC: repair_api_boundaries
// DESC: Verifies real document revision fences, two-round production limits, unchanged code preservation, and complete bounded repair context.
// ------------------=
pub fn repair_api_boundaries() {
    let request=b"Create a graphical calculator app";
    let mut document=Box::new(ui::text_editor::TextDocument::new());
    let source=b"#include <infinity/app.h>\nint main(void){return missing();}\n";
    assert!(document.replace_generation_preview(source,true));
    let mut panel=ui::app_assistant::Panel::new();panel.request[..request.len()].copy_from_slice(request);
    panel.request_len=request.len();panel.generation=ui::app_assistant::GenerationStatus::Complete;
    ui::app_assistant::write(2,panel);
    let mut attempt=Attempt {complete:true,compiled:false,diagnostics:vec![1,2,3],
        compiled_revision:document_revision(&document),repairable:true};
    let mut prompt=vec![0;16*1024];
    for round in 1..=2 {
        assert!(queue_repair(request,&attempt,&document,&mut prompt).unwrap()>source.len());
        assert_eq!(document.bytes(),source);assert_eq!(ui::app_assistant::read(2).repair_attempts,round);
        let mut panel=ui::app_assistant::read(2);panel.generation=ui::app_assistant::GenerationStatus::Complete;
        ui::app_assistant::write(2,panel);
    }
    assert!(queue_repair(request,&attempt,&document,&mut prompt).is_err());
    panel.repair_attempts=0;ui::app_assistant::write(2,panel);
    assert!(document.insert(b' '));
    assert!(queue_repair(request,&attempt,&document,&mut prompt).is_err());
    assert_eq!(ui::app_assistant::read(2).repair_attempts,0);
    attempt.compiled_revision=document_revision(&document);attempt.repairable=false;
    assert!(queue_repair(request,&attempt,&document,&mut prompt).is_err());
    ui::app_assistant::reset();
}
