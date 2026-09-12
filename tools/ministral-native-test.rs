#[path = "../kernel/runtime/ai/qwen/mod.rs"]
mod qwen;

// ------------------------=
// FUNC: main
// DESC: Exercises pinned native Ministral weights, tokenizer, cancellation and actual forward output on the host.
// ------------------=
fn main() {
    let bytes = std::fs::read("model-cache/Ministral-3-3B-Instruct-2512-Q4_K_M.gguf").unwrap();
    let bytes = Box::leak(bytes.into_boxed_slice());
    let arena = Box::leak(vec![0u128; 1280 * 1024 * 1024 / 16].into_boxed_slice());
    let arena = unsafe {
        std::slice::from_raw_parts_mut(arena.as_mut_ptr().cast::<u8>(), arena.len() * 16)
    };
    let mut service = qwen::service::Service::load(bytes, arena).unwrap();
    assert!(!service.busy());
    service.submit("Hello".as_bytes()).unwrap();
    assert!(service.busy());
    service.cancel();
    assert!(!service.busy());
    service.clear_conversation();
    if std::env::args().any(|arg| arg == "--forward") {
        service.submit(b"Reply with only the word Hello.").unwrap();
        let start = std::time::Instant::now();
        let mut tokens = 0;
        while service.busy() && tokens < 12 {
            if service.poll().unwrap() {
                tokens += 1;
                println!(
                    "{}: {:?}",
                    tokens,
                    String::from_utf8_lossy(service.output())
                );
            }
        }
        assert!(tokens > 0);
        assert!(!service.output().is_empty());
        println!(
            "Native host output in {:?}; this is not installed-guest proof",
            start.elapsed()
        );
        service.cancel();
    }
}
