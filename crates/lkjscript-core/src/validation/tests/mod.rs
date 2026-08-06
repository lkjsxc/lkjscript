#![allow(clippy::expect_used)]

use super::*;
use crate::{FunctionProto, Op, ProductFieldRef, ProductMetadata, ValidationPolicy};

fn unit_chunk() -> Chunk {
    let mut chunk = Chunk::new();
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    chunk
}

fn index_instruction(op: Op, index: u64) -> Vec<u8> {
    let mut code = vec![op as u8];
    code.extend_from_slice(&index.to_le_bytes());
    code
}

fn error(chunk: Chunk) -> String {
    validate_chunk(chunk, ValidationPolicy::Unrestricted)
        .expect_err("chunk must fail validation")
        .to_string()
}

mod control;
mod copy_calls;
mod enums;
mod failure_cleanup;
mod metadata;
mod product_metadata;
mod resource_calls;
mod signature_metadata;
mod structural;
mod types;
mod unique;
mod wide_operands;
