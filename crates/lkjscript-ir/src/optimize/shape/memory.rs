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
