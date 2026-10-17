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
    let bytes=Box::leak(std::fs::read("model-cache/Hermes-3-Llama-3.2-3B.Q4_K_M.gguf").unwrap().into_boxed_slice());
    let arena=Box::leak(vec![0u128;1280*1024*1024/16].into_boxed_slice());
    let arena=unsafe{std::slice::from_raw_parts_mut(arena.as_mut_ptr().cast::<u8>(),arena.len()*16)};
    let mut ai=Box::new(AiRuntime::new());
    assert!(ai.load_hermes(bytes,arena));
    ai.bind_chat_owner([1;16]);
    let workers:Vec<_>=(0..4).map(|id|std::thread::spawn(move||unsafe{qwen::workers::run_host_worker(id)})).collect();
    let start=std::time::Instant::now();
    while qwen::workers::online()!=4 {assert!(start.elapsed().as_secs()<10);std::thread::yield_now();}
    let id=application_host::attach([1;16],[2;16]).unwrap().instance;
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
    application_host::detach();
    unsafe {qwen::workers::stop_host_workers();}
    for worker in workers {worker.join().unwrap();}
}
