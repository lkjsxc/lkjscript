//! Dense bytecode opcodes and their centralized decode/stack metadata.

mod decode;
mod metadata;
mod model;
mod stack;

pub use decode::{DecodedInstruction, DecodedOperand};
pub use metadata::{ControlFlow, OpInfo, OperandLayout, StackEffect};
pub use model::Op;

#[cfg(test)]
mod tests;
