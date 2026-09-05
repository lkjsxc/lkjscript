//! Disposable reference value indexes, reconstructed only from canonical owner inventories.
//! Dense identities are ordered exact references for value interoperability, not compiler inputs.

use super::prepare::{
    NormalizedRecordField, NormalizedRecordLayout, NormalizedVariantCase, NormalizedVariantLayout,
};
use super::reference::{NormalizedReferenceReadWork, reference_error, reference_resource};
use super::value_schema::NormalizedValueSchema;
use crate::platform::execution::ExecutionError;
use crate::platform::kernel::{
    CaseReference, ComparisonPolicy, DeclarationPayload, DeclarationReference, FieldReference,
    KernelSnapshot, Name, OwnerKey, OwnerRecord, PackageId, StructuralTypeField, TypeForm,
    TypeObject, TypeObjectDigest, encode_type_object,
};
use crate::platform::semantic_id::{TargetId, TypeParameterId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default)]
pub struct NormalizedReferenceSchema {
    pub functions: Vec<DeclarationReference>,
    pub records: Vec<NormalizedRecordLayout>,
    pub variants: Vec<NormalizedVariantLayout>,
    pub types: BTreeMap<TypeObjectDigest, TypeObject>,
    pub targets: BTreeMap<(PackageId, Name), TargetId>,
    pub tests: BTreeMap<DeclarationReference, ComparisonPolicy>,
    pub work: NormalizedReferenceReadWork,
}

impl NormalizedReferenceSchema {
    pub fn reconstruct<'a>(
        snapshots: impl IntoIterator<Item = &'a KernelSnapshot>,
    ) -> Result<Self, ExecutionError> {
        let mut schema = Self::default();
        let mut packages = BTreeSet::new();
        let mut functions = BTreeSet::new();
        let mut records = BTreeMap::new();
        let mut variants = BTreeMap::new();
        let mut visited = 0_usize;
        for snapshot in snapshots {
            let package = snapshot.root.package_id;
            if !packages.insert(package) || packages.len() > 10_000 {
                return Err(inventory_error("duplicate or excessive canonical packages"));
            }
            visited = visited
                .checked_add(snapshot.owners.len())
                .and_then(|count| count.checked_add(snapshot.types.len()))
                .and_then(|count| count.checked_add(snapshot.dependency_types.len()))
                .filter(|count| *count <= 16_000_000)
                .ok_or_else(|| {
                    reference_resource(
                        "normalized_reference_inventory_bound",
                        "canonical reference inventory exceeds 16000000 visits",
                    )
                })?;
            schema.work.owner_reads = schema
                .work
                .owner_reads
                .saturating_add(snapshot.owners.len() as u64);
            for (digest, object) in snapshot.types.iter().chain(&snapshot.dependency_types) {
                if schema
                    .types
                    .insert(*digest, object.clone())
                    .is_some_and(|previous| previous != *object)
                {
                    return Err(inventory_error("canonical type identity conflict"));
                }
            }
            for (key, owner) in &snapshot.owners {
                match (key, owner) {
                    (OwnerKey::Target(id), OwnerRecord::Target(target)) => {
                        if schema
                            .targets
                            .insert((package, target.name.clone()), *id)
                            .is_some()
                        {
                            return Err(inventory_error("duplicate canonical target name"));
                        }
                    }
                    (OwnerKey::Declaration(id), OwnerRecord::Declaration(declaration)) => {
                        let reference = DeclarationReference {
                            package,
                            declaration: *id,
                        };
                        match &declaration.payload {
                            DeclarationPayload::Function(_)
                            | DeclarationPayload::External(_)
                            | DeclarationPayload::Constant { .. } => {
                                functions.insert(reference);
                            }
                            DeclarationPayload::Record { fields } => {
                                let mut layout = Vec::with_capacity(fields.len());
                                for field in fields {
                                    let Some(OwnerRecord::Field(record)) =
                                        snapshot.owners.get(&OwnerKey::Field(*field))
                                    else {
                                        return Err(inventory_error(
                                            "missing canonical record field",
                                        ));
                                    };
                                    if record.declaration != *id {
                                        return Err(inventory_error(
                                            "foreign canonical record field",
                                        ));
                                    }
                                    layout.push(NormalizedRecordField {
                                        reference: FieldReference {
                                            package,
                                            field: *field,
                                        },
                                        name: record.name.clone(),
                                        ty: record.ty,
                                    });
                                }
                                records.insert(
                                    reference,
                                    NormalizedRecordLayout {
                                        declaration: reference,
                                        fields: layout.into(),
                                    },
                                );
                            }
                            DeclarationPayload::Variant { cases } => {
                                let mut layout = Vec::with_capacity(cases.len());
                                for case in cases {
                                    let Some(OwnerRecord::Case(record)) =
                                        snapshot.owners.get(&OwnerKey::Case(*case))
                                    else {
                                        return Err(inventory_error(
                                            "missing canonical variant case",
                                        ));
                                    };
                                    if record.declaration != *id {
                                        return Err(inventory_error(
                                            "foreign canonical variant case",
                                        ));
                                    }
                                    layout.push(NormalizedVariantCase {
                                        reference: CaseReference {
                                            package,
                                            case: *case,
                                        },
                                        name: record.name.clone(),
                                        payload: record.payload,
                                    });
                                }
                                variants.insert(
                                    reference,
                                    NormalizedVariantLayout {
                                        declaration: reference,
                                        cases: layout.into(),
                                    },
                                );
                            }
                            DeclarationPayload::Test { comparison, .. } => {
                                schema.tests.insert(reference, *comparison);
                            }
                            DeclarationPayload::Interface { .. }
                            | DeclarationPayload::Component { .. } => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        schema.functions = functions.into_iter().collect();
        schema.records = records.into_values().collect();
        schema.variants = variants.into_values().collect();
        Ok(schema)
    }

    pub fn substitute_type(
        &self,
        digest: TypeObjectDigest,
        substitutions: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        depth: usize,
    ) -> Option<TypeObjectDigest> {
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return None;
        }
        let descend = |ty| self.substitute_type(ty, substitutions, depth.saturating_add(1));
        let form = match &self.types.get(&digest)?.form {
            TypeForm::TypeParameter { parameter } => {
                let resolved = substitutions.get(parameter)?;
                return self.types.contains_key(resolved).then_some(*resolved);
            }
            TypeForm::StructuralRecord { fields } => TypeForm::StructuralRecord {
                fields: fields
                    .iter()
                    .map(|field| {
                        Some(StructuralTypeField {
                            name: field.name.clone(),
                            ty: descend(field.ty)?,
                        })
                    })
                    .collect::<Option<_>>()?,
            },
            TypeForm::List { item } => TypeForm::List {
                item: descend(*item)?,
            },
            TypeForm::Map { key, value } => TypeForm::Map {
                key: descend(*key)?,
                value: descend(*value)?,
            },
            TypeForm::Option { item } => TypeForm::Option {
                item: descend(*item)?,
            },
            TypeForm::Result { ok, error } => TypeForm::Result {
                ok: descend(*ok)?,
                error: descend(*error)?,
            },
            TypeForm::Stream { item } => TypeForm::Stream {
                item: descend(*item)?,
            },
            TypeForm::Function { parameters, result } => TypeForm::Function {
                parameters: parameters
                    .iter()
                    .copied()
                    .map(descend)
                    .collect::<Option<_>>()?,
                result: descend(*result)?,
            },
            form => form.clone(),
        };
        let (resolved, _) = encode_type_object(&TypeObject::new(form).ok()?).ok()?;
        self.types.contains_key(&resolved).then_some(resolved)
    }
}

impl NormalizedValueSchema for NormalizedReferenceSchema {
    fn records(&self) -> &[NormalizedRecordLayout] {
        &self.records
    }
    fn variants(&self) -> &[NormalizedVariantLayout] {
        &self.variants
    }
    fn types(&self) -> &BTreeMap<TypeObjectDigest, TypeObject> {
        &self.types
    }
}

fn inventory_error(message: &str) -> ExecutionError {
    reference_error("normalized_reference_inventory", message)
}
