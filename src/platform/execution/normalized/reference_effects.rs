//! Reference-only row derivation. This reads exact canonical atoms and does not use
//! the production substitution, containment, or prepared application tables.

use crate::platform::execution::ExecutionError;
use crate::platform::kernel::{EffectParameterReference, EffectRow, RequirementReference};
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Bindings = BTreeMap<EffectParameterReference, EffectRow>;

pub(super) fn close(
    row: &EffectRow,
    bindings: &Bindings,
    mut admit: impl FnMut(usize) -> Result<(), ExecutionError>,
) -> Result<EffectRow, ExecutionError> {
    row.validate().map_err(|_| failure())?;
    let mut count = row.requirements.len();
    for parameter in &row.parameters {
        let value = bindings.get(parameter).ok_or_else(failure)?;
        value.validate().map_err(|_| failure())?;
        if !value.parameters.is_empty() {
            return Err(failure());
        }
        count = count
            .checked_add(value.requirements.len())
            .ok_or_else(failure)?;
    }
    admit(count)?;
    let mut atoms = BTreeSet::<RequirementReference>::new();
    atoms.extend(row.requirements.iter().copied());
    for parameter in &row.parameters {
        atoms.extend(
            bindings
                .get(parameter)
                .ok_or_else(failure)?
                .requirements
                .iter()
                .copied(),
        );
    }
    if atoms.len() > crate::platform::kernel::contract::MAXIMUM_CHILDREN {
        return Err(failure());
    }
    Ok(EffectRow {
        requirements: atoms.into_iter().collect(),
        parameters: Vec::new(),
    })
}

fn failure() -> ExecutionError {
    super::reference::reference_error(
        "normalized_reference_effect_scope",
        "effect application lacks canonical closed arguments in its exact declaration scope",
    )
}
