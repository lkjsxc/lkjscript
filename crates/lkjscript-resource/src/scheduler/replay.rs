use std::collections::BTreeMap;

use super::state::legal;
use super::{ReferenceScheduler, SchedulingPolicy, SchedulingTrace};
use crate::{ResourceError, ResourceResult, TaskId, VerifiedTaskGraph};

impl ReferenceScheduler {
    pub fn replay<P: SchedulingPolicy>(
        graph: &VerifiedTaskGraph,
        policy: &P,
        workers: usize,
        failures: &BTreeMap<TaskId, String>,
        trace: &SchedulingTrace,
    ) -> ResourceResult<()> {
        if trace.graph != graph.id() || trace.truncated {
            return Err(ResourceError::new(
                "replay-identity",
                "trace is truncated or for another graph",
            ));
        }
        for event in &trace.events {
            if !legal(event.from, event.to) {
                return Err(ResourceError::new(
                    "replay-transition",
                    "illegal state transition",
                ));
            }
        }
        let expected = Self::run(graph, policy, workers, failures, usize::MAX)?.trace;
        if expected.events != trace.events {
            return Err(ResourceError::new(
                "replay-mismatch",
                "trace does not replay",
            ));
        }
        Ok(())
    }
}
