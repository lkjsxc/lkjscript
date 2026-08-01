fn validate_table_limits(chunk: &Chunk, limits: &ValidationLimits) -> Result<()> {
    let lengths = [
        ("witnesses", chunk.memory_witnesses.len(), crate::MAX_MEMORY_WITNESSES),
        ("types", chunk.structural_types.len(), MAX_STRUCTURAL_TYPES),
        (
            "layouts",
            chunk.structural_layouts.len(),
            MAX_STRUCTURAL_LAYOUTS,
        ),
        (
            "representations",
            chunk.structural_representations.len(),
            MAX_STRUCTURAL_REPRESENTATIONS,
        ),
        (
            "destinations",
            chunk.structural_destinations.len(),
            MAX_STRUCTURAL_DESTINATIONS,
        ),
        (
            "destination fields",
            chunk.structural_destination_fields.len(),
            MAX_STRUCTURAL_OPERATION_REFS,
        ),
        (
            "aggregate fields",
            chunk.structural_aggregate_fields.len(),
            MAX_STRUCTURAL_OPERATION_REFS,
        ),
        (
            "payloads",
            chunk.structural_payloads.len(),
            MAX_STRUCTURAL_OPERATION_REFS,
        ),
    ];
    for (name, length, hard_limit) in lengths {
        let limit = hard_limit.min(limits.max_table_entries);
        if length > limit {
            return Err(Error::msg(format!(
                "bytecode structural {name} table has {length} entries, limit {limit}",
            )));
        }
    }

    let carries_structural = !chunk.memory_witnesses.is_empty()
        || !chunk.structural_types.is_empty()
        || !chunk.structural_layouts.is_empty()
        || !chunk.structural_representations.is_empty()
        || !chunk.structural_destinations.is_empty()
        || !chunk.structural_destination_fields.is_empty()
        || !chunk.structural_aggregate_fields.is_empty()
        || !chunk.structural_payloads.is_empty()
        || chunk.products.iter().any(|product| product.region);
    if carries_structural != chunk.memory_plan.is_some() {
        return Err(Error::msg(
            "bytecode structural metadata requires one exact MemoryPlanId",
        ));
    }
    Ok(())
}
