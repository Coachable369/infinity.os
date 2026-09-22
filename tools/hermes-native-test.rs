#[path = "../kernel/runtime/ai/qwen/mod.rs"]
mod qwen;

// ------------------------=
// FUNC: main
// DESC: Exercises pinned real Hermes weights, cancellation and optional native forward execution; not guest proof.
// ------------------=
fn main() {
    let bytes = Box::leak(
        std::fs::read("model-cache/Hermes-3-Llama-3.2-3B.Q4_K_M.gguf")
            .unwrap()
            .into_boxed_slice(),
    );
    let arena = Box::leak(vec![0u128; 1280 * 1024 * 1024 / 16].into_boxed_slice());
    let arena = unsafe {
        std::slice::from_raw_parts_mut(arena.as_mut_ptr().cast::<u8>(), arena.len() * 16)
    };
    // Hermes must not be admitted through the existing pinned Qwen loader.
    let invalid_arena = Box::leak(vec![0u8; 16].into_boxed_slice());
    assert!(qwen::service::Service::load(bytes, invalid_arena).is_err());
    let mut service = qwen::service::Service::load_hermes(bytes, arena).unwrap();
    service.submit(b"hello").unwrap();
    assert!(service.busy());
    service.cancel();
    assert!(!service.busy());
    service.clear_conversation();
    if std::env::args().any(|arg| arg == "--forward") {
        service.submit(b"hello").unwrap();
        let start = std::time::Instant::now();
        let mut tokens = 0;
        while service.busy() && tokens < 32 {
            if service.poll().unwrap() {
                tokens += 1;
                println!(
                    "{tokens} {:?}: {}",
                    start.elapsed(),
                    String::from_utf8_lossy(service.output())
                );
            }
        }
        assert!(tokens > 0);
        assert!(!service.output().is_empty());
        service.cancel();
    }
}
