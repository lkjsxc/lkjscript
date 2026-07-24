//! Stack VM with validated input, structured outcomes, and bounded execution.

mod arena;
mod host;
mod host_buf;
mod host_ext;
mod host_term;
mod run;

use lkjscript_core::{ExecutionConfig, ExecutionOutcome, JitHook, NullJit, ValidatedChunk};

pub use run::Vm;

pub fn run_chunk(chunk: &ValidatedChunk, config: &ExecutionConfig) -> ExecutionOutcome {
    run_chunk_with_args(chunk, &[], config)
}

pub fn run_chunk_with_args(
    chunk: &ValidatedChunk,
    args: &[String],
    config: &ExecutionConfig,
) -> ExecutionOutcome {
    Vm::new(chunk, NullJit, args.to_vec(), config.clone()).run()
}

pub fn run_chunk_with_jit<J: JitHook>(
    chunk: &ValidatedChunk,
    jit: J,
    config: &ExecutionConfig,
) -> ExecutionOutcome {
    Vm::new(chunk, jit, Vec::new(), config.clone()).run()
}
