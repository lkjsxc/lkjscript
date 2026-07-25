//! Dense bytecode opcodes and their centralized decode/stack metadata.

mod decode;
mod metadata;
mod model;
mod stack;

pub use decode::DecodedInstruction;
pub use metadata::{ControlFlow, OpInfo, StackEffect};
pub use model::Op;

#[cfg(test)]
mod tests;
