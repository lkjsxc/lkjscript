use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use lkjscript_compiler::{compile_package_path, compile_package_path_with_metrics, CompileMetrics};
use lkjscript_core::ExecutionPolicy;
use lkjscript_jit::JitConfig;

use crate::engine;
use crate::metrics::{self, MetricReport};
use crate::output;

mod args;

pub fn command(args: &[String]) -> Result<ExitCode, String> {
    let options = args::parse_run(args)?;
    let source = PathBuf::from(&options.file);
    let metrics_enabled = metrics::enabled();
    let (program, compile_metrics) = if metrics_enabled {
        compile_package_path_with_metrics(&source).map_err(|error| error.to_string())?
    } else {
        (
            compile_package_path(&source).map_err(|error| error.to_string())?,
            CompileMetrics::default(),
        )
    };
    let required = program.bytecode().required_capabilities();
    let inputs = lkjscript_vm::ExecutionInputs {
        arguments: options.script_args.clone(),
        capabilities: required.to_vec(),
        host: lkjscript_host::HostEnvironment::portable(),
    };
    let execution_config = trusted_local_execution_policy();
    let jit_config = JitConfig {
        retain_machine_code_diagnostics: output::diagnostics_enabled()
            || env::var_os("LKJSCRIPT_JIT_DUMP_DIR").is_some(),
        collect_metrics: metrics_enabled,
        ..JitConfig::default()
    };
    let engine_started = metrics_enabled.then(Instant::now);
    let execution = engine::execute(
        &program,
        &inputs,
        &execution_config,
        jit_config,
        metrics_enabled,
    )?;
    let engine_duration = engine_started.map_or(Duration::ZERO, |started| started.elapsed());
    if output::diagnostics_enabled() {
        output::print_jit_diagnostics(
            program.ssa(),
            execution.stats.as_ref(),
            execution.path,
            execution.decline.as_ref(),
            execution.native_entered,
        );
    }
    if metrics_enabled {
        metrics::emit(MetricReport {
            execution_path: execution.path,
            decline: execution.decline.as_ref(),
            native_entered: execution.native_entered,
            compile: &compile_metrics,
            native: execution.native_timings,
            native_stats: execution.stats.as_ref(),
            vm_execution: execution.vm_duration,
            execution_total: engine_duration,
            outcome: &execution.outcome,
        })?;
    }
    output::outcome_exit_code(execution.outcome)
}

fn trusted_local_execution_policy() -> ExecutionPolicy {
    ExecutionPolicy::unrestricted()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_local_run_selects_unrestricted_execution() {
        assert_eq!(
            trusted_local_execution_policy(),
            ExecutionPolicy::Unrestricted
        );
    }
}
