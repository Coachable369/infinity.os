// ------------------------=
// FUNC: main
// DESC: Links the real native Qwen CPU kernels for every host harness including nested worker tests.
// ------------------=
fn main() {
    let source = "../../kernel/runtime/ai/qwen/cpu_math.c";
    println!("cargo:rerun-if-changed={source}");
    cc::Build::new()
        .file(source)
        .opt_level(3)
        .flag_if_supported("-ffp-contract=off")
        .compile("infinity_qwen_cpu_math");
}
