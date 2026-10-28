//! Native resource inventory and placement policy. Namespace and object identity
//! are deliberately absent from physical-resource selection.
pub mod deletion;
pub mod healing;
pub mod manifest;
pub mod metadata;
pub mod metadata_bundle;
pub mod metadata_repair;
pub mod observed;
pub mod placement;
pub mod replica;
pub mod resource_protocol;
pub mod resources;
#[cfg(test)]
mod tests;
