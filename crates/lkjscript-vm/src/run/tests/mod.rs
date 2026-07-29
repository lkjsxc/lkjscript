use super::*;

use lkjscript_core::{
    validate_chunk, Chunk, Constant, ExecutionConfig, ExecutionOutcome, Op, ResourceLimitKind,
    ValidationLimits,
};

use super::{NoTier as NullJit, Vm};

fn validated(ops: &[Op]) -> lkjscript_core::ValidatedChunk {
    let mut chunk = Chunk::new();
    for op in ops {
        chunk.main.emit(*op);
    }
    validate(chunk)
}

fn validate(chunk: Chunk) -> lkjscript_core::ValidatedChunk {
    validate_chunk(chunk, &ValidationLimits::default()).expect("test chunk validates")
}

mod capabilities;
mod enums;
mod execution;
mod resources;
mod symbols;
mod teardown;
