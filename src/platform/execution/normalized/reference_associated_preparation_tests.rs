//! Canonical expected meaning is derived without producer selections or lookup.
//! These are private preparation checks; the global associated admission gate stays on.
use super::*;
use crate::platform::execution::normalized::prepare::{
    NormalizedAssociatedTypeBinding, NormalizedCode, NormalizedFunction, NormalizedFunctionBody,
    NormalizedImplementationApplication, NormalizedImplementationArgument, NormalizedInstruction,
    NormalizedParameter,
};
use crate::platform::kernel::{
    AssociatedTypeBinding, AssociatedTypeMember, DeclarationRecord, DeclarationVisibility, Name,
    OwnerHeader, OwnerKind, StructuralTypeField, TypeParameterRecord,
};
use crate::platform::semantic_id::{DeclarationId, ParameterId, TypeParameterId};

const SEED: &[u8] = b"canonical-associated-preparation";
const SOURCE: &str = r#"
declarations.begin
(units (module create proof
  (owned-contract create Read (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_c1000000000000000000000000000001 read
      (parameters (Self borrow)) (returns Self (borrow-from 0))))
  (function create read (visibility public) (effect pure)
    (type-parameter create V (constraint owned))
    (parameter create value (type V) (use borrow))
    (returns V (borrow-from value)) (body (local value)))
  (owned-implementation create Cell (visibility public)
    (type-parameter create T (constraint owned)) (contract Read) (self T)
    (method method_c1000000000000000000000000000001 read (types T)))
  (function create consumer (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_c1000000000000000000000000000001 ops Read T)
    (parameter create value (type T) (use borrow))
    (returns T (borrow-from value)) (body (local value)))))
declarations.end
"#;

pub(in super::super::super) struct Fixture {
    pub(in super::super::super) source: KernelSnapshot,
    pub(in super::super::super) contract: DeclarationReference,
    pub(in super::super::super) consumer: DeclarationReference,
    pub(in super::super::super) reader: DeclarationReference,
    pub(in super::super::super) implementations: [DeclarationReference; 3],
    pub(in super::super::super) member: AssociatedTypeMemberId,
    pub(in super::super::super) spare: AssociatedTypeMemberId,
    pub(in super::super::super) cell: TypeObjectDigest,
    pub(in super::super::super) block: TypeObjectDigest,
    pub(in super::super::super) projection: TypeObjectDigest,
}
fn name(value: &str) -> Name {
    Name::new(value).unwrap()
}
fn named(source: &KernelSnapshot, value: &str) -> DeclarationReference {
    source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(owner))
                if owner.name.as_str() == value =>
            {
                Some(DeclarationReference {
                    package: source.root.package_id,
                    declaration: *id,
                })
            }
            _ => None,
        })
        .unwrap()
}
fn payload(source: &mut KernelSnapshot, scope: DeclarationReference) -> &mut DeclarationPayload {
    let OwnerRecord::Declaration(owner) = source
        .owners
        .get_mut(&OwnerKey::Declaration(scope.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    &mut owner.payload
}
fn add(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    source.types.insert(ty, object);
    ty
}
pub(in super::super::super) fn base_source() -> KernelSnapshot {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE).unwrap()
}
pub(in super::super::super) fn make_fixture() -> Fixture {
    let mut source = base_source();
    let contract = named(&source, "Read");
    let consumer = named(&source, "consumer");
    let reader = named(&source, "read");
    let first = named(&source, "Cell");
    let member = AssociatedTypeMemberId::migrate(SEED, 0);
    let spare = AssociatedTypeMemberId::migrate(SEED, 1);
    let cell = add(&mut source, TypeForm::OwnedI64Cell);
    let scalar = add(&mut source, TypeForm::I64);
    let block = add(&mut source, TypeForm::OwnedSequence { item: scalar });
    let abstract_member = add(
        &mut source,
        TypeForm::AssociatedContractMember { contract, member },
    );
    let DeclarationPayload::OwnedContract(value) = payload(&mut source, contract) else {
        unreachable!()
    };
    value.associated_types = vec![
        AssociatedTypeMember {
            id: spare,
            name: name("Unused"),
            bound: AssociatedTypeBound::Owned,
        },
        AssociatedTypeMember {
            id: member,
            name: name("Block"),
            bound: AssociatedTypeBound::Owned,
        },
    ];
    value.methods[0].parameters[0].ty = abstract_member;
    value.methods[0].result = abstract_member;
    let DeclarationPayload::Function(value) = payload(&mut source, consumer) else {
        unreachable!()
    };
    let witness = value.implementation_parameters[0].id;
    let parameter = value.parameters[0];
    let projection = add(
        &mut source,
        TypeForm::AssociatedWitnessProjection {
            scope: consumer,
            parameter: witness,
            contract,
            member,
        },
    );
    let DeclarationPayload::Function(value) = payload(&mut source, consumer) else {
        unreachable!()
    };
    value.result = projection;
    let OwnerRecord::Parameter(value) = source
        .owners
        .get_mut(&OwnerKey::Parameter(parameter))
        .unwrap()
    else {
        unreachable!()
    };
    value.ty = projection;
    let OwnerRecord::Declaration(first_owner) =
        source.owners[&OwnerKey::Declaration(first.declaration)].clone()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(first_scheme) = first_owner.payload.clone() else {
        unreachable!()
    };
    let mut implementations = [first; 3];
    for (index, output) in [cell, block, cell].into_iter().enumerate() {
        let reference = if index == 0 {
            first
        } else {
            DeclarationReference {
                package: source.root.package_id,
                declaration: DeclarationId::migrate(SEED, index as u64),
            }
        };
        implementations[index] = reference;
        let formal = if index == 0 {
            first_scheme.type_parameters[0]
        } else {
            TypeParameterId::migrate(SEED, index as u64)
        };
        if index != 0 {
            source.owners.insert(
                OwnerKey::TypeParameter(formal),
                OwnerRecord::TypeParameter(TypeParameterRecord {
                    header: OwnerHeader::new(
                        OwnerKey::TypeParameter(formal),
                        OwnerKind::TypeParameter,
                    ),
                    declaration: reference.declaration,
                    name: name("T"),
                    constraints: TypeParameterConstraints::Owned,
                }),
            );
        }
        let formal_type = add(&mut source, TypeForm::TypeParameter { parameter: formal });
        let mut scheme = first_scheme.clone();
        scheme.type_parameters = vec![formal];
        scheme.self_type = formal_type;
        scheme.methods[0].type_arguments = vec![output];
        scheme.associated_types = vec![
            AssociatedTypeBinding { member, ty: output },
            AssociatedTypeBinding {
                member: spare,
                ty: cell,
            },
        ];
        scheme
            .associated_types
            .sort_by_key(|binding| binding.member);
        source.owners.insert(
            OwnerKey::Declaration(reference.declaration),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(
                    OwnerKey::Declaration(reference.declaration),
                    OwnerKind::OwnedImplementation,
                ),
                module: first_owner.module,
                name: name(&format!("supplier{index}")),
                visibility: DeclarationVisibility::Public,
                payload: DeclarationPayload::OwnedImplementation(scheme),
            }),
        );
    }
    Fixture {
        source,
        contract,
        consumer,
        reader,
        implementations,
        member,
        spare,
        cell,
        block,
        projection,
    }
}
pub(in super::super::super) fn operand(fixture: &Fixture, index: usize) -> ImplementationOperand {
    ImplementationOperand::Concrete {
        implementation: fixture.implementations[index],
        type_arguments: vec![fixture.cell],
        implementations: vec![],
    }
}
pub(in super::super::super) fn prove(
    fixture: &Fixture,
    indexes: &[usize],
    limits: Limits,
    control: &ExecutionControl,
) -> Result<Proof, ExecutionError> {
    let witnesses = indexes
        .iter()
        .map(|index| vec![operand(fixture, *index)])
        .collect::<Vec<_>>();
    let types = [fixture.cell];
    let entries = witnesses
        .iter()
        .map(|implementations| Entry {
            function: fixture.consumer,
            types: &types,
            implementations,
            effects: &[],
            requirements: &[],
        })
        .collect::<Vec<_>>();
    derive(&[&fixture.source], &entries, limits, control)
}
fn selected(fixture: &Fixture, index: usize) -> NormalizedImplementationArgument {
    let OwnerRecord::Declaration(owner) =
        &fixture.source.owners[&OwnerKey::Declaration(fixture.implementations[index].declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(scheme) = &owner.payload else {
        unreachable!()
    };
    let output = if index == 1 {
        fixture.block
    } else {
        fixture.cell
    };
    Arc::new(NormalizedImplementationApplication {
        identity: index as u32,
        depth: 0,
        implementation: fixture.implementations[index],
        contract: fixture.contract,
        implementation_generation: 33,
        contract_generation: 33,
        implementation_type_parameters: scheme.type_parameters.clone().into(),
        implementation_parameter_ids: Arc::from([]),
        implementation_type_arguments: Arc::from([fixture.cell]),
        implementation_type_parameter_constraints: Arc::from([TypeParameterConstraints::Owned]),
        self_type: fixture.cell,
        type_arguments: Arc::from([]),
        contract_type_parameter_constraints: Arc::from([]),
        implementations: Arc::from([]),
        prerequisites: Arc::from([]),
        associated_types: Arc::from([
            NormalizedAssociatedTypeBinding {
                member: fixture.spare,
                template: fixture.cell,
                ty: fixture.cell,
            },
            NormalizedAssociatedTypeBinding {
                member: fixture.member,
                template: output,
                ty: output,
            },
        ]),
    })
}
fn normalized(fixture: &Fixture, selected: NormalizedImplementationArgument) -> NormalizedFunction {
    let OwnerRecord::Declaration(owner) =
        &fixture.source.owners[&OwnerKey::Declaration(fixture.consumer.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &owner.payload else {
        unreachable!()
    };
    NormalizedFunction {
        source_generation: owner.header.contract_version,
        implementation_parameters: function.implementation_parameters.clone().into(),
        implementation_arguments: Arc::from([selected]),
        callsites: Arc::from([]),
        requirement_parameters: Arc::from([]),
        requirement_arguments: Arc::from([]),
        effect_parameters: Arc::from([]),
        effect: function.effect.clone(),
        effect_arguments: Arc::from([]),
        declaration: fixture.consumer,
        graph_function: true,
        type_parameters: function.type_parameters.clone().into(),
        type_arguments: Arc::from([fixture.cell]),
        type_parameter_constraints: Arc::from([TypeParameterConstraints::Owned]),
        parameter_count: function.parameters.len() as u32,
        parameters: function
            .parameters
            .iter()
            .map(|parameter| {
                let OwnerRecord::Parameter(record) =
                    &fixture.source.owners[&OwnerKey::Parameter(*parameter)]
                else {
                    unreachable!()
                };
                NormalizedParameter {
                    parameter: *parameter,
                    name: record.name.clone(),
                    ty: record.ty,
                    use_mode: record.use_mode,
                    resource_requirement: None,
                }
            })
            .collect::<Vec<_>>()
            .into(),
        result: function.result,
        result_borrow: Some(0),
        task_requirements: Arc::from([]),
        body: NormalizedFunctionBody::Code(NormalizedCode {
            parameter_count: 1,
            local_count: 1,
            instructions: Arc::from([NormalizedInstruction::LoadLocal {
                local: 0,
                use_mode: ParameterUse::Borrow,
            }]),
        }),
    }
}

#[test]
fn canonical_associated_preparation_distinguishes_witnesses_and_exact_mapped_targets() {
    let fixture = make_fixture();
    let proof = prove(
        &fixture,
        &[0, 1, 2],
        Limits::default(),
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert_eq!(proof.selections.len(), 3);
    let consumers = proof
        .applications
        .iter()
        .filter(|(application, _)| application.function == fixture.consumer)
        .collect::<Vec<_>>();
    assert_eq!(consumers.len(), 3);
    for (application, function) in consumers {
        let selected = &proof.selections[application.witnesses[0]];
        let output = if selected.implementation == fixture.implementations[1] {
            fixture.block
        } else {
            fixture.cell
        };
        assert_eq!(function.parameters, vec![(output, ParameterUse::Borrow)]);
        assert_eq!(function.result, output);
        assert_eq!(function.result_borrow, Some(0));
        let target = proof
            .methods
            .iter()
            .find(|((identity, _), _)| *identity == selected.identity)
            .unwrap()
            .1;
        assert_eq!(target.function, fixture.reader);
        assert_eq!(target.types, vec![output]);
        assert_eq!(selected.members.len(), 2);
        assert_eq!(selected.members[0].member, fixture.spare);
    }
    assert_eq!(
        proof
            .applications
            .keys()
            .filter(|application| application.function == fixture.reader)
            .count(),
        2
    );
}

#[test]
fn canonical_associated_preparation_compares_real_materialization_and_rejects_corrupt_catalogues() {
    let fixture = make_fixture();
    let control = ExecutionControl::uncancelled();
    let proof = prove(&fixture, &[0, 1, 2], Limits::default(), &control).unwrap();
    let catalog = (0..3)
        .map(|index| selected(&fixture, index))
        .collect::<Vec<_>>();
    for index in 0..3 {
        let expected = proof
            .selections
            .iter()
            .find(|selection| selection.implementation == fixture.implementations[index])
            .unwrap();
        proof
            .compare_selection(expected, &catalog[index], &catalog, &proof.types, &control)
            .unwrap();
        let function = normalized(&fixture, Arc::clone(&catalog[index]));
        let ordinary = BTreeMap::from([(function.type_parameters[0], fixture.cell)]);
        let mut types = fixture.source.types.clone();
        let actual =
            crate::platform::execution::normalized::prepared_types::materialize_associated_roots(
                &function,
                &ordinary,
                &[fixture.projection],
                &mut types,
                &control,
            )
            .unwrap();
        let application = proof
            .applications
            .iter()
            .find(|(application, _)| {
                application.function == fixture.consumer
                    && proof.selections[application.witnesses[0]].implementation
                        == fixture.implementations[index]
            })
            .unwrap()
            .1;
        assert_eq!(actual, vec![application.result]);
    }
    for change in 0..7 {
        let mut changed = (*catalog[0]).clone();
        match change {
            0 => Arc::make_mut(&mut changed.associated_types)[1].ty = fixture.block,
            1 => Arc::make_mut(&mut changed.associated_types)[1].member = fixture.spare,
            2 => changed.contract = fixture.implementations[1],
            3 => changed.implementation = fixture.implementations[2],
            4 => changed.implementation_type_arguments = Arc::from([fixture.block]),
            5 => {
                changed.implementation_type_parameter_constraints =
                    Arc::from([TypeParameterConstraints::OwnedTransferable])
            }
            _ => changed.implementation_generation = 32,
        }
        let mut changed_catalog = catalog.clone();
        changed_catalog[0] = Arc::new(changed);
        assert!(
            proof
                .compare_selection(
                    &proof.selections[0],
                    &changed_catalog[0],
                    &changed_catalog,
                    &proof.types,
                    &control
                )
                .is_err()
        );
    }
    let detached = Arc::new((*catalog[0]).clone());
    assert!(
        proof
            .compare_selection(
                &proof.selections[0],
                &detached,
                &catalog,
                &proof.types,
                &control
            )
            .is_err()
    );
    let mut wrong_types = proof.types.clone();
    wrong_types.insert(fixture.cell, TypeObject::new(TypeForm::ByteBuffer).unwrap());
    assert!(
        proof
            .compare_selection(
                &proof.selections[0],
                &catalog[0],
                &catalog,
                &wrong_types,
                &control
            )
            .is_err()
    );
    for source in [fixture.reader, fixture.implementations[0]] {
        let mut changed = normalized(&fixture, Arc::clone(&catalog[0]));
        changed.declaration = source;
        let ordinary = BTreeMap::from([(changed.type_parameters[0], fixture.cell)]);
        assert!(
            crate::platform::execution::normalized::prepared_types::materialize_associated_roots(
                &changed,
                &ordinary,
                &[fixture.projection],
                &mut proof.types.clone(),
                &control
            )
            .is_err()
        );
    }
    let cancelled = ExecutionControl::cancel_after_checks(0);
    assert_eq!(
        proof
            .compare_selection(
                &proof.selections[0],
                &catalog[0],
                &catalog,
                &proof.types,
                &cancelled
            )
            .unwrap_err()
            .class,
        ExecutionFailureClass::Cancelled
    );
}

#[test]
fn canonical_associated_preparation_rechecks_unused_members_and_exact_owner_occurrences() {
    let fixture = make_fixture();
    for change in 0..7 {
        let mut changed = make_fixture();
        match change {
            0 => {
                let unit = add(&mut changed.source, TypeForm::Unit);
                let DeclarationPayload::OwnedImplementation(scheme) =
                    payload(&mut changed.source, changed.implementations[0])
                else {
                    unreachable!()
                };
                scheme
                    .associated_types
                    .iter_mut()
                    .find(|binding| binding.member == changed.spare)
                    .unwrap()
                    .ty = unit;
            }
            1 => {
                let DeclarationPayload::OwnedImplementation(scheme) =
                    payload(&mut changed.source, changed.implementations[0])
                else {
                    unreachable!()
                };
                scheme
                    .associated_types
                    .iter_mut()
                    .find(|binding| binding.member == changed.spare)
                    .unwrap()
                    .ty = changed.projection;
            }
            2 => {
                let DeclarationPayload::Function(function) =
                    payload(&mut changed.source, changed.consumer)
                else {
                    unreachable!()
                };
                let parameter = function.parameters[0];
                changed
                    .source
                    .owners
                    .get_mut(&OwnerKey::Parameter(parameter))
                    .unwrap()
                    .set_encoding_for_edit(32);
            }
            3 => {
                let DeclarationPayload::Function(function) =
                    payload(&mut changed.source, changed.consumer)
                else {
                    unreachable!()
                };
                let body = function.body;
                changed
                    .source
                    .owners
                    .get_mut(&OwnerKey::Expression(body))
                    .unwrap()
                    .set_encoding_for_edit(32);
            }
            4 => {
                let owner = changed
                    .source
                    .owners
                    .get_mut(&OwnerKey::Declaration(changed.consumer.declaration))
                    .unwrap();
                owner.set_encoding_for_edit(32);
            }
            _ => {
                let DeclarationPayload::Function(function) =
                    payload(&mut changed.source, changed.consumer)
                else {
                    unreachable!()
                };
                let body = function.body;
                let original = function.parameters[0];
                let OwnerRecord::Parameter(mut foreign) =
                    changed.source.owners[&OwnerKey::Parameter(original)].clone()
                else {
                    unreachable!()
                };
                let parameter = ParameterId::migrate(SEED, 90);
                foreign.header =
                    OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter);
                foreign.ty = changed.cell;
                foreign.parent = ParameterParent::Function(if change == 5 {
                    changed.reader.declaration
                } else {
                    changed.consumer.declaration
                });
                changed.source.owners.insert(
                    OwnerKey::Parameter(parameter),
                    OwnerRecord::Parameter(foreign),
                );
                let OwnerRecord::Expression(expression) = changed
                    .source
                    .owners
                    .get_mut(&OwnerKey::Expression(body))
                    .unwrap()
                else {
                    unreachable!()
                };
                expression.operation = ExpressionOperation::Local {
                    value: LocalValueReference::FunctionParameter(parameter),
                };
            }
        }
        assert_eq!(
            prove(
                &changed,
                &[0],
                Limits::default(),
                &ExecutionControl::uncancelled()
            )
            .unwrap_err()
            .class,
            ExecutionFailureClass::Infrastructure,
            "corruption {change}"
        );
    }
    assert!(
        prove(
            &fixture,
            &[0],
            Limits::default(),
            &ExecutionControl::uncancelled()
        )
        .is_ok()
    );
}

#[test]
fn canonical_associated_preparation_has_exact_capacity_and_cancelled_retry_boundaries() {
    let fixture = make_fixture();
    let healthy = prove(
        &fixture,
        &[0, 1, 2],
        Limits::default(),
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    for limits in [
        Limits {
            steps: healthy.steps - 1,
            bytes: healthy.bytes,
        },
        Limits {
            steps: healthy.steps,
            bytes: healthy.bytes - 1,
        },
    ] {
        assert_eq!(
            prove(
                &fixture,
                &[0, 1, 2],
                limits,
                &ExecutionControl::uncancelled()
            )
            .unwrap_err()
            .class,
            ExecutionFailureClass::Resource
        );
    }
    let exact = prove(
        &fixture,
        &[0, 1, 2],
        Limits {
            steps: healthy.steps,
            bytes: healthy.bytes,
        },
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert_eq!((exact.steps, exact.bytes), (healthy.steps, healthy.bytes));
    for checks in [0, 10, healthy.steps as u64 / 2] {
        assert_eq!(
            prove(
                &fixture,
                &[0, 1, 2],
                Limits::default(),
                &ExecutionControl::cancel_after_checks(checks)
            )
            .unwrap_err()
            .class,
            ExecutionFailureClass::Cancelled
        );
    }
    let retry = prove(
        &fixture,
        &[0, 1, 2],
        Limits::default(),
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert_eq!((retry.steps, retry.bytes), (healthy.steps, healthy.bytes));
    assert!(crate::platform::execution::normalized::reference_schema::NormalizedReferenceSchema::independently_admit_witness_source(&fixture.source).is_err());
}

#[test]
fn canonical_associated_preparation_closes_source_effects_before_opaque_caller_types() {
    use crate::platform::kernel::{EffectParameterRecord, RequirementReference};
    use crate::platform::semantic_id::{EffectParameterId, RequirementId};
    let mut fixture = make_fixture();
    let ordinary = TypeParameterId::migrate(SEED, 99);
    let effect = EffectParameterId::migrate(SEED, 99);
    let effect_reference = EffectParameterReference {
        package: fixture.consumer.package,
        parameter: effect,
    };
    fixture.source.owners.insert(
        OwnerKey::TypeParameter(ordinary),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(ordinary), OwnerKind::TypeParameter),
            declaration: fixture.consumer.declaration,
            name: name("Opaque"),
            constraints: TypeParameterConstraints::Transferable,
        }),
    );
    fixture.source.owners.insert(
        OwnerKey::EffectParameter(effect),
        OwnerRecord::EffectParameter(EffectParameterRecord {
            header: OwnerHeader::new(
                OwnerKey::EffectParameter(effect),
                OwnerKind::EffectParameter,
            ),
            declaration: fixture.consumer.declaration,
            name: name("E"),
        }),
    );
    let parameter = add(
        &mut fixture.source,
        TypeForm::TypeParameter {
            parameter: ordinary,
        },
    );
    let unit = add(&mut fixture.source, TypeForm::Unit);
    let requirement = RequirementReference {
        package: fixture.consumer.package,
        requirement: RequirementId::migrate(SEED, 99),
    };
    let caller_row = EffectRow {
        requirements: vec![RequirementOperand::Concrete(requirement)],
        parameters: vec![],
    };
    let caller = add(
        &mut fixture.source,
        TypeForm::TaskFunction {
            parameters: vec![],
            result: unit,
            effect: caller_row.clone(),
        },
    );
    let result = add(
        &mut fixture.source,
        TypeForm::TaskFunction {
            parameters: vec![fixture.projection, parameter],
            result: unit,
            effect: EffectRow {
                requirements: vec![],
                parameters: vec![effect_reference],
            },
        },
    );
    let DeclarationPayload::Function(function) = payload(&mut fixture.source, fixture.consumer)
    else {
        unreachable!()
    };
    function.type_parameters.push(ordinary);
    function.effect_parameters.push(effect);
    function.result = result;
    function.result_borrow = None;
    let operands = [operand(&fixture, 0)];
    let types = [fixture.cell, caller];
    let effects = [EffectRow::default()];
    let proof = derive(
        &[&fixture.source],
        &[Entry {
            function: fixture.consumer,
            types: &types,
            implementations: &operands,
            effects: &effects,
            requirements: &[],
        }],
        Limits::default(),
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    let expected = TypeObject::new(TypeForm::TaskFunction {
        parameters: vec![fixture.cell, caller],
        result: unit,
        effect: EffectRow::default(),
    })
    .unwrap();
    let expected = encode_type_object(&expected).unwrap().0;
    let actual = proof
        .applications
        .iter()
        .find(|(application, _)| application.function == fixture.consumer)
        .unwrap()
        .1;
    assert_eq!(actual.result, expected);
    assert!(
        matches!(&proof.types[&caller].form, TypeForm::TaskFunction { effect, .. } if effect == &caller_row)
    );
    assert_eq!(proof.types[&caller], fixture.source.types[&caller]);
}

#[test]
fn canonical_associated_preparation_closes_untaken_containers_and_both_call_lanes() {
    use crate::platform::kernel::ExpressionRecord;
    let mut fixture = make_fixture();
    let sequence = add(
        &mut fixture.source,
        TypeForm::OwnedSequence {
            item: fixture.projection,
        },
    );
    let product = add(
        &mut fixture.source,
        TypeForm::OwnedProduct {
            fields: vec![StructuralTypeField {
                name: name("payload"),
                ty: fixture.projection,
            }],
        },
    );
    let choice = add(
        &mut fixture.source,
        TypeForm::OwnedChoice {
            cases: vec![StructuralTypeField {
                name: name("payload"),
                ty: fixture.projection,
            }],
        },
    );
    let mut expression = |ordinal, operation| {
        let id = ExpressionId::migrate(SEED, ordinal);
        fixture.source.owners.insert(
            OwnerKey::Expression(id),
            OwnerRecord::Expression(ExpressionRecord::new(id, operation).unwrap()),
        );
        id
    };
    let false_value = expression(100, ExpressionOperation::Bool { value: false });
    let empty = expression(
        101,
        ExpressionOperation::SequenceEmpty {
            sequence_type: sequence,
        },
    );
    let inactive = expression(
        102,
        ExpressionOperation::If {
            condition: false_value,
            when_true: empty,
            when_false: empty,
        },
    );
    let left = expression(
        103,
        ExpressionOperation::Call {
            function: fixture.reader,
            type_arguments: vec![product],
            effect_arguments: vec![],
            requirement_arguments: vec![],
            arguments: vec![],
        },
    );
    let right = expression(
        104,
        ExpressionOperation::Call {
            function: fixture.reader,
            type_arguments: vec![choice],
            effect_arguments: vec![],
            requirement_arguments: vec![],
            arguments: vec![],
        },
    );
    let lanes = expression(105, ExpressionOperation::Parallel { left, right });
    let callable = expression(
        106,
        ExpressionOperation::FunctionValue {
            function: fixture.reader,
            type_arguments: vec![sequence],
            effect_arguments: vec![],
            requirement_arguments: vec![],
        },
    );
    let body = expression(
        107,
        ExpressionOperation::Sequence {
            items: vec![inactive, lanes, callable],
        },
    );
    let DeclarationPayload::Function(function) = payload(&mut fixture.source, fixture.consumer)
    else {
        unreachable!()
    };
    function.body = body;
    // This deliberately exercises canonical preparation roots, not execution or
    // task/custody admission of the enclosing synthetic body.
    let proof = prove(
        &fixture,
        &[0, 1],
        Limits::default(),
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    for (application, function) in proof
        .applications
        .iter()
        .filter(|(application, _)| application.function == fixture.consumer)
    {
        let selection = &proof.selections[application.witnesses[0]];
        let output = if selection.implementation == fixture.implementations[1] {
            fixture.block
        } else {
            fixture.cell
        };
        let expected_sequence =
            encode_type_object(&TypeObject::new(TypeForm::OwnedSequence { item: output }).unwrap())
                .unwrap()
                .0;
        let expected_product = encode_type_object(
            &TypeObject::new(TypeForm::OwnedProduct {
                fields: vec![StructuralTypeField {
                    name: name("payload"),
                    ty: output,
                }],
            })
            .unwrap(),
        )
        .unwrap()
        .0;
        let expected_choice = encode_type_object(
            &TypeObject::new(TypeForm::OwnedChoice {
                cases: vec![StructuralTypeField {
                    name: name("payload"),
                    ty: output,
                }],
            })
            .unwrap(),
        )
        .unwrap()
        .0;
        assert_eq!(
            function.roots[&(OwnerKey::Expression(empty), sequence)],
            expected_sequence
        );
        assert_eq!(function.targets[&left].types, vec![expected_product]);
        assert_eq!(function.targets[&right].types, vec![expected_choice]);
        assert_eq!(function.targets[&callable].types, vec![expected_sequence]);
        let index = if output == fixture.block { 1 } else { 0 };
        let actual_function = normalized(&fixture, selected(&fixture, index));
        let ordinary = BTreeMap::from([(actual_function.type_parameters[0], fixture.cell)]);
        let actual =
            crate::platform::execution::normalized::prepared_types::materialize_associated_roots(
                &actual_function,
                &ordinary,
                &[sequence, product, choice],
                &mut fixture.source.types.clone(),
                &ExecutionControl::uncancelled(),
            )
            .unwrap();
        assert_eq!(
            actual,
            vec![expected_sequence, expected_product, expected_choice]
        );
    }
}

#[test]
fn canonical_associated_schema_private_core_moves_types_and_excludes_raw_ancestors() {
    let mut fixture = make_fixture();
    let unit = add(&mut fixture.source, TypeForm::Unit);
    let raw_function = add(
        &mut fixture.source,
        TypeForm::Function {
            parameters: vec![fixture.projection],
            result: unit,
        },
    );
    let raw_task = add(
        &mut fixture.source,
        TypeForm::TaskFunction {
            parameters: vec![],
            result: fixture.projection,
            effect: EffectRow::default(),
        },
    );
    let raw_list = add(&mut fixture.source, TypeForm::List { item: raw_function });
    let OwnerRecord::Declaration(declaration) =
        &fixture.source.owners[&OwnerKey::Declaration(fixture.consumer.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &declaration.payload else {
        unreachable!()
    };
    let ordinary_parameter = function.type_parameters[0];
    let ordinary_formal = add(
        &mut fixture.source,
        TypeForm::TypeParameter {
            parameter: ordinary_parameter,
        },
    );
    let ordinary_callable = add(
        &mut fixture.source,
        TypeForm::Function {
            parameters: vec![ordinary_formal],
            result: unit,
        },
    );
    let control = ExecutionControl::uncancelled();
    let proof = prove(&fixture, &[0, 1, 2], Limits::default(), &control).unwrap();
    let schema =
        super::super::associated_schema_fixture(&[&fixture.source], proof, &control).unwrap();
    assert!(schema.associated_proof().unwrap().types.is_empty());
    assert!(schema.types.contains_key(&fixture.block));
    for raw in [fixture.projection, raw_function, raw_task, raw_list] {
        for property in [
            &schema.buffer_free_types,
            &schema.capture_safe_types,
            &schema.transferable_types,
            &schema.shareable_types,
            &schema.ordinary_types,
            &schema.comparable_types,
            &schema.application_free_types,
        ] {
            assert!(!property.contains(&raw));
        }
    }
    assert!(schema.capture_safe_types.contains(&ordinary_callable));
    assert!(schema.ordinary_types.contains(&ordinary_callable));
    assert!(schema.application_free_types.contains(&ordinary_callable));
    assert!(schema.transferable_types.contains(&fixture.block));
    assert!(schema.shareable_types.contains(&fixture.block));
    assert_eq!(super::super::super::reference_schema::NormalizedReferenceSchema::independently_admit_witness_source(&fixture.source).unwrap_err().code, "owned_associated_semantics_unsupported");
    for checks in [0, 20] {
        let proof = prove(&fixture, &[0, 1, 2], Limits::default(), &control).unwrap();
        assert_eq!(
            super::super::associated_schema_fixture(
                &[&fixture.source],
                proof,
                &ExecutionControl::cancel_after_checks(checks)
            )
            .unwrap_err()
            .class,
            ExecutionFailureClass::Cancelled
        );
    }
    let proof = prove(&fixture, &[0, 1, 2], Limits::default(), &control).unwrap();
    let retry =
        super::super::associated_schema_fixture(&[&fixture.source], proof, &control).unwrap();
    assert_eq!(
        (retry.type_derivation_steps, retry.type_metadata_bytes),
        (schema.type_derivation_steps, schema.type_metadata_bytes)
    );
}

#[test]
fn canonical_associated_schema_context_seals_owner_selection_and_source_templates() {
    let fixture = make_fixture();
    let control = ExecutionControl::uncancelled();
    let proof = prove(&fixture, &[0, 1, 2], Limits::default(), &control).unwrap();
    let selected = proof
        .selections
        .iter()
        .find(|selected| selected.implementation == fixture.implementations[1])
        .unwrap();
    assert!(Arc::ptr_eq(
        proof
            .selection(selected.implementation, &selected.actuals, &[], &control)
            .unwrap(),
        selected
    ));
    let application = proof
        .applications
        .keys()
        .find(|application| {
            application.function == fixture.consumer && application.witnesses == [selected.identity]
        })
        .unwrap();
    let declaration = OwnerKey::Declaration(fixture.consumer.declaration);
    assert_eq!(
        proof
            .closed_root(
                application,
                (fixture.consumer.package, declaration),
                fixture.projection,
                &control
            )
            .unwrap(),
        fixture.block
    );
    assert!(
        proof
            .closed_root(
                application,
                (
                    fixture.consumer.package,
                    OwnerKey::Declaration(fixture.reader.declaration)
                ),
                fixture.projection,
                &control
            )
            .is_err()
    );
    let contract = proof.contract_template(fixture.contract, &control).unwrap();
    let method = contract.methods[0].id;
    let mapping = proof.mapping_template(selected, method, &control).unwrap();
    assert_eq!(mapping.type_arguments, [fixture.block]);
    let target = proof.method_target(selected, method, &control).unwrap();
    assert_eq!(
        proof.application(target, &control).unwrap().result,
        fixture.block
    );
    let foreign = prove(&fixture, &[1], Limits::default(), &control).unwrap();
    assert!(
        proof
            .method_target(&foreign.selections[0], method, &control)
            .is_err()
    );
    assert!(
        proof
            .closed_root(
                application,
                (fixture.consumer.package, declaration),
                fixture.projection,
                &ExecutionControl::cancel_after_checks(0)
            )
            .is_err()
    );
    let source_function = &proof.symbolic[&fixture.consumer];
    assert_eq!(source_function.result, fixture.projection);
    assert!(source_function.occurrences.contains_key(&declaration));
    assert!(proof.flow_work.contexts > 0);
}

#[test]
fn canonical_associated_schema_checks_unselected_owned_bindings_and_growth_before_completion() {
    let mut fixture = make_fixture();
    let unit = add(&mut fixture.source, TypeForm::Unit);
    let DeclarationPayload::OwnedImplementation(scheme) =
        payload(&mut fixture.source, fixture.implementations[2])
    else {
        unreachable!()
    };
    scheme
        .associated_types
        .iter_mut()
        .find(|binding| binding.member == fixture.spare)
        .unwrap()
        .ty = unit;
    let error = derive(
        &[&fixture.source],
        &[],
        Limits::default(),
        &ExecutionControl::uncancelled(),
    )
    .unwrap_err();
    assert_eq!(error.code, "reference_associated_preparation");
    assert_eq!(super::super::super::reference_schema::NormalizedReferenceSchema::independently_admit_witness_source(&fixture.source).unwrap_err().code, error.code);

    let mut fixture = make_fixture();
    let unit = add(&mut fixture.source, TypeForm::Unit);
    let grow = DeclarationReference {
        package: fixture.reader.package,
        declaration: DeclarationId::migrate(SEED, 10_000),
    };
    let formal = TypeParameterId::migrate(SEED, 10_000);
    let parameter = ParameterId::migrate(SEED, 10_000);
    let local = ExpressionId::migrate(SEED, 10_000);
    let body = ExpressionId::migrate(SEED, 10_001);
    let formal_type = add(
        &mut fixture.source,
        TypeForm::TypeParameter { parameter: formal },
    );
    let expanded = add(
        &mut fixture.source,
        TypeForm::OwnedSequence { item: formal_type },
    );
    let OwnerRecord::Declaration(mut declaration) =
        fixture.source.owners[&OwnerKey::Declaration(fixture.reader.declaration)].clone()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut declaration.payload else {
        unreachable!()
    };
    let original_parameter = function.parameters[0];
    function.type_parameters = vec![formal];
    function.parameters = vec![parameter];
    function.result = unit;
    function.result_borrow = None;
    function.body = body;
    declaration.header = OwnerHeader::new(
        OwnerKey::Declaration(grow.declaration),
        OwnerKind::PureFunction,
    );
    declaration.name = name("finiteGrowthControl");
    fixture.source.owners.insert(
        OwnerKey::Declaration(grow.declaration),
        OwnerRecord::Declaration(declaration),
    );
    fixture.source.owners.insert(
        OwnerKey::TypeParameter(formal),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(formal), OwnerKind::TypeParameter),
            declaration: grow.declaration,
            name: name("T"),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    let OwnerRecord::Parameter(mut parameter_record) =
        fixture.source.owners[&OwnerKey::Parameter(original_parameter)].clone()
    else {
        unreachable!()
    };
    parameter_record.header =
        OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter);
    parameter_record.parent = ParameterParent::Function(grow.declaration);
    parameter_record.ty = unit;
    parameter_record.use_mode = ParameterUse::Unrestricted;
    fixture.source.owners.insert(
        OwnerKey::Parameter(parameter),
        OwnerRecord::Parameter(parameter_record),
    );
    fixture.source.owners.insert(
        OwnerKey::Expression(local),
        OwnerRecord::Expression(
            crate::platform::kernel::ExpressionRecord::new(
                local,
                ExpressionOperation::Local {
                    value: LocalValueReference::FunctionParameter(parameter),
                },
            )
            .unwrap(),
        ),
    );
    fixture.source.owners.insert(
        OwnerKey::Expression(body),
        OwnerRecord::Expression(
            crate::platform::kernel::ExpressionRecord::new(
                body,
                ExpressionOperation::Call {
                    function: grow,
                    type_arguments: vec![expanded],
                    effect_arguments: vec![],
                    requirement_arguments: vec![],
                    arguments: vec![local],
                },
            )
            .unwrap(),
        ),
    );
    let error = derive(
        &[&fixture.source],
        &[],
        Limits::default(),
        &ExecutionControl::uncancelled(),
    )
    .unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Trap);
    assert_eq!(error.code, "kernel_associated_reference_growth");
    let error = super::super::super::reference_schema::NormalizedReferenceSchema::independently_admit_witness_source(&fixture.source).unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Trap);
    assert_eq!(error.code, "kernel_associated_reference_growth");
}
