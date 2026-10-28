//! One shared budget for installed package storage and the native guest mount.
//! These are explicit resource limits, not claims of package incompatibility.
pub const ARCHIVE_BYTES: usize = 32 * 1024 * 1024;
pub const STAGING_BYTES: usize = 64 * 1024 * 1024;
pub const FILE_BYTES: usize = 16 * 1024 * 1024;
pub const ENTRIES: usize = 2048;
pub const EXTENSION_SNAPSHOT_BYTES: usize = 16 * 1024 * 1024;
