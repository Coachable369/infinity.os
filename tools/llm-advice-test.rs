#![allow(dead_code)]
#[path = "../kernel/runtime/mod.rs"] mod runtime;
#[path = "../kernel/ui/mod.rs"] mod ui;
use runtime::ai::{AiRuntime, chat::{ChatRuntime, GenerationState}, text_request::*};

// ------------------------=
// FUNC: output_text
// DESC: Supplies a diagnostic-only sink for the production host harness.
// ------------------=
fn output_text(_: &[u8]) {}

// ------------------------=
// FUNC: contract
// DESC: Exercises typed parsing, inference admission, response ownership, cancellation, capacity, and identity isolation without using displayed prose as an oracle.
// ------------------=
fn contract() {
    let mut terminal=Box::new(ui::command_terminal::State::new());terminal.session([1;16],true);
    let mut bytes=vec![b'x';211];bytes.extend_from_slice(b"\n\nend");
    write_response_lines(&bytes,|line|terminal.line(line));
    assert_eq!(terminal.count,3);assert_eq!(terminal.length(terminal.first),211);
    assert_eq!(terminal.length(terminal.first+1),0);assert_eq!(terminal.length(terminal.first+2),3);
    for columns in [67,100,160]{
        terminal.reflow(columns,20);
        assert_eq!(terminal.length(terminal.first),211);
        for column in 0..211{assert_eq!(terminal.byte(ui::command_terminal::Position{row:terminal.first,column}),b'x');}
        assert_eq!(terminal.visual(1).unwrap(),ui::command_terminal::Position{row:terminal.first,column:columns});
    }
    assert_eq!(parse_command(b"ai ask Explain scheduling"), Some(Command::Prompt(AdviceMode::Answer, b"Explain scheduling")));
    assert_eq!(parse_command(b"llm decide Which option?"), Some(Command::Prompt(AdviceMode::Decide, b"Which option?")));
    assert_eq!(parse_command(b"ai decide Which option?"), Some(Command::Prompt(AdviceMode::Decide, b"Which option?")));
    assert_eq!(parse_command(b"llm Compare two options"), Some(Command::Prompt(AdviceMode::Answer, b"Compare two options")));
    assert_eq!(parse_command(b"ai response"), Some(Command::Response));
    assert_eq!(parse_command(b"ai cancel"), Some(Command::Cancel));
    for domain in [b"ai".as_slice(),b"llm"] {
        for space in [b" ".as_slice(),b"  ",b"\t",b" \t "] {
            for (verb,expected) in [(b"help".as_slice(),Command::Help),(b"response",Command::Response),
                (b"cancel",Command::Cancel),(b"new",Command::New),(b"ask",Command::Help),(b"decide",Command::Help)] {
                let bytes=[b" \t".as_slice(),domain,space,verb,b" \t"].concat();
                assert_eq!(parse_command(&bytes),Some(expected));
            }
        }
    }
    assert_eq!(parse_command(b"llm\task\t Keep  this\tspacing.  "),Some(Command::Prompt(AdviceMode::Answer,b"Keep  this\tspacing.  ")));
    assert_eq!(parse_command(b"ai  decide  Cost or speed?"),Some(Command::Prompt(AdviceMode::Decide,b"Cost or speed?")));
    for command in [b"ai cancel another".as_slice(),b"llm response extra",b"llm new now",b"ai help extra"] {
        assert_eq!(parse_command(command),Some(Command::Invalid));
    }
    assert_eq!(parse_command(b"ai ask    "), Some(Command::Help));
    assert_eq!(parse_command(b"ai asking"), None);
    assert_eq!(parse_command(b"ai status"), None);
    assert!(decision_question(b"Should I prioritize low cost or reliability?"));
    assert!(decision_question(b"decide whether to restart the service"));
    assert!(!decision_question(b"restart the service"));
    assert!(!decision_question(b"open task manager"));
    assert!(prepare_prompt(AdviceMode::Decide, b"Compare options", Some(Facts::default()), &mut [0;6144]).is_ok());
    assert_eq!(prepare_prompt(AdviceMode::Decide, b"", None, &mut [0;6144]), Err(AdviceError::InvalidInput));
    assert_eq!(prepare_prompt(AdviceMode::Answer, &[b'x';4097], None, &mut [0;6144]), Err(AdviceError::InvalidInput));
    assert_eq!(prepare_prompt(AdviceMode::Answer, b"hi\x1b", None, &mut [0;6144]), Err(AdviceError::InvalidInput));
    assert_eq!(prepare_prompt(AdviceMode::Decide, b"Compare options", None, &mut [0;32]), Err(AdviceError::ContextFull));
    let ranked = weighted::rank(b"weights=80,20 scores=9,4/4,9").unwrap().unwrap();
    assert_eq!(ranked.scores[..2], [800,500]); assert_eq!(ranked.winner(),Some(1));
    let inverted = weighted::rank(b"weights=20,80 scores=9,4/4,9").unwrap().unwrap();
    assert_eq!(inverted.scores[..2], [500,800]); assert_eq!(inverted.winner(),Some(2));
    assert_eq!(weighted::rank(b"weights=50,50 scores=9,4/4,9").unwrap().unwrap().winner(),None);
    assert_eq!(weighted::rank(b"weights=80,20 scores=9,4/4,9 eligible=0,1").unwrap().unwrap().winner(),Some(2));
    assert_eq!(weighted::rank(b"weights=80,20 scores=9,4/4,9 eligible=0,0"),Err(weighted::Error::NoEligibleOption));
    assert_eq!(weighted::rank(b"weights=80,80 scores=9,4/4,9"),Err(weighted::Error::Weights));
    assert_eq!(weighted::rank(b"weights=80,20 scores=9/4,9"),Err(weighted::Error::Scores));
    assert_eq!(weighted::rank(b"weights=80,20 weights=80,20 scores=9,4/4,9"),Err(weighted::Error::Syntax));
    assert_eq!(weighted::rank(b"weights=80,20 scores=9,4/4,9 eligible=1,2"),Err(weighted::Error::Eligibility));
    assert_eq!(weighted::rank(b"weights=80,20 scores=65536,4/4,9"),Err(weighted::Error::Scores));
    assert_eq!(weighted::rank(b"Should I start weight training?"),Ok(None));
    let natural=weighted::rank(b"Cost weight 80%, reliability weight 20%. Option 1: cost 9, reliability 4. Option 2: cost 4, reliability 9.").unwrap().unwrap();
    assert_eq!(natural,ranked);
    assert_eq!(weighted::rank(b"Speed weight 20%, safety weight 80%. Option 1: safety 4, speed 9. Option 2: speed 4, safety 9.").unwrap().unwrap().winner(),Some(2));
    assert_eq!(weighted::rank(b"Cost weight 80%, reliability weight 20%. Option 1: cost 9.5, reliability 4. Option 2: cost 4, reliability 9."),Err(weighted::Error::Scores));
    assert_eq!(weighted::rank(b"Cost weight 80%, reliability weight 20%. Option 1: cost 9. Option 2: cost 4, reliability 9."),Err(weighted::Error::Scores));
    assert_eq!(weighted::rank(b"Cost weight 80%, reliability weight 20%. Option 1: cost 9, reliability 4. Option 3: cost 4, reliability 9."),Err(weighted::Error::Scores));
    for suffix in [b" Explain why option 1 is better.".as_slice(),b" Compare option 1 with option 2."] {
        let input=[b"Cost weight 80%, reliability weight 20%. Option 1: cost 9, reliability 4. Option 2: cost 4, reliability 9.".as_slice(),suffix].concat();
        assert_eq!(weighted::rank(&input),Ok(Some(ranked)));
    }
    assert_eq!(weighted::rank(b"Compare option 1 with option 2. Cost weight 80%, reliability weight 20%. Option 1 cost 9, reliability 4. Option 2 cost 4, reliability 9."),Ok(Some(ranked)));
    assert_eq!(weighted::rank(b"Cost weight 80%, reliability weight 20%. Option 1: cost 9, reliability 4. Option 2: cost 4, reliability 9. Option 1: cost 8, reliability 5."),Err(weighted::Error::Scores));
    for (input,error) in [(b"weights=80,80 scores=9,4/4,9".as_slice(),weighted::Error::Weights),
        (b"weights=80,20 scores=9/4,9",weighted::Error::Scores),
        (b"weights=80,20 scores=9,4/4,9 eligible=1",weighted::Error::Eligibility),
        (b"weights=80,20 scores=9,4/4,9 eligible=0,0",weighted::Error::NoEligibleOption),
        (b"weights=80,20 weights=80,20 scores=9,4/4,9",weighted::Error::Syntax)] {
        assert_eq!(prepare_prompt(AdviceMode::Decide,input,None,&mut [0;6144]),Err(AdviceError::InvalidDecision(error)));
    }
    let mut ai = Box::new(AiRuntime::new());
    assert_eq!(ai.submit_text_advice([1;16], b"hi", AdviceMode::Answer, None, true), Err(AdviceError::Denied));
    ai.bind_chat_owner([1;16]);
    assert_eq!(ai.reset_text_advice([1;16],0),Err(AdviceError::NotOwned));
    assert_eq!(ai.reset_text_advice([2;16],0),Err(AdviceError::Denied));
    assert!(ai.chat.push_input(b'd'));
    let before = ai.chat.state_hash();
    assert_eq!(ai.submit_text_advice([1;16], b"hi", AdviceMode::Answer, None, true), Err(AdviceError::Unavailable));
    assert_eq!(before, ai.chat.state_hash());
    ai.chat.generation_state = GenerationState::Running;
    assert_eq!(ai.submit_text_advice([1;16], b"hi", AdviceMode::Answer, None, true), Err(AdviceError::Busy));
    ai.chat.set_enabled(false);
    assert_eq!(ai.submit_text_advice([1;16], b"hi", AdviceMode::Answer, None, true), Err(AdviceError::Denied));
    let mut chat = ChatRuntime::new();
    chat.push_input(b'q'); chat.begin_native_turn(); chat.generation_state = GenerationState::Running;
    let pending = PendingReply { owner:[1;16], session:[2;16], turn:chat.turn_id(), terminal:true };
    assert_eq!(pending.state([1;16], [2;16], true, &chat), ReplyState::Waiting);
    assert!(response(&chat).is_none());
    chat.update_native_response(&[65,66,67]); chat.generation_state = GenerationState::Complete;
    chat.push_input(b'd');chat.set_input_cursor(0);
    let old_hash=chat.state_hash();let old_count=chat.message_count();let old_turn=chat.turn_id();
    chat.set_advice_notice(AdviceError::InvalidDecision(weighted::Error::Weights));
    assert_eq!(chat.advice_notice(),Some(AdviceError::InvalidDecision(weighted::Error::Weights)));
    assert_eq!(chat.input(),b"d");assert_eq!(chat.input_cursor(),0);
    assert_eq!(chat.message_count(),old_count);assert_eq!(chat.turn_id(),old_turn);
    assert_eq!(chat.generation_state,GenerationState::Complete);assert_ne!(chat.state_hash(),old_hash);
    assert_eq!(response(&chat), Some([65,66,67].as_slice()));
    assert_eq!(pending.state([1;16], [2;16], true, &chat), ReplyState::Ready(GenerationState::Complete));
    assert_eq!(pending.state([9;16], [2;16], true, &chat), ReplyState::Revoked);
    assert_eq!(pending.state([1;16], [9;16], true, &chat), ReplyState::Revoked);
    assert_eq!(pending.state([1;16], [2;16], false, &chat), ReplyState::Revoked);
    chat.push_input(b'n'); chat.begin_native_turn();
    assert_eq!(chat.advice_notice(),None);
    assert!(response(&chat).is_none());
    assert_eq!(pending.state([1;16], [2;16], true, &chat), ReplyState::Superseded);
}

// ------------------------=
// FUNC: native
// DESC: Exercises real pinned Hermes weighted-choice inference through the production typed endpoint, checking a machine-readable choice and no executable actions.
// ------------------=
fn native() {
    let weights = Box::leak(std::fs::read("model-cache/Hermes-3-Llama-3.2-3B.Q4_K_M.gguf").unwrap().into_boxed_slice());
    let arena = Box::leak(vec![0u128; 1280 * 1024 * 1024 / 16].into_boxed_slice());
    let arena = unsafe { std::slice::from_raw_parts_mut(arena.as_mut_ptr().cast::<u8>(), arena.len()*16) };
    let mut ai = Box::new(AiRuntime::new());
    assert!(ai.load_hermes(weights, arena));
    let threads: Vec<_> = (0..4).map(|index| std::thread::spawn(move || unsafe {
        runtime::ai::qwen::workers::run_host_worker(index)
    })).collect();
    let start = std::time::Instant::now();
    while runtime::ai::qwen::workers::online()!=4 {
        assert!(start.elapsed().as_secs()<10); std::thread::yield_now();
    }
    let cases: &[(&[u8], u8)] = &[
        (b"Choose the option with the highest weighted sum. Scores are out of 10, higher is better. Cost weight 80%, reliability weight 20%. Option 1: cost 9, reliability 4. Option 2: cost 4, reliability 9. Reply with exactly CHOICE:1 or CHOICE:2, and no other text.", 1),
        (b"Choose the option with the highest weighted sum. Scores are out of 10, higher is better. Cost weight 20%, reliability weight 80%. Option 1: cost 9, reliability 4. Option 2: cost 4, reliability 9. Reply with exactly CHOICE:1 or CHOICE:2, and no other text.", 2),
        (b"Choose by the verified weighted scores. Criteria are cost and reliability, higher scores are better. weights=80,20 scores=9,4/4,9 Reply with exactly CHOICE:1 or CHOICE:2, and no other text.", 1),
        (b"Choose by the verified weighted scores. Criteria are cost and reliability, higher scores are better. weights=20,80 scores=9,4/4,9 Reply with exactly CHOICE:1 or CHOICE:2, and no other text.", 2),
        (b"Choose the highest scoring eligible option. weights=80,20 scores=9,4/4,9 eligible=0,1 Reply with exactly CHOICE:1 or CHOICE:2, and no other text.", 2),
        (b"Choose the option that meets all requirements. Hard limit: cost at most 100 units. Option 1: cost 120, quality 10. Option 2: cost 80, quality 8. Reply with exactly CHOICE:1 or CHOICE:2, and no other text.", 2),
    ];
    for (index, (input, expected)) in cases.iter().enumerate() {
        let owner = [index as u8 + 1;16];
        ai.bind_chat_owner(owner);
        ai.chat.push_input(b'd'); ai.chat.push_input(b'r'); ai.chat.set_input_cursor(1);
        let turn = ai.submit_text_advice(owner, input, AdviceMode::Decide, None, true).unwrap();
        let running_hash=ai.chat.state_hash();
        assert_eq!(ai.reset_text_advice(owner,turn),Err(AdviceError::Busy));
        assert_eq!(ai.chat.state_hash(),running_hash);
        assert_eq!(ai.chat.input(), b"dr"); assert_eq!(ai.chat.input_cursor(),1);
        assert_eq!(ai.chat.turn_id(), turn);
        assert_eq!(ai.chat.message(0).unwrap().text(), *input);
        assert_eq!(ai.submit_text_advice(owner, b"interrupt", AdviceMode::Answer, None, true), Err(AdviceError::Busy));
        let started = std::time::Instant::now();
        while ai.chat.generation_state == GenerationState::Running {
            ai.poll_qwen();
            assert!(!ai.control_pending()); assert_eq!(ai.take_control(owner, true), None);
            assert!(started.elapsed().as_secs()<180);
        }
        assert_eq!(ai.chat.generation_state, GenerationState::Complete);
        let output = response(&ai.chat).unwrap();
        let choice = core::str::from_utf8(output).unwrap().trim().strip_prefix("CHOICE:").and_then(|value|value.parse::<u8>().ok());
        assert_eq!(choice, Some(*expected), "model decision output: {}", String::from_utf8_lossy(output));
        assert!(ai.qwen_tokens>0);
        println!("native weighted decision {index}: choice={expected}, tokens={}, elapsed_ms={}", ai.qwen_tokens, started.elapsed().as_millis());
        let completed_hash=ai.chat.state_hash();
        assert_eq!(ai.reset_text_advice([0xff;16],turn),Err(AdviceError::Denied));
        assert_eq!(ai.reset_text_advice(owner,turn+1),Err(AdviceError::NotOwned));
        assert_eq!(ai.chat.state_hash(),completed_hash);
        let model=ai.chat.selected_model();let model_index=ai.chat.selected_model_index();
        ai.chat.select_next_model();
        assert_eq!(ai.reset_text_advice(owner,turn),Err(AdviceError::NotOwned));
        ai.chat.select_model_index(model_index);
        ai.reset_text_advice(owner,turn).unwrap();
        assert_eq!(ai.chat.message_count(),0);assert_ne!(ai.chat.turn_id(),turn);
        assert_eq!(ai.chat.generation_state,GenerationState::Ready);
        assert_eq!(ai.chat.selected_model(),model);assert_eq!(ai.chat.input(),b"dr");assert_eq!(ai.chat.input_cursor(),1);
        let fresh=ai.submit_text_advice(owner,b"Say OK.",AdviceMode::Answer,None,true).unwrap();
        assert_eq!(ai.qwen_metrics.reused_tokens,0);
        assert!(ai.cancel_chat());
        ai.chat.begin_native_turn();
        let superseded=ai.chat.state_hash();
        assert_eq!(ai.reset_text_advice(owner,fresh),Err(AdviceError::NotOwned));
        assert_eq!(ai.chat.state_hash(),superseded);
    }
    let owner=[9;16]; ai.bind_chat_owner(owner);
    let turn=ai.submit_text_advice(owner,b"open editor and then create a file",AdviceMode::Answer,None,false).unwrap();
    assert!(turn>0); assert_eq!(ai.take_control(owner,true),None); assert!(!ai.control_pending());
    assert!(ai.cancel_chat()); assert_eq!(ai.chat.generation_state,GenerationState::Cancelled);
    ai.bind_chat_owner([10;16]); assert_eq!(ai.chat.message_count(),0);
    unsafe { runtime::ai::qwen::workers::stop_host_workers(); }
    for thread in threads { thread.join().unwrap(); }
}

// ------------------------=
// FUNC: main
// DESC: Runs bounded behavioral contracts and optionally the real native model decisions; this is not installed-VM acceptance.
// ------------------=
fn main() {
    contract();
    if std::env::args().any(|arg| arg=="--native") { native(); }
    println!("PASS typed LLM advice behavioral acceptance");
}
