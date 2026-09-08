//! Exact production modules, independently instantiated inside each native guest.
#[path = "../../kernel/runtime/capability.rs"]
pub mod capability;
#[path = "../../kernel/runtime/crypto.rs"]
pub mod crypto;
#[path = "../../kernel/runtime/execution.rs"]
pub mod execution;
#[path = "../../kernel/runtime/network/mod.rs"]
pub mod network;
#[path = "../../kernel/runtime/node/mod.rs"]
pub mod node;
