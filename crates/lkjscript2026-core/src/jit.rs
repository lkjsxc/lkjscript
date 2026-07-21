//! Future JIT insertion point. Sprint 4 ships the stub only.

use crate::chunk::Chunk;

/// Called by the VM before interpreting a hot proto. Default is a no-op.
pub trait JitHook {
    fn maybe_compile(&mut self, _chunk: &Chunk, _proto: u32) -> bool {
        false
    }
}

#[derive(Debug, Default)]
pub struct NullJit;

impl JitHook for NullJit {}
