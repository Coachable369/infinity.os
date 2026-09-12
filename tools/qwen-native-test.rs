#[path = "../kernel/runtime/ai/qwen/mod.rs"]
mod qwen;

// ------------------------=
// FUNC: main
// DESC: Exercises native model parsing against the pinned official artifact; this is not guest proof.
// ------------------=
fn main() {
    let bytes = std::fs::read("model-cache/Qwen3-8B-Q4_K_M.gguf").unwrap();
    let model = qwen::gguf::Model::parse(&bytes).unwrap();
    let mut metadata = qwen::gguf::Reader {
        bytes: &bytes,
        position: 16,
    };
    let count = metadata.u64().unwrap();
    for _ in 0..count {
        let key = metadata.string().unwrap();
        let kind = metadata.u32().unwrap();
        if key.starts_with(b"qwen3.") {
            println!("metadata {} kind={kind}", String::from_utf8_lossy(key));
        }
        metadata.skip(kind).unwrap();
    }
    let weights = qwen::transformer::Weights::load(model).unwrap();
    let array_count = |key: &[u8]| {
        let (_, mut values) = model.metadata(key).unwrap();
        assert_eq!(values.u32().unwrap(), 8);
        values.u64().unwrap() as usize
    };
    let mut vocabulary = vec![qwen::tokenizer::EMPTY; array_count(b"tokenizer.ggml.tokens")];
    let mut merges = vec![qwen::tokenizer::EMPTY; array_count(b"tokenizer.ggml.merges")];
    let tokenizer = qwen::tokenizer::Tokenizer::load(model, &mut vocabulary, &mut merges).unwrap();
    let mut tokens = [0u32; 256];
    let count = tokenizer.encode("Hello, world!", &mut tokens).unwrap();
    assert_eq!(&tokens[..count], &[9707, 11, 1879, 0]);
    for input in ["Hello, café!", "你好，世界", "مرحبا", "🙂\nsecond line"] {
        let count = tokenizer.encode(input, &mut tokens).unwrap();
        let mut restored = Vec::new();
        for token in &tokens[..count] {
            let mut piece = [0; 1024];
            let length = tokenizer.decode(*token, &mut piece).unwrap();
            restored.extend_from_slice(&piece[..length]);
        }
        assert_eq!(restored, input.as_bytes());
    }
    if std::env::args().any(|arg| arg == "--forward") {
        let mut kv = vec![0f32; qwen::transformer::KV_FLOATS];
        let mut work = vec![0f32; qwen::transformer::WORK_FLOATS];
        let mut engine = qwen::transformer::Engine::new(weights, &mut kv, &mut work).unwrap();
        let length = tokenizer.encode("<unused>", &mut tokens).unwrap();
        assert!(length > 0);
        let mut prompt = vec![tokenizer.special(b"<|im_start|>").unwrap()];
        let count = tokenizer.encode("user\nHi", &mut tokens).unwrap();
        prompt.extend_from_slice(&tokens[..count]);
        prompt.push(tokenizer.special(b"<|im_end|>").unwrap());
        let count = tokenizer.encode("\n", &mut tokens).unwrap();
        prompt.extend_from_slice(&tokens[..count]);
        prompt.push(tokenizer.special(b"<|im_start|>").unwrap());
        let count = tokenizer.encode("assistant\n", &mut tokens).unwrap();
        prompt.extend_from_slice(&tokens[..count]);
        assert_eq!(tokenizer.special(b"<think>").unwrap(), 151667);
        assert_eq!(tokenizer.special(b"</think>").unwrap(), 151668);
        prompt.push(151667);
        let count = tokenizer.encode("\n\n", &mut tokens).unwrap();
        prompt.extend_from_slice(&tokens[..count]);
        prompt.push(151668);
        prompt.extend_from_slice(&tokens[..count]);
        let begin = std::time::Instant::now();
        let mut next = 0;
        for &token in &prompt {
            engine.begin(token).unwrap();
            loop {
                if let qwen::transformer::Progress::Token(value) = engine.step().unwrap() {
                    next = value;
                    break;
                }
            }
        }
        let reference_next = next;
        engine.reset();
        for (index, &token) in prompt.iter().enumerate() {
            let predict = index + 1 == prompt.len();
            engine.begin_with_prediction(token, predict).unwrap();
            loop {
                match engine.step().unwrap() {
                    qwen::transformer::Progress::Prefilled => {
                        assert!(!predict);
                        break;
                    }
                    qwen::transformer::Progress::Token(value) => {
                        assert!(predict);
                        assert_eq!(value, reference_next);
                        next = value;
                        break;
                    }
                    qwen::transformer::Progress::Working => (),
                    state => panic!("Unexpected state {state:?}"),
                }
            }
        }
        // An interrupted token must not make partial KV rows part of the prefix.
        engine.begin(next).unwrap();
        // Advance beyond the first layer so this exercises partially written KV,
        // not merely an interrupted query projection before any cache writes.
        for _ in 0..8000 {
            engine.step().unwrap();
        }
        engine.cancel();
        assert_eq!(engine.resume_prefix(), prompt.len());
        engine.begin(next).unwrap();
        let warm_next = loop {
            if let qwen::transformer::Progress::Token(value) = engine.step().unwrap() {
                break value;
            }
        };
        let mut replay = prompt.clone();
        replay.push(next);
        engine.reset();
        assert_eq!(engine.resume_prefix(), 0);
        for (index, &token) in replay.iter().enumerate() {
            engine
                .begin_with_prediction(token, index + 1 == replay.len())
                .unwrap();
            loop {
                match engine.step().unwrap() {
                    qwen::transformer::Progress::Token(value) => {
                        assert_eq!(value, warm_next);
                        next = value;
                        break;
                    }
                    qwen::transformer::Progress::Prefilled => break,
                    qwen::transformer::Progress::Working => (),
                    state => panic!("Unexpected cache replay state {state:?}"),
                }
            }
        }
        for _ in 0..8 {
            if next == 151645 {
                break;
            }
            let mut output = [0; 1024];
            let length = tokenizer.decode(next, &mut output).unwrap();
            println!(
                "token={next}, bytes={:?}",
                String::from_utf8_lossy(&output[..length])
            );
            engine.begin(next).unwrap();
            loop {
                if let qwen::transformer::Progress::Token(value) = engine.step().unwrap() {
                    next = value;
                    break;
                }
            }
        }
        println!(
            "Host diagnostic elapsed {:?}; NOT installed-system proof",
            begin.elapsed()
        );
        engine.reset();
        engine.begin(9707).unwrap();
        engine.cancel();
        assert_eq!(
            engine.step().unwrap(),
            qwen::transformer::Progress::Cancelled
        );
    }
    for key in [
        b"general.architecture".as_slice(),
        b"qwen3.block_count",
        b"qwen3.embedding_length",
        b"tokenizer.ggml.pre",
    ] {
        let (kind, mut value) = model.metadata(key).unwrap();
        if kind == 8 {
            println!(
                "{}: {}",
                String::from_utf8_lossy(key),
                String::from_utf8_lossy(value.string().unwrap())
            );
        } else {
            println!("{}: {}", String::from_utf8_lossy(key), value.u32().unwrap());
        }
    }
    for name in [
        b"token_embd.weight".as_slice(),
        b"blk.0.attn_q.weight",
        b"blk.0.attn_q_norm.weight",
        b"blk.0.ffn_down.weight",
        b"output.weight",
    ] {
        let tensor = model.tensor(name).unwrap();
        println!(
            "{} {:?} type={}",
            String::from_utf8_lossy(name),
            tensor.dimensions,
            tensor.kind
        );
        let mut row = vec![0f32; tensor.dimensions[0]];
        qwen::quant::row(tensor, 0, &mut row).unwrap();
        assert!(row.iter().all(|v| v.is_finite()));
        assert!(row.iter().any(|v| *v != 0.0));
    }
    assert!(qwen::gguf::Model::parse(&bytes[..bytes.len() - 1024]).is_err());
    assert_eq!(qwen::quant::half(0x3c00), 1.0);
    assert_eq!(qwen::quant::half(0xbc00), -1.0);
    assert_eq!(qwen::quant::half(1), 2f32.powi(-24));
}
