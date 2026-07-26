use lkjscript_jit::JitStats;

use crate::metrics_json::string;

pub fn render(stats: &JitStats) -> String {
    let functions = stats
        .functions
        .iter()
        .map(|function| {
            format!(
                concat!(
                    "{{\"id\":{},\"name\":{},\"state\":{},\"calls\":{},",
                    "\"attempts\":{},\"failure\":{},\"code_object\":{},",
                    "\"epoch\":{},\"native_entries\":{}}}"
                ),
                function.function().raw(),
                string(function.name()),
                string(&format!("{:?}", function.state())),
                function.call_count(),
                function.attempts(),
                function.last_failure().map_or_else(
                    || "null".to_string(),
                    |failure| string(&format!("{failure:?}"))
                ),
                function
                    .code_object()
                    .map_or_else(|| "null".to_string(), |id| id.to_string()),
                function.epoch(),
                function.native_entries(),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let objects = stats
        .code_objects
        .iter()
        .map(|object| {
            let optimization = object.optimization_stats.unwrap_or_default();
            format!(
                concat!(
                    "{{\"identity\":{},\"tier\":{},\"functions\":{},\"code_bytes\":{},",
                    "\"metadata_bytes\":{},\"optimization_metadata_bytes_estimate\":{},",
                    "\"accounted_allocation_bytes\":{},\"relocations\":{},\"safepoints\":{},",
                    "\"optimization_ns\":{},\"lowering_encoding_ns\":{},",
                    "\"installation_ns\":{},\"work_units\":{},",
                    "\"optimization_work_units\":{},\"input_instructions\":{},",
                    "\"output_instructions\":{},\"instruction_growth\":{},",
                    "\"cleanup_removed_instructions\":{},\"iterations\":{},",
                    "\"optimizing_passes\":{},\"discovery_passes\":{},",
                    "\"checker_passes\":{},\"reconstruction_passes\":{},",
                    "\"cleanup_passes\":{},\"validation_passes\":{},",
                    "\"certificate_records\":{},\"certificate_bytes_estimate\":{},",
                    "\"algebraic_rewrites\":{},\"gvn_rewrites\":{},",
                    "\"checked_i64_rewrites\":{},\"native_entries\":{},",
                    "\"wx_verified\":{}}}"
                ),
                object.identity,
                string(&format!("{:?}", object.tier)),
                object.functions.len(),
                object.code_bytes,
                object.metadata_bytes,
                object.optimization_metadata_bytes_estimate,
                object.accounted_allocation_bytes,
                object.relocation_count,
                object.safepoint_count,
                object.compile_stats.optimization().as_nanos(),
                object.compile_stats.lowering_and_encoding().as_nanos(),
                object.compile_stats.installation().as_nanos(),
                object.compile_stats.work_units(),
                optimization.work_units,
                optimization.input_instructions,
                optimization.output_instructions,
                optimization.instruction_growth,
                optimization.cleanup_removed_instructions,
                optimization.iterations,
                optimization.optimizing_passes,
                optimization.discovery_passes,
                optimization.checker_passes,
                optimization.reconstruction_passes,
                optimization.cleanup_passes,
                optimization.validation_passes,
                optimization.certificate_records,
                optimization.certificate_bytes_estimate,
                optimization.algebraic_rewrites,
                optimization.gvn_rewrites,
                optimization.checked_i64_rewrites,
                object.native_entry_count,
                object.wx_transition_verified,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        concat!(
            "{{\"compile_failures\":{},\"vm_fallbacks\":{},\"native_entries\":{},",
            "\"baseline_native_entries\":{},\"optimizing_native_entries\":{},",
            "\"baseline_code_objects\":{},\"optimizing_code_objects\":{},",
            "\"optimizing_passes\":{},\"optimization_discovery_passes\":{},",
            "\"optimization_checker_passes\":{},\"optimization_reconstruction_passes\":{},",
            "\"optimization_cleanup_passes\":{},\"optimization_validation_passes\":{},",
            "\"optimization_certificate_records\":{},",
            "\"optimization_certificate_bytes_estimate\":{},\"algebraic_rewrites\":{},",
            "\"gvn_rewrites\":{},\"checked_i64_rewrites\":{},\"direct_native_calls\":{},",
            "\"poll_calls\":{},\"native_invocations\":{},\"auto_threshold\":{},",
            "\"auto_enabled\":{},\"code_cache_peak_objects\":{},",
            "\"code_cache_peak_bytes\":{},\"metadata_cache_peak_bytes\":{},",
            "\"accounted_allocation_peak_bytes\":{},\"allocations\":{},",
            "\"allocation_bytes_estimate\":{},\"collections\":{},",
            "\"peak_live_heap_bytes_estimate\":{},\"maximum_roots\":{},",
            "\"runtime_heap_attempts\":{},\"runtime_heap_successes\":{},",
            "\"barrier_count\":{},\"peak_native_frame_depth\":{},",
            "\"vm_to_native_transitions\":{},\"native_to_vm_transitions\":{},",
            "\"functions\":[{}],\"objects\":[{}]}}"
        ),
        stats.compile_failures,
        stats.vm_fallbacks,
        stats.native_entries,
        stats.baseline_native_entries,
        stats.optimizing_native_entries,
        stats.baseline_code_objects,
        stats.optimizing_code_objects,
        stats.optimizing_passes,
        stats.optimization_discovery_passes,
        stats.optimization_checker_passes,
        stats.optimization_reconstruction_passes,
        stats.optimization_cleanup_passes,
        stats.optimization_validation_passes,
        stats.optimization_certificate_records,
        stats.optimization_certificate_bytes_estimate,
        stats.algebraic_rewrites,
        stats.gvn_rewrites,
        stats.checked_i64_rewrites,
        stats.direct_native_calls,
        stats.poll_calls,
        stats.native_invocations,
        stats.auto_threshold,
        stats.auto_enabled,
        stats.code_cache_peak_objects,
        stats.code_cache_peak_bytes,
        stats.metadata_cache_peak_bytes,
        stats.accounted_allocation_peak_bytes,
        stats.allocations,
        stats.allocation_bytes_estimate,
        stats.collections,
        stats.peak_live_heap_bytes_estimate,
        stats.maximum_roots,
        stats.runtime_heap_attempts,
        stats.runtime_heap_successes,
        stats.barrier_count,
        stats.peak_native_frame_depth,
        stats.vm_to_native_transitions,
        stats.native_to_vm_transitions,
        functions,
        objects,
    )
}
