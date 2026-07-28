use super::executor::RangeDiscovery;
use crate::optimize::*;

pub(super) fn merge_discovery<'a>(
    mut outputs: Vec<(lkjscript_resource::TaskId, RangeDiscovery<'a>)>,
    function_count: usize,
    budget: &mut Budget,
) -> Result<(DiscoveryIndexes<'a>, OptimizationCertificate), OptimizationError> {
    outputs.sort_by_key(|(task, _)| *task);
    budget.charge(function_count as u64)?;
    budget.charge(CERTIFICATE_HEADER_BYTES_ESTIMATE)?;
    let mut functions = Vec::with_capacity(function_count);
    let mut records = Vec::new();
    for (expected, (task, output)) in outputs.into_iter().enumerate() {
        if task.slot as usize != expected || output.start != functions.len() {
            return Err(input_index_error(
                "scheduled discovery result order or range is incomplete",
            ));
        }
        budget.charge(output.work)?;
        for function in output.functions {
            functions.push(function.indexes);
            records.extend(function.records);
        }
    }
    if functions.len() != function_count {
        return Err(input_index_error(
            "scheduled discovery omitted a verified SSA function",
        ));
    }
    records.sort_by_key(|record| {
        (
            record.function,
            record.block,
            record.value,
            edit_kind_order(record.kind),
        )
    });
    for (sequence, record) in records.iter_mut().enumerate() {
        record.sequence = sequence as u64;
    }
    let certificate = OptimizationCertificate { records };
    if certificate.records.len() as u64 > budget.limits.max_certificate_records
        || certificate_size_estimate(&certificate)? > budget.limits.max_certificate_bytes_estimate
    {
        return Err(budget_error());
    }
    Ok((DiscoveryIndexes { functions }, certificate))
}

fn edit_kind_order(kind: OptimizationEditKind) -> u8 {
    match kind {
        OptimizationEditKind::AlgebraicIdentity => 0,
        OptimizationEditKind::GlobalValueNumbering => 1,
        OptimizationEditKind::CheckedI64GlobalValueNumbering => 2,
    }
}
