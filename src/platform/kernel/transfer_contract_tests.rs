//! Production admission of reusable symbolic structured groups and their bounds.
use super::*;

const GROUP: &str = r#"declarations.begin
(units (module create transfer-proof
  (function create data-child (visibility private) (effect (task))
    (type-parameter create T)
    (parameter create value (type T)) (returns T) (body (local value)))
  (function create owner-child (visibility private) (effect (task))
    (type-parameter create O (constraint owned))
    (parameter create value (type O) (use consume)) (returns O) (body (local value)))
  (function create group (visibility public) (effect (task))
    (type-parameter create T (constraint transferable))
    (type-parameter create O (constraint owned transferable))
    (parameter create data (type T))
    (parameter create value (type O) (use consume))
    (returns (owned-product (field left O) (field right T)))
    (body (parallel (call owner-child (types O) (local value))
                    (call data-child (types T) (local data)))))))
declarations.end
"#;

fn author(source: &str) -> Result<KernelSnapshot, String> {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(source)
}

fn named(snapshot: &KernelSnapshot, name: &str) -> crate::platform::semantic_id::DeclarationId {
    snapshot
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Declaration(d) if d.name.as_str() == name => match d.header.owner {
                OwnerKey::Declaration(id) => Some(id),
                _ => None,
            },
            _ => None,
        })
        .unwrap()
}

#[test]
fn transfer_symbolic_group_admits_mixed_metadata_before_any_application_exists() {
    let snapshot = author(GROUP).unwrap();
    let scope = named(&snapshot, "group");
    let (left, right) = snapshot
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(e) => match e.operation {
                ExpressionOperation::Parallel { left, right } => Some((left, right)),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    assert!(
        parallel::admit_call(&snapshot, left, Some(scope))
            .unwrap()
            .result_owned
    );
    assert!(
        !parallel::admit_call(&snapshot, right, Some(scope))
            .unwrap()
            .result_owned
    );
    assert!(parallel::admit_call(&snapshot, left, None).is_err());
    assert!(parallel::admit_call(&snapshot, right, None).is_err());
    let OwnerRecord::Declaration(declaration) = &snapshot.owners[&OwnerKey::Declaration(scope)]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &declaration.payload else {
        unreachable!()
    };
    assert!(transfer::admit(&snapshot, function.result, Some(scope)).unwrap());
    assert!(owned_product::validate(&snapshot, function.result, None).is_err());
}

#[test]
fn transfer_symbolic_groups_reject_missing_bounds_even_without_concrete_callers() {
    let snapshot = author(GROUP).unwrap();
    let scope = named(&snapshot, "group");
    for (name, constraint) in [
        ("T", TypeParameterConstraints::None),
        ("T", TypeParameterConstraints::CaptureSafe),
        ("O", TypeParameterConstraints::Owned),
    ] {
        let mut weakened = snapshot.clone();
        for owner in weakened.owners.values_mut() {
            if let OwnerRecord::TypeParameter(p) = owner
                && p.declaration == scope
                && p.name.as_str() == name
            {
                p.constraints = constraint;
            }
        }
        let errors = validate_full(&weakened).unwrap_err();
        assert!(
            errors.iter().any(|e| matches!(
                e.code.as_str(),
                "kernel_parallel_call" | "kernel_owned_product" | "kernel_transfer_constraint"
            )),
            "missing {name} transfer bound: {errors:?}"
        );
    }
}

#[test]
fn transfer_ordinary_bound_entails_capture_safe_without_admitting_callable_transfer() {
    let source = r#"declarations.begin
(units (module create capture-proof
  (function create capture-helper (visibility private) (effect pure)
    (type-parameter create T (constraint capture-safe))
    (parameter create value (type T)) (returns T) (body (local value)))
  (function create forward (visibility public) (effect pure)
    (type-parameter create T (constraint transferable))
    (parameter create value (type T)) (returns T)
    (body (call capture-helper (types T) (local value))))))
declarations.end
"#;
    let snapshot = author(source).unwrap();
    let scope = named(&snapshot, "forward");
    let OwnerRecord::Declaration(declaration) = &snapshot.owners[&OwnerKey::Declaration(scope)]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &declaration.payload else {
        unreachable!()
    };
    assert!(!transfer::admit(&snapshot, function.result, Some(scope)).unwrap());
    let weakened = source.replace("(constraint transferable)", "(constraint capture-safe)");
    let snapshot = author(&weakened).unwrap();
    let scope = named(&snapshot, "forward");
    let OwnerRecord::Declaration(declaration) = &snapshot.owners[&OwnerKey::Declaration(scope)]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &declaration.payload else {
        unreachable!()
    };
    assert!(transfer::admit(&snapshot, function.result, Some(scope)).is_err());
}

#[test]
fn transfer_closed_implementation_self_needs_no_function_assumptions() {
    let snapshot = author(include_str!(
        "../../../tests/fixtures/owned-choices-witness.lkjc"
    ))
    .unwrap();
    let mut checked = 0;
    for owner in snapshot.owners.values() {
        if let OwnerRecord::Declaration(declaration) = owner
            && let DeclarationPayload::OwnedImplementation(implementation) = &declaration.payload
            && let OwnerKey::Declaration(scope) = declaration.header.owner
        {
            assert!(transfer::admit(&snapshot, implementation.self_type, Some(scope)).unwrap());
            checked += 1;
        }
    }
    assert_eq!(checked, 2);
}
