use std::env;
use std::path::PathBuf;
use std::time::Duration;

use lkjscript_compiler::CompileMetrics;
use lkjscript_core::ExecutionOutcome;
use lkjscript_jit::{BaselineAttemptTimings, BaselineDeclineReason, CodeObjectRecord, JitStats};

use crate::engine::ExecutionPath;
use crate::metrics_json::{outcome, string};

pub struct MetricReport<'a> {
    pub execution_path: ExecutionPath,
    pub decline: Option<&'a BaselineDeclineReason>,
    pub native_entered: bool,
    pub compile: &'a CompileMetrics,
    pub native: BaselineAttemptTimings,
    pub native_stats: Option<&'a JitStats>,
    pub vm_execution: Duration,
    pub execution_total: Duration,
    pub outcome: &'a ExecutionOutcome,
}

pub fn enabled() -> bool {
    env::var_os("LKJSCRIPT_METRICS").is_some() || env::var_os("LKJSCRIPT_METRICS_FILE").is_some()
}

pub fn emit(report: MetricReport<'_>) -> Result<(), String> {
    let native_decline = report
        .decline
        .map_or_else(|| "null".to_string(), decline_json);
    let native = NativeSummary::from_stats(report.native_stats)?;
    let native_artifact = native_artifact_json(native.as_ref());
    let native_runtime = native
        .as_ref()
        .map_or_else(|| "null".to_string(), NativeSummary::runtime_json);
    let json = format!(
        concat!(
            "{{\"schema\":\"lkjscript.metrics\",\"contract\":\"{contract}\",",
            "\"execution_path\":{execution_path},\"native_decline\":{native_decline},",
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
            "\"prepare\":{prepare},\"native\":{native_execution},\"vm\":{vm},",
            "\"total\":{total}}},",
            "\"native_artifact\":{native_artifact},",
            "\"native_runtime\":{native_runtime},",
            "\"source_files\":{source_files}}}"
        ),
        contract = lkjscript_contracts::METRICS_DIGEST,
        execution_path = string(report.execution_path.as_str()),
        native_decline = native_decline,
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
        native_execution = report.native.native_execution.as_nanos(),
        vm = report.vm_execution.as_nanos(),
        total = report.execution_total.as_nanos(),
        native_artifact = native_artifact,
        native_runtime = native_runtime,
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

fn decline_json(decline: &BaselineDeclineReason) -> String {
    let function = decline
        .function()
        .map_or_else(|| "null".to_string(), |function| function.raw().to_string());
    format!(
        "{{\"stage\":{},\"code\":{},\"function\":{function},\"detail\":{}}}",
        string(decline.stage()),
        string(decline.code()),
        string(&decline.detail()),
    )
}

#[derive(Default)]
struct NativeSummary {
    objects: u64,
    functions: u64,
    code_bytes: u64,
    metadata_bytes: u64,
    mapped_bytes: u64,
    work_units: u64,
    relocations: u64,
    native_entries: u64,
    direct_calls: u64,
    native_invocations: u64,
    time_to_first_entry: Option<Duration>,
    peak_frame_depth: u64,
    peak_stack_bytes: u64,
    heap_attempts: u64,
    heap_successes: u64,
    unique_allocations: u64,
    unique_drops: u64,
    unique_cleanup_attempts: u64,
    unique_cleanup_releases: u64,
    unique_live_owners: u64,
    unique_live_loans: u64,
    unique_release_backlog: u64,
    unique_teardown_failures: u64,
    structural_publications: u64,
    structural_live_objects: u64,
    structural_live_roots: u64,
    structural_release_backlog: u64,
    structural_teardown_failures: u64,
}

impl NativeSummary {
    fn from_stats(stats: Option<&JitStats>) -> Result<Option<Self>, String> {
        let Some(stats) = stats else {
            return Ok(None);
        };
        let mut summary = Self {
            objects: usize_to_u64(stats.code_objects.len(), "native object count")?,
            native_entries: stats.native_entries,
            direct_calls: stats.direct_native_calls,
            native_invocations: stats.native_invocations,
            time_to_first_entry: stats.time_to_first_native_entry,
            peak_frame_depth: usize_to_u64(
                stats.peak_native_frame_depth,
                "native peak frame depth",
            )?,
            peak_stack_bytes: usize_to_u64(
                stats.peak_native_stack_bytes,
                "native peak stack bytes",
            )?,
            heap_attempts: stats.runtime_heap_attempts,
            heap_successes: stats.runtime_heap_successes,
            unique_allocations: stats.native_unique.allocations,
            unique_drops: stats.native_unique.drops,
            unique_cleanup_attempts: stats.native_unique.cleanup_attempts,
            unique_cleanup_releases: stats.native_unique.cleanup_releases,
            unique_live_owners: stats.native_unique.live_owners,
            unique_live_loans: stats.native_unique.live_loans,
            unique_release_backlog: stats.native_unique.release_backlog,
            unique_teardown_failures: stats.native_unique.teardown_failures,
            structural_publications: stats.native_structural.roots_published,
            structural_live_objects: stats.native_structural.live_objects,
            structural_live_roots: stats.native_structural.live_roots,
            structural_release_backlog: stats.native_structural.release_backlog,
            structural_teardown_failures: stats.native_structural.teardown_failures,
            ..Self::default()
        };
        for object in &stats.code_objects {
            summary.add_object(object)?;
        }
        Ok(Some(summary))
    }

    fn add_object(&mut self, object: &CodeObjectRecord) -> Result<(), String> {
        self.functions = checked_add(
            self.functions,
            usize_to_u64(object.functions.len(), "native function count")?,
            "native function count",
        )?;
        self.code_bytes = checked_add(self.code_bytes, object.code_bytes, "native code bytes")?;
        self.metadata_bytes = checked_add(
            self.metadata_bytes,
            object.metadata_bytes,
            "native metadata bytes",
        )?;
        self.mapped_bytes = checked_add(
            self.mapped_bytes,
            object.accounted_allocation_bytes,
            "native mapped bytes",
        )?;
        self.work_units = checked_add(
            self.work_units,
            object.compile_stats.work_units(),
            "native work units",
        )?;
        self.relocations = checked_add(
            self.relocations,
            usize_to_u64(object.relocation_count, "native relocation count")?,
            "native relocation count",
        )?;
        Ok(())
    }

    fn artifact_json(&self) -> String {
        format!(
            concat!(
                "{{\"availability\":\"published-installed-object\",",
                "\"objects\":{},\"functions\":{},\"code_bytes\":{},",
                "\"metadata_bytes\":{},\"mapped_bytes\":{},",
                "\"work_units\":{},\"relocations\":{}}}"
            ),
            self.objects,
            self.functions,
            self.code_bytes,
            self.metadata_bytes,
            self.mapped_bytes,
            self.work_units,
            self.relocations,
        )
    }

    fn runtime_json(&self) -> String {
        format!(
            concat!(
                "{{\"counter_semantics\":\"saturating\",",
                "\"entries\":{},\"direct_calls\":{},\"invocations\":{},",
                "\"time_to_first_entry_ns\":{},\"peak_frame_depth\":{},",
                "\"peak_stack_bytes\":{},\"heap_attempts\":{},",
                "\"heap_successes\":{},\"unique_allocations\":{},\"unique_drops\":{},",
                "\"unique_cleanup_attempts\":{},\"unique_cleanup_releases\":{},",
                "\"unique_live_owners\":{},\"unique_live_loans\":{},",
                "\"unique_release_backlog\":{},\"unique_teardown_failures\":{},",
                "\"structural_publications\":{},\"structural_live_objects\":{},",
                "\"structural_live_roots\":{},\"structural_release_backlog\":{},",
                "\"structural_teardown_failures\":{}}}"
            ),
            self.native_entries,
            self.direct_calls,
            self.native_invocations,
            optional_duration(self.time_to_first_entry),
            self.peak_frame_depth,
            self.peak_stack_bytes,
            self.heap_attempts,
            self.heap_successes,
            self.unique_allocations,
            self.unique_drops,
            self.unique_cleanup_attempts,
            self.unique_cleanup_releases,
            self.unique_live_owners,
            self.unique_live_loans,
            self.unique_release_backlog,
            self.unique_teardown_failures,
            self.structural_publications,
            self.structural_live_objects,
            self.structural_live_roots,
            self.structural_release_backlog,
            self.structural_teardown_failures,
        )
    }
}

fn native_artifact_json(summary: Option<&NativeSummary>) -> String {
    summary
        .filter(|summary| summary.objects != 0)
        .map_or_else(|| "null".to_string(), NativeSummary::artifact_json)
}

fn optional_duration(duration: Option<Duration>) -> String {
    duration.map_or_else(|| "null".to_string(), |value| value.as_nanos().to_string())
}

fn usize_to_u64(value: usize, label: &str) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| format!("{label} exceeds metric representation"))
}

fn checked_add(left: u64, right: u64, label: &str) -> Result<u64, String> {
    left.checked_add(right)
        .ok_or_else(|| format!("{label} exceeds metric representation"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_or_unpublished_native_artifact_is_not_reported_as_zero() {
        assert_eq!(native_artifact_json(None), "null");
        assert_eq!(
            native_artifact_json(Some(&NativeSummary::default())),
            "null"
        );
    }
}
