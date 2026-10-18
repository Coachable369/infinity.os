//! Native image-inference building blocks. No network, heap, OS or language-model dependencies.
#![no_std]
pub mod flow;
pub mod job;
pub mod tensor;
pub mod attention;
pub mod weights;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Shape,
    NonFinite,
    Bounds,
    Unauthorized,
    Busy,
    NotReady,
    Budget,
    State,
    Cancelled,
}
