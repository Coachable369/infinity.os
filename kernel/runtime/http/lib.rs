//! Allocation-free transport mechanisms for the native HTTP service.
//! Callers must authorize destinations before connect, and cancel on revocation.
//! This module grants no network capabilities and implements no TLS bypass.
#![no_std]

pub mod async_stream;
pub mod body;
pub mod client;
pub mod clock;
pub mod device;
pub mod dhcp;
pub mod egress;
pub mod geturl;
pub mod https;
pub mod request;
pub mod response;
pub mod selector;
pub mod sockets;
mod rsa;
pub mod task;
pub mod tls;
pub mod transport;
pub use rand_chacha;
pub use rand_core;
pub use smoltcp;

#[cfg(test)]
extern crate std;
