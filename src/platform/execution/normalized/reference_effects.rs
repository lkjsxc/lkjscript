//! Reference-only requirement and row derivation from exact canonical atoms.
//! This does not use production substitution, containment, or prepared application tables.

use crate::platform::execution::ExecutionError;
use crate::platform::kernel::{
    EffectParameterReference, EffectRow, RequirementOperand, RequirementParameterReference,
    RequirementReference,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Bindings = BTreeMap<EffectParameterReference, EffectRow>;
pub(super) type RequirementBindings = BTreeMap<RequirementParameterReference, RequirementReference>;

pub(super) fn resolve(
    operand: RequirementOperand,
    bindings: &RequirementBindings,
) -> Result<RequirementReference, ExecutionError> {
    match operand {
        RequirementOperand::Concrete(reference) => Ok(reference),
        RequirementOperand::Parameter(parameter) => {
            bindings.get(&parameter).copied().ok_or_else(failure)
        }
    }
}

pub(super) fn close(
    row: &EffectRow,
    bindings: &Bindings,
    requirements: &RequirementBindings,
    mut admit: impl FnMut(usize) -> Result<(), ExecutionError>,
) -> Result<EffectRow, ExecutionError> {
    row.validate().map_err(|_| failure())?;
    let mut count = row.requirements.len();
    for parameter in &row.parameters {
        let value = bindings.get(parameter).ok_or_else(failure)?;
        value.validate().map_err(|_| failure())?;
        if !value.is_closed() {
            return Err(failure());
        }
        count = count
            .checked_add(value.requirements.len())
            .ok_or_else(failure)?;
    }
    admit(count)?;
    let mut atoms = BTreeSet::<RequirementReference>::new();
    for operand in &row.requirements {
        atoms.insert(resolve(*operand, requirements)?);
    }
    // A supplied row is already in the caller's closed context; never resubstitute it.
    for parameter in &row.parameters {
        for operand in &bindings.get(parameter).ok_or_else(failure)?.requirements {
            let RequirementOperand::Concrete(reference) = operand else {
                return Err(failure());
            };
            atoms.insert(*reference);
        }
    }
    if atoms.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN {
        return Err(failure());
    }
    Ok(EffectRow {
        requirements: atoms
            .into_iter()
            .map(RequirementOperand::Concrete)
            .collect(),
        parameters: Vec::new(),
    })
}

fn failure() -> ExecutionError {
    super::reference::reference_error(
        "normalized_reference_effect_scope",
        "application lacks canonical closed arguments in its exact declaration scope",
    )
}
