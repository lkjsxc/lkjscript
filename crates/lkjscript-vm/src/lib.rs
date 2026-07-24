//! Stack VM with validated input, structured outcomes, and bounded execution.

mod host;
mod host_buf;
mod host_ext;
mod host_term;
mod run;

use lkjscript_core::{ExecutionConfig, ExecutionOutcome, ValidatedChunk};
use lkjscript_jit::{JitSession, JitStats};

pub use run::{NoTier, Vm};

pub fn run_chunk(chunk: &ValidatedChunk, config: &ExecutionConfig) -> ExecutionOutcome {
    run_chunk_with_args(chunk, &[], config)
}

pub fn run_chunk_with_args(
    chunk: &ValidatedChunk,
    args: &[String],
    config: &ExecutionConfig,
) -> ExecutionOutcome {
    Vm::new(chunk, NoTier, args.to_vec(), config.clone()).run()
}

pub fn run_chunk_auto(
    chunk: &ValidatedChunk,
    args: &[String],
    config: &ExecutionConfig,
    session: JitSession,
) -> (ExecutionOutcome, JitStats) {
    Vm::new(chunk, session, args.to_vec(), config.clone()).run_auto()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use lkjscript_core::{validate_chunk, Chunk, Constant, ExecutionOutcome, Op, ValidationLimits};

    use super::run_chunk;

    #[test]
    fn explicit_trap_opcode_returns_a_structured_vm_trap() {
        let mut chunk = Chunk::new();
        chunk
            .constants
            .push(Constant::Str("explicit SSA trap".into()));
        chunk.main.emit_op_u16(Op::Trap, 0);
        let chunk = validate_chunk(chunk, &ValidationLimits::default()).expect("validate trap");
        match run_chunk(&chunk, &lkjscript_core::ExecutionConfig::default()) {
            ExecutionOutcome::Trapped(trap) => assert_eq!(trap.as_str(), "explicit SSA trap"),
            other => panic!("unexpected trap outcome: {other:?}"),
        }
    }
}
