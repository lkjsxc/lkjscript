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
    pub static_string_artifacts: u64,
    pub final_empty: bool,
}

impl EvalStructuralObservation {
    pub(crate) fn capture(session: &EvaluatorStructuralSession, final_empty: bool) -> Self {
        Self {
            metrics: session.runtime.metrics(),
            roots: session.runtime.root_stats(),
            events: session.runtime.events().iter().copied().collect(),
            destination_cleanup: session.runtime.cleanup_reports().cloned().collect(),
            static_string_artifacts: session.static_string_count() as u64,
            final_empty,
        }
    }

    pub fn assert_empty(&self) -> Result<(), String> {
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
