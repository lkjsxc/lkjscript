//! Private closure models behind the global Unsupported gate. These are not
//! admitted associated artifacts or evidence of public associated execution.
use super::*;
use crate::platform::execution::{
    ExecutionControl, normalized::prepare::PreparedApplicationContext,
};
use crate::platform::kernel::{
    AssociatedTypeBinding, AssociatedTypeBound, AssociatedTypeMember, DeclarationPayload,
    KernelSnapshot, Name,
};

fn named(source: &KernelSnapshot, name: &str) -> DeclarationReference {
    source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record))
                if record.name.as_str() == name =>
            {
                Some(DeclarationReference {
                    package: source.root.package_id,
                    declaration: *declaration,
                })
            }
            _ => None,
        })
        .unwrap()
}

fn intern(program: &mut NormalizedProgram, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let (digest, _) = crate::platform::kernel::encode_type_object(&object).unwrap();
    program.types.insert(digest, object);
    digest
}

struct Fixture {
    source: KernelSnapshot,
    program: NormalizedProgram,
    member: AssociatedTypeMemberId,
    unused: AssociatedTypeMemberId,
    plain: DeclarationReference,
    plus: DeclarationReference,
    both: DeclarationReference,
    contract: DeclarationReference,
    scope: DeclarationReference,
    parameter: ImplementationParameterId,
    cell: TypeObjectDigest,
    buffer: TypeObjectDigest,
}

fn fixture() -> Fixture {
    let (mut source, mut program) = crate::platform::execution::normalized::tests::owned_implementation_scheme_tests::duplicate_dag_fixture(1);
    let plain = named(&source, "Plain");
    let plus = named(&source, "CellPlus");
    let both = named(&source, "Both");
    let contract = named(&source, "Keep");
    let scope = named(&source, "dag-0");
    let parameter = program
        .functions
        .iter()
        .find(|f| f.declaration == scope)
        .unwrap()
        .implementation_parameters[0]
        .id;
    let cell = intern(&mut program, TypeForm::OwnedI64Cell);
    let buffer = intern(&mut program, TypeForm::ByteBuffer);
    let member = AssociatedTypeMemberId::migrate(b"prepared-associated-model", 0);
    let unused = AssociatedTypeMemberId::migrate(b"prepared-associated-model", 1);
    // Deliberately author a contract order independent of member-ID ordering.
    let OwnerRecord::Declaration(owner) = source
        .owners
        .get_mut(&OwnerKey::Declaration(contract.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedContract(value) = &mut owner.payload else {
        unreachable!()
    };
    value.associated_types = vec![
        AssociatedTypeMember {
            id: unused,
            name: Name::new("Unused").unwrap(),
            bound: AssociatedTypeBound::Owned,
        },
        AssociatedTypeMember {
            id: member,
            name: Name::new("Block").unwrap(),
            bound: AssociatedTypeBound::Owned,
        },
    ];
    for (reference, output) in [(plain, cell), (plus, buffer), (both, cell)] {
        let OwnerRecord::Declaration(owner) = source
            .owners
            .get_mut(&OwnerKey::Declaration(reference.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::OwnedImplementation(value) = &mut owner.payload else {
            unreachable!()
        };
        value.associated_types = vec![
            AssociatedTypeBinding { member, ty: output },
            AssociatedTypeBinding {
                member: unused,
                ty: output,
            },
        ];
        if reference == plain || reference == both {
            value.associated_types[0].ty = value.self_type;
        }
        value.associated_types.sort_by_key(|binding| binding.member);
    }
    Fixture {
        source,
        program,
        member,
        unused,
        plain,
        plus,
        both,
        contract,
        scope,
        parameter,
        cell,
        buffer,
    }
}

struct Probe {
    result: Result<Vec<NormalizedImplementationArgument>, Diagnostic>,
    catalog: Vec<NormalizedImplementationArgument>,
    steps: usize,
    bytes: usize,
    applied: usize,
    normalized: usize,
}

fn probe(
    fixture: &mut Fixture,
    operands: &[ImplementationOperand],
    bytes: usize,
    steps: usize,
    control: &ExecutionControl,
) -> Probe {
    let implementations = fixture
        .source
        .owners
        .iter()
        .filter_map(|(key, record)| match (key, record) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record)) => {
                match &record.payload {
                    DeclarationPayload::OwnedImplementation(value) => Some((
                        DeclarationReference {
                            package: fixture.source.root.package_id,
                            declaration: *declaration,
                        },
                        value,
                    )),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect();
    let contracts = fixture
        .source
        .owners
        .iter()
        .filter_map(|(key, record)| match (key, record) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record)) => {
                match &record.payload {
                    DeclarationPayload::OwnedContract(value) => Some((
                        DeclarationReference {
                            package: fixture.source.root.package_id,
                            declaration: *declaration,
                        },
                        value,
                    )),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect();
    let runtime_owners = fixture
        .source
        .owners
        .iter()
        .map(|(key, value)| ((fixture.source.root.package_id, *key), value.clone()))
        .collect();
    let mut work = Budget {
        steps,
        bytes,
        control,
        type_lookup: type_lookup::TypeLookupBuilder::default(),
    };
    let mut closing = Closing {
        templates: Arc::clone(&fixture.program.functions),
        functions: Vec::new(),
        implementations,
        contracts,
        runtime_owners: &runtime_owners,
        owned_records: fixture.program.owned_record_declarations.clone(),
        targets: BTreeMap::new(),
        instances: BTreeMap::new(),
        normalized: BTreeMap::new(),
        witnesses: BTreeMap::new(),
        pending: Vec::new(),
        types: &mut fixture.program.types,
        work: &mut work,
    };
    let result = operands
        .iter()
        .map(|operand| {
            let selected = closing.operand(operand, None, &Bindings::new(), &BTreeMap::new())?;
            closing
                .normalized
                .get(&selected)
                .cloned()
                .ok_or_else(missing)
        })
        .collect();
    Probe {
        result,
        catalog: closing.normalized.values().cloned().collect(),
        steps: closing.work.steps,
        bytes: closing.work.bytes,
        applied: closing.witnesses.len(),
        normalized: closing.normalized.len(),
    }
}

fn plain(fixture: &Fixture) -> ImplementationOperand {
    ImplementationOperand::Concrete {
        implementation: fixture.plain,
        type_arguments: vec![fixture.cell],
        implementations: Vec::new(),
    }
}

fn plus(fixture: &Fixture) -> ImplementationOperand {
    ImplementationOperand::Concrete {
        implementation: fixture.plus,
        type_arguments: Vec::new(),
        implementations: Vec::new(),
    }
}

pub(in super::super::super) struct AssociatedModel {
    pub program: NormalizedProgram,
    pub left: FunctionIndex,
    pub right: FunctionIndex,
    pub bindings: BTreeMap<TypeParameterId, TypeObjectDigest>,
    pub projection: TypeObjectDigest,
    pub member: AssociatedTypeMemberId,
    pub cell: TypeObjectDigest,
    pub buffer: TypeObjectDigest,
}

/// Exposes only a private normalized model; canonical artifact admission remains
/// Unsupported and is intentionally not rerun or relabelled for this fixture.
pub(in super::super::super) fn prepared_associated_model() -> AssociatedModel {
    let mut fixture = fixture();
    let operands = [plain(&fixture), plus(&fixture)];
    let closed = probe(
        &mut fixture,
        &operands,
        0,
        0,
        &ExecutionControl::uncancelled(),
    );
    let selected = closed.result.unwrap();
    let projection = intern(
        &mut fixture.program,
        TypeForm::AssociatedWitnessProjection {
            scope: fixture.scope,
            parameter: fixture.parameter,
            contract: fixture.contract,
            member: fixture.member,
        },
    );
    let template = fixture
        .program
        .functions
        .iter()
        .find(|f| f.declaration == fixture.scope && !f.type_arguments.is_empty())
        .unwrap()
        .clone();
    let bindings = template
        .type_parameters
        .iter()
        .copied()
        .zip(template.type_arguments.iter().copied())
        .collect();
    let mut left = template.clone();
    let mut right = template;
    left.implementation_arguments = Arc::from([selected[0].clone()]);
    right.implementation_arguments = Arc::from([selected[1].clone()]);
    fixture.program.functions = Arc::from([left, right]);
    fixture.program.implementation_applications = closed.catalog.into();
    AssociatedModel {
        left: FunctionIndex(0, fixture.program.value_origin),
        right: FunctionIndex(1, fixture.program.value_origin),
        program: fixture.program,
        bindings,
        projection,
        member: fixture.member,
        cell: fixture.cell,
        buffer: fixture.buffer,
    }
}

#[test]
fn associated_closed_members_preserve_exact_witness_identity_and_contract_order() {
    let mut fixture = fixture();
    let operands = [plain(&fixture), plus(&fixture), plain(&fixture)];
    let measured = probe(
        &mut fixture,
        &operands,
        0,
        0,
        &ExecutionControl::uncancelled(),
    );
    let selected = measured.result.unwrap();
    assert_eq!((measured.applied, measured.normalized), (2, 2));
    assert!(Arc::ptr_eq(&selected[0], &selected[2]));
    assert_ne!(selected[0].identity, selected[1].identity);
    assert_eq!(selected[0].self_type, selected[1].self_type);
    assert_eq!(
        selected[0]
            .associated_types
            .iter()
            .map(|binding| binding.member)
            .collect::<Vec<_>>(),
        vec![fixture.unused, fixture.member]
    );
    assert_eq!(selected[0].associated_types[1].ty, fixture.cell);
    assert_eq!(selected[1].associated_types[1].ty, fixture.buffer);
    // Equal output types do not merge different selected schemes.
    let OwnerRecord::Declaration(owner) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.plus.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(value) = &mut owner.payload else {
        unreachable!()
    };
    for binding in &mut value.associated_types {
        binding.ty = fixture.cell;
    }
    let probe = probe(
        &mut fixture,
        &operands[..2],
        0,
        0,
        &ExecutionControl::uncancelled(),
    );
    let selected = probe.result.unwrap();
    assert_ne!(selected[0].identity, selected[1].identity);
    assert_eq!(
        selected[0].associated_types[1].ty,
        selected[1].associated_types[1].ty
    );
}

#[test]
fn associated_closed_members_admit_unused_members_and_reject_hidden_projection_actuals() {
    for fault in 0..5 {
        let mut fixture = fixture();
        let operand = plain(&fixture);
        let bad = match fault {
            0 => intern(&mut fixture.program, TypeForm::I64),
            1 => {
                let projected = intern(
                    &mut fixture.program,
                    TypeForm::AssociatedWitnessProjection {
                        scope: fixture.scope,
                        parameter: fixture.parameter,
                        contract: fixture.contract,
                        member: fixture.member,
                    },
                );
                intern(
                    &mut fixture.program,
                    TypeForm::Applied {
                        declaration: fixture.plain,
                        arguments: vec![projected],
                    },
                )
            }
            _ => fixture.cell,
        };
        let OwnerRecord::Declaration(owner) = fixture
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(fixture.plain.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        if fault == 4 {
            owner.header.contract_version -= 1;
        }
        let DeclarationPayload::OwnedImplementation(value) = &mut owner.payload else {
            unreachable!()
        };
        match fault {
            0 | 1 => {
                value
                    .associated_types
                    .iter_mut()
                    .find(|binding| binding.member == fixture.unused)
                    .unwrap()
                    .ty = bad
            }
            2 => {
                value.associated_types.pop();
            }
            3 => {
                value.associated_types[1].member = value.associated_types[0].member;
            }
            _ => {}
        }
        let probe = probe(
            &mut fixture,
            &[operand],
            0,
            0,
            &ExecutionControl::uncancelled(),
        );
        assert!(probe.result.is_err(), "fault {fault}");
        assert_eq!((probe.applied, probe.normalized), (0, 0), "fault {fault}");
    }
}

#[test]
fn associated_closed_members_refuse_a_bad_unused_prerequisite_before_parent_publication() {
    let mut fixture = fixture();
    let i64_type = intern(&mut fixture.program, TypeForm::I64);
    let OwnerRecord::Declaration(owner) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.plus.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(value) = &mut owner.payload else {
        unreachable!()
    };
    value
        .associated_types
        .iter_mut()
        .find(|binding| binding.member == fixture.unused)
        .unwrap()
        .ty = i64_type;
    let operand = ImplementationOperand::Concrete {
        implementation: fixture.both,
        type_arguments: vec![fixture.cell],
        implementations: vec![plain(&fixture), plus(&fixture)],
    };
    let probe = probe(
        &mut fixture,
        &[operand],
        0,
        0,
        &ExecutionControl::uncancelled(),
    );
    assert!(probe.result.is_err());
    assert!(
        probe
            .catalog
            .iter()
            .all(|selected| selected.implementation != fixture.both)
    );
    assert_eq!(
        (probe.applied, probe.normalized),
        (1, 1),
        "only the independently complete first child is retained privately"
    );
}

#[test]
fn associated_closed_selection_preserves_scope_generation_and_opaque_actuals() {
    let model = prepared_associated_model();
    let control = ExecutionControl::uncancelled();
    let left =
        PreparedApplicationContext::new(&model.program, model.left, &model.bindings, &control)
            .unwrap();
    let right =
        PreparedApplicationContext::new(&model.program, model.right, &model.bindings, &control)
            .unwrap();
    let form = &model.program.types[&model.projection].form;
    assert_eq!(
        left.environment().resolve_leaf(form).unwrap(),
        Some(model.cell)
    );
    assert_eq!(
        right.environment().resolve_leaf(form).unwrap(),
        Some(model.buffer)
    );
    assert!(
        ClosedTypeEnvironment::forbidden()
            .resolve_leaf(form)
            .is_err()
    );
    let mut wrong = form.clone();
    let TypeForm::AssociatedWitnessProjection { scope, .. } = &mut wrong else {
        unreachable!()
    };
    *scope = model.program.functions[0].implementation_arguments[0].implementation;
    assert!(left.environment().resolve_leaf(&wrong).is_err());
    let selected = &model.program.functions[0].implementation_arguments[0];
    let contract_form = TypeForm::AssociatedContractMember {
        contract: selected.contract,
        member: model.member,
    };
    assert!(left.environment().resolve_leaf(&contract_form).is_err());
    assert_eq!(
        ClosedTypeEnvironment::for_contract(selected)
            .resolve_leaf(&contract_form)
            .unwrap(),
        Some(model.cell)
    );
    let mut types = model.program.types.clone();
    let parameter = *model.bindings.keys().next().unwrap();
    let object = TypeObject::new(TypeForm::TypeParameter { parameter }).unwrap();
    let (root, _) = crate::platform::kernel::encode_type_object(&object).unwrap();
    types.insert(root, object);
    let opaque = BTreeMap::from([(parameter, model.projection)]);
    let mut work = Budget::new(&control);
    assert_eq!(
        substitute_in(
            &mut types,
            root,
            &TypeApplication {
                bindings: &opaque,
                associated: Some(right.environment()),
                function: None
            },
            0,
            &mut work
        )
        .unwrap(),
        model.projection,
        "caller replacement is not captured by the source witness environment"
    );
    let mut older = model.program.functions[0].clone();
    older.source_generation -= 1;
    assert!(
        ClosedTypeEnvironment::for_function(&older)
            .unwrap()
            .resolve_leaf(form)
            .is_err()
    );
}

#[test]
fn associated_application_context_rejects_incomplete_keys_foreign_origins_and_uncatalogued_nodes() {
    let model = prepared_associated_model();
    let healthy = ExecutionControl::uncancelled();
    let mut ordinary = model.bindings.clone();
    ordinary.clear();
    assert!(
        PreparedApplicationContext::new(&model.program, model.left, &ordinary, &healthy).is_err()
    );
    let parameter = *model.bindings.keys().next().unwrap();
    let foreign = TypeParameterId::migrate(b"associated-context-foreign", 0);
    ordinary.insert(foreign, model.cell);
    assert!(
        PreparedApplicationContext::new(&model.program, model.left, &ordinary, &healthy).is_err()
    );
    ordinary.insert(parameter, model.cell);
    assert!(
        PreparedApplicationContext::new(&model.program, model.left, &ordinary, &healthy).is_err(),
        "a complete admitted formal inventory still rejects an extra foreign caller key"
    );
    let mut detached = model.program.clone();
    let function = &mut Arc::make_mut(&mut detached.functions)[0];
    function.implementation_arguments =
        Arc::from([Arc::new((*function.implementation_arguments[0]).clone())]);
    assert!(
        PreparedApplicationContext::new(&detached, model.left, &model.bindings, &healthy).is_err()
    );
    let foreign_program = fixture().program;
    assert!(
        PreparedApplicationContext::new(
            &model.program,
            FunctionIndex(model.left.0, foreign_program.value_origin),
            &model.bindings,
            &healthy
        )
        .is_err()
    );
    let cancelled = ExecutionControl::cancel_after_checks(1);
    let error = match PreparedApplicationContext::new(
        &model.program,
        model.left,
        &model.bindings,
        &cancelled,
    ) {
        Ok(_) => panic!("actual context traversal must observe cancellation"),
        Err(error) => error,
    };
    assert_eq!(error.class, DiagnosticClass::Cancelled);
    assert_eq!(error.code, "execution_cancelled");
    assert!(
        PreparedApplicationContext::new(&model.program, model.left, &model.bindings, &healthy)
            .is_ok()
    );
}

#[test]
fn associated_member_closure_reserves_before_publication_and_retries_after_cancellation() {
    let mut fixture = fixture();
    let operand = plain(&fixture);
    let healthy = ExecutionControl::uncancelled();
    let measured = probe(&mut fixture, std::slice::from_ref(&operand), 0, 0, &healthy);
    assert!(measured.result.is_ok());
    // Literal inventory for this one-formal, two-member leaf: three copied
    // type digests, two constraint rows, two closure-map nodes, one interned
    // witness, one ordinary binding, two member/index/name rows, and the retained
    // application. No substituted type or prerequisite obligation is allocated.
    // Headers: nine application slices = 18 words, outer application Arc = 2,
    // witness/prerequisite Arcs = 4, and ten map/set nodes = 30.
    let literal_bytes = 3 * std::mem::size_of::<TypeObjectDigest>()
        + 2 * std::mem::size_of::<TypeParameterConstraints>()
        + 2 * std::mem::size_of::<(TypeObjectDigest, bool)>()
        + std::mem::size_of::<(WitnessIdentity, AppliedWitness)>()
        + std::mem::size_of::<AppliedWitnessNode>()
        + std::mem::size_of::<(TypeParameterId, TypeObjectDigest)>()
        + 2 * std::mem::size_of::<NormalizedAssociatedTypeBinding>()
        + 2 * std::mem::size_of::<(AssociatedTypeMemberId, &AssociatedTypeBinding)>()
        + 2 * std::mem::size_of::<AssociatedTypeMemberId>()
        + 2 * std::mem::size_of::<&Name>()
        + std::mem::size_of::<TypeParameterId>()
        + std::mem::size_of::<NormalizedImplementationApplication>()
        + std::mem::size_of::<(AppliedWitness, NormalizedImplementationArgument)>()
        + (18 + 2 + 4 + 30) * std::mem::size_of::<usize>();
    assert_eq!(
        measured.bytes, literal_bytes,
        "literal allocation inventory"
    );
    assert!(measured.steps > 0 && measured.bytes > 0);
    let exact = probe(
        &mut fixture,
        std::slice::from_ref(&operand),
        MAXIMUM_METADATA_BYTES - measured.bytes,
        MAXIMUM_WORK - measured.steps,
        &healthy,
    );
    assert!(exact.result.is_ok());
    assert_eq!(
        (exact.bytes, exact.steps),
        (MAXIMUM_METADATA_BYTES, MAXIMUM_WORK)
    );
    for (bytes, steps) in [
        (MAXIMUM_METADATA_BYTES - measured.bytes + 1, 0),
        (0, MAXIMUM_WORK - measured.steps + 1),
    ] {
        let short = probe(
            &mut fixture,
            std::slice::from_ref(&operand),
            bytes,
            steps,
            &healthy,
        );
        assert_eq!(short.result.unwrap_err().class, DiagnosticClass::Resource);
        assert_eq!((short.applied, short.normalized), (0, 0));
    }
    let cancelled = ExecutionControl::cancel_after_checks((measured.steps / 2) as u64);
    let stopped = probe(
        &mut fixture,
        std::slice::from_ref(&operand),
        0,
        0,
        &cancelled,
    );
    assert_eq!(
        stopped.result.unwrap_err().class,
        DiagnosticClass::Cancelled
    );
    assert_eq!((stopped.applied, stopped.normalized), (0, 0));
    assert!(
        probe(&mut fixture, &[operand], 0, 0, &healthy)
            .result
            .is_ok()
    );
}

#[test]
fn associated_borrowed_member_selection_retains_a_linear_shared_prerequisite_dag() {
    let mut fixture = fixture();
    let mut operand = plain(&fixture);
    for _ in 0..12 {
        operand = ImplementationOperand::Concrete {
            implementation: fixture.both,
            type_arguments: vec![fixture.cell],
            implementations: vec![operand.clone(), operand],
        };
    }
    // Written ingress is a finite tree here; normalized retained edges are shared.
    let probe = probe(
        &mut fixture,
        &[operand],
        0,
        0,
        &ExecutionControl::uncancelled(),
    );
    let selected = probe.result.unwrap();
    assert_eq!((probe.applied, probe.normalized), (13, 13));
    let mut node = &selected[0];
    for _ in 0..12 {
        assert!(Arc::ptr_eq(
            &node.implementations[0],
            &node.implementations[1]
        ));
        let env = ClosedTypeEnvironment::for_selected(node).unwrap();
        let form = TypeForm::AssociatedWitnessProjection {
            scope: node.implementation,
            parameter: node.implementation_parameter_ids[0],
            contract: fixture.contract,
            member: fixture.member,
        };
        assert_eq!(env.resolve_leaf(&form).unwrap(), Some(fixture.cell));
        assert_eq!(node.associated_types.len(), 2);
        node = &node.implementations[0];
    }
}
