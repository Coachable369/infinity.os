//! Native resource inventory and placement policy. Namespace and object identity
//! are deliberately absent from physical-resource selection.
pub mod resources;
pub mod placement;
pub mod replica;
pub mod manifest;
#[cfg(test)]
mod tests;
