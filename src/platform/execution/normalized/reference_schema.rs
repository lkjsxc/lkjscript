//! Disposable reference value indexes, reconstructed only from canonical owner inventories.
//! Dense identities are ordered exact references for value interoperability, not compiler inputs.

use super::prepare::{
    NormalizedRecordField, NormalizedRecordLayout, NormalizedVariantCase, NormalizedVariantLayout,
};
use super::reference::{NormalizedReferenceReadWork, reference_error, reference_resource};
use super::value_schema::NormalizedValueSchema;
use crate::platform::admission::{Meter, Observation};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::ExecutionError;
use crate::platform::kernel::{
    CaseReference, ComparisonPolicy, DeclarationPayload, DeclarationReference, FieldReference,
    KernelSnapshot, Name, OwnerKey, OwnerRecord, PackageId, StructuralTypeField, TypeForm,
    TypeObject, TypeObjectDigest, encode_type_object,
};
use crate::platform::semantic_id::{TargetId, TypeParameterId};
use std::collections::{BTreeMap, BTreeSet};

#[path = "reference_dependency_binding.rs"]
mod dependency_binding;
#[path = "reference_instantiation_scratch.rs"]
mod instantiation_scratch;
#[path = "reference_transfer_substitution.rs"]
mod transfer_substitution;

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

// Private canonical derivations use the same independent supplier proof as reconstruction.
pub(super) fn bind_associated_dependencies(
    snapshots: &[&KernelSnapshot],
    control: &crate::platform::execution::ExecutionControl,
    work: &mut Meter,
    bytes: &mut Meter,
) -> Result<(), ExecutionError> {
    validate_dependency_closure(snapshots, control, work)?;
    dependency_binding::validate_with_meters(snapshots, control, work, bytes)
}

#[derive(Clone, Debug, Default)]
pub struct NormalizedReferenceSchema {
    pub(super) read_view_proof:
        Option<std::sync::Arc<super::reference::read_view_admission::Proof>>,
    pub(super) associated_proof:
        Option<std::sync::Arc<super::reference_types::associated_preparation::Proof>>,
    pub(super) source_admission_steps: u64,
    pub(super) source_admission_saturated: bool,
    pub(super) type_derivation_steps: u64,
    pub(super) type_derivation_saturated: bool,
    pub(super) type_metadata_bytes: u64,
    pub(super) type_metadata_saturated: bool,
    pub(super) affine_variants: Vec<bool>,
    pub(super) capture_safe_types: BTreeSet<TypeObjectDigest>,
    pub(super) transferable_types: BTreeSet<TypeObjectDigest>,
    pub(super) shareable_types: BTreeSet<TypeObjectDigest>,
    pub(super) ordinary_types: BTreeSet<TypeObjectDigest>,
    pub(super) buffer_free_types: BTreeSet<TypeObjectDigest>,
    pub(super) comparable_types: BTreeSet<TypeObjectDigest>,
    pub(super) application_free_types: BTreeSet<TypeObjectDigest>,
    pub(super) owned_record_declarations: BTreeSet<DeclarationReference>,
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
    #[cfg(test)]
    pub(crate) fn independently_admit_witness_source(
        snapshot: &KernelSnapshot,
    ) -> Result<Self, ExecutionError> {
        // Deliberately bypass production source admission so mutation tests
        // establish the canonical reference's own complete witness proof.
        let mut schema = Self::default();
        schema.types.extend(snapshot.types.clone());
        schema.types.extend(snapshot.dependency_types.clone());
        super::reference_types::complete(
            &mut schema,
            &[snapshot],
            &crate::platform::execution::ExecutionControl::uncancelled(),
        )?;
        Ok(schema)
    }

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
        let mut inputs = Vec::new();
        let mut source_work = Meter::new(None);
        for snapshot in snapshots {
            control.check()?;
            let package = snapshot.root.package_id;
            if !packages.insert(package) {
                return Err(inventory_error("duplicate canonical packages"));
            }
            // Raw snapshots carry no current artifact admission token. Check complete local
            // meaning, including nominal/callable flow and foreign application scope, before
            // deriving any exact instances. Ordinary source admission has no work quota.
            let report = crate::platform::kernel::validate_full_checked(
                snapshot,
                None,
                &mut crate::platform::kernel::ValidationWork::default(),
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
            source_charge(
                &mut source_work,
                Observation {
                    consumed: report.work_consumed,
                    saturated: report.work_saturated,
                },
            )?;
            schema.source_admission_steps = source_work.observation().consumed;
            schema.source_admission_saturated = source_work.observation().saturated;
            inputs.try_reserve(1).map_err(|_| source_allocation())?;
            inputs.push(snapshot);
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
                            DeclarationPayload::ReadView(view) => {
                                for field in &view.fields {
                                    control.check()?;
                                    if !schema.types.contains_key(&field.ty) {
                                        return Err(inventory_error(
                                            "inline view field type is absent from its canonical type inventory",
                                        ));
                                    }
                                }
                            }
                            DeclarationPayload::OwnedContract(_)
                            | DeclarationPayload::OwnedImplementation(_) => {}
                            DeclarationPayload::Function(_)
                            | DeclarationPayload::External(_)
                            | DeclarationPayload::Constant { .. } => {
                                functions.insert(reference);
                            }
                            DeclarationPayload::Record {
                                fields,
                                type_parameters,
                            }
                            | DeclarationPayload::OwnedRecord {
                                fields,
                                type_parameters,
                            } => {
                                let constraints =
                                    nominal_constraints(snapshot, *id, type_parameters)?;
                                let mut layout = source_items(fields.len())?;
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
                                        owned: matches!(
                                            declaration.payload,
                                            DeclarationPayload::OwnedRecord { .. }
                                        ),
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
                                let mut layout = source_items(cases.len())?;
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
        let after_local_full = source_work.observation();
        validate_dependency_closure(&inputs, control, &mut source_work)?;
        dependency_binding::validate(&inputs, control, &mut source_work)?;
        let after_dependency_correspondence = source_work.observation();
        schema.source_admission_steps = source_work.observation().consumed;
        schema.source_admission_saturated = source_work.observation().saturated;
        schema.functions = source_items(functions.len())?;
        schema.functions.extend(functions);
        schema.records = source_items(records.len())?;
        schema.records.extend(records.into_values());
        schema.variants = source_items(variants.len())?;
        schema.variants.extend(variants.into_values());
        schema.affine_variants = source_items(schema.variants.len())?;
        for variant in &schema.variants {
            let mut affine = false;
            for case in variant.cases.iter() {
                if let Some(payload) = case.payload {
                    let ty = schema.types.get(&payload).ok_or_else(|| {
                        inventory_error("missing canonical affinity payload type")
                    })?;
                    affine |= matches!(ty.form, TypeForm::CapabilityResource { .. });
                }
            }
            schema.affine_variants.push(affine);
        }
        // A new exact phase counter avoids treating a saturated earlier observation
        // as an exact seed. Phase totals are combined only as passive observations.
        let mut composed_work = crate::platform::kernel::ValidationWork::default();
        crate::platform::kernel::callable_flow::validate_snapshot_callable_closure(
            &inputs,
            &|| {
                control.check().map_err(|error| {
                    Diagnostic::new(DiagnosticClass::Cancelled, error.code, error.message)
                })
            },
            &mut composed_work,
            None,
        )
        .map_err(source_error)?;
        let mut composed = Observation::from_usize(composed_work.consumed);
        composed.saturated |= composed_work.saturated;
        source_charge(&mut source_work, composed)?;
        let after_composed = source_work.observation();
        schema.source_admission_steps = after_composed.consumed;
        schema.source_admission_saturated = after_composed.saturated;
        super::reference_types::complete(&mut schema, &inputs, control).map_err(|mut error| {
            if error.code == "kernel_associated_reference_capacity"
                && error.message.contains("reference_flow phase=")
            {
                error.message.push_str(&format!(
                    "; reference_source_admission packages={} after_local_full={} after_dependency_correspondence={} after_composed={} observation_saturated={}",
                    inputs.len(), after_local_full.consumed, after_dependency_correspondence.consumed, after_composed.consumed, after_composed.saturated,
                ));
            }
            error
        })?;
        if schema.read_view_proof.is_none() {
            schema.register_read_views(&inputs, control)?;
        }
        Ok(schema)
    }

    pub(super) fn owned_record_layout(
        &self,
        ty: TypeObjectDigest,
    ) -> Option<&NormalizedRecordLayout> {
        let (declaration, arguments) = match &self.types.get(&ty)?.form {
            TypeForm::Named { declaration } => (*declaration, &[][..]),
            TypeForm::Applied {
                declaration,
                arguments,
            } => (*declaration, arguments.as_slice()),
            _ => return None,
        };
        let layout = self.records.get(*self.record_instances.get(&ty)?)?;
        (layout.owned
            && layout.declaration == declaration
            && layout.arguments.as_ref() == arguments
            && layout.type_parameters.len() == arguments.len())
        .then_some(layout)
    }

    pub(super) fn associated_proof(
        &self,
    ) -> Option<&std::sync::Arc<super::reference_types::associated_preparation::Proof>> {
        self.associated_proof.as_ref()
    }

    pub(super) fn register_read_views(
        &mut self,
        snapshots: &[&KernelSnapshot],
        control: &crate::platform::execution::ExecutionControl,
    ) -> Result<(), ExecutionError> {
        let proof = super::reference::read_view_admission::Proof::reconstruct_with_types(
            snapshots,
            &self.types,
            control,
        )?;
        for (ty, object) in proof.types() {
            control.check()?;
            if self
                .types
                .get(ty)
                .is_some_and(|existing| existing != object)
            {
                return Err(inventory_error(
                    "read-view type identity disagrees with canonical schema",
                ));
            }
            self.types.insert(*ty, object.clone());
        }
        self.read_view_proof = Some(std::sync::Arc::new(proof));
        Ok(())
    }

    pub(super) fn is_owned_type(&self, ty: TypeObjectDigest) -> bool {
        self.types.get(&ty).is_some_and(|object| {
            matches!(
                object.form,
                TypeForm::RegionCap {
                    mode: crate::platform::kernel::RegionMode::Unique
                        | crate::platform::kernel::RegionMode::Frozen
                        | crate::platform::kernel::RegionMode::Pinned,
                    ..
                } | TypeForm::ByteBuffer
                    | TypeForm::OwnedI64Cell
                    | TypeForm::OwnedProduct { .. }
                    | TypeForm::OwnedChoice { .. }
                    | TypeForm::OwnedSequence { .. }
            )
        }) || self
            .types
            .get(&ty)
            .is_some_and(|object| match &object.form {
                TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => {
                    self.owned_record_declarations.contains(declaration)
                }
                _ => false,
            })
    }

    pub(super) fn is_read_view_type(&self, ty: TypeObjectDigest) -> bool {
        self.types.get(&ty).is_some_and(|object| {
            matches!(
                object.form,
                TypeForm::ReadViewApplied { .. }
                    | TypeForm::AssociatedReadViewMember { .. }
                    | TypeForm::AssociatedReadViewProjection { .. }
                    | TypeForm::RegionCap {
                        mode: crate::platform::kernel::RegionMode::Shared { .. },
                        ..
                    }
            )
        })
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
        self.instantiated_with_effects(
            digest,
            substitutions,
            &BTreeMap::new(),
            &BTreeMap::new(),
            depth,
        )
    }

    pub(super) fn instantiated_with_effects(
        &self,
        digest: TypeObjectDigest,
        substitutions: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        effects: &super::reference_effects::Bindings,
        requirements: &super::reference_effects::RequirementBindings,
        depth: usize,
    ) -> Option<TypeObjectDigest> {
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return None;
        }
        let descend = |ty| {
            self.instantiated_with_effects(
                ty,
                substitutions,
                effects,
                requirements,
                depth.saturating_add(1),
            )
        };
        let form = match &self.types.get(&digest)?.form {
            TypeForm::TypeParameter { parameter } => {
                let resolved = substitutions.get(parameter)?;
                return self.types.contains_key(resolved).then_some(*resolved);
            }
            TypeForm::OwnedSequence { item } => TypeForm::OwnedSequence {
                item: descend(*item)?,
            },
            TypeForm::OwnedChoice { cases } => TypeForm::OwnedChoice {
                cases: cases
                    .iter()
                    .map(|case| {
                        Some(StructuralTypeField {
                            name: case.name.clone(),
                            ty: descend(case.ty)?,
                        })
                    })
                    .collect::<Option<_>>()?,
            },
            TypeForm::OwnedProduct { fields } => TypeForm::OwnedProduct {
                fields: fields
                    .iter()
                    .map(|f| {
                        Some(StructuralTypeField {
                            name: f.name.clone(),
                            ty: descend(f.ty)?,
                        })
                    })
                    .collect::<Option<_>>()?,
            },
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
            TypeForm::ReadViewApplied {
                declaration,
                arguments,
                read_scope,
            } => TypeForm::ReadViewApplied {
                declaration: *declaration,
                arguments: arguments
                    .iter()
                    .copied()
                    .map(descend)
                    .collect::<Option<_>>()?,
                read_scope: *read_scope,
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
            TypeForm::Region { region } => TypeForm::Region {
                region: descend(*region)?,
            },
            TypeForm::RelPtr { region, item } => TypeForm::RelPtr {
                region: descend(*region)?,
                item: descend(*item)?,
            },
            TypeForm::RegionSlice { region, item } => TypeForm::RegionSlice {
                region: descend(*region)?,
                item: descend(*item)?,
            },
            TypeForm::RegionCap { region, mode } => TypeForm::RegionCap {
                region: descend(*region)?,
                mode: *mode,
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
                effect: super::reference_effects::close(effect, effects, requirements, |_| Ok(()))
                    .ok()?,
            },
            _ => return Some(digest),
        };
        let (resolved, _) = encode_type_object(&TypeObject::new(form).ok()?).ok()?;
        Some(resolved)
    }
}

fn source_charge(work: &mut Meter, amount: Observation) -> Result<(), ExecutionError> {
    work.charge(amount).map_err(|_| {
        reference_resource(
            "normalized_reference_inventory_bound",
            "canonical source admission exhausted the selected work allowance",
        )
    })
}

fn source_allocation() -> ExecutionError {
    reference_resource(
        "normalized_reference_inventory_bound",
        "canonical source admission cannot reserve representable or available storage",
    )
}

fn source_items<T>(count: usize) -> Result<Vec<T>, ExecutionError> {
    let mut items = Vec::new();
    items
        .try_reserve_exact(count)
        .map_err(|_| source_allocation())?;
    Ok(items)
}

// Bind exact source inventories and reject package dependency cycles here.
// Supplied witnesses can still create callable cycles across the package DAG;
// complete composition is proved separately before instance derivation.
fn validate_dependency_closure(
    snapshots: &[&KernelSnapshot],
    control: &crate::platform::execution::ExecutionControl,
    work: &mut Meter,
) -> Result<(), ExecutionError> {
    let mut tick = || {
        control.check()?;
        source_charge(work, Observation::exact(1))?;
        Ok::<_, ExecutionError>(())
    };
    let mut remaining = snapshots
        .iter()
        .map(|snapshot| (snapshot.root.package_id, snapshot.dependencies.len()))
        .collect::<BTreeMap<_, _>>();
    let mut parents = BTreeMap::<PackageId, Vec<PackageId>>::new();
    for snapshot in snapshots {
        tick()?;
        for dependency in snapshot.dependencies.keys() {
            tick()?;
            if !remaining.contains_key(dependency) {
                return Err(inventory_error(
                    "complete canonical dependency body is unavailable",
                ));
            }
            let values = parents.entry(*dependency).or_default();
            values.try_reserve(1).map_err(|_| source_allocation())?;
            values.push(snapshot.root.package_id);
        }
    }
    let mut ready = source_items(remaining.len())?;
    ready.extend(
        remaining
            .iter()
            .filter_map(|(package, count)| (*count == 0).then_some(*package)),
    );
    let mut admitted = 0_usize;
    while let Some(package) = ready.pop() {
        tick()?;
        admitted = admitted.checked_add(1).ok_or_else(source_allocation)?;
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

#[cfg(test)]
#[path = "reference_schema_capacity_tests.rs"]
mod capacity_tests;
