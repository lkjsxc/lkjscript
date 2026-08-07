//! Stack VM with validated input, structured outcomes, and bounded execution.

mod host;
mod host_bytes;
mod host_ext;
mod host_term;
mod run;

use std::time::Instant;

use lkjscript_core::{CapabilityKind, ExecutionOutcome, ExecutionPolicy, ValidatedChunk};

pub use run::Vm;

#[derive(Clone, Debug, Default)]
pub struct ExecutionInputs {
    pub arguments: Vec<String>,
    pub capabilities: Vec<CapabilityKind>,
    pub host: lkjscript_host::HostEnvironment,
}

pub fn run_chunk(
    chunk: &ValidatedChunk,
    inputs: &ExecutionInputs,
    config: &ExecutionPolicy,
) -> ExecutionOutcome {
    run_chunk_from_start(chunk, inputs, config, Instant::now())
}

#[doc(hidden)]
pub fn run_chunk_from_start(
    chunk: &ValidatedChunk,
    inputs: &ExecutionInputs,
    config: &ExecutionPolicy,
    started: Instant,
) -> ExecutionOutcome {
    Vm::new_started(chunk, inputs.clone(), config.clone(), started).run()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use lkjscript_core::{validate_chunk, Chunk, Constant, ExecutionOutcome, Op, ValidationPolicy};

    use super::run_chunk;

    #[test]
    fn explicit_trap_opcode_returns_a_structured_vm_trap() {
        let mut chunk = Chunk::new();
        chunk
            .constants
            .push(Constant::Str("explicit SSA trap".into()));
        chunk.main.emit_op_u64(Op::LoadConst, 0);
        chunk.main.emit(Op::Trap);
        let chunk = validate_chunk(chunk, ValidationPolicy::Unrestricted).expect("validate trap");
        match run_chunk(
            &chunk,
            &crate::ExecutionInputs::default(),
            &lkjscript_core::ExecutionPolicy::unrestricted(),
        ) {
            ExecutionOutcome::Trapped(trap) => assert_eq!(trap.as_str(), "explicit SSA trap"),
            other => panic!("unexpected trap outcome: {other:?}"),
        }
    }
}
