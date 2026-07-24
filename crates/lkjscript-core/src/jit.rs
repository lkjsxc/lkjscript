//! PLACEHOLDER: observation-only seam for future native-code experiments.

use crate::ValidatedChunk;

/// PLACEHOLDER: observes interpreted calls but cannot compile or execute code.
///
/// A real JIT contract must replace this trait with callable compiled-code
/// objects, execution transfer, failure behavior, and deoptimization/fallback.
pub trait JitHook {
    fn observe_call(&mut self, _chunk: &ValidatedChunk, _proto: u32) {}
}

#[derive(Debug, Default)]
pub struct NullJit;

impl JitHook for NullJit {}
