use std::env;
use std::path::PathBuf;
use std::time::Duration;

use lkjscript_compiler::CompileMetrics;
use lkjscript_core::ExecutionOutcome;
use lkjscript_jit::JitStats;

use crate::args::Engine;
use crate::metrics_jit::render as jit;
use crate::metrics_json::{outcome, string};

pub struct MetricReport<'a> {
    pub engine: Engine,
    pub configured_threshold: u64,
    pub auto_enabled: bool,
    pub compile: &'a CompileMetrics,
    pub vm_execution: Duration,
    pub engine_execution: Duration,
    pub outcome: &'a ExecutionOutcome,
    pub stats: Option<&'a JitStats>,
}

pub fn enabled() -> bool {
    env::var_os("LKJSCRIPT_METRICS").is_some() || env::var_os("LKJSCRIPT_METRICS_FILE").is_some()
}

pub fn emit(report: MetricReport<'_>) -> Result<(), String> {
    let engine = match report.engine {
        Engine::Vm => "vm",
        Engine::Auto => "auto",
        Engine::BaselineJit => "baseline-jit",
        Engine::OptimizingJit => "optimizing-jit",
    };
    let mut optimization = Duration::ZERO;
    let mut native_lowering = Duration::ZERO;
    let mut installation = Duration::ZERO;
    if let Some(stats) = report.stats {
        for object in &stats.code_objects {
            optimization = optimization.saturating_add(object.compile_stats.optimization());
            native_lowering =
                native_lowering.saturating_add(object.compile_stats.lowering_and_encoding());
            installation = installation.saturating_add(object.compile_stats.installation());
        }
    }
    let time_to_first_native = report
        .stats
        .and_then(|stats| stats.time_to_first_native_entry)
        .map_or_else(|| "null".to_string(), duration_ns);
    let first_native = report
        .stats
        .and_then(|stats| stats.first_native_call)
        .map_or_else(|| "null".to_string(), duration_ns);
    let native_execution = report
        .stats
        .map_or(Duration::ZERO, |stats| stats.native_execution);
    let jit = report.stats.map_or_else(|| "null".to_string(), jit);
    let json = format!(
        concat!(
            "{{\"schema\":\"lkjscript.metrics\",\"contract\":\"{contract}\",\"engine\":{engine},",
            "\"configured_auto_threshold\":{configured_threshold},",
            "\"auto_enabled\":{auto_enabled},\"outcome\":{outcome},",
            "\"timings_ns\":{{\"compile_total\":{compile_total},",
            "\"source_loading\":{source_loading},\"parse\":{parsing},",
            "\"hir_analysis\":{hir_analysis},\"effect_analysis\":{effect_analysis},",
            "\"ssa_construction\":{ssa_construction},",
            "\"ssa_verification\":{ssa_verification},\"normalization\":{normalization},",
            "\"reference_bytecode_lowering\":{bytecode_lowering},",
            "\"reference_bytecode_validation\":{bytecode_validation},",
            "\"optimizing_passes\":{optimization},",
            "\"native_lowering_encoding\":{native_lowering},",
            "\"relocation_wx_installation\":{installation},",
            "\"time_to_first_native_entry\":{time_to_first_native},",
            "\"first_native_call\":{first_native},\"native_execution\":{native_execution},",
            "\"vm_execution\":{vm_execution},\"engine_execution\":{engine_execution}}},",
            "\"source_files\":{source_files},\"jit\":{jit}}}"
        ),
        contract = lkjscript_contracts::METRICS_DIGEST,
        engine = string(engine),
        configured_threshold = report.configured_threshold,
        auto_enabled = report.auto_enabled,
        outcome = outcome(report.outcome),
        compile_total = report.compile.total.as_nanos(),
        source_loading = report.compile.source_loading.as_nanos(),
        parsing = report.compile.parsing.as_nanos(),
        hir_analysis = report.compile.hir_analysis.as_nanos(),
        effect_analysis = report.compile.effect_analysis.as_nanos(),
        ssa_construction = report.compile.ssa_construction.as_nanos(),
        ssa_verification = report.compile.ssa_verification.as_nanos(),
        normalization = report.compile.normalization.as_nanos(),
        bytecode_lowering = report.compile.bytecode_lowering.as_nanos(),
        bytecode_validation = report.compile.bytecode_validation.as_nanos(),
        optimization = optimization.as_nanos(),
        native_lowering = native_lowering.as_nanos(),
        installation = installation.as_nanos(),
        time_to_first_native = time_to_first_native,
        first_native = first_native,
        native_execution = native_execution.as_nanos(),
        vm_execution = report.vm_execution.as_nanos(),
        engine_execution = report.engine_execution.as_nanos(),
        source_files = report.compile.source_files,
        jit = jit,
    );
    let line = format!("LKJSCRIPT_METRICS {json}\n");
    if let Some(path) = env::var_os("LKJSCRIPT_METRICS_FILE") {
        std::fs::write(PathBuf::from(path), line)
            .map_err(|error| format!("write metrics file: {error}"))?;
    } else {
        eprint!("{line}");
    }
    Ok(())
}

fn duration_ns(duration: Duration) -> String {
    duration.as_nanos().to_string()
}
