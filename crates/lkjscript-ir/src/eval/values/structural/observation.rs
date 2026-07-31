use lkjscript_core::{
    DestinationCleanupReport, StructuralEvent, StructuralRootTableStats,
    StructuralValueRuntimeMetrics,
};

use super::EvaluatorStructuralSession;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EvalStructuralObservation {
    pub metrics: StructuralValueRuntimeMetrics,
    pub roots: StructuralRootTableStats,
    pub events: Vec<StructuralEvent>,
    pub destination_cleanup: Vec<DestinationCleanupReport>,
    pub static_string_artifacts: u32,
    pub collector_allocations: u64,
    pub collector_collections: u64,
    pub collector_roots: u64,
    pub collector_barriers: u64,
    pub final_empty: bool,
}

impl EvalStructuralObservation {
    pub(crate) fn capture(session: &EvaluatorStructuralSession, final_empty: bool) -> Self {
        Self {
            metrics: session.runtime.metrics(),
            roots: session.runtime.root_stats(),
            events: session.runtime.events().iter().copied().collect(),
            destination_cleanup: session.runtime.cleanup_reports().cloned().collect(),
            static_string_artifacts: u32::try_from(session.static_string_count())
                .unwrap_or(u32::MAX),
            collector_allocations: 0,
            collector_collections: 0,
            collector_roots: 0,
            collector_barriers: 0,
            final_empty,
        }
    }

    pub fn assert_collector_free_and_empty(&self) -> Result<(), String> {
        if self.collector_allocations != 0
            || self.collector_collections != 0
            || self.collector_roots != 0
            || self.collector_barriers != 0
        {
            return Err("evaluator structural execution observed collector use".into());
        }
        if !self.final_empty
            || self.roots.live_roots != 0
            || self.roots.live_loans != 0
            || self.metrics.live_objects != 0
            || self.metrics.live_views != 0
            || self.metrics.live_destinations != 0
            || self.metrics.release_backlog != 0
        {
            return Err("evaluator structural execution retained live state".into());
        }
        Ok(())
    }
}
