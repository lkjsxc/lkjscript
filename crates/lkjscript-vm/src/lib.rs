//! Stack VM with validated input, structured outcomes, and bounded execution.

mod host;
mod host_buf;
mod host_ext;
mod host_term;
mod run;

use lkjscript_core::{CapabilityKind, ExecutionConfig, ExecutionOutcome, ValidatedChunk};
use lkjscript_jit::{JitSession, JitStats};

pub use run::{NoTier, Vm};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExecutionInputs {
    pub arguments: Vec<String>,
    pub capabilities: Vec<CapabilityKind>,
}

pub fn run_chunk(
    chunk: &ValidatedChunk,
    inputs: &ExecutionInputs,
    config: &ExecutionConfig,
) -> ExecutionOutcome {
    Vm::new(chunk, NoTier, inputs.clone(), config.clone()).run()
}

pub fn run_chunk_auto(
    chunk: &ValidatedChunk,
    inputs: &ExecutionInputs,
    config: &ExecutionConfig,
    session: JitSession,
) -> (ExecutionOutcome, JitStats) {
    Vm::new(chunk, session, inputs.clone(), config.clone()).run_auto()
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
        chunk.main.emit_op_u16(Op::LoadConst, 0);
        chunk.main.emit(Op::Trap);
        let chunk = validate_chunk(chunk, &ValidationLimits::default()).expect("validate trap");
        match run_chunk(
            &chunk,
            &crate::ExecutionInputs::default(),
            &lkjscript_core::ExecutionConfig::default(),
        ) {
            ExecutionOutcome::Trapped(trap) => assert_eq!(trap.as_str(), "explicit SSA trap"),
            other => panic!("unexpected trap outcome: {other:?}"),
        }
    }
}
