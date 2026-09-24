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
        // Optional real-model scheduler experiment. The delay represents host
        // event/firmware work, not measured guest latency. Default stays unpaced.
        let pump = std::env::var("HERMES_TEST_PUMP").unwrap_or_default();
        assert!(matches!(pump.as_str(), "" | "legacy" | "deadline"));
        let io_us = std::env::var("HERMES_TEST_IO_US").ok()
            .map(|v| v.parse::<u64>().unwrap()).unwrap_or(0);
        while service.busy() && tokens < 32 {
            let mut budget = qwen::pump::PumpBudget::new(Some(start.elapsed().as_nanos() as u64));
            let mut polls = 0;
            while service.busy() && tokens < 32 {
                if pump == "legacy" && polls >= 256 { break; }
                if !pump.is_empty() && !budget.next(Some(start.elapsed().as_nanos() as u64)) { break; }
                polls += 1;
                if service.poll().unwrap() {
                    tokens += 1;
                    token_times.push(start.elapsed().as_nanos() as u64);
                    if !pump.is_empty() { break; }
                }
            }
            if !pump.is_empty() && service.busy() && io_us != 0 {
                std::thread::sleep(std::time::Duration::from_micros(io_us));
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
