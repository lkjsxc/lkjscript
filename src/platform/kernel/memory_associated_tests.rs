//! Canonical flow witnesses exercise custody without enabling global admission.
use super::*;
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, BindingId, DeclarationId, ImplementationParameterId, MethodId,
    ModuleId, ParameterId, TypeParameterId,
};

const SEED: &[u8] = b"associated-production-memory-flow";

#[derive(Clone, Copy)]
pub(crate) enum FlowKind {
    Direct,
    Witness,
    Method,
}

pub(crate) struct Fixture {
    pub(crate) source: KernelSnapshot,
    pub(crate) caller: DeclarationReference,
    pub(crate) callee: DeclarationReference,
    pub(crate) call: ExpressionId,
    pub(crate) borrow: ExpressionId,
    pub(crate) local: ExpressionId,
    pub(crate) binding: BindingId,
    pub(crate) first: ParameterId,
    pub(crate) second: ParameterId,
}

fn name(value: &str) -> Name {
    Name::new(value).unwrap()
}
fn intern(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    source.types.insert(digest, object);
    digest
}
fn insert_declaration(
    source: &mut KernelSnapshot,
    module: ModuleId,
    ordinal: u64,
    kind: OwnerKind,
    payload: DeclarationPayload,
) -> DeclarationReference {
    let id = DeclarationId::migrate(SEED, ordinal);
    source.owners.insert(
        OwnerKey::Declaration(id),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(id), kind),
            module,
            name: name(&format!("flow{ordinal}")),
            visibility: DeclarationVisibility::Public,
            payload,
        }),
    );
    DeclarationReference {
        package: source.root.package_id,
        declaration: id,
    }
}
fn insert_type_parameter(
    source: &mut KernelSnapshot,
    declaration: DeclarationReference,
    ordinal: u64,
) -> (TypeParameterId, TypeObjectDigest) {
    let parameter = TypeParameterId::migrate(SEED, ordinal);
    source.owners.insert(
        OwnerKey::TypeParameter(parameter),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(parameter), OwnerKind::TypeParameter),
            declaration: declaration.declaration,
            name: name(&format!("T{ordinal}")),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    (
        parameter,
        intern(source, TypeForm::TypeParameter { parameter }),
    )
}
fn expression(
    source: &mut KernelSnapshot,
    ordinal: u64,
    operation: ExpressionOperation,
) -> ExpressionId {
    let id = ExpressionId::migrate(SEED, ordinal);
    source.owners.insert(
        OwnerKey::Expression(id),
        OwnerRecord::Expression(ExpressionRecord::new(id, operation).unwrap()),
    );
    id
}
fn value_parameter(
    source: &mut KernelSnapshot,
    scope: DeclarationReference,
    ordinal: u64,
    ty: TypeObjectDigest,
) -> ParameterId {
    let id = ParameterId::migrate(SEED, ordinal);
    source.owners.insert(
        OwnerKey::Parameter(id),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(id), OwnerKind::Parameter),
            parent: ParameterParent::Function(scope.declaration),
            name: name(&format!("value{ordinal}")),
            ty,
            use_mode: ParameterUse::Borrow,
            resource_requirement: None,
        }),
    );
    id
}
fn projected(
    source: &mut KernelSnapshot,
    scope: DeclarationReference,
    parameter: ImplementationParameterId,
    contract: DeclarationReference,
    member: AssociatedTypeMemberId,
) -> TypeObjectDigest {
    intern(
        source,
        TypeForm::AssociatedWitnessProjection {
            scope,
            parameter,
            contract,
            member,
        },
    )
}

pub(crate) fn fixture(kind: FlowKind) -> Fixture {
    let mut source = super::super::tests::transport_snapshot();
    let module = source
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Module(module) => match module.header.owner {
                OwnerKey::Module(id) => Some(id),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    let unit = intern(&mut source, TypeForm::Unit);
    let method = MethodId::migrate(SEED, 0);
    let member = AssociatedTypeMemberId::migrate(SEED, 0);
    let self_parameter = TypeParameterId::migrate(SEED, 0);
    let contract = DeclarationReference {
        package: source.root.package_id,
        declaration: DeclarationId::migrate(SEED, 0),
    };
    let abstract_member = intern(
        &mut source,
        TypeForm::AssociatedContractMember { contract, member },
    );
    insert_declaration(
        &mut source,
        module,
        0,
        OwnerKind::OwnedContract,
        DeclarationPayload::OwnedContract(OwnedContract {
            self_parameter,
            type_parameters: vec![],
            associated_types: vec![AssociatedTypeMember {
                id: member,
                name: name("Out"),
                bound: AssociatedTypeBound::Owned,
            }],
            methods: vec![OwnedMethod {
                id: method,
                name: name("read"),
                parameters: vec![OwnedMethodParameter {
                    ty: abstract_member,
                    use_mode: ParameterUse::Borrow,
                }],
                result: abstract_member,
                result_borrow: Some(0),
                effect: FunctionEffect::Pure,
            }],
        }),
    );
    insert_type_parameter(&mut source, contract, 0);
    let placeholder = expression(&mut source, 0, ExpressionOperation::Unit {});
    let bare = || FunctionDeclaration {
        implementation_parameters: vec![],
        requirement_parameters: vec![],
        effect_parameters: vec![],
        type_parameters: vec![],
        parameters: vec![],
        result: unit,
        result_borrow: None,
        effect: FunctionEffect::Pure,
        body: placeholder,
    };
    let callee = insert_declaration(
        &mut source,
        module,
        1,
        OwnerKind::PureFunction,
        DeclarationPayload::Function(bare()),
    );
    let caller = insert_declaration(
        &mut source,
        module,
        2,
        OwnerKind::PureFunction,
        DeclarationPayload::Function(bare()),
    );
    let (callee_type_parameter, callee_type) = insert_type_parameter(&mut source, callee, 1);
    let (caller_type_parameter, caller_type) = insert_type_parameter(&mut source, caller, 2);
    let callee_witness = ImplementationParameterId::migrate(SEED, 0);
    let caller_witness = ImplementationParameterId::migrate(SEED, 1);
    let callee_projection = projected(&mut source, callee, callee_witness, contract, member);
    let caller_projection = projected(&mut source, caller, caller_witness, contract, member);
    let witness = |id, self_type| ImplementationParameter {
        id,
        name: name("reader"),
        contract,
        self_type,
        type_arguments: vec![],
    };
    let callee_result = if matches!(kind, FlowKind::Direct) {
        callee_type
    } else {
        callee_projection
    };
    let callee_parameter = value_parameter(&mut source, callee, 0, callee_result);
    let callee_body = expression(
        &mut source,
        1,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(callee_parameter),
        },
    );
    if let OwnerRecord::Declaration(record) = source
        .owners
        .get_mut(&OwnerKey::Declaration(callee.declaration))
        .unwrap()
    {
        record.payload = DeclarationPayload::Function(FunctionDeclaration {
            implementation_parameters: if matches!(kind, FlowKind::Direct) {
                vec![]
            } else {
                vec![witness(callee_witness, callee_type)]
            },
            type_parameters: vec![callee_type_parameter],
            parameters: vec![callee_parameter],
            result: callee_result,
            result_borrow: Some(callee_parameter),
            body: callee_body,
            ..bare()
        });
    }
    let first = value_parameter(&mut source, caller, 1, caller_projection);
    let second = value_parameter(&mut source, caller, 2, caller_projection);
    let local = expression(
        &mut source,
        2,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(first),
        },
    );
    let operand = ImplementationOperand::Parameter {
        scope: caller,
        parameter: caller_witness,
    };
    let call = expression(
        &mut source,
        3,
        match kind {
            FlowKind::Direct => ExpressionOperation::Call {
                function: callee,
                type_arguments: vec![caller_projection],
                arguments: vec![local],
                effect_arguments: vec![],
                requirement_arguments: vec![],
            },
            FlowKind::Witness => ExpressionOperation::ImplementationCall {
                function: callee,
                type_arguments: vec![caller_type],
                implementations: vec![operand],
                arguments: vec![local],
                effect_arguments: vec![],
                requirement_arguments: vec![],
            },
            FlowKind::Method => ExpressionOperation::MethodCall {
                witness: operand,
                contract,
                method,
                arguments: vec![local],
            },
        },
    );
    let binding = BindingId::migrate(SEED, 0);
    source.owners.insert(
        OwnerKey::Binding(binding),
        OwnerRecord::Binding(BindingRecord {
            header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
            name: name("view"),
            kind: BindingKind::OwnedBorrow,
            value: None,
            declared_type: Some(caller_projection),
        }),
    );
    let returned = expression(
        &mut source,
        4,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(binding),
        },
    );
    let borrow = expression(
        &mut source,
        5,
        ExpressionOperation::BorrowCall {
            call,
            binding,
            body: returned,
        },
    );
    if let OwnerRecord::Declaration(record) = source
        .owners
        .get_mut(&OwnerKey::Declaration(caller.declaration))
        .unwrap()
    {
        record.payload = DeclarationPayload::Function(FunctionDeclaration {
            implementation_parameters: vec![witness(caller_witness, caller_type)],
            type_parameters: vec![caller_type_parameter],
            parameters: vec![first, second],
            result: caller_projection,
            result_borrow: Some(first),
            body: borrow,
            ..bare()
        });
    }
    Fixture {
        source,
        caller,
        callee,
        call,
        borrow,
        local,
        binding,
        first,
        second,
    }
}

pub(crate) fn check_caller(fixture: &Fixture) -> Result<(), Diagnostic> {
    let key = OwnerKey::Declaration(fixture.caller.declaration);
    validate_owner(
        &fixture.source,
        key,
        fixture.source.owners.get(&key).unwrap(),
    )
}

#[test]
fn projected_direct_witness_and_method_borrowed_results_preserve_exact_source() {
    for kind in [FlowKind::Direct, FlowKind::Witness, FlowKind::Method] {
        let fixture = fixture(kind);
        check_caller(&fixture).unwrap();
        let invocation = borrow_invocation(
            &fixture.source,
            fixture.call,
            Some(fixture.caller.declaration),
        )
        .unwrap();
        assert_eq!(invocation.source, fixture.local);
        assert_eq!(invocation.source_type, invocation.result);
    }
}

#[test]
fn equal_projected_types_cannot_swap_borrowed_result_source() {
    for kind in [FlowKind::Direct, FlowKind::Witness, FlowKind::Method] {
        let mut fixture = fixture(kind);
        let OwnerRecord::Declaration(record) = fixture
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(fixture.caller.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::Function(function) = &mut record.payload else {
            unreachable!()
        };
        function.result_borrow = Some(fixture.second);
        assert_eq!(
            check_caller(&fixture).unwrap_err().code,
            "kernel_buffer_ownership"
        );
    }
}

#[test]
fn parent_parameter_binding_local_and_wrapped_call_generations_are_independent() {
    for kind in [FlowKind::Direct, FlowKind::Witness, FlowKind::Method] {
        for target in 0..6 {
            let mut fixture = fixture(kind);
            let key = match target {
                0 => OwnerKey::Declaration(fixture.caller.declaration),
                1 => OwnerKey::Parameter(fixture.first),
                2 => OwnerKey::Binding(fixture.binding),
                3 => OwnerKey::Expression(fixture.local),
                4 => OwnerKey::Expression(fixture.call),
                _ => OwnerKey::Expression(fixture.borrow),
            };
            fixture
                .source
                .owners
                .get_mut(&key)
                .unwrap()
                .set_encoding_for_edit(32);
            assert!(
                check_caller(&fixture).is_err(),
                "consumer {target} unexpectedly acquired Graph 33"
            );
        }
    }
}

#[test]
fn projected_actual_owned_only_bound_does_not_grant_transfer_or_share() {
    for constraints in [
        TypeParameterConstraints::OwnedTransferable,
        TypeParameterConstraints::OwnedShareable,
    ] {
        let mut fixture = fixture(FlowKind::Direct);
        let formal = fixture
            .source
            .owners
            .values_mut()
            .find_map(|owner| match owner {
                OwnerRecord::TypeParameter(parameter)
                    if parameter.declaration == fixture.callee.declaration =>
                {
                    Some(parameter)
                }
                _ => None,
            })
            .unwrap();
        formal.constraints = constraints;
        assert!(check_caller(&fixture).is_err());
    }
}
