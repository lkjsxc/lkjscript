use lkjscript_resource::{TaskExecutor, TaskId};

use super::grants::DiscoveryGrant;
use crate::optimize::*;
use crate::Program;

pub(super) struct FunctionDiscovery<'a> {
    pub(super) function: usize,
    pub(super) indexes: DiscoveryFunctionIndexes<'a>,
    pub(super) records: Vec<OptimizationCertificateRecord>,
    pub(super) work: u64,
}

pub(super) struct DiscoveryExecutor<'a> {
    pub(super) program: &'a Program,
    pub(super) grants: &'a [DiscoveryGrant],
    pub(super) limits: OptimizationLimits,
}

impl<'a> TaskExecutor for DiscoveryExecutor<'a> {
    type Output = FunctionDiscovery<'a>;
    type Error = OptimizationError;

    fn execute(
        &self,
        task: TaskId,
        _worker: lkjscript_resource::WorkerId,
    ) -> Result<Self::Output, Self::Error> {
        let function = task.slot as usize;
        let input = self
            .program
            .functions
            .get(function)
            .ok_or_else(|| input_index_error("scheduled discovery task ID is stale"))?;
        let grant = self
            .grants
            .get(function)
            .ok_or_else(|| input_index_error("scheduled discovery grant is missing"))?;
        let mut budget = Budget::new(grant.limits(self.limits));
        let indexes = DiscoveryFunctionIndexes::build(input, &mut budget)?;
        let records = discover_function_edits(input, &indexes, &mut budget)?;
        Ok(FunctionDiscovery {
            function,
            indexes,
            records,
            work: budget.work,
        })
    }
}
