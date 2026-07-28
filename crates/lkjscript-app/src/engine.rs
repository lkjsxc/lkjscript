use std::time::{Duration, Instant};

use lkjscript_compiler::ExecutableProgram;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome};
use lkjscript_jit::{
    execute_forced_with_capabilities, execute_optimizing_with_capabilities, JitConfig, JitSession,
    JitStats,
};
use lkjscript_vm::{run_chunk, run_chunk_auto, ExecutionInputs};

use crate::args::{Engine, RunOptions};

pub struct Execution {
    pub outcome: ExecutionOutcome,
    pub stats: Option<JitStats>,
    pub vm_duration: Duration,
}

pub fn execute(
    options: &RunOptions,
    program: &ExecutableProgram,
    inputs: &ExecutionInputs,
    config: &ExecutionConfig,
    jit_config: JitConfig,
    measure: bool,
) -> Result<Execution, String> {
    let mut vm_duration = Duration::ZERO;
    let (outcome, stats) = match options.engine {
        Engine::Vm => {
            let started = measure.then(Instant::now);
            let outcome = run_chunk(program.bytecode(), inputs, config);
            if let Some(started) = started {
                vm_duration = started.elapsed();
            }
            (outcome, None)
        }
        Engine::BaselineJit => {
            let execution = execute_forced_with_capabilities(
                program.ssa(),
                &inputs.capabilities,
                config,
                jit_config,
            )
            .map_err(|error| format!("engine error: {error}"))?;
            (execution.outcome, Some(execution.stats))
        }
        Engine::OptimizingJit => {
            let execution = execute_optimizing_with_capabilities(
                program.ssa(),
                &inputs.capabilities,
                config,
                jit_config,
            )
            .map_err(|error| format!("engine error: {error}"))?;
            (execution.outcome, Some(execution.stats))
        }
        Engine::Auto => {
            let session = JitSession::new_auto(program.ssa(), program.bytecode_links(), jit_config);
            let started = measure.then(Instant::now);
            let (outcome, stats) = run_chunk_auto(program.bytecode(), inputs, config, session);
            if let Some(started) = started {
                vm_duration = started.elapsed();
            }
            (outcome, Some(stats))
        }
    };
    Ok(Execution {
        outcome,
        stats,
        vm_duration,
    })
}
