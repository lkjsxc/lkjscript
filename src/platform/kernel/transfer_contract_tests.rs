//! Production admission of reusable symbolic structured groups and their bounds.
use super::*;
use std::collections::BTreeMap;

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

#[test]
fn transfer_symbolic_sequences_require_element_bounds_before_concrete_applications() {
    let sequence = "(owned-sequence O)";
    let source = GROUP
        .replace(
            "(parameter create value (type O) (use consume))",
            &format!("(parameter create value (type {sequence}) (use consume))"),
        )
        .replace("(returns O)", &format!("(returns {sequence})"))
        .replace("(field left O)", &format!("(field left {sequence})"));
    let snapshot = author(&source).unwrap();
    let scope = named(&snapshot, "group");
    let child = snapshot
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(e) => match e.operation {
                ExpressionOperation::Parallel { left, .. } => Some(left),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    let call = parallel::admit_call(&snapshot, child, Some(scope)).unwrap();
    assert!(call.result_owned);
    assert!(matches!(
        snapshot.types[&call.result].form,
        TypeForm::OwnedSequence { .. }
    ));
    assert!(transfer::admit(&snapshot, call.result, None).is_err());
    assert!(!owned_contract::ordinary_transfer(&snapshot, call.result, Some(scope)).unwrap());

    let TypeForm::OwnedSequence { item } = snapshot.types[&call.result].form else {
        unreachable!()
    };
    let TypeForm::TypeParameter { parameter } = snapshot.types[&item].form else {
        unreachable!()
    };
    let mut concrete = snapshot.clone();
    let object = TypeObject::new(TypeForm::ByteBuffer).unwrap();
    let buffer = encode_type_object(&object).unwrap().0;
    concrete.types.insert(buffer, object);
    let mut applied = parallel_types::AppliedTypes::new(&concrete);
    let closed = applied
        .substitute(call.result, &BTreeMap::from([(parameter, buffer)]), 0)
        .unwrap();
    assert_eq!(
        applied.type_object(closed).unwrap().unwrap().form,
        TypeForm::OwnedSequence { item: buffer }
    );
    assert!(transfer::admit(&applied, closed, None).unwrap());

    let mut weakened = snapshot.clone();
    for owner in weakened.owners.values_mut() {
        if let OwnerRecord::TypeParameter(p) = owner
            && p.declaration == scope
            && p.name.as_str() == "O"
        {
            p.constraints = TypeParameterConstraints::Owned;
        }
    }
    assert!(parallel::admit_call(&weakened, child, Some(scope)).is_err());
}

#[test]
fn transfer_closed_sequence_witnesses_preserve_exact_self_type() {
    let source = r#"declarations.begin
(units (module create sequence-witness
  (owned-contract create Finish (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_98000000000000000000000000000001 finish
      (parameters (Self consume)) (returns I64)))
  (function create finish-cells (visibility public) (effect pure)
    (parameter create values (type (owned-sequence OwnedI64Cell)) (use consume))
    (returns I64) (body (i64 7)))
  (owned-implementation create Cells (visibility public) (contract Finish)
    (self (owned-sequence OwnedI64Cell))
    (method method_98000000000000000000000000000001 finish-cells))))
declarations.end
"#;
    let snapshot = author(source).unwrap();
    let implementation = snapshot
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Declaration(d) => match &d.payload {
                DeclarationPayload::OwnedImplementation(i) => Some(i),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    owned_contract::validate_implementation(&snapshot, implementation).unwrap();
    assert!(transfer::admit(&snapshot, implementation.self_type, None).unwrap());
    assert!(!owned_contract::ordinary_closed(&snapshot, implementation.self_type).unwrap());
    let mismatch = source.replace(
        "(self (owned-sequence OwnedI64Cell))",
        "(self (owned-sequence ByteBuffer))",
    );
    assert!(author(&mismatch).is_err());
}

#[test]
fn transfer_rejects_owned_sequence_hidden_in_phantom_ordinary_arguments() {
    let source = r#"declarations.begin
(units (module create phantom-sequence
  (record create Marker (visibility public)
    (type-parameter create T) (field create tag (type I64)))
  (function create unused (visibility public) (effect pure)
    (parameter create marker (type (Marker Bytes)))
    (returns Unit) (body (unit)))))
declarations.end
"#;
    let mut snapshot = author(source).unwrap();
    let declaration = snapshot
        .types
        .values()
        .find_map(|object| match object.form {
            TypeForm::Applied { declaration, .. } => Some(declaration),
            _ => None,
        })
        .unwrap();
    let mut insert_type = |form| {
        let object = TypeObject::new(form).unwrap();
        let digest = encode_type_object(&object).unwrap().0;
        snapshot.types.insert(digest, object);
        digest
    };
    let item = insert_type(TypeForm::ByteBuffer);
    let sequence = insert_type(TypeForm::OwnedSequence { item });
    let phantom = insert_type(TypeForm::Applied {
        declaration,
        arguments: vec![sequence],
    });
    assert!(transfer::admit(&snapshot, sequence, None).unwrap());
    assert!(transfer::admit(&snapshot, phantom, None).is_err());
    assert!(!owned_contract::ordinary_closed(&snapshot, phantom).unwrap());
}
