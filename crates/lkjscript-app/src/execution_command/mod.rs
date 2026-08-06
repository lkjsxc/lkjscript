use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use lkjscript_compiler::{compile_path, compile_path_with_metrics, CompileMetrics};
use lkjscript_core::{ExecutionConfig, Limits};
use lkjscript_jit::JitConfig;

use crate::engine;
use crate::metrics::{self, MetricReport};
use crate::output;

mod args;

pub(crate) use args::{Engine, RunOptions};

pub fn command(args: &[String]) -> Result<ExitCode, String> {
    let options = args::parse_run(args)?;
    let source = PathBuf::from(&options.file);
    let (_, manifest) =
        lkjscript_compiler::package::verify(&source).map_err(|error| error.to_string())?;
    let metrics_enabled = metrics::enabled();
    let (program, compile_metrics) = if metrics_enabled {
        compile_path_with_metrics(&source, &Limits::default()).map_err(|error| error.to_string())?
    } else {
        (
            compile_path(&source, &Limits::default()).map_err(|error| error.to_string())?,
            CompileMetrics::default(),
        )
    };
    let required = program.bytecode().required_capabilities();
    for capability in required {
        if manifest
            .capabilities
            .binary_search_by_key(&capability.as_str(), String::as_str)
            .is_err()
        {
            return Err(format!(
                "package does not grant required {} capability",
                capability.as_str()
            ));
        }
    }
    let inputs = lkjscript_vm::ExecutionInputs {
        arguments: options.script_args.clone(),
        capabilities: required.to_vec(),
        host: lkjscript_host::HostEnvironment::portable(),
    };
    let execution_config = ExecutionConfig::default();
    let jit_config = JitConfig {
        auto_threshold: options.auto_threshold,
        auto_enabled: options.auto_enabled,
        retain_machine_code_diagnostics: output::diagnostics_enabled()
            || env::var_os("LKJSCRIPT_JIT_DUMP_DIR").is_some(),
        collect_metrics: metrics_enabled,
        ..JitConfig::default()
    };
    let engine_started = metrics_enabled.then(Instant::now);
    let execution = engine::execute(
        &options,
        &program,
        &inputs,
        &execution_config,
        jit_config,
        metrics_enabled,
    )?;
    let engine_duration = engine_started.map_or(Duration::ZERO, |started| started.elapsed());
    if output::diagnostics_enabled() {
        if let Some(stats) = &execution.stats {
            output::print_jit_diagnostics(program.ssa(), stats);
        }
    }
    if metrics_enabled {
        metrics::emit(MetricReport {
            engine: options.engine,
            configured_threshold: options.auto_threshold,
            auto_enabled: options.auto_enabled,
            compile: &compile_metrics,
            vm_execution: execution.vm_duration,
            engine_execution: engine_duration,
            outcome: &execution.outcome,
            stats: execution.stats.as_ref(),
        })?;
    }
    output::outcome_exit_code(execution.outcome)
}
