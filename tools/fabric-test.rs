#![allow(dead_code)]
mod storage;
#[path = "../kernel/storage/fabric.rs"] mod native_fabric;
#[cfg(test)] mod fabric_persistence_tests;
#[path = "../kernel/ui/mod.rs"] mod ui;
#[path = "../kernel/runtime/mod.rs"] mod runtime;
// ------------------------=
// FUNC: output_text
// DESC: Suppresses human diagnostics; test acceptance uses typed results and state only.
// ------------------=
fn output_text(_: &[u8]) {}
// ------------------------=
// FUNC: main
// DESC: Provides the harness entry point; production fabric behavior is exercised by cargo test.
// ------------------=
fn main() {}
