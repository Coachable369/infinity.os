#[path = "../kernel/ui/geometry.rs"]
pub mod geometry;
mod ui {
    pub use crate::geometry;
}
#[path = "../kernel/ui/voice_indicator.rs"]
mod indicator;
