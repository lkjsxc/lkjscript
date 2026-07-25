#![allow(clippy::expect_used)]

use super::*;
use crate::{FunctionProto, Op, ProductFieldRef, ProductMetadata, ValidationLimits};

fn unit_chunk() -> Chunk {
    let mut chunk = Chunk::new();
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    chunk
}

fn error(chunk: Chunk) -> String {
    validate_chunk(chunk, &ValidationLimits::default())
        .expect_err("chunk must fail validation")
        .to_string()
}

mod control;
mod metadata;
mod types;
