//! Reuse only a live product's exact, allocation-bound admission.
//! No raw value, caller-supplied result type, address cache or ambient authority
//! can mint a checked child. Raw input, packing and raw unpack stay independent.
use super::super::super::prepare::NormalizedProgram;
use super::super::super::value::{NormalizedValue, RawArguments, RawValue, ValueOrigin};
use super::{Class, Value, admission_error};
use crate::platform::execution::{ExecutionControl, ExecutionError};
use crate::platform::kernel::{Name, TypeForm, TypeObjectDigest};

/// The original field vector owns every unreturned child. Its iterative cleanup
/// also covers cancellation or failure after earlier fields reached local slots.
/// This object has no constructor accepting raw children or asserted field types.
pub(in super::super) struct ProductExtraction {
    children: RawArguments,
    program: ValueOrigin,
    ty: TypeObjectDigest,
    index: usize,
}

fn product_field_count(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
) -> Result<usize, ExecutionError> {
    if let Some(layout) = program.owned_record_layout(ty) {
        return Ok(layout.fields.len());
    }
    match program.types.get(&ty).map(|object| &object.form) {
        Some(TypeForm::OwnedProduct { fields }) => Ok(fields.len()),
        _ => Err(admission_error(
            "extraction requires an exact closed product layout",
        )),
    }
}

fn product_field_type(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    index: usize,
) -> Result<TypeObjectDigest, ExecutionError> {
    if let Some(layout) = program.owned_record_layout(ty) {
        return layout
            .fields
            .get(index)
            .map(|field| field.ty)
            .ok_or_else(|| admission_error("extraction selects an absent nominal field"));
    }
    match program.types.get(&ty).map(|object| &object.form) {
        Some(TypeForm::OwnedProduct { fields }) => fields
            .get(index)
            .map(|field| field.ty)
            .ok_or_else(|| admission_error("extraction selects an absent product field")),
        _ => Err(admission_error("extraction lost its exact product layout")),
    }
}

impl ProductExtraction {
    pub(in super::super) fn next(
        &mut self,
        program: &NormalizedProgram,
        control: &ExecutionControl,
        admit_other: &mut impl FnMut(NormalizedValue, TypeObjectDigest) -> Result<Value, ExecutionError>,
    ) -> Result<Option<Value>, ExecutionError> {
        control.check()?;
        if self.program != program.value_origin {
            return Err(admission_error(
                "extracted fields belong to another prepared program",
            ));
        }
        if self.children.is_empty() {
            return Ok(None);
        }
        // Resolve the index through the sealed parent's exact layout. Neither a
        // raw child nor a caller's asserted ordinary type can establish this proof.
        let ty = product_field_type(program, self.ty, self.index)?;
        let raw = RawValue::new(
            self.children
                .next()
                .ok_or_else(|| admission_error("extraction lost its next field"))?,
        );
        self.index += 1;
        let value = if program.ordinary_types.contains(&ty) {
            Value {
                raw: raw.into_raw(),
                origin: self.program,
                class: Class::Free,
                borrow: None,
            }
        } else {
            // An ordinary-field proof cannot confer child ownership. Preserve
            // the evaluator's exact representation, custody and admission checks.
            admit_other(raw.into_raw(), ty)?
        };
        control.check()?;
        Ok(Some(value))
    }
}

impl Value {
    pub(in super::super) fn owned_product_field_count(
        program: &NormalizedProgram,
        ty: TypeObjectDigest,
    ) -> Result<usize, ExecutionError> {
        product_field_count(program, ty)
    }

    pub(in super::super) fn unpack_owned(
        self,
        program: &NormalizedProgram,
        domain: ValueOrigin,
        selected: TypeObjectDigest,
        control: &ExecutionControl,
    ) -> Result<ProductExtraction, ExecutionError> {
        control.check()?;
        if self.origin != program.value_origin || self.class != Class::Memory {
            return Err(admission_error(
                "extraction requires this program's checked owner",
            ));
        }
        // Frame source/placement markers also occur on ordinary consumed owner
        // arguments. They grant no loan. A retained result packet or ancestor
        // guard cannot cross this consuming boundary, even in release builds.
        if self
            .borrow
            .as_ref()
            .is_some_and(|borrow| borrow.packet.is_some() || !borrow.guards.is_empty())
        {
            return Err(admission_error(
                "extraction cannot consume borrowed result custody",
            ));
        }
        let NormalizedValue::OwnedProduct(token) = &self.raw else {
            return Err(admission_error(
                "extraction requires an owned product token",
            ));
        };
        if token.ty() != selected {
            return Err(admission_error("extraction selects another product type"));
        }
        token.validate_admission(program.value_origin)?;
        // Exact admission is independent of the current invocation's custody.
        // Owner placement and zero loans exclude mutation during extraction.
        token.validate(domain, true)?;
        let count = product_field_count(program, token.ty())?;
        control.check()?;
        let NormalizedValue::OwnedProduct(token) = self.into_raw() else {
            unreachable!("validated product token")
        };
        let children = RawArguments::new(token.unpack(domain, selected, control)?);
        if children.len() != count {
            return Err(admission_error(
                "extracted product has a foreign field count",
            ));
        }
        control.check()?;
        Ok(ProductExtraction {
            children,
            program: program.value_origin,
            ty: selected,
            index: 0,
        })
    }

    pub(in super::super) fn owned_record_metadata(
        &self,
        program: &NormalizedProgram,
        domain: ValueOrigin,
        selected: super::super::super::value::RecordLayoutIndex,
        offset: u32,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        if self.origin != program.value_origin
            || self.class != Class::Memory
            || selected.1 != program.value_origin
        {
            return Err(admission_error(
                "nominal metadata requires this program's checked owner",
            ));
        }
        let NormalizedValue::OwnedProduct(token) = &self.raw else {
            return Err(admission_error(
                "nominal metadata requires an owned record token",
            ));
        };
        token.validate_admission(program.value_origin)?;
        let exact = program
            .owned_record_layout(token.ty())
            .ok_or_else(|| admission_error("nominal metadata has no exact owned layout"))?;
        let selector = program
            .records
            .get(selected.0 as usize)
            .filter(|layout| layout.owned && layout.declaration == exact.declaration)
            .ok_or_else(|| admission_error("nominal metadata selects a foreign declaration"))?;
        let field = exact
            .fields
            .get(offset as usize)
            .filter(|field| {
                selector
                    .fields
                    .get(offset as usize)
                    .is_some_and(|selected| selected.reference == field.reference)
            })
            .ok_or_else(|| admission_error("nominal metadata selects a foreign field"))?;
        if !program.ordinary_types.contains(&field.ty) {
            return Err(admission_error(
                "nominal metadata cannot expose an owning child",
            ));
        }
        let raw = token.read_metadata(domain, offset as usize, control, reserve)?;
        control.check()?;
        Ok(Self {
            raw,
            origin: program.value_origin,
            class: Class::Free,
            borrow: None,
        })
    }
    pub(in super::super) fn sequence_item(
        &self,
        program: &NormalizedProgram,
        domain: ValueOrigin,
        index: i64,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        if self.origin != program.value_origin || self.class != Class::Memory {
            return Err(admission_error(
                "sequence projection requires this program's checked owner",
            ));
        }
        let NormalizedValue::OwnedSequence(token) = &self.raw else {
            return Err(admission_error(
                "sequence projection requires an exact sequence",
            ));
        };
        token.validate_admission(program.value_origin)?;
        let Some(object) = program.types.get(&token.ty()) else {
            return Err(admission_error("sequence has no exact prepared type"));
        };
        let TypeForm::OwnedSequence { item } = &object.form else {
            return Err(admission_error(
                "sequence token has a foreign prepared type",
            ));
        };
        if !program.comparable_types.contains(item) {
            return Err(admission_error(
                "sequence projection requires ordinary first-order data",
            ));
        }
        // Exact parent admission proves the selected child's closed type. The
        // read placement excludes mutation until this independent clone exists.
        let raw = token.get(domain, index, control, reserve)?;
        control.check()?;
        Ok(Self {
            raw,
            origin: program.value_origin,
            class: Class::Free,
            borrow: None,
        })
    }
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

#[cfg(test)]
#[path = "vm_owned_extraction_tests.rs"]
mod extraction_tests;
