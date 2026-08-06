use crate::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NativeWitnessEntry {
    slot: u16,
    group: lkjscript_ir::MemoryWitnessGroupId,
    member: u16,
    representation: lkjscript_ir::StructuralRepresentationId,
    value_type: lkjscript_native::StructuralTypeIdentity,
    storage: lkjscript_ir::StructuralStorage,
    operations: Vec<lkjscript_core::MemoryWitnessOperation>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct NativeWitnessCatalog {
    entries: Vec<NativeWitnessEntry>,
}

impl NativeWitnessCatalog {
    pub(crate) fn build(program: &lkjscript_ir::Program) -> Result<Self, EngineError> {
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(program.memory.representations.len())
            .map_err(|_| witness_error("native witness catalog allocation failed"))?;
        for representation in program
            .memory
            .representations
            .iter()
            .filter(|representation| {
                representation.category == lkjscript_ir::StructuralValueCategory::Owner
                    && matches!(
                        representation.storage,
                        lkjscript_ir::StructuralStorage::UniqueStructural
                            | lkjscript_ir::StructuralStorage::SealedRegion
                    )
            })
        {
            let slot = u16::try_from(entries.len())
                .map_err(|_| witness_error("native witness slot exceeds u16"))?;
            let witness = program
                .memory
                .witness(representation.witness)
                .ok_or_else(|| witness_error("native representation witness is absent"))?;
            if representation.witness_group != witness.group
                || representation.witness_member != witness.ordinal
            {
                return Err(witness_error(
                    "native witness representation route is not exact",
                ));
            }
            let value_type = native_witness_type(program, witness)?;
            let member = u16::try_from(witness.ordinal)
                .map_err(|_| witness_error("native witness member exceeds u16"))?;
            entries.push(NativeWitnessEntry {
                slot,
                group: witness.group,
                member,
                representation: representation.id,
                value_type,
                storage: representation.storage,
                operations: witness.facts.operations.clone(),
            });
        }
        Ok(Self { entries })
    }

    pub(crate) fn resolve(
        &self,
        slot: u16,
        operation: lkjscript_core::MemoryWitnessOperation,
    ) -> Result<&NativeWitnessEntry, NativeServiceError> {
        let entry = self
            .entries
            .get(usize::from(slot))
            .filter(|entry| entry.slot == slot)
            .ok_or(NativeServiceError::Trap)?;
        if entry.operations.binary_search(&operation).is_err()
            || !matches!(
                entry.storage,
                lkjscript_ir::StructuralStorage::UniqueStructural
                    | lkjscript_ir::StructuralStorage::SealedRegion
            )
        {
            return Err(NativeServiceError::Trap);
        }
        Ok(entry)
    }
}

impl NativeWitnessEntry {
    pub(crate) const fn value_type(&self) -> lkjscript_native::StructuralTypeIdentity {
        self.value_type
    }

    pub(crate) const fn storage(&self) -> lkjscript_native::StructuralStorageRoute {
        match self.storage {
            lkjscript_ir::StructuralStorage::UniqueStructural => {
                lkjscript_native::StructuralStorageRoute::Unique
            }
            lkjscript_ir::StructuralStorage::SealedRegion => {
                lkjscript_native::StructuralStorageRoute::Sealed
            }
            _ => unreachable!(),
        }
    }
}

fn native_witness_type(
    program: &lkjscript_ir::Program,
    witness: &lkjscript_ir::MemoryWitnessDescriptor,
) -> Result<lkjscript_native::StructuralTypeIdentity, EngineError> {
    let item = program
        .memory
        .type_for(&witness.ty)
        .ok_or_else(|| witness_error("native witness structural type is absent"))?;
    let layout = program
        .memory
        .layouts
        .get(item.layout.index().unwrap_or(usize::MAX))
        .filter(|layout| layout.id == item.layout)
        .ok_or_else(|| witness_error("native witness layout is absent"))?;
    let kind = match layout.kind {
        lkjscript_ir::StructuralLayoutKind::String => lkjscript_native::StructuralKind::String,
        lkjscript_ir::StructuralLayoutKind::Path => lkjscript_native::StructuralKind::Path,
        lkjscript_ir::StructuralLayoutKind::Product { .. } => {
            lkjscript_native::StructuralKind::Product
        }
        lkjscript_ir::StructuralLayoutKind::Enum { .. } => lkjscript_native::StructuralKind::Enum,
    };
    let runtime = lkjscript_ir::runtime_structural_type(Some(program), &item.ty)
        .map_err(|error| witness_error(&error.to_string()))?
        .ok_or_else(|| witness_error("native witness runtime type is absent"))?;
    Ok(lkjscript_native::StructuralTypeIdentity::new(
        runtime.layout.get(),
        runtime.semantic_type.get(),
        kind,
        item.mode == lkjscript_ir::StructuralTypeMode::Copy,
    ))
}

fn witness_error(message: &str) -> EngineError {
    EngineError::new(FailureCode::BackendVerification, None, message.to_string())
}
