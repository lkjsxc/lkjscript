//! Complete literal closures for exact roots, prerequisite paths and mutual flow.
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};

#[derive(Clone, Copy, Debug)]
enum Binding {
    Alias,
    Sequence,
    Constant,
}

struct Common {
    source: KernelSnapshot,
    contract: DeclarationReference,
    plain: DeclarationReference,
    member: AssociatedTypeMemberId,
    method: MethodId,
    unit: TypeObjectDigest,
    cell: TypeObjectDigest,
    buffer: TypeObjectDigest,
}

struct Family {
    common: Common,
    entry: CanonicalEntry,
    function: DeclarationReference,
    relay: Option<DeclarationReference>,
    wrapper: Option<DeclarationReference>,
    projection: TypeObjectDigest,
    expression: ExpressionId,
    witness_inventories: Vec<(DeclarationReference, Vec<ImplementationParameterId>)>,
}

fn common(binding: Binding) -> Common {
    let mut source = crate::platform::kernel::tests::transport_snapshot();
    source.root.package_id = PackageId::migrate(SEED, 100);
    source.owners.clear();
    source.types.clear();
    source.dependency_types.clear();
    source.dependencies.clear();
    source.representation_dependencies.clear();
    source.dependency_interfaces.clear();
    let module = ModuleId::migrate(SEED, 0);
    source.owners.insert(
        OwnerKey::Module(module),
        OwnerRecord::Module(ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
            name: name("exact-flow-families"),
        }),
    );
    let unit = ty(&mut source, TypeForm::Unit);
    let cell = ty(&mut source, TypeForm::OwnedI64Cell);
    let buffer = ty(&mut source, TypeForm::ByteBuffer);
    let contract = reference(&source, 100);
    let plain = reference(&source, 101);
    let noop = reference(&source, 102);
    let member = AssociatedTypeMemberId::migrate(SEED, 100);
    let method = MethodId::migrate(SEED, 100);
    let (contract_self, _) = formal(&mut source, contract, 1000);
    let (plain_self, plain_self_ty) = formal(&mut source, plain, 1010);
    let (plain_x, plain_x_ty) = formal(&mut source, plain, 1011);
    let (noop_self, _) = formal(&mut source, noop, 1020);
    declaration(
        &mut source,
        100,
        DeclarationPayload::OwnedContract(OwnedContract {
            self_parameter: contract_self,
            type_parameters: vec![],
            methods: vec![OwnedMethod {
                id: method,
                name: name("step"),
                parameters: vec![],
                result: crate::platform::kernel::CallableResult {
                    ty: unit,
                    relation: crate::platform::kernel::ResultRelation::Value,
                },
                effect: FunctionEffect::Pure,

                read_scope_parameters: Vec::new(),
                read_scope_sources: Vec::new(),
            }],
            associated_types: vec![AssociatedTypeMember {
                id: member,
                name: name("Out"),
                bound: AssociatedTypeBound::Owned,
            }],
        }),
    );
    let body = expression(&mut source, 1020, ExpressionOperation::Unit {});
    function(&mut source, 102, vec![noop_self], vec![], unit, body);
    let output = match binding {
        Binding::Alias => plain_x_ty,
        Binding::Sequence => ty(&mut source, TypeForm::OwnedSequence { item: plain_x_ty }),
        Binding::Constant => cell,
    };
    declaration(
        &mut source,
        101,
        DeclarationPayload::OwnedImplementation(OwnedImplementation {
            type_parameters: vec![plain_self, plain_x],
            implementation_parameters: vec![],
            contract,
            self_type: plain_self_ty,
            type_arguments: vec![],
            methods: vec![OwnedMethodImplementation {
                method,
                function: noop,
                type_arguments: vec![plain_self_ty],
                implementations: vec![],

                read_scope_arguments: Vec::new(),
            }],
            associated_types: vec![AssociatedTypeBinding {
                member,
                value: crate::platform::kernel::AssociatedTypeBindingValue::Owned { ty: output },
            }],
        }),
    );
    Common {
        source,
        contract,
        plain,
        member,
        method,
        unit,
        cell,
        buffer,
    }
}

fn function(
    source: &mut KernelSnapshot,
    ordinal: u64,
    parameters: Vec<TypeParameterId>,
    witnesses: Vec<ImplementationParameter>,
    result: TypeObjectDigest,
    body: ExpressionId,
) -> DeclarationReference {
    declaration(
        source,
        ordinal,
        DeclarationPayload::Function(FunctionDeclaration {
            type_parameters: parameters,
            implementation_parameters: witnesses,
            requirement_parameters: vec![],
            effect_parameters: vec![],
            parameters: vec![],
            result: crate::platform::kernel::CallableResult {
                ty: result,
                relation: crate::platform::kernel::ResultRelation::Value,
            },
            effect: FunctionEffect::Pure,
            body,

            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
        }),
    )
}

fn witness(
    ordinal: u64,
    contract: DeclarationReference,
    self_type: TypeObjectDigest,
) -> ImplementationParameter {
    ImplementationParameter {
        id: ImplementationParameterId::migrate(SEED, ordinal),
        name: name(&format!("w{ordinal}")),
        contract,
        self_type,
        type_arguments: vec![],
    }
}

fn projection(
    common: &mut Common,
    scope: DeclarationReference,
    parameter: ImplementationParameterId,
) -> TypeObjectDigest {
    ty(
        &mut common.source,
        TypeForm::AssociatedWitnessProjection {
            scope,
            parameter,
            contract: common.contract,
            member: common.member,
        },
    )
}

fn parameter(scope: DeclarationReference, id: ImplementationParameterId) -> ImplementationOperand {
    ImplementationOperand::Parameter {
        scope,
        parameter: id,
    }
}

fn call(
    source: &mut KernelSnapshot,
    ordinal: u64,
    function: DeclarationReference,
    types: Vec<TypeObjectDigest>,
    implementations: Vec<ImplementationOperand>,
) -> ExpressionId {
    expression(
        source,
        ordinal,
        ExpressionOperation::ImplementationCall {
            function,
            type_arguments: types,
            implementations,
            effect_arguments: vec![],
            requirement_arguments: vec![],
            arguments: vec![],

            read_scope_arguments: Vec::new(),
        },
    )
}

fn leaf(
    common: &Common,
    self_type: TypeObjectDigest,
    x: TypeObjectDigest,
) -> ImplementationOperand {
    scheme_operand(common.plain, vec![self_type, x])
}

fn roots(binding: Binding, identical: bool) -> Family {
    let mut common = common(binding);
    let f = reference(&common.source, 110);
    let (self_id, self_ty) = formal(&mut common.source, f, 1100);
    let left = witness(1100, common.contract, self_ty);
    let right = witness(1101, common.contract, self_ty);
    let left_out = projection(&mut common, f, left.id);
    let right_out = projection(&mut common, f, right.id);
    let operands = vec![
        leaf(&common, self_ty, right_out),
        leaf(&common, self_ty, left_out),
    ];
    let body = call(&mut common.source, 1100, f, vec![self_ty], operands);
    let ids = vec![left.id, right.id];
    function(
        &mut common.source,
        110,
        vec![self_id],
        vec![left, right],
        common.unit,
        body,
    );
    let entry = CanonicalEntry {
        function: f,
        arguments: vec![common.cell],
        implementations: vec![
            leaf(&common, common.cell, common.cell),
            leaf(
                &common,
                common.cell,
                if identical {
                    common.cell
                } else {
                    common.buffer
                },
            ),
        ],
    };
    Family {
        common,
        entry,
        function: f,
        relay: None,
        wrapper: None,
        projection: left_out,
        expression: body,
        witness_inventories: vec![(f, ids)],
    }
}

fn wrapped(
    wrapper: DeclarationReference,
    common: &Common,
    self_type: TypeObjectDigest,
    first: TypeObjectDigest,
    second: TypeObjectDigest,
) -> ImplementationOperand {
    ImplementationOperand::Concrete {
        implementation: wrapper,
        type_arguments: vec![self_type],
        implementations: vec![
            leaf(common, self_type, first),
            leaf(common, self_type, second),
        ],
    }
}

fn paths(binding: Binding, identical: bool) -> Family {
    let mut common = common(binding);
    let path_contract = reference(&common.source, 120);
    let wrapper = reference(&common.source, 121);
    let relay = reference(&common.source, 122);
    let f = reference(&common.source, 110);
    let (path_self, _) = formal(&mut common.source, path_contract, 1200);
    let (wrapper_self, wrapper_self_ty) = formal(&mut common.source, wrapper, 1210);
    let (relay_self, relay_self_ty) = formal(&mut common.source, relay, 1220);
    let (f_self, f_self_ty) = formal(&mut common.source, f, 1100);
    declaration(
        &mut common.source,
        120,
        DeclarationPayload::OwnedContract(OwnedContract {
            self_parameter: path_self,
            type_parameters: vec![],
            methods: vec![OwnedMethod {
                id: common.method,
                name: name("step"),
                parameters: vec![],
                result: crate::platform::kernel::CallableResult {
                    ty: common.unit,
                    relation: crate::platform::kernel::ResultRelation::Value,
                },
                effect: FunctionEffect::Pure,

                read_scope_parameters: Vec::new(),
                read_scope_sources: Vec::new(),
            }],
            associated_types: vec![],
        }),
    );
    let first = witness(1210, common.contract, wrapper_self_ty);
    let second = witness(1211, common.contract, wrapper_self_ty);
    let prerequisite_ids = vec![first.id, second.id];
    declaration(
        &mut common.source,
        121,
        DeclarationPayload::OwnedImplementation(OwnedImplementation {
            type_parameters: vec![wrapper_self],
            implementation_parameters: vec![first.clone(), second.clone()],
            contract: path_contract,
            self_type: wrapper_self_ty,
            type_arguments: vec![],
            methods: vec![OwnedMethodImplementation {
                method: common.method,
                function: relay,
                type_arguments: vec![wrapper_self_ty],
                implementations: vec![parameter(wrapper, first.id), parameter(wrapper, second.id)],

                read_scope_arguments: Vec::new(),
            }],
            associated_types: vec![],
        }),
    );
    let x = witness(1220, common.contract, relay_self_ty);
    let y = witness(1221, common.contract, relay_self_ty);
    let x_out = projection(&mut common, relay, x.id);
    let y_out = projection(&mut common, relay, y.id);
    // Only concrete Plain leaves occur beneath the new Wrapper operands. No
    // lexical witness is wrapped into a growing prerequisite tree.
    let operands = vec![
        wrapped(wrapper, &common, relay_self_ty, y_out, x_out),
        wrapped(wrapper, &common, relay_self_ty, x_out, y_out),
    ];
    let body = call(&mut common.source, 1220, f, vec![relay_self_ty], operands);
    let relay_ids = vec![x.id, y.id];
    function(
        &mut common.source,
        122,
        vec![relay_self],
        vec![x, y],
        common.unit,
        body,
    );
    let left = witness(1100, path_contract, f_self_ty);
    let right = witness(1101, path_contract, f_self_ty);
    let left_call = expression(
        &mut common.source,
        1101,
        ExpressionOperation::MethodCall {
            witness: parameter(f, left.id),
            contract: path_contract,
            method: common.method,
            arguments: vec![],

            read_scope_arguments: Vec::new(),
        },
    );
    let right_call = expression(
        &mut common.source,
        1102,
        ExpressionOperation::MethodCall {
            witness: parameter(f, right.id),
            contract: path_contract,
            method: common.method,
            arguments: vec![],

            read_scope_arguments: Vec::new(),
        },
    );
    let sequence = expression(
        &mut common.source,
        1100,
        ExpressionOperation::Sequence {
            items: vec![left_call, right_call],
        },
    );
    let root_ids = vec![left.id, right.id];
    function(
        &mut common.source,
        110,
        vec![f_self],
        vec![left, right],
        common.unit,
        sequence,
    );
    let other = if identical {
        common.cell
    } else {
        common.buffer
    };
    let entry = CanonicalEntry {
        function: f,
        arguments: vec![common.cell],
        implementations: vec![
            wrapped(wrapper, &common, common.cell, common.cell, other),
            wrapped(wrapper, &common, common.cell, other, common.cell),
        ],
    };
    Family {
        common,
        entry,
        function: f,
        relay: Some(relay),
        wrapper: Some(wrapper),
        projection: x_out,
        expression: body,
        witness_inventories: vec![
            (f, root_ids),
            (wrapper, prerequisite_ids),
            (relay, relay_ids),
        ],
    }
}

fn mutual(binding: Binding, outer_sequence: bool, untaken: bool) -> Family {
    let mut common = common(binding);
    let f = reference(&common.source, 110);
    let g = reference(&common.source, 111);
    let (f_self, f_self_ty) = formal(&mut common.source, f, 1100);
    let (g_self, g_self_ty) = formal(&mut common.source, g, 1110);
    let (g_u, g_u_ty) = formal(&mut common.source, g, 1111);
    let w = witness(1100, common.contract, f_self_ty);
    let v = witness(1110, common.contract, g_self_ty);
    let output = projection(&mut common, f, w.id);
    let actual = if outer_sequence {
        ty(&mut common.source, TypeForm::OwnedSequence { item: output })
    } else {
        output
    };
    let f_call = call(
        &mut common.source,
        1100,
        g,
        vec![f_self_ty, actual],
        vec![parameter(f, w.id)],
    );
    let g_operands = vec![leaf(&common, g_self_ty, g_u_ty)];
    let g_body = call(&mut common.source, 1110, f, vec![g_self_ty], g_operands);
    let f_body = if untaken {
        let condition = expression(
            &mut common.source,
            1101,
            ExpressionOperation::Bool { value: true },
        );
        let unit = expression(&mut common.source, 1102, ExpressionOperation::Unit {});
        expression(
            &mut common.source,
            1103,
            ExpressionOperation::If {
                condition,
                when_true: unit,
                when_false: f_call,
            },
        )
    } else {
        f_call
    };
    let ids = vec![(f, vec![w.id]), (g, vec![v.id])];
    function(
        &mut common.source,
        110,
        vec![f_self],
        vec![w],
        common.unit,
        f_body,
    );
    function(
        &mut common.source,
        111,
        vec![g_self, g_u],
        vec![v],
        common.unit,
        g_body,
    );
    let entry = CanonicalEntry {
        function: f,
        arguments: vec![common.cell],
        implementations: vec![leaf(&common, common.cell, common.buffer)],
    };
    Family {
        common,
        entry,
        function: f,
        relay: Some(g),
        wrapper: None,
        projection: output,
        expression: f_call,
        witness_inventories: ids,
    }
}

fn read(family: &Family) -> CanonicalProgram {
    CanonicalProgram::read(
        &[&family.common.source],
        std::slice::from_ref(&family.entry),
    )
    .unwrap()
}

fn ground(program: &CanonicalProgram, kind: &str) -> model::Term {
    let ids: Vec<_> = program
        .atoms
        .iter()
        .filter(|(_, atom)| atom.kind == kind && atom.fields.is_empty())
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(ids.len(), 1, "literal atom {kind}");
    model::Term::Ground(ids[0])
}

fn selected_leaf(program: &CanonicalProgram, family: &Family, x: model::Term) -> model::Witness {
    model::Witness::Selected {
        scheme: program.definition(family.common.plain).unwrap(),
        arguments: vec![ground(program, "owned_i64_cell"), x],
        prerequisites: vec![],
    }
}

fn selected_wrapper(
    program: &CanonicalProgram,
    family: &Family,
    first: model::Term,
    second: model::Term,
) -> model::Witness {
    model::Witness::Selected {
        scheme: program.definition(family.wrapper.unwrap()).unwrap(),
        arguments: vec![ground(program, "owned_i64_cell")],
        prerequisites: vec![
            selected_leaf(program, family, first),
            selected_leaf(program, family, second),
        ],
    }
}

fn state(
    program: &CanonicalProgram,
    function: DeclarationReference,
    arguments: Vec<model::Term>,
    implementations: Vec<model::Witness>,
) -> model::State {
    model::State {
        function: program.definition(function).unwrap(),
        arguments,
        implementations,
    }
}

fn root_states(
    program: &CanonicalProgram,
    family: &Family,
    binding: Binding,
    identical: bool,
) -> BTreeSet<model::State> {
    let cell = ground(program, "owned_i64_cell");
    let other = if identical {
        cell.clone()
    } else {
        ground(program, "byte_buffer")
    };
    let mut expected = BTreeSet::from([state(
        program,
        family.function,
        vec![cell.clone()],
        vec![
            selected_leaf(program, family, cell.clone()),
            selected_leaf(program, family, other.clone()),
        ],
    )]);
    let (left, right) = match binding {
        Binding::Alias => (other, cell.clone()),
        Binding::Constant => (cell.clone(), cell.clone()),
        Binding::Sequence => panic!("expanding closure is not a finite literal"),
    };
    expected.insert(state(
        program,
        family.function,
        vec![cell],
        vec![
            selected_leaf(program, family, left),
            selected_leaf(program, family, right),
        ],
    ));
    expected
}

fn path_states(
    program: &CanonicalProgram,
    family: &Family,
    binding: Binding,
    identical: bool,
) -> BTreeSet<model::State> {
    let cell = ground(program, "owned_i64_cell");
    let other = if identical {
        cell.clone()
    } else {
        ground(program, "byte_buffer")
    };
    let mut expected = BTreeSet::new();
    let orientations = match binding {
        Binding::Alias => vec![(cell.clone(), other.clone()), (other.clone(), cell.clone())],
        Binding::Constant => vec![
            (cell.clone(), other.clone()),
            (other.clone(), cell.clone()),
            (cell.clone(), cell.clone()),
        ],
        Binding::Sequence => panic!("expanding closure is not a finite literal"),
    };
    let mapping = *program
        .definitions
        .iter()
        .find_map(|(id, identity)| {
            (*identity
                == DefinitionIdentity::MethodMapping(family.wrapper.unwrap(), family.common.method))
            .then_some(id)
        })
        .unwrap();
    expected.insert(state(
        program,
        family.function,
        vec![cell.clone()],
        vec![
            selected_wrapper(program, family, cell.clone(), other.clone()),
            selected_wrapper(program, family, other.clone(), cell.clone()),
        ],
    ));
    let (first, second) = match binding {
        Binding::Alias => (other.clone(), cell.clone()),
        Binding::Constant => (cell.clone(), cell.clone()),
        Binding::Sequence => unreachable!(),
    };
    expected.insert(state(
        program,
        family.function,
        vec![cell.clone()],
        vec![
            selected_wrapper(program, family, first.clone(), second.clone()),
            selected_wrapper(program, family, second, first),
        ],
    ));
    for (first, second) in orientations {
        let implementations = vec![
            selected_leaf(program, family, first),
            selected_leaf(program, family, second),
        ];
        expected.insert(model::State {
            function: mapping,
            arguments: vec![cell.clone()],
            implementations: implementations.clone(),
        });
        expected.insert(state(
            program,
            family.relay.unwrap(),
            vec![cell.clone()],
            implementations,
        ));
    }
    expected
}

fn mutual_states(
    program: &CanonicalProgram,
    family: &Family,
    binding: Binding,
) -> BTreeSet<model::State> {
    let cell = ground(program, "owned_i64_cell");
    let buffer = ground(program, "byte_buffer");
    let mut expected = BTreeSet::from([state(
        program,
        family.function,
        vec![cell.clone()],
        vec![selected_leaf(program, family, buffer.clone())],
    )]);
    let output = match binding {
        Binding::Alias => buffer.clone(),
        Binding::Constant => cell.clone(),
        Binding::Sequence => panic!("expanding closure is not a finite literal"),
    };
    expected.insert(state(
        program,
        family.relay.unwrap(),
        vec![cell.clone(), output],
        vec![selected_leaf(program, family, buffer)],
    ));
    if matches!(binding, Binding::Constant) {
        expected.insert(state(
            program,
            family.function,
            vec![cell.clone()],
            vec![selected_leaf(program, family, cell.clone())],
        ));
        expected.insert(state(
            program,
            family.relay.unwrap(),
            vec![cell.clone(), cell.clone()],
            vec![selected_leaf(program, family, cell)],
        ));
    }
    expected
}

fn assert_production(source: &KernelSnapshot, finite: bool) {
    for eager in [false, true] {
        let mut work = 0;
        let result = if eager {
            crate::platform::kernel::test_work::validate_snapshot_callable_closure_eager(
                &[source],
                &|| Ok(()),
                &mut work,
                Some(contract::MAXIMUM_VALIDATION_WORK),
            )
        } else {
            crate::platform::kernel::test_work::validate_snapshot_callable_closure(
                &[source],
                &|| Ok(()),
                &mut work,
                Some(contract::MAXIMUM_VALIDATION_WORK),
            )
        };
        match (finite, result) {
            (true, Ok(_)) => {}
            (false, Err(error)) => {
                assert_eq!(error.class, DiagnosticClass::Semantic);
                assert_eq!(error.code, "kernel_callable_expansion");
            }
            (_, other) => panic!("eager={eager}, finite={finite}: {other:?}"),
        }
        assert!(work > 0);
    }
}

fn assert_decision(family: &Family, program: &CanonicalProgram, finite: bool) {
    let proof = model::reference_classify(&program.program, model::Limits::default()).unwrap();
    assert_eq!(proof.finite, finite);
    assert_eq!(proof.positive_diagonal.is_some(), !finite);
    for (scope, ids) in &family.witness_inventories {
        let function = program.definition(*scope).unwrap();
        for (ordinal, id) in ids.iter().enumerate() {
            assert_eq!(program.witness_parameters[&(function, ordinal)], *id);
        }
    }
    assert_production(&family.common.source, finite);
}

fn assert_closure(program: &CanonicalProgram, expected: BTreeSet<model::State>, count: usize) {
    assert_eq!(expected.len(), count, "literal fixture inventory");
    let actual = program
        .program
        .concrete_closure(model::Limits {
            concrete_states: count,
            ..model::Limits::default()
        })
        .unwrap();
    assert_eq!(
        actual, expected,
        "complete functions, root order, actuals and prerequisites"
    );
    assert_eq!(
        program
            .program
            .concrete_closure(model::Limits {
                concrete_states: count - 1,
                ..model::Limits::default()
            })
            .unwrap_err(),
        model::Refusal::Capacity("concrete states")
    );
}

fn assert_outputs(
    program: &CanonicalProgram,
    family: &Family,
    binding: Binding,
    witness: &model::Witness,
) {
    let model::Witness::Selected {
        scheme,
        arguments,
        prerequisites,
    } = witness
    else {
        panic!("closed fixture has a symbolic witness")
    };
    if *scheme == program.definition(family.common.plain).unwrap() {
        let member = *program
            .members
            .iter()
            .find_map(|(id, identity)| {
                (*identity == (family.common.contract, family.common.member)).then_some(id)
            })
            .unwrap();
        let expected = match binding {
            Binding::Alias => arguments[1].clone(),
            Binding::Constant => ground(program, "owned_i64_cell"),
            Binding::Sequence => {
                let tag = *program
                    .atoms
                    .iter()
                    .find_map(|(id, atom)| {
                        (atom.kind == "owned_sequence" && atom.fields.is_empty()).then_some(id)
                    })
                    .unwrap();
                model::Term::Constructor(tag, vec![arguments[1].clone()])
            }
        };
        assert_eq!(program.program.project(witness, member).unwrap(), expected);
        assert!(prerequisites.is_empty());
    } else {
        assert_eq!(
            *scheme,
            program.definition(family.wrapper.unwrap()).unwrap()
        );
        assert_eq!(prerequisites.len(), 2);
        for child in prerequisites {
            assert_outputs(program, family, binding, child);
        }
    }
}

fn replace_in_operand(
    operand: &mut ImplementationOperand,
    old: TypeObjectDigest,
    new: TypeObjectDigest,
) {
    if let ImplementationOperand::Concrete {
        type_arguments,
        implementations,
        ..
    } = operand
    {
        for ty in type_arguments {
            if *ty == old {
                *ty = new;
            }
        }
        for child in implementations {
            replace_in_operand(child, old, new);
        }
    }
}

fn assert_bad_projection_scope(family: &Family) {
    let mut source = family.common.source.clone();
    let TypeForm::AssociatedWitnessProjection {
        parameter,
        contract,
        member,
        ..
    } = source.types[&family.projection].form
    else {
        unreachable!()
    };
    let wrong = ty(
        &mut source,
        TypeForm::AssociatedWitnessProjection {
            scope: family.common.plain,
            parameter,
            contract,
            member,
        },
    );
    let OwnerRecord::Expression(expression) = source
        .owners
        .get_mut(&OwnerKey::Expression(family.expression))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        type_arguments,
        implementations,
        ..
    } = &mut expression.operation
    else {
        unreachable!()
    };
    for ty in type_arguments {
        if *ty == family.projection {
            *ty = wrong;
        }
    }
    for operand in implementations {
        replace_in_operand(operand, family.projection, wrong);
    }
    assert_eq!(
        CanonicalProgram::read(&[&source], std::slice::from_ref(&family.entry))
            .err()
            .unwrap(),
        model::Refusal::Scope
    );
    for eager in [false, true] {
        let result = if eager {
            crate::platform::kernel::test_work::validate_snapshot_callable_closure_eager(
                &[&source],
                &|| Ok(()),
                &mut 0,
                Some(contract::MAXIMUM_VALIDATION_WORK),
            )
        } else {
            crate::platform::kernel::test_work::validate_snapshot_callable_closure(
                &[&source],
                &|| Ok(()),
                &mut 0,
                Some(contract::MAXIMUM_VALIDATION_WORK),
            )
        };
        assert_eq!(result.unwrap_err().class, DiagnosticClass::Semantic);
    }
    assert_production(&family.common.source, true);
}

fn assert_limits_cancel_retry(family: &Family) {
    use std::cell::Cell;
    for eager in [false, true] {
        let checks = Cell::new(0usize);
        let checkpoint = || {
            checks.set(checks.get() + 1);
            Ok(())
        };
        let mut work = 0;
        let measured =
            crate::platform::kernel::test_work::validate_snapshot_callable_closure_with_limits(
                &[&family.common.source],
                &checkpoint,
                &mut work,
                Some(contract::MAXIMUM_VALIDATION_WORK),
                64 * 1_048_576,
                eager,
            )
            .unwrap();
        let complete_checks = checks.get();
        assert!(work > 1 && measured.metadata_bytes > 1 && complete_checks > 1);
        let exact =
            crate::platform::kernel::test_work::validate_snapshot_callable_closure_with_limits(
                &[&family.common.source],
                &|| Ok(()),
                &mut 0,
                Some(work),
                measured.metadata_bytes,
                eager,
            )
            .unwrap();
        assert_eq!(exact, measured);
        for (maximum_work, maximum_bytes) in [
            (work - 1, measured.metadata_bytes),
            (work, measured.metadata_bytes - 1),
        ] {
            let error =
                crate::platform::kernel::test_work::validate_snapshot_callable_closure_with_limits(
                    &[&family.common.source],
                    &|| Ok(()),
                    &mut 0,
                    Some(maximum_work),
                    maximum_bytes,
                    eager,
                )
                .unwrap_err();
            assert_eq!(error.class, DiagnosticClass::Resource);
        }
        checks.set(0);
        let cancel = || {
            let next = checks.get() + 1;
            checks.set(next);
            if next == complete_checks - 1 {
                Err(Diagnostic::new(
                    DiagnosticClass::Cancelled,
                    "canonical_family_cancelled",
                    "late complete shared-path flow cancellation",
                ))
            } else {
                Ok(())
            }
        };
        let error =
            crate::platform::kernel::test_work::validate_snapshot_callable_closure_with_limits(
                &[&family.common.source],
                &cancel,
                &mut 0,
                Some(work),
                measured.metadata_bytes,
                eager,
            )
            .unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Cancelled);
        assert_eq!(error.code, "canonical_family_cancelled");
        let mut retry_work = 0;
        let retry =
            crate::platform::kernel::test_work::validate_snapshot_callable_closure_with_limits(
                &[&family.common.source],
                &|| Ok(()),
                &mut retry_work,
                Some(work),
                measured.metadata_bytes,
                eager,
            )
            .unwrap();
        assert_eq!((retry, retry_work), (measured, work));
    }
}

fn bounded_reference(
    family: &Family,
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
    work: &mut usize,
    maximum_work: usize,
    maximum_bytes: usize,
) -> Result<(model::Proof, usize), model::Refusal> {
    // Translation and classification share one allowance. A fresh proof cannot
    // conceal its raw-reader allocation behind a second independent budget.
    let budget = model::Budget::new(checkpoint, work, maximum_work, maximum_bytes);
    let program = CanonicalProgram::read_with_budget(
        &[&family.common.source],
        std::slice::from_ref(&family.entry),
        &budget,
    )?;
    let proof =
        model::reference_classify_with_budget(&program.program, model::Limits::default(), &budget)?;
    Ok((proof, budget.bytes()))
}

fn assert_reference_limits_cancel_retry(family: &Family) {
    use std::cell::Cell;
    let checks = Cell::new(0usize);
    let checkpoint = || {
        checks.set(checks.get() + 1);
        Ok(())
    };
    let mut work = 0;
    let (measured, bytes) = bounded_reference(
        family,
        &checkpoint,
        &mut work,
        contract::MAXIMUM_VALIDATION_WORK,
        64 * 1_048_576,
    )
    .unwrap();
    let complete_checks = checks.get();
    assert!(measured.finite && work > 1 && bytes > 1 && complete_checks > 1);
    let mut exact_work = 0;
    let (exact, exact_bytes) =
        bounded_reference(family, &|| Ok(()), &mut exact_work, work, bytes).unwrap();
    assert_eq!((exact_work, exact_bytes), (work, bytes));
    assert_eq!(
        (
            exact.finite,
            exact.positive_diagonal,
            exact.slots,
            exact.edges
        ),
        (
            measured.finite,
            measured.positive_diagonal,
            measured.slots,
            measured.edges
        )
    );
    for (maximum_work, maximum_bytes, expected) in [
        (work - 1, bytes, "reference work"),
        (work, bytes - 1, "reference storage"),
    ] {
        let error = match bounded_reference(family, &|| Ok(()), &mut 0, maximum_work, maximum_bytes)
        {
            Ok(_) => panic!("one-less shared reference budget must refuse"),
            Err(error) => error,
        };
        assert_eq!(error, model::Refusal::Capacity(expected));
    }
    checks.set(0);
    let cancel = || {
        let next = checks.get() + 1;
        checks.set(next);
        if next == complete_checks - 1 {
            Err(Diagnostic::new(
                DiagnosticClass::Cancelled,
                "canonical_reference_family_cancelled",
                "late independently read shared-path proof cancellation",
            ))
        } else {
            Ok(())
        }
    };
    let error = match bounded_reference(family, &cancel, &mut 0, work, bytes) {
        Ok(_) => panic!("late reference checkpoint must cancel"),
        Err(error) => error,
    };
    let model::Refusal::Control(error) = error else {
        panic!("reference cancellation changed its failure class")
    };
    assert_eq!(error.class, DiagnosticClass::Cancelled);
    assert_eq!(error.code, "canonical_reference_family_cancelled");
    let mut retry_work = 0;
    let (retry, retry_bytes) =
        bounded_reference(family, &|| Ok(()), &mut retry_work, work, bytes).unwrap();
    assert!(retry.finite && retry.positive_diagonal.is_none());
    assert_eq!((retry_work, retry_bytes), (work, bytes));
}

#[test]
fn canonical_two_witness_roots_keep_exact_alias_permutation() {
    for (binding, identical, count) in [
        (Binding::Alias, false, 2),
        (Binding::Alias, true, 1),
        (Binding::Constant, false, 2),
    ] {
        let family = roots(binding, identical);
        let program = read(&family);
        assert_decision(&family, &program, true);
        let expected = root_states(&program, &family, binding, identical);
        for state in &expected {
            assert_eq!(state.implementations.len(), 2);
            for root in &state.implementations {
                assert_outputs(&program, &family, binding, root);
            }
        }
        assert_closure(&program, expected, count);
    }
    let family = roots(Binding::Sequence, false);
    assert_decision(&family, &read(&family), false);
    assert_bad_projection_scope(&roots(Binding::Alias, false));
}

#[test]
fn canonical_shared_prerequisite_paths_keep_ordered_member_actuals() {
    for (binding, identical, count) in [
        (Binding::Alias, false, 6),
        (Binding::Alias, true, 3),
        (Binding::Constant, false, 8),
    ] {
        let family = paths(binding, identical);
        let program = read(&family);
        assert_decision(&family, &program, true);
        let expected = path_states(&program, &family, binding, identical);
        for state in &expected {
            assert_eq!(state.implementations.len(), 2);
            for root in &state.implementations {
                assert_outputs(&program, &family, binding, root);
            }
        }
        assert_closure(&program, expected, count);
    }
    let family = paths(Binding::Sequence, false);
    assert_decision(&family, &read(&family), false);
    let control = paths(Binding::Alias, false);
    assert_bad_projection_scope(&control);
    assert_limits_cancel_retry(&control);
    assert_reference_limits_cancel_retry(&control);
    let mut source = control.common.source.clone();
    let OwnerRecord::Expression(expression) = source
        .owners
        .get_mut(&OwnerKey::Expression(control.expression))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        implementations, ..
    } = &mut expression.operation
    else {
        unreachable!()
    };
    let ImplementationOperand::Concrete {
        implementations: children,
        ..
    } = &mut implementations[0]
    else {
        unreachable!()
    };
    children.pop();
    assert_eq!(
        CanonicalProgram::read(&[&source], std::slice::from_ref(&control.entry))
            .err()
            .unwrap(),
        model::Refusal::Arity
    );
}

#[test]
fn canonical_mixed_member_ordinary_mutual_cycles_match_independent_decisions() {
    for (binding, count) in [(Binding::Alias, 2), (Binding::Constant, 4)] {
        let family = mutual(binding, false, false);
        let program = read(&family);
        assert_decision(&family, &program, true);
        let expected = mutual_states(&program, &family, binding);
        for state in &expected {
            assert_eq!(state.implementations.len(), 1);
            assert_outputs(&program, &family, binding, &state.implementations[0]);
        }
        assert_closure(&program, expected, count);
    }
    for (binding, outer, untaken) in [
        (Binding::Sequence, false, false),
        (Binding::Sequence, false, true),
        (Binding::Alias, true, false),
        (Binding::Alias, true, true),
    ] {
        let family = mutual(binding, outer, untaken);
        let program = read(&family);
        assert_decision(&family, &program, false);
        if untaken {
            assert!(
                program.program.functions[&program.definition(family.function).unwrap()]
                    .calls
                    .iter()
                    .any(|call| call.untaken)
            );
        }
    }
    assert_bad_projection_scope(&mutual(Binding::Alias, false, false));
}

// The discarded function value is the sole f -> g edge. The existing mutual
// fixture supplies only g -> f; replace its f -> g call instead of adding a
// second route that could conceal missed function-value demand.
fn discarded_function_value(binding: Binding) -> Family {
    let mut family = mutual(binding, false, false);
    let relay = family.relay.unwrap();
    let OwnerRecord::Declaration(record) = family
        .common
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(relay.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        unreachable!()
    };
    // FunctionValue cannot carry lexical implementation parameters. The relay
    // already reconstructs a closed Plain operand from its ordinary actuals.
    function.implementation_parameters.clear();
    family
        .witness_inventories
        .retain(|(scope, _)| *scope != relay);
    let OwnerRecord::Expression(record) = family
        .common
        .source
        .owners
        .get_mut(&OwnerKey::Expression(family.expression))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        function,
        type_arguments,
        ..
    } = record.operation.clone()
    else {
        unreachable!()
    };
    assert_eq!(function, relay);
    record.operation = ExpressionOperation::FunctionValue {
        function,
        type_arguments,
        effect_arguments: vec![],
        requirement_arguments: vec![],

        read_scope_arguments: Vec::new(),
    };
    let unit = expression(
        &mut family.common.source,
        1101,
        ExpressionOperation::Unit {},
    );
    let body = expression(
        &mut family.common.source,
        1102,
        ExpressionOperation::Sequence {
            items: vec![family.expression, unit],
        },
    );
    let OwnerRecord::Declaration(record) = family
        .common
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(family.function.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        unreachable!()
    };
    function.body = body;
    family
}

// The entry remains Noop<Cell>. The unused Relay mapping and its generic target
// still form a complete-source cycle: g<Self,U> -> Relay<Plain<Self,U>>.step
// -> g<Self,Reader.Out>. The mapping supplies the only return edge and the only
// associated output actual; no direct recursive function call exists.
fn unused_mapping(binding: Binding) -> Family {
    let mut common = common(binding);
    let relay_contract = reference(&common.source, 120);
    let wrapper = reference(&common.source, 121);
    let relay = reference(&common.source, 122);
    let noop = reference(&common.source, 102);
    let (contract_self, _) = formal(&mut common.source, relay_contract, 1200);
    let (wrapper_self, wrapper_self_ty) = formal(&mut common.source, wrapper, 1210);
    let (relay_self, relay_self_ty) = formal(&mut common.source, relay, 1220);
    let (relay_u, relay_u_ty) = formal(&mut common.source, relay, 1221);
    declaration(
        &mut common.source,
        120,
        DeclarationPayload::OwnedContract(OwnedContract {
            self_parameter: contract_self,
            type_parameters: vec![],
            methods: vec![OwnedMethod {
                id: common.method,
                name: name("step"),
                parameters: vec![],
                result: crate::platform::kernel::CallableResult {
                    ty: common.unit,
                    relation: crate::platform::kernel::ResultRelation::Value,
                },
                effect: FunctionEffect::Pure,

                read_scope_parameters: Vec::new(),
                read_scope_sources: Vec::new(),
            }],
            associated_types: vec![],
        }),
    );
    let reader = witness(1210, common.contract, wrapper_self_ty);
    let output = projection(&mut common, wrapper, reader.id);
    declaration(
        &mut common.source,
        121,
        DeclarationPayload::OwnedImplementation(OwnedImplementation {
            type_parameters: vec![wrapper_self],
            implementation_parameters: vec![reader.clone()],
            contract: relay_contract,
            self_type: wrapper_self_ty,
            type_arguments: vec![],
            methods: vec![OwnedMethodImplementation {
                method: common.method,
                function: relay,
                type_arguments: vec![wrapper_self_ty, output],
                implementations: vec![],

                read_scope_arguments: Vec::new(),
            }],
            associated_types: vec![],
        }),
    );
    let selected_reader = leaf(&common, relay_self_ty, relay_u_ty);
    let body = expression(
        &mut common.source,
        1220,
        ExpressionOperation::MethodCall {
            witness: ImplementationOperand::Concrete {
                implementation: wrapper,
                type_arguments: vec![relay_self_ty],
                implementations: vec![selected_reader],
            },
            contract: relay_contract,
            method: common.method,
            arguments: vec![],

            read_scope_arguments: Vec::new(),
        },
    );
    function(
        &mut common.source,
        122,
        vec![relay_self, relay_u],
        vec![],
        common.unit,
        body,
    );
    let entry = CanonicalEntry {
        function: noop,
        arguments: vec![common.cell],
        implementations: vec![],
    };
    Family {
        common,
        entry,
        function: noop,
        relay: Some(relay),
        wrapper: Some(wrapper),
        projection: output,
        expression: body,
        witness_inventories: vec![(wrapper, vec![reader.id])],
    }
}

#[test]
fn canonical_discarded_function_value_is_the_decisive_associated_growth_edge() {
    for (binding, finite) in [(Binding::Alias, true), (Binding::Sequence, false)] {
        let mut family = discarded_function_value(binding);
        let program = read(&family);
        let from = program.definition(family.function).unwrap();
        let relay = program.definition(family.relay.unwrap()).unwrap();
        let calls = &program.program.functions[&from].calls;
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].kind, model::CallKind::FunctionValue);
        assert_eq!(calls[0].target, relay);
        assert!(matches!(
            calls[0].arguments[1],
            model::Term::Projection { .. }
        ));
        let returns = &program.program.functions[&relay].calls;
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].kind, model::CallKind::Function);
        assert_eq!(returns[0].target, from);
        assert_decision(&family, &program, finite);

        // Removing only the discarded value removes the cycle. A missed value
        // must therefore change the expanding verdict, rather than be masked.
        let OwnerRecord::Expression(record) = family
            .common
            .source
            .owners
            .get_mut(&OwnerKey::Expression(family.expression))
            .unwrap()
        else {
            unreachable!()
        };
        record.operation = ExpressionOperation::Unit {};
        let control = read(&family);
        assert!(control.program.functions[&from].calls.is_empty());
        assert_decision(&family, &control, true);
    }
}

#[test]
fn canonical_unused_method_mapping_is_the_decisive_associated_growth_edge() {
    for (binding, finite) in [(Binding::Alias, true), (Binding::Sequence, false)] {
        let mut family = unused_mapping(binding);
        let program = read(&family);
        let wrapper = family.wrapper.unwrap();
        let mapping = *program
            .definitions
            .iter()
            .find_map(|(id, identity)| {
                (*identity == DefinitionIdentity::MethodMapping(wrapper, family.common.method))
                    .then_some(id)
            })
            .unwrap();
        let relay = program.definition(family.relay.unwrap()).unwrap();
        let calls = &program.program.functions[&mapping].calls;
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].kind, model::CallKind::UnusedMapping);
        assert_eq!(calls[0].target, relay);
        assert!(matches!(
            calls[0].arguments[1],
            model::Term::Projection { .. }
        ));
        let returns = &program.program.functions[&relay].calls;
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].kind, model::CallKind::Function);
        assert_eq!(returns[0].target, mapping);
        let entry = program.program.entries[0].clone();
        assert!(program.program.functions[&entry.function].calls.is_empty());
        // Even the expanding program has a one-state executable entry closure.
        // Its unrelated finite prefix cannot discharge complete source growth.
        assert_closure(&program, BTreeSet::from([entry]), 1);
        assert_decision(&family, &program, finite);

        // Keep the mapping and method cycle, but reset its output actual to a
        // constant. This removes only the binding-dependent positive recurrence.
        let OwnerRecord::Declaration(record) = family
            .common
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(wrapper.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::OwnedImplementation(implementation) = &mut record.payload else {
            unreachable!()
        };
        implementation.methods[0].type_arguments[1] = family.common.cell;
        assert_decision(&family, &read(&family), true);
    }
}
