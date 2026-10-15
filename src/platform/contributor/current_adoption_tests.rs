//! Current maintained adoption, independent of migration inventories.
use super::*;

fn function(
    source: &KernelSnapshot,
    declaration: DeclarationId,
) -> (
    &super::super::kernel::DeclarationRecord,
    &super::super::kernel::FunctionDeclaration,
) {
    let OwnerRecord::Declaration(record) = &source.owners[&OwnerKey::Declaration(declaration)]
    else {
        panic!("current maintained declaration absent")
    };
    let DeclarationPayload::Function(function) = &record.payload else {
        panic!("current maintained declaration is not a function")
    };
    (record, function)
}

#[test]
fn maintained_queue_worker_borrows_payload_before_consuming_its_exact_lease() {
    use crate::platform::kernel::{DeclarationVisibility, ParameterUse};
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("applications/lkjournal");
    let before = std::fs::read(path.join("HEAD")).unwrap();
    let source = GraphRepository::open(&path)
        .unwrap()
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    let owner: DeclarationId = "decl_7f443401f4946c55fa239c5430e8ad93".parse().unwrap();
    let borrower: DeclarationId = "decl_08eec4f6b013dea79cfed578f85b7db9".parse().unwrap();
    let (owner_record, owner_function) = function(&source, owner);
    let (borrow_record, borrow_function) = function(&source, borrower);
    assert_eq!(owner_record.name.as_str(), "process-lease");
    assert_eq!(borrow_record.name.as_str(), "lease-payload");
    assert_eq!(borrow_record.visibility, DeclarationVisibility::Private);
    assert_eq!(borrow_function.parameters.len(), 1);
    assert_eq!(borrow_function.effect, owner_function.effect);
    let lease = *owner_function.parameters.last().unwrap();
    let OwnerRecord::Parameter(owned_parameter) = &source.owners[&OwnerKey::Parameter(lease)]
    else {
        panic!("current worker lease parameter absent")
    };
    let OwnerRecord::Parameter(borrowed_parameter) =
        &source.owners[&OwnerKey::Parameter(borrow_function.parameters[0])]
    else {
        panic!("current payload borrow parameter absent")
    };
    assert_eq!(owned_parameter.use_mode, ParameterUse::Consume);
    assert_eq!(borrowed_parameter.use_mode, ParameterUse::Borrow);
    assert_eq!(borrowed_parameter.ty, owned_parameter.ty);
    assert_eq!(
        borrowed_parameter.resource_requirement,
        owned_parameter.resource_requirement
    );
    let OwnerRecord::Expression(body) = &source.owners[&OwnerKey::Expression(owner_function.body)]
    else {
        panic!("current worker body absent")
    };
    let ExpressionOperation::Let { bindings, .. } = &body.operation else {
        panic!("current worker must bind payload before renewing its lease")
    };
    assert_eq!(bindings.len(), 2);
    for (position, binding) in bindings.iter().enumerate() {
        let OwnerRecord::Binding(binding) = &source.owners[&OwnerKey::Binding(*binding)] else {
            panic!("current worker ordered binding absent")
        };
        let OwnerRecord::Expression(expression) =
            &source.owners[&OwnerKey::Expression(binding.value.unwrap())]
        else {
            panic!("current worker binding expression absent")
        };
        let arguments = if position == 0 {
            let ExpressionOperation::Call {
                function,
                arguments,
                ..
            } = &expression.operation
            else {
                panic!("payload must be read through its borrowed helper")
            };
            assert_eq!(function.package, source.root.package_id);
            assert_eq!(function.declaration, borrower);
            assert_eq!(arguments.len(), 1);
            arguments
        } else {
            let ExpressionOperation::CapabilityCall { arguments, .. } = &expression.operation
            else {
                panic!("lease renewal must follow completion of the borrowed helper")
            };
            arguments
        };
        let OwnerRecord::Expression(argument) = &source.owners[&OwnerKey::Expression(arguments[0])]
        else {
            panic!("current worker exact lease argument absent")
        };
        assert!(matches!(argument.operation,
            ExpressionOperation::Local { value: LocalValueReference::FunctionParameter(parameter) }
                if parameter == lease));
    }
    assert_eq!(std::fs::read(path.join("HEAD")).unwrap(), before);
}
