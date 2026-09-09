//! Native resource inventory and placement policy. Namespace and object identity
//! are deliberately absent from physical-resource selection.
pub mod resources;
pub mod resource_protocol;
pub mod placement;
pub mod replica;
pub mod manifest;
pub mod observed;
pub mod healing;
pub mod deletion;
pub mod metadata;
pub mod metadata_repair;
pub mod metadata_bundle;
#[cfg(test)]
mod tests;
