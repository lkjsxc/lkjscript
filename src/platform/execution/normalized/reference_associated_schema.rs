//! Private associated schema core. Publication remains gated on the remaining
//! symbolic custody and permission obligations; this module grants no admission.
use super::super::prepare::{
    NormalizedRecordField, NormalizedRecordLayout, NormalizedVariantCase, NormalizedVariantLayout,
};
use super::*;
use crate::platform::kernel::{CaseReference, FieldReference};
use crate::platform::semantic_id::{CaseId, FieldId};

fn inventory(
    schema: &mut NormalizedReferenceSchema,
    snapshots: &[&KernelSnapshot],
    visits: &mut usize,
    bytes: &mut usize,
    control: &crate::platform::execution::ExecutionControl,
) -> Result<(), ExecutionError> {
    // These layouts are rebuilt directly from canonical owners. Existing schema
    // layouts are disposable and cannot supply expected field or case meaning.
    schema.records.clear();
    schema.variants.clear();
    schema.record_instances.clear();
    schema.variant_instances.clear();
    schema.owned_record_declarations.clear();
    schema.functions.clear();
    let mut functions = BTreeSet::new();
    for snapshot in snapshots {
        for (key, owner) in &snapshot.owners {
            control.check()?;
            *visits = visits
                .checked_add(1)
                .filter(|n| *n <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK)
                .ok_or_else(capacity)?;
            let (OwnerKey::Declaration(id), OwnerRecord::Declaration(declaration)) = (key, owner)
            else {
                continue;
            };
            let reference = DeclarationReference {
                package: snapshot.root.package_id,
                declaration: *id,
            };
            if matches!(
                declaration.payload,
                DeclarationPayload::Function(_)
                    | DeclarationPayload::External(_)
                    | DeclarationPayload::Constant { .. }
            ) {
                index_node::<DeclarationReference>(bytes)?;
                functions.insert(reference);
            }
            let (parameters, fields, cases, owned): (
                &[TypeParameterId],
                &[FieldId],
                &[CaseId],
                bool,
            ) = match &declaration.payload {
                DeclarationPayload::Record {
                    type_parameters,
                    fields,
                } => (type_parameters, fields, &[], false),
                DeclarationPayload::OwnedRecord {
                    type_parameters,
                    fields,
                } => (type_parameters, fields, &[], true),
                DeclarationPayload::Variant {
                    type_parameters,
                    cases,
                } => (type_parameters, &[], cases, false),
                _ => continue,
            };
            // Four owned Arc slices publish each nominal layout, including empty arguments.
            allocate::<usize>(bytes, 8)?;
            allocate::<TypeParameterId>(bytes, parameters.len())?;
            allocate::<crate::platform::kernel::TypeParameterConstraints>(bytes, parameters.len())?;
            let mut constraints = Vec::with_capacity(parameters.len());
            for id in parameters {
                control.check()?;
                *visits = visits
                    .checked_add(1)
                    .filter(|n| *n <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK)
                    .ok_or_else(capacity)?;
                let OwnerRecord::TypeParameter(parameter) = snapshot
                    .owners
                    .get(&OwnerKey::TypeParameter(*id))
                    .ok_or_else(failure)?
                else {
                    return Err(failure());
                };
                if parameter.declaration != reference.declaration {
                    return Err(failure());
                }
                admit_parameter_generation(
                    snapshot
                        .owners
                        .get(&OwnerKey::TypeParameter(*id))
                        .ok_or_else(failure)?,
                )?;
                constraints.push(parameter.constraints);
            }
            if matches!(declaration.payload, DeclarationPayload::Variant { .. }) {
                allocate::<NormalizedVariantCase>(bytes, cases.len())?;
                allocate::<NormalizedVariantLayout>(bytes, 1)?;
                let mut layout = Vec::with_capacity(cases.len());
                for id in cases {
                    control.check()?;
                    *visits = visits
                        .checked_add(1)
                        .filter(|n| {
                            *n <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK
                        })
                        .ok_or_else(capacity)?;
                    let OwnerRecord::Case(case) = snapshot
                        .owners
                        .get(&OwnerKey::Case(*id))
                        .ok_or_else(failure)?
                    else {
                        return Err(failure());
                    };
                    if case.declaration != reference.declaration {
                        return Err(failure());
                    }
                    allocate::<u8>(bytes, case.name.as_str().len())?;
                    layout.push(NormalizedVariantCase {
                        reference: CaseReference {
                            package: reference.package,
                            case: *id,
                        },
                        name: case.name.clone(),
                        payload: case.payload,
                    });
                }
                schema.variants.push(NormalizedVariantLayout {
                    declaration: reference,
                    type_parameters: parameters.to_vec().into(),
                    type_parameter_constraints: constraints.into(),
                    arguments: Arc::from([]),
                    cases: layout.into(),
                });
            } else {
                allocate::<NormalizedRecordField>(bytes, fields.len())?;
                allocate::<NormalizedRecordLayout>(bytes, 1)?;
                let mut layout = Vec::with_capacity(fields.len());
                for id in fields {
                    control.check()?;
                    *visits = visits
                        .checked_add(1)
                        .filter(|n| {
                            *n <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK
                        })
                        .ok_or_else(capacity)?;
                    let OwnerRecord::Field(field) = snapshot
                        .owners
                        .get(&OwnerKey::Field(*id))
                        .ok_or_else(failure)?
                    else {
                        return Err(failure());
                    };
                    if field.declaration != reference.declaration {
                        return Err(failure());
                    }
                    allocate::<u8>(bytes, field.name.as_str().len())?;
                    layout.push(NormalizedRecordField {
                        reference: FieldReference {
                            package: reference.package,
                            field: *id,
                        },
                        name: field.name.clone(),
                        ty: field.ty,
                    });
                }
                if owned {
                    index_node::<DeclarationReference>(bytes)?;
                    schema.owned_record_declarations.insert(reference);
                }
                schema.records.push(NormalizedRecordLayout {
                    declaration: reference,
                    owned,
                    type_parameters: parameters.to_vec().into(),
                    type_parameter_constraints: constraints.into(),
                    arguments: Arc::from([]),
                    fields: layout.into(),
                });
            }
        }
    }
    allocate::<DeclarationReference>(bytes, functions.len())?;
    schema.functions = functions.into_iter().collect();
    Ok(())
}

fn capacity() -> ExecutionError {
    ExecutionError::resource(
        "reference_associated_schema_work",
        "associated canonical schema core exhausted its cumulative work allowance",
    )
}

pub(super) fn core(
    schema: &mut NormalizedReferenceSchema,
    snapshots: &[&KernelSnapshot],
    mut proof: associated_preparation::Proof,
    control: &crate::platform::execution::ExecutionControl,
) -> Result<(), ExecutionError> {
    control.check()?;
    let mut visits = usize::try_from(schema.source_admission_steps)
        .map_err(|_| capacity())?
        .checked_add(proof.steps)
        .filter(|n| *n <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK)
        .ok_or_else(capacity)?;
    let mut bytes = proof.bytes;
    // A complete object inventory has one owner. The retained proof stores only
    // immutable identities and source/application metadata after this move.
    for (ty, object) in &schema.types {
        control.check()?;
        visits = visits
            .checked_add(1)
            .filter(|n| *n <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK)
            .ok_or_else(capacity)?;
        if proof.types.get(ty) != Some(object) {
            return Err(failure());
        }
    }
    schema.types = proof.take_types();
    allocate::<associated_preparation::Proof>(&mut bytes, 1)?;
    allocate::<usize>(&mut bytes, 2)?;
    let proof = Arc::new(proof);
    schema.associated_proof = Some(Arc::clone(&proof));
    inventory(schema, snapshots, &mut visits, &mut bytes, control)?;
    for _ in snapshots {
        control.check()?;
        visits = visits
            .checked_add(1)
            .filter(|n| *n <= crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK)
            .ok_or_else(capacity)?;
        index_node::<(PackageId, &KernelSnapshot)>(&mut bytes)?;
    }
    let mut closure = Closure {
        associated_proof: Some(Arc::clone(&proof)),
        associated_application: None,
        parallel_targets: BTreeSet::new(),
        constrained_applications: BTreeSet::new(),
        symbolic: false,
        snapshots: snapshots
            .iter()
            .map(|snapshot| (snapshot.root.package_id, *snapshot))
            .collect(),
        types: &mut schema.types,
        visits,
        allocated: bytes,
        control,
        effects: BTreeMap::new(),
        requirements: BTreeMap::new(),
        implementation_bindings: BTreeMap::new(),
        checked_witnesses: BTreeSet::new(),
        interned_witnesses: BTreeMap::new(),
    };
    closure.owned_records()?;
    for application in proof.applications.keys() {
        closure.tick()?;
        let OwnerRecord::Declaration(owner) = closure.owner(
            application.function.package,
            OwnerKey::Declaration(application.function.declaration),
        )?
        else {
            return Err(failure());
        };
        let DeclarationPayload::Function(function) = &owner.payload else {
            continue;
        };
        allocate::<TypeObjectDigest>(&mut closure.allocated, application.types.len())?;
        allocate::<usize>(&mut closure.allocated, application.witnesses.len())?;
        allocate::<crate::platform::kernel::EffectRow>(
            &mut closure.allocated,
            application.effects.len(),
        )?;
        for row in &application.effects {
            allocate::<crate::platform::kernel::EffectParameterReference>(
                &mut closure.allocated,
                row.parameters.len(),
            )?;
            allocate::<crate::platform::kernel::RequirementOperand>(
                &mut closure.allocated,
                row.requirements.len(),
            )?;
        }
        allocate::<crate::platform::kernel::RequirementReference>(
            &mut closure.allocated,
            application.requirements.len(),
        )?;
        closure.associated_application = Some(application.clone());
        let mut bindings = BTreeMap::new();
        for (formal, actual) in function.type_parameters.iter().zip(&application.types) {
            closure.tick()?;
            index_node::<(TypeParameterId, TypeObjectDigest)>(&mut closure.allocated)?;
            bindings.insert(*formal, *actual);
        }
        borrowed_admission::body(
            &mut closure,
            application.function.package,
            function.body,
            &bindings,
            Some(application.function),
        )?;
    }
    closure.associated_application = None;
    closure.nominals(
        &mut schema.records,
        &mut schema.variants,
        &mut schema.record_instances,
        &mut schema.variant_instances,
    )?;
    closure.sequence_elements()?;
    closure.product_depths(
        &schema.records,
        &schema.record_instances,
        &schema.owned_record_declarations,
    )?;
    allocate::<bool>(&mut closure.allocated, schema.variants.len())?;
    schema.affine_variants.clear();
    for variant in &schema.variants {
        let mut affine = false;
        for case in variant.cases.iter() {
            closure.tick()?;
            if let Some(payload) = case.payload {
                affine |= matches!(
                    closure.types.get(&payload).ok_or_else(failure)?.form,
                    TypeForm::CapabilityResource { .. }
                );
            }
        }
        schema.affine_variants.push(affine);
    }
    visits = closure.visits;
    bytes = closure.allocated;
    schema.buffer_free_types = property_types(
        schema,
        &mut visits,
        &mut bytes,
        control,
        Retention::BufferFree,
    )?;
    schema.capture_safe_types =
        property_types(schema, &mut visits, &mut bytes, control, Retention::Capture)?;
    schema.transferable_types = property_types(
        schema,
        &mut visits,
        &mut bytes,
        control,
        Retention::Transfer,
    )?;
    schema.shareable_types =
        property_types(schema, &mut visits, &mut bytes, control, Retention::Share)?;
    schema.ordinary_types = property_types(
        schema,
        &mut visits,
        &mut bytes,
        control,
        Retention::Ordinary,
    )?;
    schema.comparable_types = property_types(
        schema,
        &mut visits,
        &mut bytes,
        control,
        Retention::Comparable,
    )?;
    schema.application_free_types = property_types(
        schema,
        &mut visits,
        &mut bytes,
        control,
        Retention::NoApplication,
    )?;
    schema.type_derivation_steps = (visits as u64)
        .checked_sub(schema.source_admission_steps)
        .ok_or_else(capacity)?;
    schema.type_metadata_bytes = bytes as u64;
    Ok(())
}
