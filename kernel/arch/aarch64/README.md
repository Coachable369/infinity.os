# AArch64 kernel support

`output.rs` provides EDK/QEMU PL011 test output and the architecture-native
`wfe` idle loop. Portable visible output is owned by the common framebuffer
renderer.
