use crate::*;

impl NativeRun {
    pub(crate) fn stats(&self) -> JitStats {
        let optimization_totals = self.optimization_totals();
        JitStats {
            code_objects: self
                .object
                .iter()
                .map(|object| CodeObjectRecord {
                    functions: object.functions.clone(),
                    contracts: object.contracts,
                    code_bytes: object.accounting.code_bytes(),
                    metadata_bytes: object.accounting.metadata_bytes(),
                    accounted_allocation_bytes: object.accounted_allocation_bytes,
                    relocation_count: object.relocations.len(),
                    runtime_calls: object.runtime_calls.clone(),
                    numeric_conversion_sites: object.numeric_conversion_sites,
                    diagnostic_machine_code: object.diagnostic_machine_code.clone(),
                    compile_stats: object.compile_stats.clone(),
                    optimization_certificate: object.optimization_certificate.clone(),
                    optimization_stats: object.optimization_stats,
                    optimization_metadata_bytes_estimate: optimization_metadata_bytes_estimate(
                        object.optimization_stats.as_ref(),
                    ),
                    native_entry_count: object.native_entry_count,
                    wx_transition_verified: object.installed.wx_transition_verified(),
                })
                .collect(),
            native_entries: self.native_entries,
            direct_native_calls: self.direct_native_calls,
            poll_calls: self.poll_calls,
            native_invocations: self.native_invocations,
            time_to_first_native_entry: self.time_to_first_native_entry,
            first_native_call: self.first_native_call,
            native_execution: self.native_execution,
            runtime_heap_attempts: self.runtime_heap_attempts,
            runtime_heap_successes: self.runtime_heap_successes,
            segmented_lists: self.lists.as_ref().map_or_else(
                Default::default,
                lkjscript_core::SegmentedListArena::metrics,
            ),
            segmented_list_reserved_bytes_estimate: self
                .lists
                .as_ref()
                .and_then(|arena| arena.reserved_bytes_estimate().ok()),
            region_products: self
                .region_products
                .as_ref()
                .and_then(|arena| arena.metrics().ok()),
            resource_runtime_calls: self.resource_runtime_calls,
            unique_runtime_calls: self.unique_runtime_calls,
            structural_runtime_calls: self.structural_runtime_calls,
            native_resources: self.native_resources,
            native_unique: self.native_unique,
            native_structural: self.native_structural,
            peak_native_frame_depth: self.peak_native_frame_depth,
            peak_native_stack_bytes: self.peak_native_stack_bytes,
            optimizing_passes: optimization_totals.optimizing_passes,
            optimization_discovery_passes: optimization_totals.discovery_passes,
            optimization_checker_passes: optimization_totals.checker_passes,
            optimization_reconstruction_passes: optimization_totals.reconstruction_passes,
            optimization_cleanup_passes: optimization_totals.cleanup_passes,
            optimization_validation_passes: optimization_totals.validation_passes,
            optimization_certificate_records: optimization_totals.certificate_records,
            optimization_certificate_bytes_estimate: optimization_totals.certificate_bytes_estimate,
            algebraic_rewrites: optimization_totals.algebraic_rewrites,
            gvn_rewrites: optimization_totals.gvn_rewrites,
            checked_i64_rewrites: optimization_totals.checked_i64_rewrites,
        }
    }
}
