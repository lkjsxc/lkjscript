mod access;
mod shape;

use crate::{GraphLimits, ResourceResult, UnverifiedTaskGraph, VerifiedTaskGraph};

pub struct TaskGraphVerifier;

impl TaskGraphVerifier {
    pub fn verify(
        graph: UnverifiedTaskGraph,
        limits: GraphLimits,
    ) -> ResourceResult<VerifiedTaskGraph> {
        shape::verify_bounds(&graph, limits)?;
        shape::verify_ids(&graph)?;
        shape::verify_scopes(&graph)?;
        let reachability = access::verify_dependencies(&graph)?;
        access::verify_accesses(&graph.tasks, &reachability)?;
        Ok(VerifiedTaskGraph::new(graph.scopes, graph.tasks))
    }
}
