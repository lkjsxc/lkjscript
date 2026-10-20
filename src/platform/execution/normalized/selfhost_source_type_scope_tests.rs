//! Context-sensitive TypeParameter admission and independently specified DAG controls.
//! The derived graph probes exercise cycles that cannot be manufactured by rehashing
//! mutually content-addressed TypeRecords. Carrier fixtures reach the new layer.
use super::*;
use crate::platform::semantic_id::DeclarationId;

fn fixture() -> KernelSnapshot {
    byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-type-scope.lkjc"
    ))
    .unwrap()
}

fn declaration(source: &KernelSnapshot, name: &str) -> DeclarationId {
    source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if record.name.as_str() == name =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap()
}

fn field_key(source: &KernelSnapshot, parent: &str, name: &str) -> OwnerKey {
    let parent = declaration(source, parent);
    source
        .owners
        .iter()
        .find_map(|(key, owner)| match owner {
            OwnerRecord::Field(record)
                if record.declaration == parent && record.name.as_str() == name =>
            {
                Some(*key)
            }
            _ => None,
        })
        .unwrap()
}

fn field_type(source: &KernelSnapshot, parent: &str, name: &str) -> TypeObjectDigest {
    source.owners[&field_key(source, parent, name)].type_roots()[0]
}

fn rebind(source: &mut KernelSnapshot) {
    let entries = source
        .owners
        .iter()
        .map(|(key, record)| {
            let (object, _) = kernel::encode_owner(record).unwrap();
            (
                kernel::owner_map_key(*key).to_vec(),
                kernel::encode_owner_binding(&kernel::OwnerBinding {
                    kind: record.kind(),
                    object,
                }),
            )
        })
        .collect::<Vec<_>>();
    let content = crate::platform::persistent_map::MapContentRoot::from_sorted(entries).unwrap();
    source.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        source.root.owners.page(),
        source.owners.len() as u64,
        content.digest(),
    );
}

fn expect(source: &KernelSnapshot, code: i64) {
    let bytes = graph::packet(source);
    if code == 17 {
        let mut oracle = source.clone();
        rebind(&mut oracle);
        let errors = kernel::validate_full(&oracle).unwrap_err();
        assert!(
            errors.iter().any(|error| {
                error.class == crate::platform::DiagnosticClass::Semantic
                    && error.code == "kernel_full_type_parameter_scope"
            }),
            "{errors:?}"
        );
        assert_eq!(
            result("read-namespace", &bytes),
            (0, bytes[48..80].to_vec()),
            "scope mutation must pass namespace and every earlier layer"
        );
        emitted_result("read-namespace", &bytes, 0);
    }
    assert_eq!(
        result("read-type-scope", &bytes),
        (
            code,
            if code == 0 {
                bytes[48..80].to_vec()
            } else {
                vec![]
            }
        )
    );
    emitted_result("read-type-scope", &bytes, code);
}

#[test]
fn selfhost_source_type_scope_admits_independent_generic_uses_and_local_names() {
    let source = fixture();
    expect(&source, 0);
    let nominal = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-nominal.lkjc"
    ))
    .unwrap();
    expect(&nominal, 0);
    let mut shadowed = source;
    for owner in shadowed.owners.values_mut() {
        if let OwnerRecord::TypeParameter(record) = owner {
            record.name = kernel::Name::new("same").unwrap();
        }
    }
    rebind(&mut shadowed);
    kernel::validate_full(&shadowed).unwrap();
    expect(&shadowed, 0);
}

#[test]
fn selfhost_source_type_scope_rejects_shared_descriptors_in_foreign_contexts() {
    let source = fixture();
    // Each digest remains valid in its original declaration. Both directions
    // cover valid-before-invalid and invalid-before-valid inventory orders.
    for (from, to) in [("Left", "Right"), ("Right", "Left")] {
        for name in ["item", "nested", "callback", "related"] {
            let ty = field_type(&source, from, name);
            let target = field_key(&source, to, name);
            let mut changed = source.clone();
            let OwnerRecord::Field(record) = changed.owners.get_mut(&target).unwrap() else {
                panic!("field");
            };
            record.ty = ty;
            expect(&changed, 17);
        }
    }
    let mut changed = source;
    let foreign = field_type(&changed, "Left", "item");
    let case = changed
        .owners
        .values_mut()
        .find_map(|owner| match owner {
            OwnerRecord::Case(record) if record.name.as_str() == "yes" => Some(record),
            _ => None,
        })
        .unwrap();
    case.payload = Some(foreign);
    expect(&changed, 17);
}

#[test]
fn selfhost_source_type_scope_checks_parameters_results_and_lexical_annotations() {
    let source = fixture();
    let foreign = field_type(&source, "Left", "item");
    let mut changed = source.clone();
    let parameter = changed
        .owners
        .values_mut()
        .find_map(|owner| match owner {
            OwnerRecord::Parameter(record) if record.name.as_str() == "other" => Some(record),
            _ => None,
        })
        .unwrap();
    parameter.ty = foreign;
    expect(&changed, 17);

    let mut changed = source.clone();
    let second = declaration(&changed, "second");
    let OwnerRecord::Declaration(record) = changed
        .owners
        .get_mut(&OwnerKey::Declaration(second))
        .unwrap()
    else {
        panic!("declaration");
    };
    let kernel::DeclarationPayload::Function(function) = &mut record.payload else {
        panic!("function");
    };
    function.result.ty = foreign;
    expect(&changed, 17);

    let mut changed = source;
    let binding = changed
        .owners
        .values_mut()
        .find_map(|owner| match owner {
            OwnerRecord::Binding(record) if record.name.as_str() == "kept" => Some(record),
            _ => None,
        })
        .unwrap();
    binding.declared_type = Some(foreign);
    expect(&changed, 17);
}

#[test]
fn selfhost_source_type_scope_checks_every_supported_expression_type_root() {
    let source = fixture();
    let foreign = field_type(&source, "Left", "item");
    let mut mutations = 0;
    for (key, owner) in &source.owners {
        let OwnerRecord::Expression(expression) = owner else {
            continue;
        };
        let mut operation = expression.operation.clone();
        match &mut operation {
            kernel::ExpressionOperation::Call { type_arguments, .. }
            | kernel::ExpressionOperation::FunctionValue { type_arguments, .. }
            | kernel::ExpressionOperation::Record { type_arguments, .. }
            | kernel::ExpressionOperation::Variant { type_arguments, .. }
                if !type_arguments.is_empty() =>
            {
                type_arguments[0] = foreign
            }
            kernel::ExpressionOperation::List { item_type, .. } => *item_type = foreign,
            _ => continue,
        }
        let mut changed = source.clone();
        let OwnerRecord::Expression(record) = changed.owners.get_mut(key).unwrap() else {
            panic!("expression");
        };
        record.operation = operation;
        expect(&changed, 17);
        mutations += 1;
    }
    assert_eq!(
        mutations, 5,
        "call/function-value/record/variant/list coverage"
    );
}

fn dag(count: u32, edges: &[(u32, u32)], work: u32, code: i64) {
    let mut bytes = vec![0; 80];
    bytes[..4].copy_from_slice(&count.to_le_bytes());
    bytes[4..8].copy_from_slice(&work.to_le_bytes());
    bytes[48..80].fill(97);
    for (from, to) in edges {
        bytes.extend_from_slice(&from.to_le_bytes());
        bytes.extend_from_slice(&to.to_le_bytes());
    }
    assert_eq!(
        result("read-type-dag", &bytes),
        (
            code,
            if code == 0 {
                bytes[48..80].to_vec()
            } else {
                vec![]
            }
        )
    );
    emitted_result("read-type-dag", &bytes, code);
}

#[test]
fn selfhost_source_type_dag_rejects_cycles_and_preserves_shared_subgraphs() {
    dag(0, &[], 0, 0);
    dag(1, &[], 0, 6);
    dag(1, &[], 1, 0);
    dag(1, &[(0, 0)], 100, 17);
    dag(3, &[(0, 1), (1, 2), (2, 0)], 1000, 17);
    dag(4, &[(0, 1), (2, 3), (3, 2)], 1000, 17);
    dag(4, &[(0, 1), (0, 2), (1, 3), (2, 3)], 1000, 0);
    dag(5, &[(4, 1), (4, 2), (2, 0), (1, 0)], 1000, 0);
    dag(2, &[(1, 0)], 6, 6);
    dag(2, &[(1, 0)], 7, 0);
}

#[test]
fn selfhost_source_type_dag_has_exact_structural_depth_boundary() {
    let admitted = (1..257).map(|node| (node, node - 1)).collect::<Vec<_>>();
    dag(257, &admitted, 1_000_000, 0);
    let rejected = (1..258).map(|node| (node, node - 1)).collect::<Vec<_>>();
    dag(258, &rejected, 1_000_000, 17);
}

#[test]
fn selfhost_source_type_scope_preserves_resource_and_unsupported_boundaries() {
    let source = fixture();
    let graph = graph::packet(&source);
    for budget in [0_u32, u32::MAX] {
        let mut input = budget.to_le_bytes().to_vec();
        input.extend_from_slice(&graph);
        assert_eq!(result("read-type-scope-budget", &input), (6, vec![]));
        emitted_result("read-type-scope-budget", &input, 6);
    }
    let mut malformed = u32::MAX.to_le_bytes().to_vec();
    malformed.extend_from_slice(&graph);
    malformed[4] ^= 1;
    assert_eq!(result("read-type-scope-budget", &malformed), (12, vec![]));
    emitted_result("read-type-scope-budget", &malformed, 12);
    let mut unsupported = source;
    let object = kernel::TypeObject::new(kernel::TypeForm::OwnedI64Cell).unwrap();
    let digest = kernel::encode_type_object(&object).unwrap().0;
    unsupported.types.insert(digest, object);
    let bytes = graph::packet(&unsupported);
    assert_eq!(result("read-type-scope", &bytes), (13, vec![]));
    emitted_result("read-type-scope", &bytes, 13);
}

#[test]
fn selfhost_source_type_scope_respects_exact_composed_allowance() {
    exact_budget_boundary("read-type-scope-budget", &fixture());
}
