//! Admission and composition cases independent of native shape matching.
use super::*;

#[test]
fn literal_edit_equal_values_keep_distinct_owned_positions() {
    let fixture = Fixture::from_source(&SOURCE.replace("(i64 17)", "(i64 7)"));
    let before = inventory(&fixture.view(), fixture.function);
    let request = fixture.edit();
    let updates = literals(&request.semantic);
    assert_eq!(updates.len(), 6);
    let selected: BTreeSet<_> = updates
        .iter()
        .map(|u| OwnerKey::Expression(u.expression))
        .collect();
    assert_eq!(selected.len(), 6);
    assert_eq!(
        updates
            .iter()
            .filter(|u| matches!(u.value, AuthoredLiteralValue::I64 { value: 8 }))
            .count(),
        2
    );
    let prepared = fixture
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap();
    fixture.repository.publish(&prepared.publication).unwrap();
    let after = inventory(&fixture.view(), fixture.function);
    assert_eq!(
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>()
    );
    for (owner, value) in before {
        assert_eq!(value != after[&owner], selected.contains(&owner));
    }
}

#[test]
fn literal_edit_follows_current_candidate_after_other_body_operations() {
    let fixture = Fixture::new();
    let original = fixture.edit();
    let patch = original
        .semantic
        .changes
        .iter()
        .find(|c| matches!(c, AuthoredChange::SetFunctionLiterals { .. }))
        .unwrap()
        .clone();
    let source = fixture
        .draft()
        .replace("(i64 7)", "(if (bool true) (i64 8) (i64 9))");
    let structural = decode_compact_change_in_repository(
        "replacement.lkjc",
        source.as_bytes(),
        &fixture.repository,
    )
    .unwrap();
    let replacement = structural
        .semantic
        .changes
        .iter()
        .find(|c| matches!(c, AuthoredChange::ReplaceFunctionBody { .. }))
        .unwrap()
        .clone();
    let mut request = original.semantic;
    request.changes = vec![replacement.clone(), patch.clone()];
    let errors = fixture
        .repository
        .prepare_authored_change(&request, Default::default())
        .unwrap_err();
    assert!(
        errors.iter().any(|e| e.code == "change_literal_created"),
        "{errors:#?}"
    );
    assert_eq!(fixture.view().revision(), request.base);
    // Ordered composition remains ordinary intent: a later complete replacement may supersede
    // an earlier scalar edit. The previous ownership index cannot resurrect retired identities.
    request.changes = vec![patch, replacement];
    let prepared = fixture
        .repository
        .prepare_authored_change(&request, Default::default())
        .unwrap();
    assert!(!prepared.logical_plan.retirements.is_empty());
    assert_eq!(fixture.view().revision(), request.base);
}

#[test]
fn literal_edit_cannot_hide_a_simultaneous_invalid_function_contract() {
    let fixture = Fixture::new();
    let mut request = fixture.edit();
    let mut changed = false;
    for change in &mut request.semantic.changes {
        if let AuthoredChange::SetFunctionContract { result, .. } = change {
            *result = AuthoredType::Bool {};
            changed = true;
        }
    }
    assert!(changed);
    assert_eq!(literals(&request.semantic).len(), 5);
    let errors = fixture
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap_err();
    assert!(
        errors.iter().any(|e| e.class == DiagnosticClass::Semantic),
        "{errors:#?}"
    );
    assert_eq!(fixture.view().revision(), request.semantic.base);
}

#[test]
fn literal_edit_oversized_scalar_and_missing_identity_are_read_only_failures() {
    let fixture = Fixture::new();
    let original = fixture.edit();
    for oversized in [false, true] {
        let mut request = original.semantic.clone();
        let mut values = literals(&request).to_vec();
        if oversized {
            values[0].value = AuthoredLiteralValue::Text {
                value: "x".repeat(crate::platform::change::MAXIMUM_AUTHORED_CHANGE_BYTES),
            };
        } else {
            values[0].expression = ExpressionId::allocate(&[91; 32], 1);
        }
        request.changes = vec![AuthoredChange::SetFunctionLiterals {
            function: DeclarationSelector::Id {
                declaration: fixture.function,
            },
            literals: values,
        }];
        let errors = fixture
            .repository
            .prepare_authored_change(&request, Default::default())
            .unwrap_err();
        let expected = if oversized {
            "change_authored_bytes"
        } else {
            "change_literal_foreign"
        };
        assert!(errors.iter().any(|e| e.code == expected), "{errors:#?}");
        assert_eq!(fixture.view().revision(), request.base);
    }
}

#[test]
fn literal_edit_same_kind_core_noop_does_not_publish_a_revision() {
    let fixture = Fixture::new();
    let original = fixture.edit();
    let existing = inventory(&fixture.view(), fixture.function);
    let update = literals(&original.semantic)
        .iter()
        .find(|u| matches!(u.value, AuthoredLiteralValue::I64 { .. }))
        .unwrap();
    let OwnerRecord::Expression(expression) = &existing[&OwnerKey::Expression(update.expression)]
    else {
        panic!("literal");
    };
    let ExpressionOperation::I64 { value } = expression.operation else {
        panic!("integer");
    };
    let mut request = original.semantic;
    request.changes = vec![AuthoredChange::SetFunctionLiterals {
        function: DeclarationSelector::Id {
            declaration: fixture.function,
        },
        literals: vec![AuthoredLiteralUpdate {
            expression: expression.id,
            value: AuthoredLiteralValue::I64 { value },
        }],
    }];
    let errors = fixture
        .repository
        .prepare_authored_change(&request, Default::default())
        .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.code == "publication_semantic_no_change"),
        "{errors:#?}"
    );
    assert_eq!(fixture.view().revision(), request.base);
}

#[test]
fn literal_edit_codec_keeps_legacy_body_identity_and_observes_float_bits() {
    let fixture = Fixture::new();
    let mut request = fixture.edit().semantic;
    let mut update = literals(&request)
        .iter()
        .find(|u| matches!(u.value, AuthoredLiteralValue::F64 { .. }))
        .unwrap()
        .clone();
    let function = DeclarationSelector::Id {
        declaration: fixture.function,
    };
    let mut fingerprints = BTreeSet::new();
    for bits in [
        0,
        0x8000_0000_0000_0000,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        crate::platform::binary64::CANONICAL_NAN_BITS,
    ] {
        update.value = AuthoredLiteralValue::F64 {
            value: crate::platform::binary64::Binary64::from_bits(bits).unwrap(),
        };
        request.changes = vec![AuthoredChange::SetFunctionLiterals {
            function: function.clone(),
            literals: vec![update.clone()],
        }];
        let bytes = crate::platform::change::canonical_authored_intent_bytes(&request).unwrap();
        assert_eq!(&bytes[..8], b"LKJACR19");
        assert!(fingerprints.insert(bytes));
    }
    request.changes = vec![AuthoredChange::ReplaceFunctionBody {
        function,
        body: AuthoredExpression {
            symbol: None,
            operation: AuthoredExpressionOperation::I64 { value: 42 },
        },
    }];
    let bytes = crate::platform::change::canonical_authored_intent_bytes(&request).unwrap();
    assert_eq!(
        &bytes[..8],
        b"LKJACR14",
        "legacy requests retain their original encoding domain"
    );
}
