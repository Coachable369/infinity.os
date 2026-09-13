//! Allocation-free transport mechanisms for the native HTTP service.
//! Callers must authorize destinations before connect, and cancel on revocation.
//! This module grants no network capabilities and implements no TLS bypass.
#![no_std]

pub mod async_stream;
pub mod body;
pub mod client;
pub mod device;
pub mod https;
pub mod request;
pub mod response;
pub mod tls;
pub mod transport;
pub use smoltcp;

#[cfg(test)]
extern crate std;
