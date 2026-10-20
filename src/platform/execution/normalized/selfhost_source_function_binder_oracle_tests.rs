//! This fixture must pass the independent Rust kernel before becoming a positive
//! selfhost carrier control. No native or source-admission claim is made here.
use super::*;

#[test]
fn selfhost_source_pure_function_binder_fixture_has_independent_kernel_acceptance() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-function-binders.lkjc"
    ))
    .unwrap();
    kernel::validate_full(&source).unwrap();
    for name in ["identity", "independent"] {
        let declaration = source
            .owners
            .values()
            .find_map(|owner| match owner {
                OwnerRecord::Declaration(record) if record.name.as_str() == name => Some(record),
                _ => None,
            })
            .unwrap();
        let OwnerKey::Declaration(id) = declaration.header.owner else {
            panic!("function declaration identity");
        };
        let kernel::DeclarationPayload::Function(function) = &declaration.payload else {
            panic!("function payload");
        };
        assert_eq!(function.requirement_parameters.len(), 1);
        assert_eq!(function.effect_parameters.len(), 1);
        assert_eq!(function.type_parameters.len(), 1);
        assert_eq!(function.parameters.len(), 1);
        assert!(function.implementation_parameters.is_empty());
        assert!(function.read_scope_parameters.is_empty());
        assert!(function.read_scope_sources.is_empty());
        assert_eq!(function.effect, kernel::FunctionEffect::Pure);

        let requirement =
            &source.owners[&OwnerKey::RequirementParameter(function.requirement_parameters[0])];
        let OwnerRecord::RequirementParameter(requirement) = requirement else {
            panic!("requirement binder");
        };
        assert_eq!(requirement.declaration, id);
        assert_eq!(requirement.name.as_str(), "R");
        assert_eq!(requirement.constraint.operations.len(), 1);
        let interface =
            &source.owners[&OwnerKey::Declaration(requirement.constraint.interface.declaration)];
        let OwnerRecord::Declaration(interface) = interface else {
            panic!("constraint interface");
        };
        assert_eq!(interface.name.as_str(), "Clock");
        assert_eq!(
            requirement.constraint.interface.package,
            source.root.package_id
        );

        let effect = &source.owners[&OwnerKey::EffectParameter(function.effect_parameters[0])];
        let OwnerRecord::EffectParameter(effect) = effect else {
            panic!("effect binder");
        };
        assert_eq!(effect.declaration, id);
        assert_eq!(effect.name.as_str(), "E");

        let body = &source.owners[&OwnerKey::Expression(function.body)];
        let OwnerRecord::Expression(body) = body else {
            panic!("body");
        };
        assert!(matches!(
            body.operation,
            kernel::ExpressionOperation::Local {
                value: kernel::LocalValueReference::FunctionParameter(parameter)
            } if parameter == function.parameters[0]
        ));
    }
}
