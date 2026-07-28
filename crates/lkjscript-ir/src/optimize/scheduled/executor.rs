use lkjscript_resource::{TaskExecutor, TaskId};

use super::graph::FunctionRange;
use crate::optimize::*;
use crate::Program;

pub(super) struct FunctionDiscovery<'a> {
    pub(super) indexes: DiscoveryFunctionIndexes<'a>,
    pub(super) records: Vec<OptimizationCertificateRecord>,
}

pub(super) struct RangeDiscovery<'a> {
    pub(super) start: usize,
    pub(super) functions: Vec<FunctionDiscovery<'a>>,
    pub(super) work: u64,
}

pub(super) struct DiscoveryExecutor<'a> {
    pub(super) program: &'a Program,
    pub(super) ranges: &'a [FunctionRange],
    pub(super) limits: OptimizationLimits,
}

impl<'a> TaskExecutor for DiscoveryExecutor<'a> {
    type Output = RangeDiscovery<'a>;
    type Error = OptimizationError;

    fn execute(&self, task: TaskId) -> Result<Self::Output, Self::Error> {
        let range = self
            .ranges
            .get(task.slot as usize)
            .ok_or_else(|| input_index_error("scheduled discovery task ID is stale"))?;
        let mut budget = Budget::new(self.limits);
        let mut functions = Vec::with_capacity(range.end.saturating_sub(range.start));
        for function in &self.program.functions[range.start..range.end] {
            let indexes = DiscoveryFunctionIndexes::build(function, &mut budget)?;
            let records = discover_function_edits(function, &indexes, &mut budget)?;
            functions.push(FunctionDiscovery { indexes, records });
        }
        Ok(RangeDiscovery {
            start: range.start,
            functions,
            work: budget.work,
        })
    }
}
