#[path = "../kernel/runtime/ai/qwen/mod.rs"]
mod qwen;

// ------------------------=
// FUNC: main
// DESC: Exercises pinned real Hermes weights, cancellation and optional native forward execution; not guest proof.
// ------------------=
fn main() {
    let forward_prompt = std::env::var("HERMES_TEST_PROMPT").unwrap_or_else(|_| "hello".into());
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
        let start = std::time::Instant::now();
        service.submit(forward_prompt.as_bytes()).unwrap();
        let mut tokens = 0;
        let mut token_times = Vec::new();
        while service.busy() && tokens < 32 {
            if service.poll().unwrap() {
                tokens += 1;
                token_times.push(start.elapsed().as_nanos() as u64);
            }
        }
        assert!(tokens > 0);
        assert!(!service.busy(), "Timing requires a complete response, not the token limit");
        assert!(!service.output().is_empty());
        if let Some(path) = std::env::var_os("HERMES_TEST_REPORT") {
            // Binary timing and generated-byte evidence avoids using formatted
            // console output or a particular answer as the test oracle.
            let mut report = Vec::new();
            for value in [1, tokens as u64, start.elapsed().as_nanos() as u64,
                          service.output().len() as u64] {
                report.extend_from_slice(&value.to_le_bytes());
            }
            for time in token_times { report.extend_from_slice(&time.to_le_bytes()); }
            report.extend_from_slice(service.output());
            std::fs::write(path, report).unwrap();
        }
        service.cancel();
    }
}
