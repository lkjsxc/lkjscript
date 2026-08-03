use crate::verify::*;

mod witness;
use crate::{
    EffectSet, Function, Instruction, InstructionKind, Program, SsaType, StructuralLayoutKind,
    StructuralValueCategory,
};
pub(super) use witness::verify as verify_witness;

pub(super) fn verify(
    program: &Program,
    function: &Function,
    instruction: &Instruction,
    types: &[SsaType],
) -> crate::Result<EffectSet> {
    match &instruction.kind {
        InstructionKind::StructuralPublish {
            representation,
            value,
        } => {
            let (ty, _) =
                lookup_representation(program, *representation, StructuralValueCategory::Owner)?;
            if value_type(types, *value)? != ty || &instruction.ty != ty {
                return fail("SSA structural publish has stale type metadata");
            }
            Ok(EffectSet::ALLOCATES)
        }
        InstructionKind::DestinationCreate {
            representation,
            active_variant,
        } => {
            let (ty, type_id) = lookup_representation(
                program,
                *representation,
                StructuralValueCategory::Destination,
            )?;
            verify_active_variant(program, type_id, *active_variant)?;
            if instruction.ty != SsaType::StructuralDestination(type_id)
                || program.memory.type_for(ty).is_none()
            {
                return fail("SSA destination create has stale type metadata");
            }
            Ok(EffectSet::ALLOCATES)
        }
        InstructionKind::DestinationFieldInit {
            destination,
            field,
            value,
        } => {
            let type_id = destination_type(types, *destination)?;
            let expected = destination_field(program, function, *destination, type_id, *field)?;
            if value_type(types, *value)? != expected
                || instruction.ty != SsaType::StructuralDestination(type_id)
            {
                return fail("SSA destination field initialization has wrong type");
            }
            Ok(EffectSet::WRITES_MEMORY.union(EffectSet::ALLOCATES))
        }
        InstructionKind::DestinationFinish { destination } => {
            let type_id = destination_type(types, *destination)?;
            let ty = structural_type(program, type_id)?;
            if &instruction.ty != ty {
                return fail("SSA destination finish has wrong owner type");
            }
            Ok(EffectSet::ALLOCATES)
        }
        InstructionKind::DestinationAbort { destination } => {
            let _type_id = destination_type(types, *destination)?;
            if instruction.ty != SsaType::Unit {
                return fail("SSA destination abort must produce Unit");
            }
            Ok(EffectSet::PURE)
        }
        InstructionKind::AggregateFieldBorrow {
            representation,
            place,
            field,
            value,
            ..
        } => {
            let (ty, type_id) =
                lookup_representation(program, *representation, StructuralValueCategory::View)?;
            let declared = place_by_id(function, *place)?;
            let field_ty = aggregate_field(program, type_id, *field)?;
            if &declared.ty != ty || value_type(types, *value)? != ty || &instruction.ty != field_ty
            {
                return fail("SSA aggregate field borrow has wrong owner, field, or result type");
            }
            Ok(EffectSet::READS_MEMORY)
        }
        InstructionKind::AggregateTag {
            representation,
            value,
        } => {
            let (ty, type_id) =
                lookup_representation(program, *representation, StructuralValueCategory::View)?;
            let layout = structural_layout(program, type_id)?;
            if !matches!(layout.kind, StructuralLayoutKind::Enum { .. })
                || value_type(types, *value)? != ty
                || instruction.ty != SsaType::I64
            {
                return fail("SSA aggregate tag requires one exact structural enum");
            }
            Ok(EffectSet::READS_MEMORY)
        }
        InstructionKind::AggregateConsumePayload {
            representation,
            place,
            variant,
            value,
        } => {
            let (ty, type_id) =
                lookup_representation(program, *representation, StructuralValueCategory::Owner)?;
            let layout = structural_layout(program, type_id)?;
            let payload = match &layout.kind {
                StructuralLayoutKind::Enum { variants, .. } => variants
                    .iter()
                    .find(|item| item.variant == *variant)
                    .ok_or_else(|| {
                        crate::IrError::new("SSA whole payload consume names an inactive variant")
                    })?,
                StructuralLayoutKind::String
                | StructuralLayoutKind::Path
                | StructuralLayoutKind::Product { .. } => {
                    return fail("SSA whole payload consume requires an enum layout")
                }
            };
            let place_matches = place
                .map(|place| place_by_id(function, place).map(|item| &item.ty == ty))
                .transpose()?
                .unwrap_or(true);
            if !place_matches
                || value_type(types, *value)? != ty
                || payload.fields.as_slice() != [instruction.ty.clone()]
            {
                return fail("SSA whole payload consume has wrong owner or payload identity");
            }
            Ok(EffectSet::PURE)
        }
        InstructionKind::StructuralCopy {
            representation,
            value,
        } => {
            let (ty, type_id) =
                lookup_representation(program, *representation, StructuralValueCategory::Owner)?;
            require_structural_copy_mode(program, type_id)?;
            if value_type(types, *value)? != ty || &instruction.ty != ty {
                return fail("SSA structural copy has stale type metadata");
            }
            Ok(EffectSet::ALLOCATES)
        }
        InstructionKind::StringUtf8View {
            representation,
            place,
            value,
            ..
        } => {
            let (ty, type_id) =
                lookup_representation(program, *representation, StructuralValueCategory::View)?;
            let layout = structural_layout(program, type_id)?;
            if !matches!(layout.kind, StructuralLayoutKind::String)
                || &place_by_id(function, *place)?.ty != ty
                || value_type(types, *value)? != ty
                || instruction.ty != SsaType::ByteSlice
            {
                return fail("SSA UTF-8 view requires one exact structural string owner");
            }
            Ok(EffectSet::READS_MEMORY)
        }
        InstructionKind::Constant(_)
        | InstructionKind::Copy(_)
        | InstructionKind::PlaceInit { .. }
        | InstructionKind::PlaceEnd { .. }
        | InstructionKind::EndBorrow { .. }
        | InstructionKind::Drop { .. }
        | InstructionKind::Move { .. }
        | InstructionKind::Borrow { .. }
        | InstructionKind::MemoryWitnessIndependentOwner { .. }
        | InstructionKind::MemoryWitnessCompare { .. }
        | InstructionKind::MemoryWitnessDispose { .. }
        | InstructionKind::FunctionRef(_)
        | InstructionKind::Runtime { .. }
        | InstructionKind::F64FromI64Exact { .. }
        | InstructionKind::F64FromI64Rounded { .. }
        | InstructionKind::I64FromF64Exact { .. }
        | InstructionKind::I64FromF64Trunc { .. }
        | InstructionKind::Call { .. }
        | InstructionKind::ProductValue { .. }
        | InstructionKind::ProductField { .. }
        | InstructionKind::WithProductField { .. }
        | InstructionKind::EnumValue { .. }
        | InstructionKind::EnumIsVariant { .. }
        | InstructionKind::EnumField { .. } => {
            fail("non-structural instruction reached structural verifier")
        }
    }
}

include!("support.rs");
