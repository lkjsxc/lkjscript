//! Independent canonical-source fixtures for the private completed schema core.
//! No producer catalogue or public associated admission token supplies expectations.

use super::reference_schema::NormalizedReferenceSchema;
use super::reference_types::associated_preparation::{self, Limits, Proof, test_support};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{
    BindingKind, BindingRecord, DeclarationPayload, DeclarationRecord, DeclarationReference,
    DeclarationVisibility, ExpressionOperation, ExpressionRecord, FunctionDeclaration,
    FunctionEffect, ImplementationOperand, KernelSnapshot, LocalValueReference, Name, OwnerHeader,
    OwnerKey, OwnerKind, OwnerRecord, ParameterParent, ParameterRecord, ParameterUse,
    StructuralTypeField, TypeForm, TypeObject, TypeObjectDigest, TypeParameterConstraints,
    TypeParameterRecord, encode_type_object,
};
use crate::platform::semantic_id::{
    BindingId, DeclarationId, ExpressionId, ImplementationParameterId, ParameterId, TypeParameterId,
};

const SEED: &[u8] = b"canonical-associated-schema-integration";

fn name(value: &str) -> Name {
    Name::new(value).unwrap()
}

fn intern(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    source.types.insert(ty, object);
    ty
}

fn function(source: &KernelSnapshot, scope: DeclarationReference) -> &FunctionDeclaration {
    let OwnerRecord::Declaration(owner) = &source.owners[&OwnerKey::Declaration(scope.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &owner.payload else {
        unreachable!()
    };
    function
}

fn function_mut(
    source: &mut KernelSnapshot,
    scope: DeclarationReference,
) -> &mut FunctionDeclaration {
    let OwnerRecord::Declaration(owner) = source
        .owners
        .get_mut(&OwnerKey::Declaration(scope.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut owner.payload else {
        unreachable!()
    };
    function
}

// These fixtures replace an authored leaf, rather than adding unattached syntax.
// Keep the admission requirement on every retained expression owner intact.
fn discard_replaced_local(source: &mut KernelSnapshot, expression: ExpressionId) {
    assert!(matches!(
        source.owners.get(&OwnerKey::Expression(expression)),
        Some(OwnerRecord::Expression(ExpressionRecord {
            operation: ExpressionOperation::Local { .. },
            ..
        }))
    ));
    source.owners.remove(&OwnerKey::Expression(expression));
}

fn replace_function_local_body(
    source: &mut KernelSnapshot,
    scope: DeclarationReference,
    body: ExpressionId,
) {
    let previous = function(source, scope).body;
    function_mut(source, scope).body = body;
    discard_replaced_local(source, previous);
}

fn effect(source: &mut KernelSnapshot, scope: DeclarationReference, effect: FunctionEffect) {
    let OwnerRecord::Declaration(owner) = source
        .owners
        .get_mut(&OwnerKey::Declaration(scope.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    owner.header.kind = if matches!(effect, FunctionEffect::Pure) {
        OwnerKind::PureFunction
    } else {
        OwnerKind::TaskFunction
    };
    let DeclarationPayload::Function(function) = &mut owner.payload else {
        unreachable!()
    };
    function.effect = effect;
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

fn operation_mut(source: &mut KernelSnapshot, id: ExpressionId) -> &mut ExpressionOperation {
    let OwnerRecord::Expression(record) = source.owners.get_mut(&OwnerKey::Expression(id)).unwrap()
    else {
        unreachable!()
    };
    &mut record.operation
}

fn parameter_mut(source: &mut KernelSnapshot, id: ParameterId) -> &mut ParameterRecord {
    let OwnerRecord::Parameter(record) = source.owners.get_mut(&OwnerKey::Parameter(id)).unwrap()
    else {
        unreachable!()
    };
    record
}

fn declared_function(
    source: &mut KernelSnapshot,
    template: DeclarationReference,
    ordinal: u64,
    label: &str,
    function: FunctionDeclaration,
) -> DeclarationReference {
    let OwnerRecord::Declaration(owner) =
        source.owners[&OwnerKey::Declaration(template.declaration)].clone()
    else {
        unreachable!()
    };
    let reference = DeclarationReference {
        package: template.package,
        declaration: DeclarationId::migrate(SEED, ordinal),
    };
    let kind = if matches!(function.effect, FunctionEffect::Pure) {
        OwnerKind::PureFunction
    } else {
        OwnerKind::TaskFunction
    };
    source.owners.insert(
        OwnerKey::Declaration(reference.declaration),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(reference.declaration), kind),
            module: owner.module,
            name: name(label),
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(function),
        }),
    );
    reference
}

fn derive(fixture: &test_support::Fixture) -> Result<Proof, ExecutionError> {
    associated_preparation::derive(
        &[&fixture.source],
        &[],
        Limits::default(),
        &ExecutionControl::uncancelled(),
    )
}

fn complete(fixture: &test_support::Fixture) -> Result<NormalizedReferenceSchema, ExecutionError> {
    super::reference_types::associated_schema_fixture(
        &[&fixture.source],
        derive(fixture)?,
        &ExecutionControl::uncancelled(),
    )
}

// Complete raw admission now checks these same authority obligations before
// the older borrowed-result and permission-discharge stages. Bind each negative
// to its exact semantic refusal, rather than merely accepting any earlier error.
fn assert_raw_refusal(error: ExecutionError, message: &str) {
    assert_eq!(
        error.class,
        ExecutionFailureClass::Infrastructure,
        "{error:?}"
    );
    assert_eq!(error.code, "reference_read_view_admission", "{error:?}");
    assert_eq!(error.message, message, "{error:?}");
}

fn assert_epoch_refusal(error: ExecutionError) {
    // Flat epoch admission rejects foreign owner bytes during preparation,
    // before any scoped-view flow proof can be constructed.
    assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
    assert_eq!(error.code, "reference_associated_preparation");
    assert_eq!(
        error.message,
        "canonical associated application has a mismatched scope, inventory, binding, or target"
    );
}

fn complete_after_checks(
    fixture: &test_support::Fixture,
    checks: u64,
) -> Result<NormalizedReferenceSchema, ExecutionError> {
    super::reference_types::associated_schema_fixture(
        &[&fixture.source],
        derive(fixture)?,
        &ExecutionControl::cancel_after_checks(checks),
    )
}

// The relay has its own formal IDs and lexical scope. Its projected signature
// must be substituted into the caller once; equal Self does not identify a witness.
fn add_relay(fixture: &mut test_support::Fixture) -> DeclarationReference {
    let scope = DeclarationReference {
        package: fixture.consumer.package,
        declaration: DeclarationId::migrate(SEED, 100),
    };
    let formal = TypeParameterId::migrate(SEED, 100);
    fixture.source.owners.insert(
        OwnerKey::TypeParameter(formal),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(formal), OwnerKind::TypeParameter),
            declaration: scope.declaration,
            name: name("RelaySelf"),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    let self_type = intern(
        &mut fixture.source,
        TypeForm::TypeParameter { parameter: formal },
    );
    let witness = ImplementationParameterId::migrate(SEED, 100);
    let projection = intern(
        &mut fixture.source,
        TypeForm::AssociatedWitnessProjection {
            scope,
            parameter: witness,
            contract: fixture.contract,
            member: fixture.member,
        },
    );
    let parameter = ParameterId::migrate(SEED, 100);
    fixture.source.owners.insert(
        OwnerKey::Parameter(parameter),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(scope.declaration),
            name: name("value"),
            ty: projection,
            use_mode: ParameterUse::Borrow,
            resource_requirement: None,
        }),
    );
    let body = expression(
        &mut fixture.source,
        100,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(parameter),
        },
    );
    let mut relay = function(&fixture.source, fixture.consumer).clone();
    relay.type_parameters = vec![formal];
    relay.implementation_parameters[0].id = witness;
    relay.implementation_parameters[0].name = name("relay_ops");
    relay.implementation_parameters[0].self_type = self_type;
    relay.parameters = vec![parameter];
    relay.result.ty = projection;
    relay.result.relation = crate::platform::kernel::ResultRelation::ReadFrom { source: parameter };
    relay.body = body;
    assert_eq!(
        declared_function(&mut fixture.source, fixture.consumer, 100, "relay", relay),
        scope
    );
    scope
}

struct Forwarding {
    fixture: test_support::Fixture,
    relay: DeclarationReference,
    wrapper: ExpressionId,
    call: ExpressionId,
    alternate: ImplementationParameterId,
}

fn forwarding() -> Forwarding {
    let mut fixture = test_support::make_fixture();
    let relay = add_relay(&mut fixture);
    let caller = function(&fixture.source, fixture.consumer).clone();
    let alternate = ImplementationParameterId::migrate(SEED, 101);
    let mut second = caller.implementation_parameters[0].clone();
    second.id = alternate;
    second.name = name("unused_equal_self_ops");
    function_mut(&mut fixture.source, fixture.consumer)
        .implementation_parameters
        .push(second);
    let local = expression(
        &mut fixture.source,
        101,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(caller.parameters[0]),
        },
    );
    let call = expression(
        &mut fixture.source,
        102,
        ExpressionOperation::ImplementationCall {
            function: relay,
            type_arguments: vec![caller.implementation_parameters[0].self_type],
            implementations: vec![ImplementationOperand::Parameter {
                scope: fixture.consumer,
                parameter: caller.implementation_parameters[0].id,
            }],
            effect_arguments: vec![],
            requirement_arguments: vec![],
            arguments: vec![local],
            read_scope_arguments: Vec::new(),
        },
    );
    let binding = BindingId::migrate(SEED, 100);
    fixture.source.owners.insert(
        OwnerKey::Binding(binding),
        OwnerRecord::Binding(BindingRecord {
            header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
            name: name("view"),
            kind: BindingKind::OwnedBorrow,
            value: None,
            declared_type: Some(fixture.projection),
        }),
    );
    let body = expression(
        &mut fixture.source,
        103,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(binding),
        },
    );
    let wrapper = expression(
        &mut fixture.source,
        104,
        ExpressionOperation::BorrowCall {
            call,
            binding,
            body,
        },
    );
    replace_function_local_body(&mut fixture.source, fixture.consumer, wrapper);
    Forwarding {
        fixture,
        relay,
        wrapper,
        call,
        alternate,
    }
}

#[test]
fn reference_associated_combined_unused_mapping_checks_types_and_empty_task_kind() {
    let mut fixture = test_support::make_fixture();
    let unit = intern(&mut fixture.source, TypeForm::Unit);
    let body = expression(&mut fixture.source, 200, ExpressionOperation::Unit {});
    replace_function_local_body(&mut fixture.source, fixture.reader, body);
    let reader = function_mut(&mut fixture.source, fixture.reader);
    reader.result.ty = unit;
    reader.result.relation = crate::platform::kernel::ResultRelation::Value;
    let OwnerRecord::Declaration(contract) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.contract.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedContract(contract) = &mut contract.payload else {
        unreachable!()
    };
    contract.methods[0].result.ty = unit;
    contract.methods[0].result.relation = crate::platform::kernel::ResultRelation::Value;
    let proof = derive(&fixture).unwrap();
    assert!(proof.symbolic.contains_key(&fixture.consumer));
    assert!(
        !proof
            .applications
            .keys()
            .any(|app| app.function == fixture.consumer)
    );
    complete(&fixture).unwrap();

    // Types, parameter use, result and empty rows remain identical. The target
    // changes from Pure to Task, so this is a callable-kind mismatch only.
    let pure = function(&fixture.source, fixture.reader).effect.clone();
    effect(
        &mut fixture.source,
        fixture.reader,
        FunctionEffect::Task {
            requirements: vec![],
            effect_parameters: vec![],
        },
    );
    assert_eq!(
        pure.row(),
        function(&fixture.source, fixture.reader).effect.row()
    );
    assert_eq!(
        derive(&fixture).unwrap_err().code,
        "reference_associated_preparation"
    );
    effect(&mut fixture.source, fixture.reader, pure);

    let OwnerRecord::Declaration(supplier) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(
            fixture.implementations[0].declaration,
        ))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(supplier) = &mut supplier.payload else {
        unreachable!()
    };
    supplier.methods[0].type_arguments = vec![fixture.block];
    // Both cell and block are Owned. The mapped parameter still has to equal
    // this exact supplier's member; no concrete consumer requested the mapping.
    assert_eq!(
        derive(&fixture).unwrap_err().code,
        "reference_associated_preparation"
    );
    let OwnerRecord::Declaration(supplier) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(
            fixture.implementations[0].declaration,
        ))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(supplier) = &mut supplier.payload else {
        unreachable!()
    };
    supplier.methods[0].type_arguments = vec![fixture.cell];
    supplier.methods.clear();
    assert_eq!(
        derive(&fixture).unwrap_err().code,
        "reference_associated_preparation"
    );
}

#[test]
fn reference_associated_combined_symbolic_custody_preserves_exact_lexical_witness() {
    let mut case = forwarding();
    let proof = derive(&case.fixture).unwrap();
    assert!(
        !proof
            .applications
            .keys()
            .any(|app| app.function == case.fixture.consumer)
    );
    assert_eq!(
        proof.symbolic[&case.fixture.consumer].result.ty,
        case.fixture.projection
    );
    complete(&case.fixture).unwrap();

    let original =
        function(&case.fixture.source, case.fixture.consumer).implementation_parameters[0].id;
    let caller = function(&case.fixture.source, case.fixture.consumer);
    assert_eq!(
        caller.implementation_parameters[0].self_type,
        caller.implementation_parameters[1].self_type
    );
    let ExpressionOperation::ImplementationCall {
        implementations, ..
    } = operation_mut(&mut case.fixture.source, case.call)
    else {
        unreachable!()
    };
    implementations[0] = ImplementationOperand::Parameter {
        scope: case.fixture.consumer,
        parameter: case.alternate,
    };
    // The alternate remains a legal in-scope Owned witness with equal Self.
    // Its member cannot type the original witness's source-tied argument.
    let proof = derive(&case.fixture).unwrap();
    let error = super::reference_types::associated_schema_fixture(
        &[&case.fixture.source],
        proof,
        &ExecutionControl::uncancelled(),
    )
    .unwrap_err();
    let alternate_projection = TypeObject::new(TypeForm::AssociatedWitnessProjection {
        scope: case.fixture.consumer,
        parameter: case.alternate,
        contract: case.fixture.contract,
        member: case.fixture.member,
    })
    .unwrap();
    let alternate_projection = encode_type_object(&alternate_projection).unwrap().0;
    assert_ne!(alternate_projection, case.fixture.projection);
    assert_raw_refusal(
        error,
        &format!(
            "call argument differs from complete substituted parameter type; caller={}/{}; argument={}; position=0; expected={alternate_projection}; actual={}",
            case.fixture.consumer.package,
            case.fixture.consumer.declaration,
            ExpressionId::migrate(SEED, 101),
            case.fixture.projection,
        ),
    );

    let foreign = function(&case.fixture.source, case.relay).implementation_parameters[0].id;
    let ExpressionOperation::ImplementationCall {
        implementations, ..
    } = operation_mut(&mut case.fixture.source, case.call)
    else {
        unreachable!()
    };
    implementations[0] = ImplementationOperand::Parameter {
        scope: case.relay,
        parameter: foreign,
    };
    assert_eq!(
        derive(&case.fixture).unwrap_err().code,
        "reference_associated_preparation"
    );
    let ExpressionOperation::ImplementationCall {
        implementations, ..
    } = operation_mut(&mut case.fixture.source, case.call)
    else {
        unreachable!()
    };
    implementations[0] = ImplementationOperand::Parameter {
        scope: case.fixture.consumer,
        parameter: original,
    };
    complete(&case.fixture).unwrap();
}

#[test]
fn reference_associated_combined_child_read_keeps_source_identity_without_type_equality() {
    let mut fixture = test_support::make_fixture();
    let product = intern(
        &mut fixture.source,
        TypeForm::OwnedProduct {
            fields: vec![StructuralTypeField {
                name: name("payload"),
                ty: fixture.projection,
            }],
        },
    );
    let parameter = function(&fixture.source, fixture.consumer).parameters[0];
    parameter_mut(&mut fixture.source, parameter).ty = product;
    let source = expression(
        &mut fixture.source,
        600,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(parameter),
        },
    );
    let binding = BindingId::migrate(SEED, 600);
    fixture.source.owners.insert(
        OwnerKey::Binding(binding),
        OwnerRecord::Binding(BindingRecord {
            header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
            name: name("payload_view"),
            kind: BindingKind::OwnedBorrow,
            value: None,
            declared_type: Some(fixture.projection),
        }),
    );
    let result = expression(
        &mut fixture.source,
        601,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(binding),
        },
    );
    let body = expression(
        &mut fixture.source,
        602,
        ExpressionOperation::BorrowOwnedField {
            product_type: product,
            source,
            field: name("payload"),
            binding,
            body: result,
        },
    );
    replace_function_local_body(&mut fixture.source, fixture.consumer, body);
    let proof = derive(&fixture).unwrap();
    assert_eq!(
        proof.symbolic[&fixture.consumer].parameters,
        [(product, ParameterUse::Borrow)]
    );
    assert_eq!(
        proof.symbolic[&fixture.consumer].result.ty,
        fixture.projection
    );
    assert_ne!(product, fixture.projection);
    super::reference_types::associated_schema_fixture(
        &[&fixture.source],
        proof,
        &ExecutionControl::uncancelled(),
    )
    .unwrap();

    let other = ParameterId::migrate(SEED, 600);
    let mut record = parameter_mut(&mut fixture.source, parameter).clone();
    record.header = OwnerHeader::new(OwnerKey::Parameter(other), OwnerKind::Parameter);
    record.name = name("other_source");
    fixture
        .source
        .owners
        .insert(OwnerKey::Parameter(other), OwnerRecord::Parameter(record));
    function_mut(&mut fixture.source, fixture.consumer)
        .parameters
        .push(other);
    *operation_mut(&mut fixture.source, source) = ExpressionOperation::Local {
        value: LocalValueReference::FunctionParameter(other),
    };
    // The selected child and both owning source types are unchanged. The declared
    // source remains the first parameter, so the other parameter cannot lend it.
    let proof = derive(&fixture).unwrap();
    let error = super::reference_types::associated_schema_fixture(
        &[&fixture.source],
        proof,
        &ExecutionControl::uncancelled(),
    )
    .unwrap_err();
    assert_raw_refusal(
        error,
        "ReadFrom body derives from another source or an owning/local value",
    );
}

#[test]
fn reference_associated_combined_unused_body_rejects_untaken_source_consume_and_foreign_epoch() {
    let mut case = forwarding();
    let scalar = intern(&mut case.fixture.source, TypeForm::I64);
    let parameter = function(&case.fixture.source, case.fixture.consumer).parameters[0];
    parameter_mut(&mut case.fixture.source, parameter).use_mode = ParameterUse::Consume;
    let consumer = function_mut(&mut case.fixture.source, case.fixture.consumer);
    consumer.result.ty = scalar;
    consumer.result.relation = crate::platform::kernel::ResultRelation::Value;
    let condition = expression(
        &mut case.fixture.source,
        300,
        ExpressionOperation::Bool { value: false },
    );
    let value = expression(
        &mut case.fixture.source,
        301,
        ExpressionOperation::I64 { value: 7 },
    );
    let false_value = expression(
        &mut case.fixture.source,
        305,
        ExpressionOperation::I64 { value: 7 },
    );
    let branch = expression(
        &mut case.fixture.source,
        302,
        ExpressionOperation::If {
            condition,
            when_true: value,
            when_false: false_value,
        },
    );
    let ExpressionOperation::BorrowCall { body, .. } =
        operation_mut(&mut case.fixture.source, case.wrapper)
    else {
        unreachable!()
    };
    let previous = std::mem::replace(body, branch);
    discard_replaced_local(&mut case.fixture.source, previous);
    complete(&case.fixture).unwrap();

    let source = expression(
        &mut case.fixture.source,
        303,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(parameter),
        },
    );
    let consume = expression(
        &mut case.fixture.source,
        304,
        ExpressionOperation::Sequence {
            items: vec![source, value],
        },
    );
    let ExpressionOperation::If { when_true, .. } = operation_mut(&mut case.fixture.source, branch)
    else {
        unreachable!()
    };
    *when_true = consume;
    let proof = derive(&case.fixture).unwrap();
    let error = super::reference_types::associated_schema_fixture(
        &[&case.fixture.source],
        proof,
        &ExecutionControl::uncancelled(),
    )
    .unwrap_err();
    assert_raw_refusal(
        error,
        "source consume/mutation occurs under a retained read/view guard",
    );

    let ExpressionOperation::If { when_true, .. } = operation_mut(&mut case.fixture.source, branch)
    else {
        unreachable!()
    };
    *when_true = value;
    case.fixture
        .source
        .owners
        .remove(&OwnerKey::Expression(consume));
    case.fixture
        .source
        .owners
        .remove(&OwnerKey::Expression(source));
    case.fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(case.wrapper))
        .unwrap()
        .set_encoding_for_edit(crate::platform::canonical_schema::SCHEMA_EPOCH + 1);
    let error = complete(&case.fixture).unwrap_err();
    assert_epoch_refusal(error);
    case.fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(case.wrapper))
        .unwrap()
        .set_encoding_for_edit(crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION);
    complete(&case.fixture).unwrap();

    let ExpressionOperation::ImplementationCall { arguments, .. } =
        operation_mut(&mut case.fixture.source, case.call)
    else {
        unreachable!()
    };
    let arguments = arguments.clone();
    *operation_mut(&mut case.fixture.source, case.call) = ExpressionOperation::Call {
        function: case.fixture.reader,
        type_arguments: vec![case.fixture.projection],
        effect_arguments: vec![],
        requirement_arguments: vec![],
        arguments,
        read_scope_arguments: Vec::new(),
    };
    // The callee's authored signature is ordinary generic V. Its caller-scoped
    // result is still a projection, and the wrapper consumes that exact result.
    let proof = derive(&case.fixture).unwrap();
    assert!(
        proof.symbolic[&case.fixture.consumer].symbolic_targets[&case.call]
            .signature
            .requires_associated_generation
    );
    super::reference_types::associated_schema_fixture(
        &[&case.fixture.source],
        proof,
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    case.fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(case.wrapper))
        .unwrap()
        .set_encoding_for_edit(crate::platform::canonical_schema::SCHEMA_EPOCH + 1);
    let error = complete(&case.fixture).unwrap_err();
    assert_epoch_refusal(error);
    case.fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(case.wrapper))
        .unwrap()
        .set_encoding_for_edit(crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION);
    complete(&case.fixture).unwrap();
}

fn phantom(
    fixture: &mut test_support::Fixture,
    bound: TypeParameterConstraints,
) -> TypeParameterId {
    let reference = DeclarationReference {
        package: fixture.consumer.package,
        declaration: DeclarationId::migrate(SEED, 400),
    };
    let formal = TypeParameterId::migrate(SEED, 400);
    fixture.source.owners.insert(
        OwnerKey::TypeParameter(formal),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(formal), OwnerKind::TypeParameter),
            declaration: reference.declaration,
            name: name("Unused"),
            constraints: bound,
        }),
    );
    let OwnerRecord::Declaration(owner) =
        fixture.source.owners[&OwnerKey::Declaration(fixture.consumer.declaration)].clone()
    else {
        unreachable!()
    };
    fixture.source.owners.insert(
        OwnerKey::Declaration(reference.declaration),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(reference.declaration),
                OwnerKind::OwnedRecord,
            ),
            module: owner.module,
            name: name("Phantom"),
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::OwnedRecord {
                type_parameters: vec![formal],
                fields: vec![],
            },
        }),
    );
    let OwnerRecord::Declaration(supplier) =
        &fixture.source.owners[&OwnerKey::Declaration(fixture.implementations[2].declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(supplier) = &supplier.payload else {
        unreachable!()
    };
    let actual = supplier.self_type;
    let applied = intern(
        &mut fixture.source,
        TypeForm::Applied {
            declaration: reference,
            arguments: vec![actual],
        },
    );
    let OwnerRecord::Declaration(supplier) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(
            fixture.implementations[2].declaration,
        ))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(supplier) = &mut supplier.payload else {
        unreachable!()
    };
    supplier
        .associated_types
        .iter_mut()
        .find(|binding| binding.member == fixture.spare)
        .unwrap()
        .value = crate::platform::kernel::AssociatedTypeBindingValue::Owned { ty: applied };
    formal
}

#[test]
fn reference_associated_combined_unused_member_phantoms_require_separate_permissions() {
    for bound in [
        TypeParameterConstraints::OwnedTransferable,
        TypeParameterConstraints::OwnedShareable,
    ] {
        let mut fixture = test_support::make_fixture();
        let phantom_formal = phantom(&mut fixture, TypeParameterConstraints::Owned);
        complete(&fixture).unwrap();
        let OwnerRecord::TypeParameter(record) = fixture
            .source
            .owners
            .get_mut(&OwnerKey::TypeParameter(phantom_formal))
            .unwrap()
        else {
            unreachable!()
        };
        assert!(record.constraints.has_owned() && bound.has_owned());
        record.constraints = bound;
        // The argument is still an exact Owned formal and this empty record is
        // still Owned. Only its movement/shared-access promise has strengthened.
        let proof = derive(&fixture).unwrap();
        assert!(proof.selections.is_empty());
        let error = super::reference_types::associated_schema_fixture(
            &[&fixture.source],
            proof,
            &ExecutionControl::uncancelled(),
        )
        .unwrap_err();
        assert_raw_refusal(
            error,
            "complete ordinary metadata/Transfer/Share obligation failed",
        );
        let OwnerRecord::Declaration(supplier) =
            &fixture.source.owners[&OwnerKey::Declaration(fixture.implementations[2].declaration)]
        else {
            unreachable!()
        };
        let DeclarationPayload::OwnedImplementation(supplier) = &supplier.payload else {
            unreachable!()
        };
        let actual = supplier.type_parameters[0];
        let OwnerRecord::TypeParameter(record) = fixture
            .source
            .owners
            .get_mut(&OwnerKey::TypeParameter(actual))
            .unwrap()
        else {
            unreachable!()
        };
        record.constraints = bound;
        complete(&fixture).unwrap();
    }
}

fn parallel_fixture(
    permission: TypeParameterConstraints,
) -> (
    test_support::Fixture,
    DeclarationReference,
    TypeObjectDigest,
) {
    let mut fixture = test_support::make_fixture();
    let bridge = add_relay(&mut fixture);
    let bridge_parameter = function(&fixture.source, bridge).parameters[0];
    parameter_mut(&mut fixture.source, bridge_parameter).use_mode = ParameterUse::Consume;
    let bridge_function = function_mut(&mut fixture.source, bridge);
    bridge_function.result.relation = crate::platform::kernel::ResultRelation::Value;
    effect(
        &mut fixture.source,
        bridge,
        FunctionEffect::Task {
            requirements: vec![],
            effect_parameters: vec![],
        },
    );
    let reader_parameter = function(&fixture.source, fixture.reader).parameters[0];
    parameter_mut(&mut fixture.source, reader_parameter).use_mode = ParameterUse::Consume;
    let reader = function_mut(&mut fixture.source, fixture.reader);
    reader.result.relation = crate::platform::kernel::ResultRelation::Value;
    effect(
        &mut fixture.source,
        fixture.reader,
        FunctionEffect::Task {
            requirements: vec![],
            effect_parameters: vec![],
        },
    );
    let OwnerRecord::Declaration(contract) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.contract.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedContract(contract) = &mut contract.payload else {
        unreachable!()
    };
    contract.methods[0].parameters[0].use_mode = ParameterUse::Consume;
    contract.methods[0].result.relation = crate::platform::kernel::ResultRelation::Value;
    contract.methods[0].effect = FunctionEffect::Task {
        requirements: vec![],
        effect_parameters: vec![],
    };
    let scalar = intern(&mut fixture.source, TypeForm::I64);
    let OwnerRecord::Declaration(supplier) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(
            fixture.implementations[1].declaration,
        ))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(supplier) = &mut supplier.payload else {
        unreachable!()
    };
    // The second selection returns its own Self formal; the first returns Cell.
    // Both contracts promise only Owned for that member.
    let supplier_self = supplier.self_type;
    supplier.methods[0].type_arguments = vec![supplier_self];
    supplier
        .associated_types
        .iter_mut()
        .find(|binding| binding.member == fixture.member)
        .unwrap()
        .value = crate::platform::kernel::AssociatedTypeBindingValue::Owned { ty: supplier_self };

    let parent = DeclarationReference {
        package: fixture.consumer.package,
        declaration: DeclarationId::migrate(SEED, 500),
    };
    let formal = TypeParameterId::migrate(SEED, 500);
    fixture.source.owners.insert(
        OwnerKey::TypeParameter(formal),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(formal), OwnerKind::TypeParameter),
            declaration: parent.declaration,
            name: name("ParentSelf"),
            constraints: permission,
        }),
    );
    let output = intern(
        &mut fixture.source,
        TypeForm::TypeParameter { parameter: formal },
    );
    let parameters = [
        ParameterId::migrate(SEED, 500),
        ParameterId::migrate(SEED, 501),
    ];
    for (id, ty) in parameters.into_iter().zip([fixture.cell, output]) {
        fixture.source.owners.insert(
            OwnerKey::Parameter(id),
            OwnerRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(id), OwnerKind::Parameter),
                parent: ParameterParent::Function(parent.declaration),
                name: name(if id == parameters[0] { "left" } else { "right" }),
                ty,
                use_mode: ParameterUse::Consume,
                resource_requirement: None,
            }),
        );
    }
    let mut children = Vec::new();
    for (index, parameter) in parameters.into_iter().enumerate() {
        let local = expression(
            &mut fixture.source,
            500 + index as u64,
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(parameter),
            },
        );
        let witness = ImplementationOperand::Concrete {
            implementation: fixture.implementations[index],
            type_arguments: vec![output],
            implementations: vec![],
        };
        children.push(expression(
            &mut fixture.source,
            502 + index as u64,
            ExpressionOperation::ImplementationCall {
                function: bridge,
                type_arguments: vec![output],
                implementations: vec![witness],
                effect_arguments: vec![],
                requirement_arguments: vec![],
                arguments: vec![local],
                read_scope_arguments: Vec::new(),
            },
        ));
    }
    let lanes = expression(
        &mut fixture.source,
        504,
        ExpressionOperation::Parallel {
            left: children[0],
            right: children[1],
        },
    );
    let value = expression(
        &mut fixture.source,
        505,
        ExpressionOperation::I64 { value: 9 },
    );
    let body = expression(
        &mut fixture.source,
        506,
        ExpressionOperation::Sequence {
            items: vec![lanes, value],
        },
    );
    let mut parent_function = function(&fixture.source, fixture.reader).clone();
    parent_function.type_parameters = vec![formal];
    parent_function.implementation_parameters.clear();
    parent_function.parameters = parameters.to_vec();
    parent_function.result.ty = scalar;
    parent_function.result.relation = crate::platform::kernel::ResultRelation::Value;
    parent_function.body = body;
    assert_eq!(
        declared_function(
            &mut fixture.source,
            fixture.reader,
            500,
            "parallel_parent",
            parent_function
        ),
        parent
    );
    (fixture, parent, output)
}

#[test]
fn reference_associated_combined_parallel_permissions_distinguish_equal_self_outputs() {
    let (fixture, _, _) = parallel_fixture(TypeParameterConstraints::OwnedTransferable);
    let schema = complete(&fixture).unwrap();
    assert!(schema.is_owned_type(fixture.cell));
    assert!(schema.transferable_types.contains(&fixture.cell));
    let (fixture, parent, output) = parallel_fixture(TypeParameterConstraints::Owned);
    let proof = derive(&fixture).unwrap();
    let source = &proof.symbolic[&parent];
    let left = &source.symbolic_targets[&ExpressionId::migrate(SEED, 502)];
    let right = &source.symbolic_targets[&ExpressionId::migrate(SEED, 503)];
    assert_eq!(
        [left.signature.result.ty, right.signature.result.ty],
        [fixture.cell, output]
    );
    assert_eq!(
        left.signature.parameters,
        [(fixture.cell, ParameterUse::Consume)]
    );
    assert_eq!(
        right.signature.parameters,
        [(output, ParameterUse::Consume)]
    );
    for target in [left, right] {
        assert_eq!(target.types, [output]);
        assert_eq!(target.witnesses.len(), 1);
        assert_eq!(target.witnesses[0].self_type, output);
    }
    for (target, expected) in [left, right].into_iter().zip(fixture.implementations) {
        let associated_preparation::SymbolicSelectionSource::Concrete { implementation, .. } =
            &target.witnesses[0].source
        else {
            unreachable!()
        };
        assert_eq!(*implementation, expected);
    }
    // The exact same type roots and witness selections passed Owned category
    // admission. The weaker formal has no movement promise for the right result.
    let error = super::reference_types::associated_schema_fixture(
        &[&fixture.source],
        proof,
        &ExecutionControl::uncancelled(),
    )
    .unwrap_err();
    assert_raw_refusal(
        error,
        "joined argument lacks independent complete ReadShareable/Shareable/Transferable closure",
    );
}

#[test]
fn reference_associated_combined_completion_budget_and_cancellation_keep_fresh_authority() {
    let case = forwarding();
    let healthy = complete(&case.fixture).unwrap();
    let steps = healthy.type_derivation_steps;
    let bytes = healthy.type_metadata_bytes;
    let proof = derive(&case.fixture).unwrap();
    assert!(steps > proof.steps && bytes > proof.bytes);
    let exact = super::reference_types::associated_schema_fixture_with_limits(
        &[&case.fixture.source],
        proof,
        Limits {
            steps: Some(steps),
            bytes: Some(bytes),
        },
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert_eq!(
        (exact.type_derivation_steps, exact.type_metadata_bytes),
        (healthy.type_derivation_steps, healthy.type_metadata_bytes)
    );
    for limits in [
        Limits {
            steps: Some(steps - 1),
            bytes: Some(bytes),
        },
        Limits {
            steps: Some(steps),
            bytes: Some(bytes - 1),
        },
    ] {
        let proof = derive(&case.fixture).unwrap();
        let error = super::reference_types::associated_schema_fixture_with_limits(
            &[&case.fixture.source],
            proof,
            limits,
            &ExecutionControl::uncancelled(),
        )
        .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Resource);
    }
    for checks in [0, 20] {
        let proof = derive(&case.fixture).unwrap();
        let error = super::reference_types::associated_schema_fixture(
            &[&case.fixture.source],
            proof,
            &ExecutionControl::cancel_after_checks(checks),
        )
        .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Cancelled);
    }
    // Find the actual first successful checkpoint allowance for this immutable
    // canonical fixture. No guessed count is used to label a permission phase.
    let mut rejected = 0;
    let mut accepted = 1;
    loop {
        match complete_after_checks(&case.fixture, accepted) {
            Ok(schema) => {
                assert_eq!(
                    (schema.type_derivation_steps, schema.type_metadata_bytes),
                    (healthy.type_derivation_steps, healthy.type_metadata_bytes)
                );
                break;
            }
            Err(error) => {
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                rejected = accepted;
                accepted = accepted
                    .checked_mul(2)
                    .expect("finite completion checkpoint inventory");
            }
        }
    }
    while accepted - rejected > 1 {
        let middle = rejected + (accepted - rejected) / 2;
        match complete_after_checks(&case.fixture, middle) {
            Ok(_) => accepted = middle,
            Err(error) => {
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                rejected = middle;
            }
        }
    }
    let error = complete_after_checks(&case.fixture, accepted - 1).unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Cancelled);
    assert!(
        error.message.contains("associated permission discharge"),
        "{}",
        error.message
    );
    let exact_checks = complete_after_checks(&case.fixture, accepted).unwrap();
    assert_eq!(
        (
            exact_checks.type_derivation_steps,
            exact_checks.type_metadata_bytes
        ),
        (healthy.type_derivation_steps, healthy.type_metadata_bytes)
    );
    let retry = complete(&case.fixture).unwrap();
    assert_eq!(
        (retry.type_derivation_steps, retry.type_metadata_bytes),
        (healthy.type_derivation_steps, healthy.type_metadata_bytes)
    );
    // The cfg(test) witness entry still bypasses full production source admission. Its
    // completed metadata must agree with the independently exercised core above; it is no seal.
    let completed =
        NormalizedReferenceSchema::independently_admit_witness_source(&case.fixture.source)
            .unwrap();
    assert!(completed.associated_proof().is_some());
    assert_eq!(completed.types, healthy.types);
    assert_eq!(completed.ordinary_types, healthy.ordinary_types);
    assert_eq!(completed.transferable_types, healthy.transferable_types);
    assert_eq!(completed.shareable_types, healthy.shareable_types);
    assert_eq!(completed.type_metadata_bytes, healthy.type_metadata_bytes);
}

#[test]
fn reference_associated_combined_nominal_context_retains_exact_formals_through_permission_discharge()
 {
    use crate::platform::kernel::FieldRecord;
    use crate::platform::semantic_id::FieldId;

    // Private raw canonical fixture: the genuine supplier check exposed this path,
    // but this targeted schema regression grants no source or artifact admission seal.
    let mut fixture = test_support::make_fixture();
    let OwnerRecord::Declaration(template) =
        &fixture.source.owners[&OwnerKey::Declaration(fixture.consumer.declaration)]
    else {
        unreachable!()
    };
    let module = template.module;
    let records = [500, 501].map(|ordinal| DeclarationReference {
        package: fixture.consumer.package,
        declaration: DeclarationId::migrate(SEED, ordinal),
    });
    let formals = [500, 501].map(|ordinal| TypeParameterId::migrate(SEED, ordinal));
    let fields = [500, 501].map(|ordinal| FieldId::migrate(SEED, ordinal));
    let mut formal_types = Vec::new();
    let mut field_types = Vec::new();
    for index in 0..2 {
        let formal_type = intern(
            &mut fixture.source,
            TypeForm::TypeParameter {
                parameter: formals[index],
            },
        );
        formal_types.push(formal_type);
        let item = if index == 0 {
            formal_type
        } else {
            intern(
                &mut fixture.source,
                TypeForm::Applied {
                    declaration: records[0],
                    arguments: vec![formal_type],
                },
            )
        };
        let field_type = intern(&mut fixture.source, TypeForm::OwnedSequence { item });
        field_types.push(field_type);
        fixture.source.owners.insert(
            OwnerKey::TypeParameter(formals[index]),
            OwnerRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::TypeParameter(formals[index]),
                    OwnerKind::TypeParameter,
                ),
                declaration: records[index].declaration,
                name: name("T"),
                constraints: TypeParameterConstraints::Transferable,
            }),
        );
        fixture.source.owners.insert(
            OwnerKey::Field(fields[index]),
            OwnerRecord::Field(FieldRecord {
                header: OwnerHeader::new(OwnerKey::Field(fields[index]), OwnerKind::Field),
                declaration: records[index].declaration,
                name: name("items"),
                ty: field_type,
            }),
        );
        fixture.source.owners.insert(
            OwnerKey::Declaration(records[index].declaration),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(
                    OwnerKey::Declaration(records[index].declaration),
                    OwnerKind::OwnedRecord,
                ),
                module,
                name: name(if index == 0 { "Inner" } else { "Outer" }),
                visibility: DeclarationVisibility::Private,
                payload: DeclarationPayload::OwnedRecord {
                    type_parameters: vec![formals[index]],
                    fields: vec![fields[index]],
                },
            }),
        );
    }
    let context = (
        records[1].package,
        OwnerKey::Declaration(records[1].declaration),
    );
    // Current nominal reader controls provide no transport or artifact admission.
    let generation = crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
    for key in [
        OwnerKey::Declaration(records[0].declaration),
        OwnerKey::Declaration(records[1].declaration),
        OwnerKey::TypeParameter(formals[0]),
        OwnerKey::TypeParameter(formals[1]),
        OwnerKey::Field(fields[0]),
        OwnerKey::Field(fields[1]),
    ] {
        fixture
            .source
            .owners
            .get_mut(&key)
            .unwrap()
            .set_encoding_for_edit(generation);
    }
    let proof = derive(&fixture).unwrap();
    let nominal = &proof.executables[&context];
    assert_eq!(nominal.occurrences[&context.1], generation);
    assert_eq!(nominal.occurrences[&OwnerKey::Field(fields[1])], generation);
    assert_eq!(
        nominal.roots[&(OwnerKey::Field(fields[1]), field_types[1])],
        field_types[1]
    );

    let proof = derive(&fixture).unwrap();
    let healthy = super::reference_types::associated_schema_fixture(
        &[&fixture.source],
        proof,
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert!(healthy.types.contains_key(&field_types[1]));
    assert!(
        healthy
            .associated_proof()
            .unwrap()
            .executables
            .contains_key(&context)
    );

    // Equal Owned category cannot supply the inner formal's separate movement
    // promise when the outer formal only promises ownership.
    for (formal, constraints) in [
        (formals[0], TypeParameterConstraints::OwnedTransferable),
        (formals[1], TypeParameterConstraints::Owned),
    ] {
        let OwnerRecord::TypeParameter(parameter) = fixture
            .source
            .owners
            .get_mut(&OwnerKey::TypeParameter(formal))
            .unwrap()
        else {
            unreachable!()
        };
        parameter.constraints = constraints;
    }
    let error = complete(&fixture).unwrap_err();
    assert_raw_refusal(
        error,
        "complete ordinary metadata/Transfer/Share obligation failed",
    );
    for formal in formals {
        let OwnerRecord::TypeParameter(parameter) = fixture
            .source
            .owners
            .get_mut(&OwnerKey::TypeParameter(formal))
            .unwrap()
        else {
            unreachable!()
        };
        parameter.constraints = TypeParameterConstraints::Transferable;
    }

    // Equal ordinary constraints cannot substitute the inner declaration's formal
    // for the outer declaration's actual lexical formal.
    let foreign_item = intern(
        &mut fixture.source,
        TypeForm::Applied {
            declaration: records[0],
            arguments: vec![formal_types[0]],
        },
    );
    let foreign_field = intern(
        &mut fixture.source,
        TypeForm::OwnedSequence { item: foreign_item },
    );
    let OwnerRecord::Field(field) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Field(fields[1]))
        .unwrap()
    else {
        unreachable!()
    };
    field.ty = foreign_field;
    assert_eq!(
        derive(&fixture).unwrap_err().code,
        "reference_associated_preparation"
    );
    let OwnerRecord::Field(field) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Field(fields[1]))
        .unwrap()
    else {
        unreachable!()
    };
    field.ty = field_types[1];
    fixture.source.types.remove(&foreign_field);
    fixture.source.types.remove(&foreign_item);
    let retry = complete(&fixture).unwrap();
    assert_eq!(retry.types, healthy.types);
    assert_eq!(
        (retry.type_derivation_steps, retry.type_metadata_bytes),
        (healthy.type_derivation_steps, healthy.type_metadata_bytes)
    );
}

#[test]
fn reference_associated_combined_unattached_local_remains_rejected() {
    let mut fixture = test_support::make_fixture();
    complete(&fixture).unwrap();
    let parameter = function(&fixture.source, fixture.consumer).parameters[0];
    expression(
        &mut fixture.source,
        700,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(parameter),
        },
    );
    let error = complete(&fixture).unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
    assert_eq!(error.code, "reference_read_view_admission");
    assert_eq!(
        error.message,
        "unattached expression syntax has no admitted callable context"
    );
}
