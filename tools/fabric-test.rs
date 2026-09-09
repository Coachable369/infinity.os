#![allow(dead_code)]
mod storage;
#[path = "../kernel/drivers/input/buffer.rs"] mod input_buffer;
#[path = "../kernel/storage/fabric.rs"] mod native_fabric;
#[path = "../kernel/storage/fabric_pool_metadata.rs"] mod fabric_pool_metadata;
#[path = "../kernel/storage/fabric_pool_metadata_service.rs"] mod fabric_pool_metadata_service;
#[path = "../kernel/storage/fabric_pool_repair.rs"] mod fabric_pool_repair;
#[cfg(test)] mod fabric_persistence_tests;
#[path = "../kernel/storage/install_identity.rs"] mod install_identity;
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
