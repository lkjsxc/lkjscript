//! Reuse only a live borrowed product's exact, allocation-bound admission.
//! No raw value, caller-supplied result type, address cache or ambient authority
//! can mint this projection. Raw input and consuming boundaries remain separate.
use super::super::super::prepare::NormalizedProgram;
use super::super::super::value::{NormalizedValue, ValueOrigin};
use super::{Class, Value, admission_error};
use crate::platform::execution::{ExecutionControl, ExecutionError};
use crate::platform::kernel::{Name, TypeForm};

impl Value {
    pub(in super::super) fn owned_metadata(
        &self,
        program: &NormalizedProgram,
        domain: ValueOrigin,
        name: &Name,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        if self.origin != program.value_origin || self.class != Class::Memory {
            return Err(admission_error(
                "metadata projection requires this program's checked owner",
            ));
        }
        let NormalizedValue::OwnedProduct(token) = &self.raw else {
            return Err(admission_error(
                "metadata projection requires an exact owned product",
            ));
        };
        // A slot's earlier classification is insufficient: raw mutation, failed
        // adoption, revocation and foreign program reuse invalidate the premise.
        token.validate_admission(program.value_origin)?;
        let Some(object) = program.types.get(&token.ty()) else {
            return Err(admission_error(
                "metadata product has no exact prepared type",
            ));
        };
        let TypeForm::OwnedProduct { fields } = &object.form else {
            return Err(admission_error("metadata type is not an owned product"));
        };
        let index = fields
            .binary_search_by(|field| field.name.cmp(name))
            .map_err(|_| admission_error("metadata field is absent from the exact product"))?;
        if !program.ordinary_types.contains(&fields[index].ty) {
            return Err(admission_error(
                "metadata projection cannot expose an owned or affine child",
            ));
        }
        // read_metadata requires a live Read placement in `domain`. That loan
        // excludes successful mutation/adoption until extraction completes.
        // Revocation either precedes the locked read and rejects, or follows a
        // clone whose immutable backing is now independently retained.
        let raw = token.read_metadata(domain, index, control, reserve)?;
        control.check()?;
        Ok(Self {
            raw,
            origin: program.value_origin,
            class: Class::Free,
            borrow: None,
        })
    }
}
