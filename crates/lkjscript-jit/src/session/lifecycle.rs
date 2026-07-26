use crate::*;

impl JitSession {
    pub fn advance_epoch(&mut self, epoch: u64) {
        if epoch == self.config.epoch {
            return;
        }
        self.config.epoch = epoch;
        for record in &mut self.functions {
            record.epoch = epoch;
            if record.state == TierState::Disabled
                && record.attempts < self.config.max_attempts_per_function
            {
                record.state = TierState::Observed;
                record.last_failure = None;
            }
        }
    }

    pub fn invalidate_object(&mut self, identity: u64, next_epoch: u64) -> bool {
        let Some(object) = self
            .objects
            .iter_mut()
            .find(|object| object.identity == identity)
        else {
            return false;
        };
        object.invalidated = true;
        for record in &mut self.functions {
            if record.code_object == Some(identity) {
                record.code_object = None;
                record.state = TierState::Observed;
            }
        }
        self.advance_epoch(next_epoch);
        true
    }

    pub fn stats(&self) -> JitStats {
        let code_cache_peak_objects = u64::try_from(self.objects.len()).unwrap_or(u64::MAX);
        let code_cache_peak_bytes = self.objects.iter().fold(0_u64, |total, object| {
            total.saturating_add(object.accounting.code_bytes())
        });
        let metadata_cache_peak_bytes = self.objects.iter().fold(0_u64, |total, object| {
            total
                .saturating_add(object.accounting.metadata_bytes())
                .saturating_add(optimization_metadata_bytes_estimate(
                    object.optimization_stats.as_ref(),
                ))
        });
        let accounted_allocation_peak_bytes = self.objects.iter().fold(0_u64, |total, object| {
            total.saturating_add(object.accounted_allocation_bytes)
        });
        let baseline_native_entries = self
            .objects
            .iter()
            .filter(|object| object.tier == Tier::Baseline)
            .fold(0_u64, |total, object| {
                total.saturating_add(object.native_entry_count)
            });
        let optimizing_native_entries = self
            .objects
            .iter()
            .filter(|object| object.tier == Tier::Optimizing)
            .fold(0_u64, |total, object| {
                total.saturating_add(object.native_entry_count)
            });
        let baseline_code_objects = self
            .objects
            .iter()
            .filter(|object| object.tier == Tier::Baseline)
            .count() as u64;
        let optimizing_code_objects = self
            .objects
            .iter()
            .filter(|object| object.tier == Tier::Optimizing)
            .count() as u64;
        let optimization_totals = self
            .objects
            .iter()
            .filter_map(|object| object.optimization_stats.as_ref())
            .fold(OptimizationStats::default(), |mut total, stats| {
                total.iterations = total.iterations.saturating_add(stats.iterations);
                total.discovery_passes = total
                    .discovery_passes
                    .saturating_add(stats.discovery_passes);
                total.checker_passes = total.checker_passes.saturating_add(stats.checker_passes);
                total.reconstruction_passes = total
                    .reconstruction_passes
                    .saturating_add(stats.reconstruction_passes);
                total.cleanup_passes = total.cleanup_passes.saturating_add(stats.cleanup_passes);
                total.validation_passes = total
                    .validation_passes
                    .saturating_add(stats.validation_passes);
                total.optimizing_passes = total
                    .optimizing_passes
                    .saturating_add(stats.optimizing_passes);
                total.certificate_records = total
                    .certificate_records
                    .saturating_add(stats.certificate_records);
                total.certificate_bytes_estimate = total
                    .certificate_bytes_estimate
                    .saturating_add(stats.certificate_bytes_estimate);
                total.algebraic_rewrites = total
                    .algebraic_rewrites
                    .saturating_add(stats.algebraic_rewrites);
                total.gvn_rewrites = total.gvn_rewrites.saturating_add(stats.gvn_rewrites);
                total.checked_i64_rewrites = total
                    .checked_i64_rewrites
                    .saturating_add(stats.checked_i64_rewrites);
                total
            });
        JitStats {
            functions: self.functions.clone(),
            code_objects: self
                .objects
                .iter()
                .map(|object| CodeObjectRecord {
                    identity: object.identity,
                    functions: object.functions.clone(),
                    tier: object.tier,
                    versions: object.versions,
                    code_bytes: object.accounting.code_bytes(),
                    metadata_bytes: object.accounting.metadata_bytes(),
                    accounted_allocation_bytes: object.accounted_allocation_bytes,
                    relocation_count: object.relocations.len(),
                    runtime_calls: object.runtime_calls.clone(),
                    numeric_conversion_sites: object.numeric_conversion_sites,
                    safepoint_count: object.safepoints.len(),
                    exact_scalar_stack_maps: object
                        .safepoints
                        .iter()
                        .all(|point| point.stack_map().roots().is_empty()),
                    diagnostic_machine_code: object.diagnostic_machine_code.clone(),
                    compile_stats: object.compile_stats.clone(),
                    optimization_certificate: object.optimization_certificate.clone(),
                    optimization_stats: object.optimization_stats,
                    optimization_metadata_bytes_estimate: optimization_metadata_bytes_estimate(
                        object.optimization_stats.as_ref(),
                    ),
                    invalidated: object.invalidated,
                    native_entry_count: object.native_entry_count,
                    wx_transition_verified: object.wx_transition_verified(),
                })
                .collect(),
            native_entries: self.native_entries,
            direct_native_calls: self.direct_native_calls,
            poll_v1_calls: self.poll_v1_calls,
            vm_fallbacks: self.vm_fallbacks,
            compile_failures: self.compile_failures,
            native_invocations: self.native_invocations,
            time_to_first_native_entry: self.time_to_first_native_entry,
            first_native_call: self.first_native_call,
            native_execution: self.native_execution,
            auto_threshold: self.config.auto_threshold,
            auto_enabled: self.config.auto_enabled,
            code_cache_peak_objects,
            code_cache_peak_bytes,
            metadata_cache_peak_bytes,
            accounted_allocation_peak_bytes,
            allocations: self.heap.total_allocations(),
            allocation_bytes_estimate: self.heap.total_allocated_bytes(),
            collections: self.heap.collections(),
            peak_live_heap_bytes_estimate: self.heap.peak_live_heap_bytes(),
            maximum_roots: self.maximum_roots,
            runtime_heap_attempts: self.runtime_heap_attempts,
            runtime_heap_successes: self.runtime_heap_successes,
            barrier_count: self.barrier_count,
            peak_native_frame_depth: self.peak_native_frame_depth,
            vm_to_native_transitions: self.vm_to_native_transitions,
            native_to_vm_transitions: self.native_to_vm_transitions,
            baseline_native_entries,
            optimizing_native_entries,
            baseline_code_objects,
            optimizing_code_objects,
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
