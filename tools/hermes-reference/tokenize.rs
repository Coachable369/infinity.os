//! Read-only production-tokenizer diagnostic; optional output is restricted to build/.
#[path = "../../kernel/runtime/ai/qwen/gguf.rs"]
mod gguf;
#[path = "../../kernel/runtime/ai/qwen/tokenizer.rs"]
mod tokenizer;

// ------------------------=
// FUNC: main
// DESC: Dumps the exact native service ChatML token sequence and compares it with upstream structured token IDs when supplied.
// ------------------=
fn main() {
    let args:Vec<_>=std::env::args().collect();
    assert!((4..=5).contains(&args.len()),"usage: native-tokenize MODEL PROMPT OUTPUT [REFERENCE_IDS]");
    assert!(args[3].starts_with("build/") && !args[3].contains(".."));
    let bytes=std::fs::read(&args[1]).unwrap();
    let text=std::fs::read_to_string(&args[2]).unwrap();
    let model=gguf::Model::parse(&bytes).unwrap();
    let count=|key:&[u8]| {
        let (kind,mut reader)=model.metadata(key).unwrap();
        assert_eq!(kind,9);assert_eq!(reader.u32().unwrap(),8);
        reader.u64().unwrap() as usize
    };
    let mut vocabulary=vec![tokenizer::EMPTY;count(b"tokenizer.ggml.tokens")];
    let mut merges=vec![tokenizer::EMPTY;count(b"tokenizer.ggml.merges")];
    let tokenizer=tokenizer::Tokenizer::load(model,&mut vocabulary,&mut merges).unwrap();
    let mut ids=Vec::new();let mut output=vec![0;4096];
    ids.push(tokenizer.special(b"<|im_start|>").unwrap());
    let n=tokenizer.encode("user\n",&mut output).unwrap();ids.extend_from_slice(&output[..n]);
    let n=tokenizer.encode(&text,&mut output).unwrap();ids.extend_from_slice(&output[..n]);
    ids.push(tokenizer.special(b"<|im_end|>").unwrap());
    let n=tokenizer.encode("\n",&mut output).unwrap();ids.extend_from_slice(&output[..n]);
    ids.push(tokenizer.special(b"<|im_start|>").unwrap());
    let n=tokenizer.encode("assistant\n",&mut output).unwrap();ids.extend_from_slice(&output[..n]);
    let encoded=ids.iter().map(|id|format!("{id}\n")).collect::<String>();
    std::fs::write(&args[3],encoded).unwrap();
    println!("native prompt tokens={}",ids.len());
    if let Some(path)=args.get(4) {
        let reference:Vec<u32>=std::fs::read_to_string(path).unwrap().lines().map(|line|line.parse().unwrap()).collect();
        if ids!=reference {
            let first=ids.iter().zip(&reference).position(|(left,right)|left!=right).unwrap_or(ids.len().min(reference.len()));
            panic!("Tokenizer mismatch at {first}: native length={} reference length={} native={:?} reference={:?}",
                ids.len(),reference.len(),ids.get(first),reference.get(first));
        }
        println!("native and upstream token ID arrays match");
    }
}
