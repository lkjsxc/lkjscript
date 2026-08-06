use super::executor::FunctionDiscovery;
use crate::optimize::*;

pub(super) fn merge_discovery<'a>(
    mut outputs: Vec<(lkjscript_resource::TaskId, FunctionDiscovery<'a>)>,
    function_count: usize,
    budget: &mut Budget,
) -> Result<(DiscoveryIndexes<'a>, OptimizationCertificate), OptimizationError> {
    outputs.sort_by_key(|(task, _)| *task);
    budget.charge(
        u64::try_from(function_count)
            .map_err(|_| input_index_error("function count exceeds u64 accounting"))?,
    )?;
    budget.charge(CERTIFICATE_HEADER_BYTES_ESTIMATE)?;
    let mut functions = Vec::with_capacity(function_count);
    let mut records = Vec::new();
    for (expected, (task, output)) in outputs.into_iter().enumerate() {
        if usize::try_from(task.slot).ok() != Some(expected) || output.function != expected {
            return Err(input_index_error(
                "scheduled discovery result order is incomplete",
            ));
        }
        budget.charge(output.work)?;
        functions.push(output.indexes);
        records.extend(output.records);
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
        record.sequence = u64::try_from(sequence)
            .map_err(|_| input_index_error("certificate sequence exceeds u64 identity"))?;
    }
    let certificate = OptimizationCertificate { records };
    if u64::try_from(certificate.records.len())
        .map_err(|_| input_index_error("certificate length exceeds u64 accounting"))?
        > budget.limits.max_certificate_records
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
