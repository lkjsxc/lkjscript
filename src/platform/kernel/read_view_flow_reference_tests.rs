//! Raw-record controls. The fixtures carry no production admission, runtime
//! return seal, source token, prepared classification, or fabricated carrier.
use super::*;
use crate::platform::semantic_id::{DeclarationId, ParameterId, ReadScopeParameterId};

const SEED: &[u8] = b"raw-associated-read-view-flow";

fn name(s: &str) -> Name {
    Name::new(s).unwrap()
}
fn decl(source: &KernelSnapshot, n: u64) -> DeclarationReference {
    DeclarationReference {
        package: source.root.package_id,
        declaration: DeclarationId::migrate(SEED, n),
    }
}
fn formal(n: u64) -> ReadScopeParameter {
    ReadScopeParameter {
        id: ReadScopeParameterId::migrate(SEED, n),
        name: name(&format!("s{n}")),
    }
}
fn scope(declaration: DeclarationReference, parameter: &ReadScopeParameter) -> ReadScopeArgument {
    ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(declaration),
        parameter: parameter.id,
    })
}
fn intern(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    source.types.insert(digest, object);
    digest
}
fn expression(source: &mut KernelSnapshot, n: u64, operation: ExpressionOperation) -> ExpressionId {
    let id = ExpressionId::migrate(SEED, n);
    source.owners.insert(
        OwnerKey::Expression(id),
        OwnerRecord::Expression(ExpressionRecord::new(id, operation).unwrap()),
    );
    id
}
fn declaration(
    source: &mut KernelSnapshot,
    reference: DeclarationReference,
    kind: OwnerKind,
    payload: DeclarationPayload,
) {
    let module = source
        .owners
        .iter()
        .find_map(|(key, owner)| matches!(owner, OwnerRecord::Module(_)).then_some(*key))
        .and_then(|key| match key {
            OwnerKey::Module(id) => Some(id),
            _ => None,
        })
        .unwrap();
    source.owners.insert(
        OwnerKey::Declaration(reference.declaration),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(reference.declaration), kind),
            module,
            name: name(&format!("d{}", reference.declaration)),
            visibility: DeclarationVisibility::Public,
            payload,
        }),
    );
}
fn parameter(
    source: &mut KernelSnapshot,
    function: DeclarationReference,
    n: u64,
    ty: TypeObjectDigest,
    mode: ParameterUse,
) -> ParameterId {
    let id = ParameterId::migrate(SEED, n);
    source.owners.insert(
        OwnerKey::Parameter(id),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(id), OwnerKind::Parameter),
            parent: ParameterParent::Function(function.declaration),
            name: name(&format!("p{n}")),
            ty,
            use_mode: mode,
            resource_requirement: None,
        }),
    );
    id
}
fn binding(
    source: &mut KernelSnapshot,
    n: u64,
    kind: BindingKind,
    ty: TypeObjectDigest,
    value: Option<ExpressionId>,
) -> BindingId {
    let id = BindingId::migrate(SEED, n);
    source.owners.insert(
        OwnerKey::Binding(id),
        OwnerRecord::Binding(BindingRecord {
            header: OwnerHeader::new(OwnerKey::Binding(id), OwnerKind::Binding),
            name: name(&format!("b{n}")),
            kind,
            declared_type: Some(ty),
            value,
        }),
    );
    id
}
#[allow(clippy::too_many_arguments)]
fn function(
    source: &mut KernelSnapshot,
    reference: DeclarationReference,
    parameters: Vec<ParameterId>,
    scope_parameter: Option<ReadScopeParameter>,
    source_scopes: Vec<ReadScopeSource<ParameterId>>,
    result: CallableResult<ParameterId>,
    effect: FunctionEffect,
    body: ExpressionId,
) {
    declaration(
        source,
        reference,
        match effect {
            FunctionEffect::Pure => OwnerKind::PureFunction,
            FunctionEffect::Task { .. } => OwnerKind::TaskFunction,
        },
        DeclarationPayload::Function(FunctionDeclaration {
            type_parameters: Vec::new(),
            implementation_parameters: Vec::new(),
            requirement_parameters: Vec::new(),
            effect_parameters: Vec::new(),
            parameters,
            read_scope_parameters: scope_parameter.into_iter().collect(),
            read_scope_sources: source_scopes,
            result,
            effect,
            body,
        }),
    );
}
fn function_mut(
    source: &mut KernelSnapshot,
    reference: DeclarationReference,
) -> &mut FunctionDeclaration {
    match source
        .owners
        .get_mut(&OwnerKey::Declaration(reference.declaration))
        .unwrap()
    {
        OwnerRecord::Declaration(d) => match &mut d.payload {
            DeclarationPayload::Function(f) => f,
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}

fn set_effect(
    source: &mut KernelSnapshot,
    reference: DeclarationReference,
    effect: FunctionEffect,
) {
    let OwnerRecord::Declaration(record) = source
        .owners
        .get_mut(&OwnerKey::Declaration(reference.declaration))
        .unwrap()
    else {
        panic!()
    };
    record.header.kind = match effect {
        FunctionEffect::Pure => OwnerKind::PureFunction,
        FunctionEffect::Task { .. } => OwnerKind::TaskFunction,
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        panic!()
    };
    function.effect = effect;
}
fn operation_mut(source: &mut KernelSnapshot, id: ExpressionId) -> &mut ExpressionOperation {
    match source.owners.get_mut(&OwnerKey::Expression(id)).unwrap() {
        OwnerRecord::Expression(e) => &mut e.operation,
        _ => unreachable!(),
    }
}

struct Fixture {
    source: KernelSnapshot,
    function: DeclarationReference,
    view: DeclarationReference,
    parameter: ReadScopeParameter,
    scope: ReadScopeArgument,
    view_type: TypeObjectDigest,
    owned: TypeObjectDigest,
    a: ParameterId,
    b: ParameterId,
    root: ExpressionId,
    reference: FieldId,
}
fn fixture() -> Fixture {
    let mut source = crate::platform::kernel::tests::transport_snapshot();
    source.root.graph_contract_version = contract::GRAPH_CONTRACT_VERSION;
    let function_ref = decl(&source, 1);
    let view = decl(&source, 2);
    let view_parameter = formal(2);
    let view_scope = scope(view, &view_parameter);
    let parameter_scope = formal(1);
    let caller_scope = scope(function_ref, &parameter_scope);
    let owned = intern(&mut source, TypeForm::OwnedI64Cell);
    let scalar = intern(&mut source, TypeForm::I64);
    let reference = FieldId::migrate(SEED, 1);
    let metadata = FieldId::migrate(SEED, 2);
    declaration(
        &mut source,
        view,
        OwnerKind::ReadView,
        DeclarationPayload::ReadView(ReadViewDeclaration {
            type_parameters: Vec::new(),
            scope_parameter: view_parameter,
            fields: vec![
                ReadViewField {
                    id: reference,
                    name: name("read"),
                    ty: owned,
                    kind: ReadViewFieldKind::ReadReference {
                        read_scope: view_scope,
                    },
                },
                ReadViewField {
                    id: metadata,
                    name: name("label"),
                    ty: scalar,
                    kind: ReadViewFieldKind::Ordinary,
                },
            ],
        }),
    );
    let view_type = intern(
        &mut source,
        TypeForm::ReadViewApplied {
            declaration: view,
            arguments: Vec::new(),
            read_scope: caller_scope,
        },
    );
    let a = parameter(&mut source, function_ref, 1, owned, ParameterUse::Borrow);
    let b = parameter(&mut source, function_ref, 2, owned, ParameterUse::Borrow);
    let read = expression(
        &mut source,
        1,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(a),
        },
    );
    let data = expression(&mut source, 2, ExpressionOperation::I64 { value: 7 });
    let root = expression(
        &mut source,
        3,
        ExpressionOperation::CreateReadView {
            declaration: view,
            type_arguments: Vec::new(),
            read_scope: caller_scope,
            source: LocalValueReference::FunctionParameter(a),
            fields: vec![
                ReadViewFieldInitializer {
                    field: metadata,
                    value: data,
                },
                ReadViewFieldInitializer {
                    field: reference,
                    value: read,
                },
            ],
        },
    );
    function(
        &mut source,
        function_ref,
        vec![a, b],
        Some(parameter_scope.clone()),
        vec![
            ReadScopeSource {
                source: a,
                scope: caller_scope,
            },
            ReadScopeSource {
                source: b,
                scope: caller_scope,
            },
        ],
        CallableResult {
            ty: view_type,
            relation: ResultRelation::ViewFrom {
                source: a,
                scope: caller_scope,
            },
        },
        FunctionEffect::Pure,
        root,
    );
    Fixture {
        source,
        function: function_ref,
        view,
        parameter: parameter_scope,
        scope: caller_scope,
        view_type,
        owned,
        a,
        b,
        root,
        reference,
    }
}
fn validate(f: &Fixture) -> Result<()> {
    Resolver::new(&f.source).validate_read_view_function(f.function)
}

#[test]
fn raw_constructor_accepts_authored_order_and_rejects_equal_scope_wrong_parameter_field() {
    let mut f = fixture();
    assert_eq!(validate(&f), Ok(()));
    *operation_mut(&mut f.source, ExpressionId::migrate(SEED, 1)) = ExpressionOperation::Local {
        value: LocalValueReference::FunctionParameter(f.b),
    };
    assert_eq!(validate(&f), Err(Failure::Authority));
}

#[test]
fn zero_reference_descriptor_keeps_the_explicit_source_anchor() {
    let mut f = fixture();
    if let OwnerRecord::Declaration(d) = f
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(f.view.declaration))
        .unwrap()
        && let DeclarationPayload::ReadView(view) = &mut d.payload
    {
        view.fields.clear();
    }
    // Remove now-unreachable initializer records: the oracle accepts raw
    // canonical ownership, not an invented duplicate semantic root.
    f.source
        .owners
        .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, 1)));
    f.source
        .owners
        .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, 2)));
    if let ExpressionOperation::CreateReadView { fields, .. } = operation_mut(&mut f.source, f.root)
    {
        fields.clear();
    }
    assert_eq!(validate(&f), Ok(()));
    if let ExpressionOperation::CreateReadView { source, .. } = operation_mut(&mut f.source, f.root)
    {
        *source = LocalValueReference::FunctionParameter(f.b);
    }
    assert_eq!(validate(&f), Err(Failure::Authority));
}

#[test]
fn an_untaken_constructor_branch_is_admitted_and_cannot_relabel_the_source() {
    let mut f = fixture();
    let condition = expression(&mut f.source, 10, ExpressionOperation::Bool { value: true });
    let wrong_ref = expression(
        &mut f.source,
        11,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(f.b),
        },
    );
    let data = expression(&mut f.source, 12, ExpressionOperation::I64 { value: 7 });
    let wrong = expression(
        &mut f.source,
        13,
        ExpressionOperation::CreateReadView {
            declaration: f.view,
            type_arguments: Vec::new(),
            read_scope: f.scope,
            source: LocalValueReference::FunctionParameter(f.b),
            fields: vec![
                ReadViewFieldInitializer {
                    field: FieldId::migrate(SEED, 2),
                    value: data,
                },
                ReadViewFieldInitializer {
                    field: f.reference,
                    value: wrong_ref,
                },
            ],
        },
    );
    let root = expression(
        &mut f.source,
        14,
        ExpressionOperation::If {
            condition,
            when_true: f.root,
            when_false: wrong,
        },
    );
    function_mut(&mut f.source, f.function).body = root;
    assert_eq!(validate(&f), Err(Failure::Authority));
}

fn reborrow_fixture() -> Fixture {
    let mut f = fixture();
    f.source
        .owners
        .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, 1)));
    f.source
        .owners
        .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, 2)));
    if let OwnerRecord::Parameter(p) = f.source.owners.get_mut(&OwnerKey::Parameter(f.a)).unwrap() {
        p.ty = f.view_type;
    }
    let function = function_mut(&mut f.source, f.function);
    function.parameters = vec![f.a];
    function.read_scope_sources.clear();
    *operation_mut(&mut f.source, f.root) = ExpressionOperation::ReborrowReadView {
        value: LocalValueReference::FunctionParameter(f.a),
        read_scope: f.scope,
    };
    f
}

#[test]
fn a_borrowed_view_parameter_requires_explicit_reborrow_and_cannot_move_or_drop() {
    let mut f = reborrow_fixture();
    assert_eq!(validate(&f), Ok(()));
    for operation in [
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(f.a),
        },
        ExpressionOperation::MoveReadView {
            value: LocalValueReference::FunctionParameter(f.a),
        },
        ExpressionOperation::DropReadView {
            value: LocalValueReference::FunctionParameter(f.a),
        },
    ] {
        *operation_mut(&mut f.source, f.root) = operation;
        assert!(matches!(
            validate(&f),
            Err(Failure::Authority | Failure::Category)
        ));
    }
}

#[test]
fn view_read_result_uses_an_accessible_reference_path_and_not_custody() {
    let mut f = reborrow_fixture();
    let child = binding(&mut f.source, 10, BindingKind::OwnedBorrow, f.owned, None);
    let tail = expression(
        &mut f.source,
        10,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(child),
        },
    );
    *operation_mut(&mut f.source, f.root) = ExpressionOperation::BorrowReadViewField {
        value: LocalValueReference::FunctionParameter(f.a),
        field: ReadViewFieldReference {
            declaration: f.view,
            field: f.reference,
        },
        binding: child,
        body: tail,
    };
    function_mut(&mut f.source, f.function).result = CallableResult {
        ty: f.owned,
        relation: ResultRelation::ReadFrom { source: f.a },
    };
    assert_eq!(validate(&f), Ok(()));
    if let ExpressionOperation::BorrowReadViewField { field, .. } =
        operation_mut(&mut f.source, f.root)
    {
        field.field = FieldId::migrate(SEED, 2);
    }
    assert_eq!(validate(&f), Err(Failure::Category));
}

#[test]
fn ordinary_let_cannot_hide_an_affine_view_initializer() {
    let mut f = fixture();
    let b = binding(
        &mut f.source,
        20,
        BindingKind::Let,
        f.view_type,
        Some(f.root),
    );
    let tail = expression(
        &mut f.source,
        20,
        ExpressionOperation::MoveReadView {
            value: LocalValueReference::LexicalBinding(b),
        },
    );
    let root = expression(
        &mut f.source,
        21,
        ExpressionOperation::Let {
            bindings: vec![b],
            body: tail,
        },
    );
    function_mut(&mut f.source, f.function).body = root;
    assert_eq!(validate(&f), Err(Failure::Category));
}

fn parent_reader_fixture(consume_during_read: bool) -> Fixture {
    let mut f = fixture();
    let parent = BindingId::migrate(SEED, 30);
    let lexical_scope = ReadScopeArgument::Lexical {
        callable: f.function,
        binding: parent,
    };
    let view_type = intern(
        &mut f.source,
        TypeForm::ReadViewApplied {
            declaration: f.view,
            arguments: Vec::new(),
            read_scope: lexical_scope,
        },
    );
    if let OwnerRecord::Parameter(p) = f.source.owners.get_mut(&OwnerKey::Parameter(f.a)).unwrap() {
        p.use_mode = ParameterUse::Consume;
    }
    let definition = function_mut(&mut f.source, f.function);
    definition.parameters = vec![f.a];
    definition.read_scope_parameters.clear();
    definition.read_scope_sources.clear();
    definition.result = CallableResult {
        ty: f.owned,
        relation: ResultRelation::Value,
    };
    if let ExpressionOperation::CreateReadView { read_scope, .. } =
        operation_mut(&mut f.source, f.root)
    {
        *read_scope = lexical_scope;
    }
    binding(
        &mut f.source,
        30,
        BindingKind::ReadView,
        view_type,
        Some(f.root),
    );
    let child = binding(&mut f.source, 31, BindingKind::OwnedBorrow, f.owned, None);
    let drop_parent = expression(
        &mut f.source,
        30,
        ExpressionOperation::DropReadView {
            value: LocalValueReference::LexicalBinding(parent),
        },
    );
    let inside = expression(
        &mut f.source,
        31,
        if consume_during_read {
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(f.a),
            }
        } else {
            ExpressionOperation::Unit {}
        },
    );
    let during = expression(
        &mut f.source,
        32,
        ExpressionOperation::Sequence {
            items: vec![drop_parent, inside],
        },
    );
    let borrowed = expression(
        &mut f.source,
        33,
        ExpressionOperation::BorrowReadViewField {
            value: LocalValueReference::LexicalBinding(parent),
            field: ReadViewFieldReference {
                declaration: f.view,
                field: f.reference,
            },
            binding: child,
            body: during,
        },
    );
    let owner = expression(
        &mut f.source,
        34,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(f.a),
        },
    );
    let after = expression(
        &mut f.source,
        35,
        ExpressionOperation::Sequence {
            items: vec![borrowed, owner],
        },
    );
    let root = expression(
        &mut f.source,
        36,
        ExpressionOperation::Let {
            bindings: vec![parent],
            body: after,
        },
    );
    function_mut(&mut f.source, f.function).body = root;
    f
}

#[test]
fn dropping_parent_handle_preserves_child_guards_then_releases_the_owner() {
    assert_eq!(validate(&parent_reader_fixture(false)), Ok(()));
    assert_eq!(
        validate(&parent_reader_fixture(true)),
        Err(Failure::Authority)
    );
}

#[test]
fn call_return_paths_forward_the_chosen_argument_through_a_reborrowed_handle() {
    let mut f = fixture();
    let factory = f.function;
    let outer = decl(&f.source, 40);
    let outer_parameter = formal(40);
    let outer_scope = scope(outer, &outer_parameter);
    let ty = intern(
        &mut f.source,
        TypeForm::ReadViewApplied {
            declaration: f.view,
            arguments: Vec::new(),
            read_scope: outer_scope,
        },
    );
    let a = parameter(&mut f.source, outer, 40, f.owned, ParameterUse::Borrow);
    let b = parameter(&mut f.source, outer, 41, f.owned, ParameterUse::Borrow);
    let a_read = expression(
        &mut f.source,
        40,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(a),
        },
    );
    let b_read = expression(
        &mut f.source,
        41,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(b),
        },
    );
    let call = expression(
        &mut f.source,
        42,
        ExpressionOperation::Call {
            function: factory,
            type_arguments: Vec::new(),
            read_scope_arguments: vec![outer_scope],
            arguments: vec![a_read, b_read],
            requirement_arguments: Vec::new(),
            effect_arguments: Vec::new(),
        },
    );
    let handle = binding(&mut f.source, 40, BindingKind::ReadView, ty, None);
    let tail = expression(
        &mut f.source,
        43,
        ExpressionOperation::ReborrowReadView {
            value: LocalValueReference::LexicalBinding(handle),
            read_scope: outer_scope,
        },
    );
    let body = expression(
        &mut f.source,
        44,
        ExpressionOperation::BorrowCall {
            call,
            binding: handle,
            body: tail,
        },
    );
    function(
        &mut f.source,
        outer,
        vec![a, b],
        Some(outer_parameter),
        vec![
            ReadScopeSource {
                source: a,
                scope: outer_scope,
            },
            ReadScopeSource {
                source: b,
                scope: outer_scope,
            },
        ],
        CallableResult {
            ty,
            relation: ResultRelation::ViewFrom {
                source: a,
                scope: outer_scope,
            },
        },
        FunctionEffect::Pure,
        body,
    );
    f.function = outer;
    assert_eq!(validate(&f), Ok(()));
    function_mut(&mut f.source, outer).result.relation = ResultRelation::ViewFrom {
        source: b,
        scope: outer_scope,
    };
    assert_eq!(validate(&f), Err(Failure::Authority));
}

#[test]
fn empty_task_effects_do_not_authorize_a_source_tied_view_result() {
    let mut f = fixture();
    assert_eq!(validate(&f), Ok(()));
    set_effect(
        &mut f.source,
        f.function,
        FunctionEffect::Task {
            requirements: Vec::new(),
            effect_parameters: Vec::new(),
        },
    );
    assert_eq!(validate(&f), Err(Failure::Authority));
    set_effect(&mut f.source, f.function, FunctionEffect::Pure);
    assert_eq!(validate(&f), Ok(()));
}

#[test]
fn closed_joined_tasks_admit_each_view_read_and_keep_task_kind_distinct() {
    let mut f = reborrow_fixture();
    let unit = intern(&mut f.source, TypeForm::Unit);
    let pair = intern(
        &mut f.source,
        TypeForm::StructuralRecord {
            fields: vec![
                StructuralTypeField {
                    name: name("left"),
                    ty: unit,
                },
                StructuralTypeField {
                    name: name("right"),
                    ty: unit,
                },
            ],
        },
    );
    let mut calls = Vec::new();
    for n in [50, 60] {
        let task = decl(&f.source, n);
        let formal = formal(n);
        let task_scope = scope(task, &formal);
        let view_type = intern(
            &mut f.source,
            TypeForm::ReadViewApplied {
                declaration: f.view,
                arguments: Vec::new(),
                read_scope: task_scope,
            },
        );
        let input = parameter(&mut f.source, task, n, view_type, ParameterUse::Borrow);
        let body = expression(&mut f.source, n, ExpressionOperation::Unit {});
        function(
            &mut f.source,
            task,
            vec![input],
            Some(formal),
            Vec::new(),
            CallableResult {
                ty: unit,
                relation: ResultRelation::Value,
            },
            FunctionEffect::Task {
                requirements: Vec::new(),
                effect_parameters: Vec::new(),
            },
            body,
        );
        let argument = expression(
            &mut f.source,
            n + 1,
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(f.a),
            },
        );
        calls.push(expression(
            &mut f.source,
            n + 2,
            ExpressionOperation::Call {
                function: task,
                type_arguments: Vec::new(),
                read_scope_arguments: vec![f.scope],
                arguments: vec![argument],
                requirement_arguments: Vec::new(),
                effect_arguments: Vec::new(),
            },
        ));
    }
    *operation_mut(&mut f.source, f.root) = ExpressionOperation::Parallel {
        left: calls[0],
        right: calls[1],
    };
    let definition = function_mut(&mut f.source, f.function);
    definition.result = CallableResult {
        ty: pair,
        relation: ResultRelation::Value,
    };
    set_effect(
        &mut f.source,
        f.function,
        FunctionEffect::Task {
            requirements: Vec::new(),
            effect_parameters: Vec::new(),
        },
    );
    assert_eq!(validate(&f), Ok(()));
    set_effect(&mut f.source, f.function, FunctionEffect::Pure);
    assert_eq!(validate(&f), Err(Failure::Category));
}

#[test]
fn capacity_refusal_is_distinct_and_a_fresh_larger_proof_keeps_source_decisions() {
    let mut f = fixture();
    let limits = Limits {
        work: 0,
        objects: contract::MAXIMUM_CHILDREN,
    };
    assert_eq!(
        Resolver::with_limits(&f.source, limits).validate_read_view_function(f.function),
        Err(Failure::Capacity)
    );
    assert_eq!(validate(&f), Ok(()));
    if let ExpressionOperation::CreateReadView { source, .. } = operation_mut(&mut f.source, f.root)
    {
        *source = LocalValueReference::FunctionParameter(f.b);
    }
    assert_eq!(validate(&f), Err(Failure::Authority));
    let _ = f.parameter;
}

fn production_view_result(
    f: &Fixture,
) -> std::result::Result<(), crate::platform::diagnostic::Diagnostic> {
    let key = OwnerKey::Declaration(f.function.declaration);
    crate::platform::kernel::memory::validate_owner(&f.source, key, &f.source.owners[&key])
}

#[test]
fn production_view_return_matches_raw_oracle_through_lexical_calls_sequences_and_branches() {
    for route in 0..5 {
        let mut f = fixture();
        match route {
            0 => {}
            1 => {
                let data = expression(&mut f.source, 20, ExpressionOperation::I64 { value: 0 });
                let root = expression(
                    &mut f.source,
                    21,
                    ExpressionOperation::Sequence {
                        items: vec![data, f.root],
                    },
                );
                function_mut(&mut f.source, f.function).body = root;
            }
            2 => {
                let condition = expression(
                    &mut f.source,
                    20,
                    ExpressionOperation::Bool { value: false },
                );
                let read = expression(
                    &mut f.source,
                    21,
                    ExpressionOperation::Local {
                        value: LocalValueReference::FunctionParameter(f.a),
                    },
                );
                let data = expression(&mut f.source, 22, ExpressionOperation::I64 { value: 9 });
                let mut operation = operation_mut(&mut f.source, f.root).clone();
                let ExpressionOperation::CreateReadView { fields, .. } = &mut operation else {
                    unreachable!()
                };
                for field in fields {
                    field.value = if field.field == f.reference {
                        read
                    } else {
                        data
                    };
                }
                let other = expression(&mut f.source, 23, operation);
                let root = expression(
                    &mut f.source,
                    24,
                    ExpressionOperation::If {
                        condition,
                        when_true: f.root,
                        when_false: other,
                    },
                );
                function_mut(&mut f.source, f.function).body = root;
            }
            3 => {
                let callee = f.function;
                let caller = decl(&f.source, 20);
                let formal = formal(20);
                let caller_scope = scope(caller, &formal);
                let view_type = intern(
                    &mut f.source,
                    TypeForm::ReadViewApplied {
                        declaration: f.view,
                        arguments: Vec::new(),
                        read_scope: caller_scope,
                    },
                );
                let first = parameter(&mut f.source, caller, 20, f.owned, ParameterUse::Borrow);
                let second = parameter(&mut f.source, caller, 21, f.owned, ParameterUse::Borrow);
                let first_arg = expression(
                    &mut f.source,
                    20,
                    ExpressionOperation::Local {
                        value: LocalValueReference::FunctionParameter(first),
                    },
                );
                let second_arg = expression(
                    &mut f.source,
                    21,
                    ExpressionOperation::Local {
                        value: LocalValueReference::FunctionParameter(second),
                    },
                );
                let call = expression(
                    &mut f.source,
                    22,
                    ExpressionOperation::Call {
                        function: callee,
                        type_arguments: Vec::new(),
                        effect_arguments: Vec::new(),
                        requirement_arguments: Vec::new(),
                        read_scope_arguments: vec![caller_scope],
                        arguments: vec![first_arg, second_arg],
                    },
                );
                let bound = binding(&mut f.source, 20, BindingKind::ReadView, view_type, None);
                let moved = expression(
                    &mut f.source,
                    23,
                    ExpressionOperation::MoveReadView {
                        value: LocalValueReference::LexicalBinding(bound),
                    },
                );
                let root = expression(
                    &mut f.source,
                    24,
                    ExpressionOperation::BorrowCall {
                        call,
                        binding: bound,
                        body: moved,
                    },
                );
                function(
                    &mut f.source,
                    caller,
                    vec![first, second],
                    Some(formal),
                    vec![
                        ReadScopeSource {
                            source: first,
                            scope: caller_scope,
                        },
                        ReadScopeSource {
                            source: second,
                            scope: caller_scope,
                        },
                    ],
                    CallableResult {
                        ty: view_type,
                        relation: ResultRelation::ViewFrom {
                            source: first,
                            scope: caller_scope,
                        },
                    },
                    FunctionEffect::Pure,
                    root,
                );
                f.function = caller;
            }
            4 => {
                let OwnerRecord::Parameter(parameter) =
                    f.source.owners.get_mut(&OwnerKey::Parameter(f.a)).unwrap()
                else {
                    unreachable!()
                };
                parameter.ty = f.view_type;
                f.source
                    .owners
                    .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, 1)));
                f.source
                    .owners
                    .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, 2)));
                *operation_mut(&mut f.source, f.root) = ExpressionOperation::ReborrowReadView {
                    value: LocalValueReference::FunctionParameter(f.a),
                    read_scope: f.scope,
                };
            }
            _ => unreachable!(),
        }
        assert_eq!(validate(&f), Ok(()), "raw oracle route {route}");
        assert_eq!(
            production_view_result(&f),
            Ok(()),
            "production route {route}"
        );
    }
}

#[test]
fn production_view_return_rejects_owned_data_stale_proofs_and_wrong_selected_sources() {
    for attack in 0..6 {
        let mut f = fixture();
        match attack {
            0 | 1 => {
                f.source
                    .owners
                    .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, 1)));
                f.source
                    .owners
                    .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, 2)));
                let operation = if attack == 0 {
                    let scalar = intern(&mut f.source, TypeForm::I64);
                    let sequence_type =
                        intern(&mut f.source, TypeForm::OwnedSequence { item: scalar });
                    ExpressionOperation::SequenceEmpty { sequence_type }
                } else {
                    ExpressionOperation::I64 { value: 9 }
                };
                *operation_mut(&mut f.source, f.root) = operation;
            }
            2 => {
                let data = expression(&mut f.source, 20, ExpressionOperation::I64 { value: 9 });
                let root = expression(
                    &mut f.source,
                    21,
                    ExpressionOperation::Sequence {
                        items: vec![f.root, data],
                    },
                );
                function_mut(&mut f.source, f.function).body = root;
            }
            3 | 4 => {
                let condition = expression(
                    &mut f.source,
                    20,
                    ExpressionOperation::Bool { value: attack == 3 },
                );
                let data = expression(&mut f.source, 21, ExpressionOperation::I64 { value: 9 });
                let (when_true, when_false) = if attack == 3 {
                    (f.root, data)
                } else {
                    (data, f.root)
                };
                let root = expression(
                    &mut f.source,
                    22,
                    ExpressionOperation::If {
                        condition,
                        when_true,
                        when_false,
                    },
                );
                function_mut(&mut f.source, f.function).body = root;
            }
            5 => {
                let ExpressionOperation::CreateReadView { source, .. } =
                    operation_mut(&mut f.source, f.root)
                else {
                    unreachable!()
                };
                *source = LocalValueReference::FunctionParameter(f.b);
                *operation_mut(&mut f.source, ExpressionId::migrate(SEED, 1)) =
                    ExpressionOperation::Local {
                        value: LocalValueReference::FunctionParameter(f.b),
                    };
            }
            _ => unreachable!(),
        }
        assert!(validate(&f).is_err(), "raw oracle attack {attack}");
        let failure = production_view_result(&f).unwrap_err();
        assert_eq!(
            failure.code, "kernel_buffer_ownership",
            "production attack {attack}"
        );
        if attack == 0 {
            assert_eq!(
                failure.message,
                "owned buffer requires a consuming result or annotated local"
            );
        }
    }
}
