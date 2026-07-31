use crate::codegen::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LocalStorageClass {
    Plain,
    StructuralOwner,
    StructuralView,
    StructuralDestination,
}

pub(super) fn local_storage_class(
    function: &Function,
    chunk: &Chunk,
    value: ValueId,
) -> LocalStorageClass {
    let ty = function
        .blocks
        .iter()
        .flat_map(|block| {
            block
                .parameters
                .iter()
                .map(|parameter| (parameter.id, &parameter.ty))
                .chain(
                    block
                        .instructions
                        .iter()
                        .map(|instruction| (instruction.id, &instruction.ty)),
                )
        })
        .find_map(|(id, ty)| (id == value).then_some(ty));
    if ty.is_some_and(|ty| matches!(ty, SsaType::StructuralDestination(_))) {
        return LocalStorageClass::StructuralDestination;
    }
    let producer = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find(|instruction| instruction.id == value);
    match producer.map(|instruction| &instruction.kind) {
        Some(
            InstructionKind::StructuralPublish { .. }
            | InstructionKind::StructuralCopy { .. }
            | InstructionKind::DestinationFinish { .. }
            | InstructionKind::AggregateConsumePayload { .. },
        ) => LocalStorageClass::StructuralOwner,
        Some(InstructionKind::Runtime { .. })
            if ty.is_some_and(|ty| structural_owner_representation(chunk, ty).is_some()) =>
        {
            LocalStorageClass::StructuralOwner
        }
        Some(InstructionKind::Borrow { .. } | InstructionKind::AggregateFieldBorrow { .. }) => {
            LocalStorageClass::StructuralView
        }
        Some(InstructionKind::Move { place, .. }) if structural_place(function, *place) => {
            LocalStorageClass::StructuralOwner
        }
        Some(InstructionKind::Call { .. })
            if ty.is_some_and(|ty| structural_owner_representation(chunk, ty).is_some()) =>
        {
            LocalStorageClass::StructuralOwner
        }
        _ if function
            .blocks
            .iter()
            .flat_map(|block| &block.parameters)
            .any(|parameter| parameter.id == value)
            && ty.is_some_and(|ty| structural_owner_representation(chunk, ty).is_some()) =>
        {
            LocalStorageClass::StructuralOwner
        }
        _ => LocalStorageClass::Plain,
    }
}

fn structural_place(function: &Function, place: lkjscript_ir::PlaceId) -> bool {
    function.places.iter().any(|metadata| {
        metadata.id == place && matches!(metadata.drop_glue, Some(DropGlueIdentity::Structural(_)))
    })
}
