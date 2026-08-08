use crate::codegen::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::codegen) enum LocalStorageClass {
    Plain,
    StructuralOwner,
    StructuralOwnerRef,
    StructuralView,
    StructuralDestination,
}

#[derive(Clone, Debug)]
pub(in crate::codegen) struct LocalMetadata {
    pub(in crate::codegen) ty: SsaType,
    pub(in crate::codegen) storage: LocalStorageClass,
    pub(in crate::codegen) producer: LocalProducerKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::codegen) enum LocalProducerKind {
    Parameter,
    Owner,
    ProductField,
    StructuralMove,
    Call,
    RuntimeOrConversion,
    View,
    Other,
}

#[derive(Clone, Copy)]
enum ParameterRole {
    None,
    Entry,
    NonEntry,
}

pub(super) fn collect_local_metadata(
    function: &Function,
    chunk: &Chunk,
) -> Result<HashMap<ValueId, LocalMetadata>> {
    let entry = function
        .blocks
        .iter()
        .find(|block| block.id == function.entry)
        .ok_or_else(|| Error::msg("SSA function entry block is missing"))?;
    let value_count = function.blocks.iter().try_fold(0_usize, |count, block| {
        count
            .checked_add(block.parameters.len())
            .and_then(|count| count.checked_add(block.instructions.len()))
            .ok_or_else(|| Error::host("SSA local metadata count overflow"))
    })?;

    let mut structural_places = HashSet::new();
    structural_places
        .try_reserve(function.places.len())
        .map_err(|_| Error::host("SSA structural-place index allocation failed"))?;
    structural_places.extend(function.places.iter().filter_map(|metadata| {
        matches!(metadata.drop_glue, Some(DropGlueIdentity::Structural(_))).then_some(metadata.id)
    }));

    let mut metadata = HashMap::new();
    metadata
        .try_reserve(value_count)
        .map_err(|_| Error::host("SSA local metadata allocation failed"))?;
    for block in &function.blocks {
        let parameter_role = if block.id == entry.id {
            ParameterRole::Entry
        } else {
            ParameterRole::NonEntry
        };
        for parameter in &block.parameters {
            insert_metadata(
                &mut metadata,
                parameter.id,
                LocalMetadata {
                    ty: parameter.ty.clone(),
                    storage: classify_local(
                        &parameter.ty,
                        None,
                        parameter_role,
                        structural_owner_representation(chunk, &parameter.ty).is_some(),
                        is_dynamic_owner_parameter(function, &parameter.ty),
                        false,
                    ),
                    producer: LocalProducerKind::Parameter,
                },
            )?;
        }
        for instruction in &block.instructions {
            let structural_move = matches!(
                instruction.kind,
                InstructionKind::Move { place, .. } if structural_places.contains(&place)
            );
            let represented_owner = matches!(
                instruction.kind,
                InstructionKind::Runtime { .. } | InstructionKind::Call { .. }
            ) && structural_owner_representation(chunk, &instruction.ty)
                .is_some();
            insert_metadata(
                &mut metadata,
                instruction.id,
                LocalMetadata {
                    ty: instruction.ty.clone(),
                    storage: classify_local(
                        &instruction.ty,
                        Some(&instruction.kind),
                        ParameterRole::None,
                        represented_owner,
                        false,
                        structural_move,
                    ),
                    producer: local_producer_kind(&instruction.kind, structural_move),
                },
            )?;
        }
    }
    Ok(metadata)
}

fn local_producer_kind(instruction: &InstructionKind, structural_move: bool) -> LocalProducerKind {
    match instruction {
        InstructionKind::StructuralPublish { .. }
        | InstructionKind::StructuralCopy { .. }
        | InstructionKind::MemoryWitnessIndependentOwner { .. }
        | InstructionKind::DestinationFinish { .. }
        | InstructionKind::AggregateConsumePayload { .. } => LocalProducerKind::Owner,
        InstructionKind::ProductField { .. } => LocalProducerKind::ProductField,
        InstructionKind::Move { .. } if structural_move => LocalProducerKind::StructuralMove,
        InstructionKind::Call { .. } => LocalProducerKind::Call,
        InstructionKind::Runtime { .. }
        | InstructionKind::F64FromI64Exact { .. }
        | InstructionKind::I64FromF64Exact { .. }
        | InstructionKind::I64FromF64Trunc { .. } => LocalProducerKind::RuntimeOrConversion,
        InstructionKind::Borrow { .. } | InstructionKind::AggregateFieldBorrow { .. } => {
            LocalProducerKind::View
        }
        _ => LocalProducerKind::Other,
    }
}

fn insert_metadata(
    metadata: &mut HashMap<ValueId, LocalMetadata>,
    value: ValueId,
    facts: LocalMetadata,
) -> Result<()> {
    if metadata.insert(value, facts).is_some() {
        Err(Error::msg("SSA local metadata has duplicate ValueId"))
    } else {
        Ok(())
    }
}

fn is_dynamic_owner_parameter(function: &Function, ty: &SsaType) -> bool {
    let SsaType::TypeParameter(parameter) = ty else {
        return false;
    };
    function
        .signature
        .memory_witness_parameters
        .iter()
        .any(|requirement| {
            requirement.parameter == *parameter
                && requirement
                    .operations
                    .contains(&lkjscript_contracts::MemoryWitnessOperation::IndependentOwner)
                && requirement
                    .operations
                    .contains(&lkjscript_contracts::MemoryWitnessOperation::Dispose)
        })
}

fn classify_local(
    ty: &SsaType,
    producer: Option<&InstructionKind>,
    parameter: ParameterRole,
    represented_owner: bool,
    dynamic_owner: bool,
    structural_move: bool,
) -> LocalStorageClass {
    if matches!(ty, SsaType::StructuralDestination(_)) {
        return LocalStorageClass::StructuralDestination;
    }
    match producer {
        Some(
            InstructionKind::StructuralPublish { .. }
            | InstructionKind::StructuralCopy { .. }
            | InstructionKind::MemoryWitnessIndependentOwner { .. }
            | InstructionKind::DestinationFinish { .. }
            | InstructionKind::AggregateConsumePayload { .. },
        ) => LocalStorageClass::StructuralOwner,
        Some(InstructionKind::Runtime { .. }) if represented_owner => {
            LocalStorageClass::StructuralOwner
        }
        Some(InstructionKind::Borrow { .. } | InstructionKind::AggregateFieldBorrow { .. }) => {
            LocalStorageClass::StructuralView
        }
        Some(InstructionKind::Move { .. }) if structural_move => LocalStorageClass::StructuralOwner,
        Some(InstructionKind::Call { .. }) if represented_owner => {
            LocalStorageClass::StructuralOwner
        }
        None if matches!(parameter, ParameterRole::Entry) && dynamic_owner => {
            LocalStorageClass::StructuralOwnerRef
        }
        None if !matches!(parameter, ParameterRole::None) && represented_owner => {
            LocalStorageClass::StructuralOwner
        }
        _ => LocalStorageClass::Plain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_preserves_every_local_storage_category_and_precedence() {
        let value = ValueId::new(0);
        let place = lkjscript_ir::PlaceId::new(0);
        let loan = lkjscript_ir::LoanId::new(0);
        let representation = lkjscript_ir::StructuralRepresentationId::new(0);
        let destination = SsaType::StructuralDestination(lkjscript_ir::StructuralTypeId::new(0));
        assert_eq!(
            classify_local(&destination, None, ParameterRole::None, false, false, false,),
            LocalStorageClass::StructuralDestination,
        );

        let publish = InstructionKind::StructuralPublish {
            representation,
            value,
        };
        assert_eq!(
            classify_local(
                &SsaType::Unit,
                Some(&publish),
                ParameterRole::None,
                false,
                false,
                false,
            ),
            LocalStorageClass::StructuralOwner,
        );

        let borrow = InstructionKind::Borrow {
            place,
            loan,
            kind: lkjscript_ir::BorrowKind::Shared,
            value,
        };
        assert_eq!(
            classify_local(
                &SsaType::ByteSlice,
                Some(&borrow),
                ParameterRole::None,
                false,
                false,
                false,
            ),
            LocalStorageClass::StructuralView,
        );

        let moved = InstructionKind::Move { place, value };
        assert_eq!(
            classify_local(
                &SsaType::Unit,
                Some(&moved),
                ParameterRole::None,
                false,
                false,
                true,
            ),
            LocalStorageClass::StructuralOwner,
        );
        assert_eq!(
            classify_local(
                &SsaType::TypeParameter("t".into()),
                None,
                ParameterRole::Entry,
                false,
                true,
                false,
            ),
            LocalStorageClass::StructuralOwnerRef,
        );
        assert_eq!(
            classify_local(
                &SsaType::Product(lkjscript_ir::ProductId::new(0)),
                None,
                ParameterRole::NonEntry,
                true,
                false,
                false,
            ),
            LocalStorageClass::StructuralOwner,
        );
        assert_eq!(
            classify_local(
                &SsaType::I64,
                None,
                ParameterRole::None,
                false,
                false,
                false,
            ),
            LocalStorageClass::Plain,
        );
    }
}
