use super::*;

pub(super) fn native_witness_slots(
    program: &lkjscript_ir::Program,
) -> Result<
    HashMap<
        (
            lkjscript_ir::MemoryWitnessId,
            lkjscript_native::StructuralStorageRoute,
        ),
        u64,
    >,
    LoweringError,
> {
    let mut slots = HashMap::new();
    for representation in program
        .memory
        .representations
        .iter()
        .filter(|representation| {
            representation.category == lkjscript_ir::StructuralValueCategory::Owner
        })
    {
        let storage = match representation.storage {
            lkjscript_ir::StructuralStorage::UniqueStructural => {
                lkjscript_native::StructuralStorageRoute::Unique
            }
            lkjscript_ir::StructuralStorage::SealedRegion => {
                lkjscript_native::StructuralStorageRoute::Sealed
            }
            _ => continue,
        };
        let slot = u64::try_from(slots.len())
            .map_err(|_| invalid_structural("native witness slot exceeds u64"))?;
        if slots
            .insert((representation.witness, storage), slot)
            .is_some()
        {
            return Err(invalid_structural(
                "native witness representation route is duplicated",
            ));
        }
    }
    Ok(slots)
}
