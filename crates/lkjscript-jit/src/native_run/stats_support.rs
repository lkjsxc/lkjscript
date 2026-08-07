use crate::*;

impl NativeRun {
    pub(super) fn optimization_totals(&self) -> OptimizationStats {
        self.object
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
            })
    }
}
