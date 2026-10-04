//! Borrowed canonical identity lookup. Resolving a child signature never builds
//! temporary type objects, clones field names, or encodes transient bytes.
use super::NormalizedReferenceSchema;
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{TypeForm, TypeObjectDigest};
use crate::platform::semantic_id::TypeParameterId;
use std::collections::BTreeMap;

struct Search<'a> {
    schema: &'a NormalizedReferenceSchema,
    bindings: &'a BTreeMap<TypeParameterId, TypeObjectDigest>,
    control: &'a ExecutionControl,
    remaining: u64,
}

impl Search<'_> {
    fn step(&mut self, depth: usize) -> Result<(), ExecutionError> {
        self.control.check()?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(invalid(
                "canonical transfer substitution exceeds type depth",
            ));
        }
        self.remaining = self.remaining.checked_sub(1).ok_or_else(|| {
            ExecutionError::resource(
                "reference_transfer_substitution_work",
                "canonical transfer identity search exhausted finite work",
            )
        })?;
        Ok(())
    }

    fn same(
        &mut self,
        template: TypeObjectDigest,
        candidate: TypeObjectDigest,
        depth: usize,
    ) -> Result<bool, ExecutionError> {
        self.step(depth)?;
        let schema = self.schema;
        let source = schema
            .types
            .get(&template)
            .ok_or_else(|| invalid("missing canonical transfer template"))?;
        let target = schema
            .types
            .get(&candidate)
            .ok_or_else(|| invalid("missing canonical transfer candidate"))?;
        if let TypeForm::TypeParameter { parameter } = &source.form {
            return self
                .bindings
                .get(parameter)
                .map(|actual| *actual == candidate)
                .ok_or_else(|| invalid("transfer type parameter escaped its exact invocation"));
        }
        Ok(match (&source.form, &target.form) {
            (TypeForm::List { item: a }, TypeForm::List { item: b })
            | (TypeForm::Option { item: a }, TypeForm::Option { item: b })
            | (TypeForm::OwnedSequence { item: a }, TypeForm::OwnedSequence { item: b })
            | (TypeForm::Stream { item: a }, TypeForm::Stream { item: b }) => {
                self.same(*a, *b, depth + 1)?
            }
            (TypeForm::Map { key: a, value: x }, TypeForm::Map { key: b, value: y })
            | (TypeForm::Result { ok: a, error: x }, TypeForm::Result { ok: b, error: y }) => {
                self.same(*a, *b, depth + 1)? && self.same(*x, *y, depth + 1)?
            }
            (
                TypeForm::StructuralRecord { fields: a },
                TypeForm::StructuralRecord { fields: b },
            )
            | (TypeForm::OwnedProduct { fields: a }, TypeForm::OwnedProduct { fields: b })
            | (TypeForm::OwnedChoice { cases: a }, TypeForm::OwnedChoice { cases: b }) => {
                if a.len() != b.len() {
                    return Ok(false);
                }
                for (a, b) in a.iter().zip(b) {
                    self.step(depth)?;
                    if a.name != b.name || !self.same(a.ty, b.ty, depth + 1)? {
                        return Ok(false);
                    }
                }
                true
            }
            (
                TypeForm::Applied {
                    declaration: a,
                    arguments: x,
                },
                TypeForm::Applied {
                    declaration: b,
                    arguments: y,
                },
            ) => {
                if a != b || x.len() != y.len() {
                    return Ok(false);
                }
                for (a, b) in x.iter().zip(y) {
                    if !self.same(*a, *b, depth + 1)? {
                        return Ok(false);
                    }
                }
                true
            }
            (
                TypeForm::Function {
                    parameters: a,
                    result: x,
                },
                TypeForm::Function {
                    parameters: b,
                    result: y,
                },
            ) => {
                if a.len() != b.len() {
                    return Ok(false);
                }
                for (a, b) in a.iter().zip(b) {
                    if !self.same(*a, *b, depth + 1)? {
                        return Ok(false);
                    }
                }
                self.same(*x, *y, depth + 1)?
            }
            (
                TypeForm::TaskFunction {
                    parameters: a,
                    result: x,
                    effect: e,
                },
                TypeForm::TaskFunction {
                    parameters: b,
                    result: y,
                    effect: f,
                },
            ) => {
                if e != f || !e.is_closed() || a.len() != b.len() {
                    return Ok(false);
                }
                for (a, b) in a.iter().zip(b) {
                    if !self.same(*a, *b, depth + 1)? {
                        return Ok(false);
                    }
                }
                self.same(*x, *y, depth + 1)?
            }
            // These forms have no stored child types. Exact canonical identity
            // also keeps nominal declarations and capability interfaces distinct.
            (
                TypeForm::Unit
                | TypeForm::Bool
                | TypeForm::I64
                | TypeForm::F64
                | TypeForm::Bytes
                | TypeForm::Text
                | TypeForm::StaticText
                | TypeForm::Secret
                | TypeForm::ByteBuffer
                | TypeForm::OwnedI64Cell
                | TypeForm::Named { .. }
                | TypeForm::CapabilityResource { .. },
                _,
            ) => template == candidate,
            _ => false,
        })
    }

    fn lookup(
        &mut self,
        template: TypeObjectDigest,
    ) -> Result<Option<TypeObjectDigest>, ExecutionError> {
        self.step(0)?;
        let source = self
            .schema
            .types
            .get(&template)
            .ok_or_else(|| invalid("missing canonical transfer template"))?;
        if let TypeForm::TypeParameter { parameter } = &source.form {
            let actual = self
                .bindings
                .get(parameter)
                .ok_or_else(|| invalid("transfer type parameter escaped its exact invocation"))?;
            return self
                .schema
                .types
                .contains_key(actual)
                .then_some(Some(*actual))
                .ok_or_else(|| invalid("bound transfer type is absent from canonical closure"));
        }
        // Most invocations retain an already closed signature. Try its exact
        // identity before walking the ordered canonical inventory.
        if self.same(template, template, 0)? {
            return Ok(Some(template));
        }
        let schema = self.schema;
        for candidate in schema.types.keys() {
            if *candidate != template && self.same(template, *candidate, 0)? {
                return Ok(Some(*candidate));
            }
        }
        Ok(None)
    }

    fn resolve(&mut self, template: TypeObjectDigest) -> Result<TypeObjectDigest, ExecutionError> {
        self.lookup(template)?.ok_or_else(|| {
            invalid("concrete transfer signature is absent from canonical type closure")
        })
    }
}

impl NormalizedReferenceSchema {
    pub(in super::super) fn admitted_type_identity(
        &self,
        template: TypeObjectDigest,
        bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        control: &ExecutionControl,
    ) -> Result<Option<TypeObjectDigest>, ExecutionError> {
        Search {
            schema: self,
            bindings,
            control,
            remaining: super::super::value::MAXIMUM_ADMISSION_ITEMS,
        }
        .lookup(template)
    }

    pub(in super::super) fn transfer_type_identity(
        &self,
        template: TypeObjectDigest,
        bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        control: &ExecutionControl,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        Search {
            schema: self,
            bindings,
            control,
            remaining: super::super::value::MAXIMUM_ADMISSION_ITEMS,
        }
        .resolve(template)
    }
}

fn invalid(message: &'static str) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "reference_transfer_substitution",
        message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::kernel::{Name, StructuralTypeField, TypeObject, encode_type_object};

    fn intern(schema: &mut NormalizedReferenceSchema, form: TypeForm) -> TypeObjectDigest {
        let object = TypeObject::new(form).unwrap();
        let digest = encode_type_object(&object).unwrap().0;
        schema.types.insert(digest, object);
        digest
    }

    fn wide() -> (
        NormalizedReferenceSchema,
        TypeObjectDigest,
        TypeObjectDigest,
        BTreeMap<TypeParameterId, TypeObjectDigest>,
    ) {
        let mut schema = NormalizedReferenceSchema::default();
        let parameter = TypeParameterId::from_bytes([97; 16]).unwrap();
        let symbolic = intern(&mut schema, TypeForm::TypeParameter { parameter });
        let scalar = intern(&mut schema, TypeForm::I64);
        let fields = |ty| {
            (0..256)
                .map(|index| StructuralTypeField {
                    name: Name::new(format!("field_{index:03}")).unwrap(),
                    ty,
                })
                .collect()
        };
        let template = intern(
            &mut schema,
            TypeForm::StructuralRecord {
                fields: fields(symbolic),
            },
        );
        let concrete = intern(
            &mut schema,
            TypeForm::StructuralRecord {
                fields: fields(scalar),
            },
        );
        (
            schema,
            template,
            concrete,
            BTreeMap::from([(parameter, scalar)]),
        )
    }

    #[test]
    fn reference_transfer_substitution_resolves_wide_names_without_rebuilding_type_storage() {
        let (schema, template, concrete, bindings) = wide();
        let control = ExecutionControl::uncancelled();
        let inventory = schema.types.len();
        let TypeForm::StructuralRecord { fields } = &schema.types[&concrete].form else {
            unreachable!()
        };
        let names = fields
            .iter()
            .map(|f| f.name.as_str().as_ptr())
            .collect::<Vec<_>>();
        assert_eq!(
            schema
                .transfer_type_identity(template, &bindings, &control)
                .unwrap(),
            concrete
        );
        assert_eq!(
            schema
                .transfer_type_identity(concrete, &BTreeMap::new(), &control)
                .unwrap(),
            concrete
        );
        assert_eq!(schema.types.len(), inventory);
        assert!(
            fields
                .iter()
                .zip(names)
                .all(|(field, name)| field.name.as_str().as_ptr() == name)
        );
        assert!(
            schema
                .transfer_type_identity(template, &BTreeMap::new(), &control)
                .is_err()
        );
    }

    #[test]
    fn reference_transfer_substitution_search_preserves_resource_and_cancellation_failures() {
        let (schema, template, _, bindings) = wide();
        let control = ExecutionControl::uncancelled();
        let mut search = Search {
            schema: &schema,
            bindings: &bindings,
            control: &control,
            remaining: 3,
        };
        assert_eq!(
            search.resolve(template).unwrap_err().class,
            ExecutionFailureClass::Resource
        );
        let cancelled = ExecutionControl::cancel_after_checks(1);
        assert_eq!(
            schema
                .transfer_type_identity(template, &bindings, &cancelled)
                .unwrap_err()
                .class,
            ExecutionFailureClass::Cancelled
        );
    }

    #[test]
    fn reference_transfer_substitution_resolves_exact_parameter_without_inventory_search() {
        let (schema, _, _, bindings) = wide();
        let symbolic = schema
            .types
            .iter()
            .find_map(|(digest, object)| {
                matches!(object.form, TypeForm::TypeParameter { .. }).then_some(*digest)
            })
            .unwrap();
        let control = ExecutionControl::uncancelled();
        let mut search = Search {
            schema: &schema,
            bindings: &bindings,
            control: &control,
            remaining: 1,
        };
        assert_eq!(
            search.resolve(symbolic).unwrap(),
            bindings.values().next().copied().unwrap()
        );
        assert_eq!(search.remaining, 0);
    }

    #[test]
    fn reference_sequence_substitution_preserves_exact_element_identity() {
        let mut schema = NormalizedReferenceSchema::default();
        let parameter = TypeParameterId::from_bytes([98; 16]).unwrap();
        let symbolic = intern(&mut schema, TypeForm::TypeParameter { parameter });
        let cell = intern(&mut schema, TypeForm::OwnedI64Cell);
        let buffer = intern(&mut schema, TypeForm::ByteBuffer);
        let template = intern(&mut schema, TypeForm::OwnedSequence { item: symbolic });
        let concrete = intern(&mut schema, TypeForm::OwnedSequence { item: cell });
        let other = intern(&mut schema, TypeForm::OwnedSequence { item: buffer });
        let control = ExecutionControl::uncancelled();
        assert_eq!(
            schema
                .transfer_type_identity(template, &BTreeMap::from([(parameter, cell)]), &control)
                .unwrap(),
            concrete
        );
        assert_eq!(
            schema
                .transfer_type_identity(template, &BTreeMap::from([(parameter, buffer)]), &control)
                .unwrap(),
            other
        );
        assert!(
            schema
                .transfer_type_identity(template, &BTreeMap::new(), &control)
                .is_err()
        );
        assert_eq!(
            schema.instantiated_identity(template, &BTreeMap::from([(parameter, cell)]), 0),
            Some(concrete)
        );
    }
}
