# x86 kernel support

The BIOS bootstrap invokes the common Rust kernel through the 32-bit C ABI.
`output.rs` owns COM1 initialization and the protected-mode `hlt` idle loop.
