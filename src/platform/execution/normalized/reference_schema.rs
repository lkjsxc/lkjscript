//! Disposable reference value indexes, reconstructed only from canonical owner inventories.
//! Dense identities are ordered exact references for value interoperability, not compiler inputs.

use super::prepare::{
    NormalizedRecordField, NormalizedRecordLayout, NormalizedVariantCase, NormalizedVariantLayout,
};
use super::reference::{NormalizedReferenceReadWork, reference_error, reference_resource};
use super::value_schema::NormalizedValueSchema;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::ExecutionError;
use crate::platform::kernel::{
    CaseReference, ComparisonPolicy, DeclarationPayload, DeclarationReference, FieldReference,
    KernelSnapshot, Name, OwnerKey, OwnerRecord, PackageId, StructuralTypeField, TypeForm,
    TypeObject, TypeObjectDigest, encode_type_object,
};
use crate::platform::semantic_id::{TargetId, TypeParameterId};
use std::collections::{BTreeMap, BTreeSet};

fn source_error(error: Diagnostic) -> ExecutionError {
    use crate::platform::execution::ExecutionFailureClass;
    let class = match error.class {
        DiagnosticClass::Cancelled => ExecutionFailureClass::Cancelled,
        DiagnosticClass::Resource => ExecutionFailureClass::Resource,
        DiagnosticClass::Semantic => ExecutionFailureClass::Trap,
        _ => ExecutionFailureClass::Infrastructure,
    };
    ExecutionError::new(class, error.code, error.message)
}

#[derive(Clone, Debug, Default)]
pub struct NormalizedReferenceSchema {
    pub(super) source_admission_steps: u64,
    pub(super) type_derivation_steps: u64,
    pub(super) type_metadata_bytes: u64,
    pub(super) affine_variants: Vec<bool>,
    pub(super) capture_safe_types: BTreeSet<TypeObjectDigest>,
    pub(super) ordinary_types: BTreeSet<TypeObjectDigest>,
    pub(super) comparable_types: BTreeSet<TypeObjectDigest>,
    pub(super) application_free_types: BTreeSet<TypeObjectDigest>,
    pub functions: Vec<DeclarationReference>,
    pub records: Vec<NormalizedRecordLayout>,
    pub variants: Vec<NormalizedVariantLayout>,
    pub(super) record_instances: BTreeMap<TypeObjectDigest, usize>,
    pub(super) variant_instances: BTreeMap<TypeObjectDigest, usize>,
    pub types: BTreeMap<TypeObjectDigest, TypeObject>,
    pub targets: BTreeMap<(PackageId, Name), TargetId>,
    pub tests: BTreeMap<DeclarationReference, ComparisonPolicy>,
    pub work: NormalizedReferenceReadWork,
}

impl NormalizedReferenceSchema {
    pub fn reconstruct<'a>(
        snapshots: impl IntoIterator<Item = &'a KernelSnapshot>,
    ) -> Result<Self, ExecutionError> {
        Self::reconstruct_with_control(
            snapshots,
            &crate::platform::execution::ExecutionControl::uncancelled(),
        )
    }

    pub(crate) fn reconstruct_with_control<'a>(
        snapshots: impl IntoIterator<Item = &'a KernelSnapshot>,
        control: &crate::platform::execution::ExecutionControl,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        let mut schema = Self::default();
        let mut packages = BTreeSet::new();
        let mut functions = BTreeSet::new();
        let mut records = BTreeMap::new();
        let mut variants = BTreeMap::new();
        let mut visited = 0_usize;
        let mut inputs = Vec::new();
        let mut source_work = 0_usize;
        for snapshot in snapshots {
            control.check()?;
            let package = snapshot.root.package_id;
            if !packages.insert(package) || packages.len() > 10_000 {
                return Err(inventory_error("duplicate or excessive canonical packages"));
            }
            // Raw snapshots carry no current artifact admission token. Check complete local
            // meaning, including nominal/callable flow and foreign application scope, before
            // deriving any exact instances. The work allowance is shared across packages.
            let report = crate::platform::kernel::validate_full_checked(
                snapshot,
                crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK - source_work,
                &mut 0,
                &|| {
                    control.check().map_err(|error| {
                        Diagnostic::new(DiagnosticClass::Cancelled, error.code, error.message)
                    })
                },
            )
            .map_err(|errors| {
                errors.into_iter().next().map_or_else(
                    || inventory_error("canonical source validation failed without a diagnostic"),
                    source_error,
                )
            })?;
            source_work += report.work_consumed as usize;
            schema.source_admission_steps = source_work as u64;
            inputs.push(snapshot);
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
                            DeclarationPayload::Record {
                                fields,
                                type_parameters,
                            } => {
                                let constraints =
                                    nominal_constraints(snapshot, *id, type_parameters)?;
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
                                        type_parameters: type_parameters.clone().into(),
                                        type_parameter_constraints: constraints.into(),
                                        arguments: Vec::new().into(),
                                        fields: layout.into(),
                                    },
                                );
                            }
                            DeclarationPayload::Variant {
                                cases,
                                type_parameters,
                            } => {
                                let constraints =
                                    nominal_constraints(snapshot, *id, type_parameters)?;
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
                                        type_parameters: type_parameters.clone().into(),
                                        type_parameter_constraints: constraints.into(),
                                        arguments: Vec::new().into(),
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
        validate_dependency_closure(&inputs, control, &mut source_work)?;
        schema.source_admission_steps = source_work as u64;
        schema.functions = functions.into_iter().collect();
        schema.records = records.into_values().collect();
        schema.variants = variants.into_values().collect();
        schema.affine_variants = schema
            .variants
            .iter()
            .map(|variant| {
                let mut affine = false;
                for case in variant.cases.iter() {
                    if let Some(payload) = case.payload {
                        let ty = schema.types.get(&payload).ok_or_else(|| {
                            inventory_error("missing canonical affinity payload type")
                        })?;
                        affine |= matches!(ty.form, TypeForm::CapabilityResource { .. });
                    }
                }
                Ok(affine)
            })
            .collect::<Result<_, ExecutionError>>()?;
        super::reference_types::complete(&mut schema, &inputs, control)?;
        Ok(schema)
    }

    pub fn substitute_type(
        &self,
        digest: TypeObjectDigest,
        substitutions: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        depth: usize,
    ) -> Option<TypeObjectDigest> {
        self.instantiated_identity(digest, substitutions, depth)
            .filter(|resolved| self.types.contains_key(resolved))
    }

    pub(super) fn instantiated_identity(
        &self,
        digest: TypeObjectDigest,
        substitutions: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        depth: usize,
    ) -> Option<TypeObjectDigest> {
        self.instantiated_with_effects(digest, substitutions, &BTreeMap::new(), depth)
    }

    pub(super) fn instantiated_with_effects(
        &self,
        digest: TypeObjectDigest,
        substitutions: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        effects: &super::reference_effects::Bindings,
        depth: usize,
    ) -> Option<TypeObjectDigest> {
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return None;
        }
        let descend = |ty| {
            self.instantiated_with_effects(ty, substitutions, effects, depth.saturating_add(1))
        };
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
            TypeForm::Applied {
                declaration,
                arguments,
            } => TypeForm::Applied {
                declaration: *declaration,
                arguments: arguments
                    .iter()
                    .copied()
                    .map(descend)
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
            TypeForm::TaskFunction {
                parameters,
                result,
                effect,
            } => TypeForm::TaskFunction {
                parameters: parameters
                    .iter()
                    .copied()
                    .map(descend)
                    .collect::<Option<_>>()?,
                result: descend(*result)?,
                effect: super::reference_effects::close(effect, effects, |_| Ok(())).ok()?,
            },
            _ => return Some(digest),
        };
        let (resolved, _) = encode_type_object(&TypeObject::new(form).ok()?).ok()?;
        Some(resolved)
    }
}

// Local callable SCCs exclude foreign applications only because complete package closures are
// acyclic. Establish that premise for raw reference inputs too; an interface is not a body proof.
fn validate_dependency_closure(
    snapshots: &[&KernelSnapshot],
    control: &crate::platform::execution::ExecutionControl,
    work: &mut usize,
) -> Result<(), ExecutionError> {
    let mut tick = || {
        control.check()?;
        *work = work
            .checked_add(1)
            .filter(|count| *count <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK)
            .ok_or_else(|| {
                reference_resource(
                    "normalized_reference_inventory_bound",
                    "canonical source admission work is exhausted",
                )
            })?;
        Ok::<_, ExecutionError>(())
    };
    let mut remaining = snapshots
        .iter()
        .map(|snapshot| (snapshot.root.package_id, snapshot.dependencies.len()))
        .collect::<BTreeMap<_, _>>();
    let mut parents = BTreeMap::<PackageId, Vec<PackageId>>::new();
    let mut edges = 0_usize;
    for snapshot in snapshots {
        tick()?;
        for dependency in snapshot.dependencies.keys() {
            tick()?;
            edges += 1;
            if edges > 100_000 {
                return Err(reference_resource(
                    "normalized_reference_inventory_bound",
                    "canonical reference dependency inventory exceeds 100000 edges",
                ));
            }
            if !remaining.contains_key(dependency) {
                return Err(inventory_error(
                    "complete canonical dependency body is unavailable",
                ));
            }
            parents
                .entry(*dependency)
                .or_default()
                .push(snapshot.root.package_id);
        }
    }
    let mut ready = remaining
        .iter()
        .filter_map(|(package, count)| (*count == 0).then_some(*package))
        .collect::<Vec<_>>();
    let mut admitted = 0_usize;
    while let Some(package) = ready.pop() {
        tick()?;
        admitted += 1;
        for parent in parents.remove(&package).unwrap_or_default() {
            tick()?;
            let count = remaining
                .get_mut(&parent)
                .ok_or_else(|| inventory_error("unknown canonical dependency parent"))?;
            *count = count
                .checked_sub(1)
                .ok_or_else(|| inventory_error("duplicate canonical dependency edge"))?;
            if *count == 0 {
                ready.push(parent);
            }
        }
    }
    if admitted != snapshots.len() {
        return Err(inventory_error(
            "canonical source dependency closure is cyclic",
        ));
    }
    Ok(())
}

impl NormalizedValueSchema for NormalizedReferenceSchema {
    fn comparable(&self, ty: TypeObjectDigest) -> bool {
        self.comparable_types.contains(&ty)
    }
    fn application_free(&self, ty: TypeObjectDigest) -> bool {
        self.application_free_types.contains(&ty)
    }
    fn value_origin(&self) -> super::value::ValueOrigin {
        Default::default()
    }

    fn records(&self) -> &[NormalizedRecordLayout] {
        &self.records
    }
    fn variants(&self) -> &[NormalizedVariantLayout] {
        &self.variants
    }
    fn types(&self) -> &BTreeMap<TypeObjectDigest, TypeObject> {
        &self.types
    }
    fn record_index(&self, ty: TypeObjectDigest) -> Option<usize> {
        self.record_instances.get(&ty).copied()
    }
    fn variant_index(&self, ty: TypeObjectDigest) -> Option<usize> {
        self.variant_instances.get(&ty).copied()
    }
}

fn nominal_constraints(
    snapshot: &KernelSnapshot,
    declaration: crate::platform::semantic_id::DeclarationId,
    parameters: &[TypeParameterId],
) -> Result<Vec<crate::platform::kernel::TypeParameterConstraints>, ExecutionError> {
    parameters
        .iter()
        .map(
            |parameter| match snapshot.owners.get(&OwnerKey::TypeParameter(*parameter)) {
                Some(OwnerRecord::TypeParameter(record)) if record.declaration == declaration => {
                    Ok(record.constraints)
                }
                _ => Err(inventory_error(
                    "nominal parameter lacks its exact canonical owner",
                )),
            },
        )
        .collect()
}

fn inventory_error(message: &str) -> ExecutionError {
    reference_error("normalized_reference_inventory", message)
}
