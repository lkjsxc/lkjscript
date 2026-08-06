fn validate_table_shape(chunk: &Chunk) -> Result<()> {
    let carries_structural = !chunk.memory_witness_groups.is_empty()
        || !chunk.memory_witnesses.is_empty()
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
