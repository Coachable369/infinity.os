use std::{env, fs, path::PathBuf, process::Command};
// ------------------------=
// FUNC: main
// DESC: Generates real bindings then executes layout assertions and Rust-to-C++ bitfield checks.
// ------------------=
fn main() {
    assert_eq!(env::var("INFINITY_BUILD_KIT_ACTIVE").as_deref(), Ok("1"));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().to_owned();
    let fixture = root.join("tools/servo-bindgen-test");
    let output = root.join("build/servo-bindgen-test");
    fs::create_dir_all(&output).unwrap();
    let bindings = bindgen::Builder::default()
        .header(fixture.join("fixture.hpp").to_str().unwrap())
        .clang_args(["-x", "c++", "-std=c++20", "-nostdinc++", "-isystem", "/opt/homebrew/opt/llvm/include/c++/v1"])
        .opaque_type("std::.*")
        .allowlist_type("Bits|Packed|Plain")
        .generate().expect("Recursive libc++ templates must not break binding generation");
    let mut rust = bindings.to_string();
    rust.push_str(r#"
extern "C" { fn check_bits(p: *const Bits) -> bool; fn check_packed(p: *const Packed) -> bool; }
// ------------------------=
// FUNC: cpp_bitfield_roundtrip
// DESC: Requires C++ to observe values written through generated Rust setters.
// ------------------=
#[test] fn cpp_bitfield_roundtrip() { unsafe {
    let mut bits: Bits = core::mem::zeroed();
    bits.set_a(5); bits.set_b(19); bits.set_c(301);
    assert!(check_bits(&bits));
    let mut packed: Packed = core::mem::zeroed();
    packed.set_a(6); packed.set_b(17); packed.set_c(400);
    assert!(check_packed(&packed));
} }
"#);
    let source = output.join("bindings-test.rs");
    fs::write(&source, rust).unwrap();
    let object = output.join("fixture.o");
    assert!(Command::new("/opt/homebrew/opt/llvm/bin/clang++")
        .args(["-std=c++20", "-c"]).arg(fixture.join("fixture.cpp")).arg("-o").arg(&object)
        .status().unwrap().success());
    let binary = output.join("binding-test");
    assert!(Command::new("rustc").arg("--test").arg(&source).arg("-o").arg(&binary)
        .arg("-C").arg(format!("link-arg={}", object.display())).status().unwrap().success());
    assert!(Command::new(binary).status().unwrap().success());
}
