use super::graph::discovery_weight;
use crate::optimize::*;
use crate::{InstructionKind, Program};

#[derive(Clone, Copy)]
pub(super) struct DiscoveryGrant {
    pub(super) work: u64,
    pub(super) records: u64,
    pub(super) bytes: u64,
}

impl DiscoveryGrant {
    pub(super) fn limits(self, mut limits: OptimizationLimits) -> OptimizationLimits {
        limits.max_work_units = self.work;
        limits.max_certificate_records = self.records;
        limits.max_certificate_bytes_estimate = self.bytes;
        limits
    }
}

pub(super) fn reserve(
    program: &Program,
    limits: OptimizationLimits,
    spent_work: u64,
) -> Result<Vec<DiscoveryGrant>, OptimizationError> {
    let function_count = program.functions.len() as u64;
    let coordinator = function_count
        .checked_add(CERTIFICATE_HEADER_BYTES_ESTIMATE)
        .ok_or_else(budget_error)?;
    let work_pool = limits
        .max_work_units
        .checked_sub(spent_work)
        .and_then(|work| work.checked_sub(coordinator))
        .ok_or_else(budget_error)?;
    let byte_pool = limits
        .max_certificate_bytes_estimate
        .checked_sub(CERTIFICATE_HEADER_BYTES_ESTIMATE)
        .ok_or_else(budget_error)?;
    let work_weights: Vec<_> = program.functions.iter().map(discovery_weight).collect();
    let record_weights: Vec<_> = program
        .functions
        .iter()
        .map(|function| {
            function
                .blocks
                .iter()
                .map(|block| block.instructions.len() as u64)
                .sum::<u64>()
                .max(1)
        })
        .collect();
    let byte_weights: Vec<_> = program.functions.iter().map(certificate_weight).collect();
    let work = partition(work_pool, &work_weights)?;
    let records = partition(limits.max_certificate_records, &record_weights)?;
    let bytes = partition(byte_pool, &byte_weights)?;
    Ok((0..program.functions.len())
        .map(|index| DiscoveryGrant {
            work: work[index],
            records: records[index],
            bytes: bytes[index],
        })
        .collect())
}

fn certificate_weight(function: &crate::Function) -> u64 {
    function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .map(|instruction| {
            let operands = match &instruction.kind {
                InstructionKind::Runtime { arguments, .. }
                | InstructionKind::Call { arguments, .. } => arguments.len() as u64,
                _ => 0,
            };
            CERTIFICATE_RECORD_FIXED_BYTES_ESTIMATE.saturating_add(operands.saturating_mul(4))
        })
        .sum::<u64>()
        .max(1)
}

fn partition(total: u64, weights: &[u64]) -> Result<Vec<u64>, OptimizationError> {
    if weights.is_empty() {
        return Ok(Vec::new());
    }
    let weight_total = weights
        .iter()
        .try_fold(0_u64, |sum, weight| sum.checked_add(*weight))
        .filter(|sum| *sum > 0)
        .ok_or_else(budget_error)?;
    let mut prefix = 0_u64;
    let mut committed = 0_u64;
    let mut grants = Vec::with_capacity(weights.len());
    for weight in weights {
        prefix = prefix.checked_add(*weight).ok_or_else(budget_error)?;
        let boundary = u64::try_from(
            u128::from(total)
                .saturating_mul(u128::from(prefix))
                .checked_div(u128::from(weight_total))
                .ok_or_else(budget_error)?,
        )
        .map_err(|_| budget_error())?;
        grants.push(boundary.saturating_sub(committed));
        committed = boundary;
    }
    Ok(grants)
}

#[cfg(test)]
mod tests {
    use super::partition;

    #[test]
    fn deterministic_partition_commits_the_exact_aggregate_once() {
        assert_eq!(partition(10, &[1, 1, 2]), Ok(vec![2, 3, 5]));
        assert_eq!(partition(2, &[1, 1, 1]), Ok(vec![0, 1, 1]));
    }
}
