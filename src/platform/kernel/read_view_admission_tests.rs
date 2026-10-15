//! Bounded production boundary controls from raw canonical records. These do not
//! supply independent-reference conclusions or fabricate runtime admissions.
use super::*;
use crate::platform::semantic_id::{
    BindingId, DeclarationId, ExpressionId, ModuleId, ParameterId, ReadScopeParameterId,
};

const SEED: &[u8] = b"read-view-production-boundaries";
fn name(value: &str) -> Name {
    Name::new(value).unwrap()
}
fn formal() -> ReadScopeParameter {
    ReadScopeParameter {
        id: ReadScopeParameterId::migrate(SEED, 0),
        name: name("source"),
    }
}
fn scope(declaration: DeclarationReference) -> ReadScopeArgument {
    ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(declaration),
        parameter: formal().id,
    })
}
fn ty(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
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
    n: u64,
    kind: OwnerKind,
    payload: DeclarationPayload,
) -> DeclarationReference {
    let id = DeclarationId::migrate(SEED, n);
    let module = source
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Module(m) => match m.header.owner {
                OwnerKey::Module(id) => Some(id),
                _ => None,
            },
            _ => None,
        })
        .unwrap_or_else(|| ModuleId::migrate(SEED, 0));
    source.owners.insert(
        OwnerKey::Declaration(id),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(id), kind),
            module,
            name: name(&format!("D{n}")),
            visibility: DeclarationVisibility::Public,
            payload,
        }),
    );
    DeclarationReference {
        package: source.root.package_id,
        declaration: id,
    }
}
fn function(source: &mut KernelSnapshot, n: u64, result: TypeObjectDigest) -> DeclarationReference {
    let body = expression(source, n, ExpressionOperation::Unit {});
    declaration(
        source,
        n,
        OwnerKind::PureFunction,
        DeclarationPayload::Function(FunctionDeclaration {
            implementation_parameters: vec![],
            requirement_parameters: vec![],
            effect_parameters: vec![],
            type_parameters: vec![],
            parameters: vec![],
            result: CallableResult {
                ty: result,
                relation: ResultRelation::Value,
            },
            read_scope_parameters: vec![],
            read_scope_sources: vec![],
            effect: FunctionEffect::Pure,
            body,
        }),
    )
}
fn function_mut(
    source: &mut KernelSnapshot,
    reference: DeclarationReference,
) -> &mut FunctionDeclaration {
    let OwnerRecord::Declaration(d) = source
        .owners
        .get_mut(&OwnerKey::Declaration(reference.declaration))
        .unwrap()
    else {
        panic!()
    };
    let DeclarationPayload::Function(f) = &mut d.payload else {
        panic!()
    };
    f
}
fn parameter(
    source: &mut KernelSnapshot,
    function: DeclarationReference,
    n: u64,
    ty: TypeObjectDigest,
    use_mode: ParameterUse,
) -> ParameterId {
    let id = ParameterId::migrate(SEED, n);
    let header = OwnerHeader::new(OwnerKey::Parameter(id), OwnerKind::Parameter);
    source.owners.insert(
        OwnerKey::Parameter(id),
        OwnerRecord::Parameter(ParameterRecord {
            header,
            parent: ParameterParent::Function(function.declaration),
            name: name(&format!("P{n}")),
            ty,
            use_mode,
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
    let header = OwnerHeader::new(OwnerKey::Binding(id), OwnerKind::Binding);
    source.owners.insert(
        OwnerKey::Binding(id),
        OwnerRecord::Binding(BindingRecord {
            header,
            kind,
            name: name(&format!("B{n}")),
            declared_type: Some(ty),
            value,
        }),
    );
    id
}
fn view(source: &mut KernelSnapshot, n: u64) -> DeclarationReference {
    declaration(
        source,
        n,
        OwnerKind::ReadView,
        DeclarationPayload::ReadView(ReadViewDeclaration {
            type_parameters: vec![],
            scope_parameter: formal(),
            fields: vec![],
        }),
    )
}
fn validate_function(
    source: &KernelSnapshot,
    function: DeclarationReference,
) -> Result<(), Diagnostic> {
    let key = OwnerKey::Declaration(function.declaration);
    super::super::memory::validate_owner(source, key, source.owners.get(&key).unwrap())
}

#[test]
fn unused_scope_actual_keeps_exact_function_authority_separate_from_view_definition() {
    let mut source = super::super::tests::transport_snapshot();
    let unit = ty(&mut source, TypeForm::Unit);
    let caller = function(&mut source, 10, unit);
    let callee = function(&mut source, 11, unit);
    function_mut(&mut source, caller)
        .read_scope_parameters
        .push(formal());
    function_mut(&mut source, callee)
        .read_scope_parameters
        .push(formal());
    let loans = BTreeSet::new();
    let authority = read_view_admission::ScopeAuthority {
        binder: Some(ReadScopeBinder::Declaration(caller)),
        lexical: Some(caller),
        live_loans: &loans,
    };
    assert!(
        read_view_admission::scope_bindings(
            &source,
            ReadScopeBinder::Declaration(callee),
            &[scope(caller)],
            authority,
            &OwnedAssociatedAdmission::new()
        )
        .is_ok()
    );
    let OwnerRecord::Declaration(d) = source
        .owners
        .get_mut(&OwnerKey::Declaration(caller.declaration))
        .unwrap()
    else {
        panic!()
    };
    d.payload = DeclarationPayload::ReadView(ReadViewDeclaration {
        type_parameters: vec![],
        scope_parameter: formal(),
        fields: vec![],
    });
    d.header.kind = OwnerKind::ReadView;
    let error = read_view_admission::scope_bindings(
        &source,
        ReadScopeBinder::Declaration(callee),
        &[scope(caller)],
        authority,
        &OwnedAssociatedAdmission::new(),
    )
    .unwrap_err();
    assert_eq!(error.code, "kernel_read_view");
    assert!(error.message.contains("named graph function"));
    let descriptor = read_view_admission::ScopeAuthority {
        lexical: None,
        ..authority
    };
    assert!(
        descriptor
            .validate(&source, scope(caller), &OwnedAssociatedAdmission::new())
            .is_ok()
    );
}

fn arm_fixture() -> (
    KernelSnapshot,
    DeclarationReference,
    ExpressionId,
    BindingId,
    BindingId,
) {
    let mut source = super::super::tests::transport_snapshot();
    let unit = ty(&mut source, TypeForm::Unit);
    let cell = ty(&mut source, TypeForm::OwnedI64Cell);
    let choice = ty(
        &mut source,
        TypeForm::OwnedChoice {
            cases: vec![
                StructuralTypeField {
                    name: name("left"),
                    ty: cell,
                },
                StructuralTypeField {
                    name: name("right"),
                    ty: cell,
                },
            ],
        },
    );
    let caller = function(&mut source, 20, unit);
    let input = parameter(&mut source, caller, 20, choice, ParameterUse::Borrow);
    function_mut(&mut source, caller).parameters.push(input);
    let descriptor = view(&mut source, 21);
    let source_read = expression(
        &mut source,
        20,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(input),
        },
    );
    let left = binding(&mut source, 20, BindingKind::OwnedBorrow, cell, None);
    let right = binding(&mut source, 21, BindingKind::OwnedBorrow, cell, None);
    let mut arms = Vec::new();
    let mut creation = source_read;
    for (n, label, loan) in [(22, "left", left), (24, "right", right)] {
        let actual = ReadScopeArgument::Lexical {
            callable: caller,
            binding: loan,
        };
        let result = ty(
            &mut source,
            TypeForm::ReadViewApplied {
                declaration: descriptor,
                arguments: vec![],
                read_scope: actual,
            },
        );
        let create = expression(
            &mut source,
            n,
            ExpressionOperation::CreateReadView {
                declaration: descriptor,
                type_arguments: vec![],
                read_scope: actual,
                source: LocalValueReference::LexicalBinding(loan),
                fields: vec![],
            },
        );
        if label == "left" {
            creation = create;
        }
        let handle = binding(&mut source, n, BindingKind::ReadView, result, Some(create));
        let drop = expression(
            &mut source,
            n + 1,
            ExpressionOperation::DropReadView {
                value: LocalValueReference::LexicalBinding(handle),
            },
        );
        let body = expression(
            &mut source,
            n + 100,
            ExpressionOperation::Let {
                bindings: vec![handle],
                body: drop,
            },
        );
        arms.push(OwnedChoiceArm {
            name: name(label),
            binding: loan,
            body,
        });
    }
    let root = expression(
        &mut source,
        30,
        ExpressionOperation::MatchBorrowedOwned {
            choice_type: choice,
            source: source_read,
            arms,
        },
    );
    function_mut(&mut source, caller).body = root;
    (source, caller, creation, left, right)
}

#[test]
fn borrowed_choice_arm_scope_is_live_only_in_its_exact_arm_with_native_loan_generation() {
    let (mut source, caller, creation, _left, right) = arm_fixture();
    assert!(validate_function(&source, caller).is_ok());
    let OwnerRecord::Expression(e) = source
        .owners
        .get_mut(&OwnerKey::Expression(creation))
        .unwrap()
    else {
        panic!()
    };
    let ExpressionOperation::CreateReadView { read_scope, .. } = &mut e.operation else {
        panic!()
    };
    *read_scope = ReadScopeArgument::Lexical {
        callable: caller,
        binding: right,
    };
    assert!(validate_function(&source, caller).is_err());
    let (mut source, caller, _, _, _) = arm_fixture();
    let OwnerRecord::Declaration(d) = source
        .owners
        .get_mut(&OwnerKey::Declaration(caller.declaration))
        .unwrap()
    else {
        panic!()
    };
    d.header.contract_version = 33;
    assert!(validate_function(&source, caller).is_err());
}

fn prospective_fixture(child_uses_scope: bool) -> (KernelSnapshot, DeclarationReference) {
    let mut source = super::super::tests::transport_snapshot();
    let unit = ty(&mut source, TypeForm::Unit);
    let cell = ty(&mut source, TypeForm::OwnedI64Cell);
    let scalar = ty(&mut source, TypeForm::I64);
    let descriptor = view(&mut source, 40);
    let producer = function(&mut source, 41, unit);
    let ordinary = parameter(
        &mut source,
        producer,
        41,
        scalar,
        ParameterUse::Unrestricted,
    );
    let input = parameter(&mut source, producer, 42, cell, ParameterUse::Borrow);
    let result = ty(
        &mut source,
        TypeForm::ReadViewApplied {
            declaration: descriptor,
            arguments: vec![],
            read_scope: scope(producer),
        },
    );
    let body = expression(
        &mut source,
        41,
        ExpressionOperation::CreateReadView {
            declaration: descriptor,
            type_arguments: vec![],
            read_scope: scope(producer),
            source: LocalValueReference::FunctionParameter(input),
            fields: vec![],
        },
    );
    let f = function_mut(&mut source, producer);
    f.parameters = vec![ordinary, input];
    f.read_scope_parameters = vec![formal()];
    f.read_scope_sources = vec![ReadScopeSource {
        source: input,
        scope: scope(producer),
    }];
    f.result = CallableResult {
        ty: result,
        relation: ResultRelation::ViewFrom {
            source: input,
            scope: scope(producer),
        },
    };
    f.body = body;
    let scout = function(&mut source, 43, scalar);
    let literal = expression(&mut source, 43, ExpressionOperation::I64 { value: 9 });
    let f = function_mut(&mut source, scout);
    if child_uses_scope {
        f.read_scope_parameters.push(formal());
    }
    f.body = literal;
    let caller = function(&mut source, 44, unit);
    let input = parameter(&mut source, caller, 44, cell, ParameterUse::Borrow);
    function_mut(&mut source, caller).parameters.push(input);
    let handle = BindingId::migrate(SEED, 44);
    let actual = ReadScopeArgument::Lexical {
        callable: caller,
        binding: handle,
    };
    let applied = ty(
        &mut source,
        TypeForm::ReadViewApplied {
            declaration: descriptor,
            arguments: vec![],
            read_scope: actual,
        },
    );
    let bound = binding(&mut source, 44, BindingKind::ReadView, applied, None);
    assert_eq!(handle, bound);
    let argument = expression(
        &mut source,
        45,
        ExpressionOperation::Call {
            function: scout,
            type_arguments: vec![],
            read_scope_arguments: if child_uses_scope {
                vec![actual]
            } else {
                vec![]
            },
            arguments: vec![],
            requirement_arguments: vec![],
            effect_arguments: vec![],
        },
    );
    let owned = expression(
        &mut source,
        46,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(input),
        },
    );
    let call = expression(
        &mut source,
        47,
        ExpressionOperation::Call {
            function: producer,
            type_arguments: vec![],
            read_scope_arguments: vec![actual],
            arguments: vec![argument, owned],
            requirement_arguments: vec![],
            effect_arguments: vec![],
        },
    );
    let drop = expression(
        &mut source,
        48,
        ExpressionOperation::DropReadView {
            value: LocalValueReference::LexicalBinding(handle),
        },
    );
    let root = expression(
        &mut source,
        49,
        ExpressionOperation::BorrowCall {
            call,
            binding: handle,
            body: drop,
        },
    );
    function_mut(&mut source, caller).body = root;
    (source, caller)
}

#[test]
fn prospective_borrow_call_scope_authenticates_direct_metadata_without_argument_permission() {
    let (source, caller) = prospective_fixture(false);
    assert!(validate_function(&source, caller).is_ok());
    let (source, caller) = prospective_fixture(true);
    let error = validate_function(&source, caller).unwrap_err();
    assert_eq!(error.code, "kernel_read_view");
    assert!(error.message.contains("live containing loan"));
}

#[test]
fn wide_value_result_retains_complete_view_exclusion_without_an_unused_owned_query() {
    let mut source = super::super::tests::transport_snapshot();
    let unit = ty(&mut source, TypeForm::Unit);
    let fields = (0..512)
        .map(|position| StructuralTypeField {
            name: name(&format!("field_{position:04}")),
            ty: unit,
        })
        .collect::<Vec<_>>();
    let result = ty(&mut source, TypeForm::StructuralRecord { fields });
    let occurrence = OwnedTypeOccurrence::authored(None, 34);
    let limits = AssociatedAdmissionLimits {
        maximum_metadata_bytes: 192 * 1024,
        ..AssociatedAdmissionLimits::default()
    };
    let admitted = OwnedAssociatedAdmission::with_limits(limits);
    // The caller owns complete occurrence/category admission. The relation proof
    // still walks every field, including unused syntax, within this shared bound.
    assert!(!super::super::memory::direct_at(&source, result, occurrence, &admitted).unwrap());
    read_view_admission::validate_result::<ParameterId>(
        &source,
        &CallableResult {
            ty: result,
            relation: ResultRelation::Value,
        },
        &[],
        &[],
        true,
        occurrence,
        &admitted,
    )
    .unwrap();
    assert!(admitted.work().1 < limits.maximum_metadata_bytes);

    let source_tied = OwnedAssociatedAdmission::with_limits(limits);
    let error = read_view_admission::validate_result(
        &source,
        &CallableResult {
            ty: result,
            relation: ResultRelation::ReadFrom {
                source: ParameterId::migrate(SEED, 9000),
            },
        },
        &[],
        &[],
        true,
        occurrence,
        &source_tied,
    )
    .unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic);
    assert_eq!(
        error.message,
        "source-tied results require a pure named function and an exact Owned or view result"
    );

    let view_declaration = view(&mut source, 9000);
    let hidden_view = ty(
        &mut source,
        TypeForm::ReadViewApplied {
            declaration: view_declaration,
            arguments: Vec::new(),
            read_scope: scope(view_declaration),
        },
    );
    // The hidden view is pushed before all ordinary children, so the depth-first
    // absence proof reaches it only after the repeated ordinary leaf has finished.
    let mut fields = vec![StructuralTypeField {
        name: name("a_hidden_view"),
        ty: hidden_view,
    }];
    fields.extend((0..512).map(|position| StructuralTypeField {
        name: name(&format!("field_{position:04}")),
        ty: unit,
    }));
    let hidden = ty(&mut source, TypeForm::StructuralRecord { fields });
    let caller_error = super::super::memory::direct_at(
        &source,
        hidden,
        occurrence,
        &OwnedAssociatedAdmission::new(),
    )
    .unwrap_err();
    assert_eq!(caller_error.class, DiagnosticClass::Semantic);
    let relation_error = read_view_admission::validate_result::<ParameterId>(
        &source,
        &CallableResult {
            ty: hidden,
            relation: ResultRelation::Value,
        },
        &[],
        &[],
        true,
        occurrence,
        &OwnedAssociatedAdmission::with_limits(limits),
    )
    .unwrap_err();
    assert_eq!(relation_error.class, DiagnosticClass::Semantic);
    assert_eq!(
        relation_error.message,
        "Value result cannot carry a read view"
    );
}
