use std::env;
use std::path::PathBuf;
use std::time::Duration;

use lkjscript_compiler::CompileMetrics;
use lkjscript_core::ExecutionOutcome;
use lkjscript_jit::BaselineAttemptTimings;

use crate::engine::ExecutionPath;
use crate::metrics_json::{outcome, string};

pub struct MetricReport<'a> {
    pub execution_path: ExecutionPath,
    pub fallback_reason: Option<&'a str>,
    pub native_entered: bool,
    pub compile: &'a CompileMetrics,
    pub native: BaselineAttemptTimings,
    pub vm_execution: Duration,
    pub execution_total: Duration,
    pub outcome: &'a ExecutionOutcome,
}

pub fn enabled() -> bool {
    env::var_os("LKJSCRIPT_METRICS").is_some() || env::var_os("LKJSCRIPT_METRICS_FILE").is_some()
}

pub fn emit(report: MetricReport<'_>) -> Result<(), String> {
    let fallback_reason = report
        .fallback_reason
        .map_or_else(|| "null".to_string(), string);
    let json = format!(
        concat!(
            "{{\"schema\":\"lkjscript.metrics\",\"contract\":\"{contract}\",",
            "\"execution_path\":{execution_path},\"fallback_reason\":{fallback_reason},",
            "\"native_entered\":{native_entered},\"outcome\":{outcome},",
            "\"timings_ns\":{{\"compile_total\":{compile_total},",
            "\"source_loading\":{source_loading},\"parse\":{parsing},",
            "\"hir_analysis\":{hir_analysis},\"effect_analysis\":{effect_analysis},",
            "\"memory_planning\":{memory_planning},",
            "\"package_validation\":{package_validation},",
            "\"ssa_construction\":{ssa_construction},",
            "\"ssa_verification\":{ssa_verification},\"normalization\":{normalization},",
            "\"reference_bytecode_lowering\":{bytecode_lowering},",
            "\"reference_bytecode_validation\":{bytecode_validation},",
            "\"preflight\":{preflight},\"lower\":{lower},\"install\":{install},",
            "\"prepare\":{prepare},\"native\":{native},\"vm\":{vm},",
            "\"total\":{total}}},\"source_files\":{source_files}}}"
        ),
        contract = lkjscript_contracts::METRICS_DIGEST,
        execution_path = string(report.execution_path.as_str()),
        fallback_reason = fallback_reason,
        native_entered = report.native_entered,
        outcome = outcome(report.outcome),
        compile_total = report.compile.total.as_nanos(),
        source_loading = report.compile.source_loading.as_nanos(),
        parsing = report.compile.parsing.as_nanos(),
        hir_analysis = report.compile.hir_analysis.as_nanos(),
        effect_analysis = report.compile.effect_analysis.as_nanos(),
        memory_planning = report.compile.memory_planning.as_nanos(),
        package_validation = report.compile.package_validation.as_nanos(),
        ssa_construction = report.compile.ssa_construction.as_nanos(),
        ssa_verification = report.compile.ssa_verification.as_nanos(),
        normalization = report.compile.normalization.as_nanos(),
        bytecode_lowering = report.compile.bytecode_lowering.as_nanos(),
        bytecode_validation = report.compile.bytecode_validation.as_nanos(),
        preflight = report.native.preflight.as_nanos(),
        lower = report.native.lowering_and_encoding.as_nanos(),
        install = report.native.installation.as_nanos(),
        prepare = report.native.preparation.as_nanos(),
        native = report.native.native_execution.as_nanos(),
        vm = report.vm_execution.as_nanos(),
        total = report.execution_total.as_nanos(),
        source_files = report.compile.source_files,
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
