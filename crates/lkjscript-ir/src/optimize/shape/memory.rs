use crate::optimize::*;
use crate::{Program, SsaType, StructuralLayoutKind};

pub(super) fn preflight(
    program: &Program,
    counter: &mut ShapeCounter<'_>,
) -> Result<(), OptimizationError> {
    counter.add_bounded(
        ShapeField::StringAndMetadataBytes,
        u64::try_from(program.memory.plan.bytes().len()).map_err(|_| budget_error())?,
    )?;
    for group in &program.memory.witness_groups {
        counter.add_metadata()?;
        for _ in &group.members {
            counter.add_metadata()?;
        }
    }
    for witness in &program.memory.witnesses {
        counter.add_metadata()?;
        counter.add_type(&witness.ty)?;
        counter.add_bounded(
            ShapeField::StringAndMetadataBytes,
            128_u64
                .checked_add(
                    u64::try_from(witness.facts.operations.len()).map_err(|_| budget_error())?,
                )
                .and_then(|value| {
                    u64::try_from(witness.dependencies.len())
                        .ok()
                        .and_then(|count| count.checked_mul(32))
                        .and_then(|bytes| value.checked_add(bytes))
                })
                .ok_or_else(budget_error)?,
        )?;
        for _ in &witness.dependencies {
            counter.add_metadata()?;
        }
    }
    for item in &program.memory.types {
        counter.add_metadata()?;
        counter.add_type(&item.ty)?;
    }
    for layout in &program.memory.layouts {
        counter.add_metadata()?;
        match &layout.kind {
            StructuralLayoutKind::String | StructuralLayoutKind::Path => {}
            StructuralLayoutKind::Product { fields, .. } => add_fields(counter, fields)?,
            StructuralLayoutKind::Enum { variants, .. } => {
                for variant in variants {
                    counter.add_metadata()?;
                    add_fields(counter, &variant.fields)?;
                }
            }
        }
    }
    for _ in &program.memory.representations {
        counter.add_metadata()?;
    }
    for _ in &program.region_products {
        counter.add_metadata()?;
    }
    Ok(())
}

fn add_fields(counter: &mut ShapeCounter<'_>, fields: &[SsaType]) -> Result<(), OptimizationError> {
    for field in fields {
        counter.add_metadata()?;
        counter.add_type(field)?;
    }
    Ok(())
}
