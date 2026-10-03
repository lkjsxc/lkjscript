//! Independent preflight for raw signatures whose concrete identity is not in
//! the canonical inventory. Existing raw generic admission may compute such a
//! digest; its temporary names, child vectors and encoding storage are reserved
//! before construction.
use super::NormalizedReferenceSchema;
use crate::platform::execution::{ExecutionControl, ExecutionError};
use crate::platform::kernel::{StructuralTypeField, TypeForm, TypeObjectDigest};
use crate::platform::semantic_id::TypeParameterId;
use std::collections::BTreeMap;

struct Scratch<'a> {
    schema: &'a NormalizedReferenceSchema,
    control: &'a ExecutionControl,
    effects: &'a super::super::reference_effects::Bindings,
    work: u64,
    bytes: u64,
}

impl Scratch<'_> {
    fn add(&mut self, bytes: u64) -> Result<(), ExecutionError> {
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .filter(|total| *total <= super::super::value::MAXIMUM_VALUE_ALLOCATION_BYTES)
            .ok_or_else(capacity)?;
        Ok(())
    }

    fn walk(&mut self, ty: TypeObjectDigest, depth: usize) -> Result<(), ExecutionError> {
        self.control.check()?;
        self.work = self.work.checked_sub(1).ok_or_else(capacity)?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(capacity());
        }
        let schema = self.schema;
        let object = schema.types.get(&ty).ok_or_else(capacity)?;
        let mut payload = 128_u64;
        match &object.form {
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => {
                self.add((fields.len() * 2 * std::mem::size_of::<StructuralTypeField>()) as u64)?;
                for field in fields {
                    self.control.check()?;
                    self.add(field.name.as_str().len() as u64)?;
                    payload = payload
                        .checked_add(128 + field.name.as_str().len() as u64)
                        .ok_or_else(capacity)?;
                    self.walk(field.ty, depth + 1)?;
                }
            }
            TypeForm::Applied { arguments, .. } => {
                self.add((arguments.len() * 2 * std::mem::size_of::<TypeObjectDigest>()) as u64)?;
                payload = payload
                    .checked_add(
                        (arguments.len() as u64)
                            .checked_mul(128)
                            .ok_or_else(capacity)?,
                    )
                    .ok_or_else(capacity)?;
                for argument in arguments {
                    self.walk(*argument, depth + 1)?;
                }
            }
            TypeForm::Function { parameters, result }
            | TypeForm::TaskFunction {
                parameters, result, ..
            } => {
                self.add((parameters.len() * 2 * std::mem::size_of::<TypeObjectDigest>()) as u64)?;
                payload = payload
                    .checked_add(
                        (parameters.len() as u64)
                            .checked_mul(128)
                            .ok_or_else(capacity)?,
                    )
                    .ok_or_else(capacity)?;
                for parameter in parameters {
                    self.walk(*parameter, depth + 1)?;
                }
                self.walk(*result, depth + 1)?;
                if let TypeForm::TaskFunction { effect, .. } = &object.form {
                    // Closing and encoding a closed row uses an ordered set,
                    // the retained row and the codec's row copy.
                    let mut atoms = effect.requirements.len();
                    for parameter in &effect.parameters {
                        self.control.check()?;
                        self.work = self.work.checked_sub(1).ok_or_else(capacity)?;
                        let count = self
                            .effects
                            .get(parameter)
                            .map_or(0, |row| row.requirements.len());
                        atoms = atoms.checked_add(count).ok_or_else(capacity)?;
                    }
                    self.add((atoms as u64).checked_mul(512).ok_or_else(capacity)?)?;
                    payload = payload
                        .checked_add((atoms as u64).checked_mul(128).ok_or_else(capacity)?)
                        .ok_or_else(capacity)?;
                }
            }
            TypeForm::List { item } | TypeForm::Option { item } | TypeForm::Stream { item } => {
                self.walk(*item, depth + 1)?;
            }
            TypeForm::Map { key, value }
            | TypeForm::Result {
                ok: key,
                error: value,
            } => {
                self.walk(*key, depth + 1)?;
                self.walk(*value, depth + 1)?;
            }
            // Canonical reference substitution retains atomic digests, including
            // exact parameter bindings, without constructing a type object.
            _ => return Ok(()),
        }
        // The payload bound includes all fixed wire atoms and length prefixes.
        // Four copies cover geometric encoder growth and the final integrity
        // envelope, with room for its fixed header/checksum.
        self.add(payload.checked_mul(4).ok_or_else(capacity)?)
    }
}

impl NormalizedReferenceSchema {
    pub(in super::super) fn type_instantiation_scratch(
        &self,
        template: TypeObjectDigest,
        effects: &super::super::reference_effects::Bindings,
        control: &ExecutionControl,
    ) -> Result<u64, ExecutionError> {
        let mut scratch = Scratch {
            schema: self,
            control,
            effects,
            work: super::super::value::MAXIMUM_ADMISSION_ITEMS,
            bytes: 0,
        };
        scratch.walk(template, 0)?;
        Ok(scratch.bytes)
    }

    pub(in super::super) fn raw_type_identity(
        &self,
        template: TypeObjectDigest,
        bindings: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        if let Some(exact) = self.admitted_type_identity(template, bindings, control)? {
            return Ok(exact);
        }
        reserve(self.type_instantiation_scratch(template, &BTreeMap::new(), control)?)?;
        self.instantiated_identity(template, bindings, 0)
            .ok_or_else(|| {
                super::super::reference::reference_error(
                    "reference_raw_instantiation",
                    "canonical raw type cannot be instantiated",
                )
            })
    }
}

fn capacity() -> ExecutionError {
    ExecutionError::resource(
        "reference_raw_instantiation_scratch",
        "canonical raw signature exceeds finite substitution scratch capacity",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::execution::ExecutionFailureClass;
    use crate::platform::kernel::{Name, TypeObject, encode_type_object};

    fn intern(schema: &mut NormalizedReferenceSchema, form: TypeForm) -> TypeObjectDigest {
        let object = TypeObject::new(form).unwrap();
        let digest = encode_type_object(&object).unwrap().0;
        schema.types.insert(digest, object);
        digest
    }

    #[test]
    fn reference_raw_signature_reserves_wide_names_before_unretained_identity_construction() {
        let mut schema = NormalizedReferenceSchema::default();
        let parameter = TypeParameterId::from_bytes([98; 16]).unwrap();
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
        let concrete = encode_type_object(
            &TypeObject::new(TypeForm::StructuralRecord {
                fields: fields(scalar),
            })
            .unwrap(),
        )
        .unwrap()
        .0;
        let bindings = BTreeMap::from([(parameter, scalar)]);
        let control = ExecutionControl::uncancelled();
        let inventory = schema.types.len();
        let mut requested = 0;
        let error = schema
            .raw_type_identity(template, &bindings, &control, &mut |bytes| {
                requested = bytes;
                Err(ExecutionError::resource(
                    "test_signature_quota",
                    "quota exhausted",
                ))
            })
            .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Resource);
        assert!(requested > 256 * std::mem::size_of::<StructuralTypeField>() as u64);
        assert_eq!(schema.types.len(), inventory);
        assert_eq!(
            schema
                .raw_type_identity(template, &bindings, &control, &mut |_| Ok(()))
                .unwrap(),
            concrete
        );
        let exact = intern(
            &mut schema,
            TypeForm::StructuralRecord {
                fields: fields(scalar),
            },
        );
        assert_eq!(
            schema
                .raw_type_identity(template, &bindings, &control, &mut |_| panic!(
                    "admitted identity needs no scratch"
                ))
                .unwrap(),
            exact
        );
    }
}
