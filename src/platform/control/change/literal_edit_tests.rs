//! Independent literal fixtures and exact before/after owner observations.
use super::*;
use crate::platform::change::{AuthoredLiteralUpdate, AuthoredLiteralValue, CanonicalBaseRead};
use crate::platform::kernel::{DeclarationPayload, ExpressionOperation, OwnerRecord};
use crate::platform::publication::{GraphRepository, RepositoryView};

#[path = "literal_edit_more_tests.rs"]
mod additional;
#[path = "literal_edit_overlap_tests.rs"]
mod overlap;

const SOURCE: &str = r#"declarations.begin
(units
  (module create literal_edits (as $module)
    (type-alias Row (record (flag Bool) (number I64) (floating F64) (dynamic Text) (fixed StaticText)))
    (function create scalars (as $function) (visibility public) (returns Row) (effect pure)
      (body (let (binding saved (i64 7)) (binding other (i64 17))
        (in (record structural (field flag (bool false)) (field number (local saved))
          (field floating (f64 -0.0)) (field dynamic (text "old🌱"))
          (field fixed (static-text "before")))))))
    (function create neighbor (as $neighbor) (visibility private) (returns Text) (effect pure)
      (body (text "neighbor")))
    (constant create marker (as $marker) (visibility private) (type I64) (value (i64 99)))))
declarations.end
"#;

struct Fixture {
    _temporary: tempfile::TempDir,
    repository: GraphRepository,
    module: crate::platform::semantic_id::ModuleId,
    function: DeclarationId,
    neighbor: DeclarationId,
    marker: DeclarationId,
}

impl Fixture {
    fn new() -> Self {
        Self::from_source(SOURCE)
    }

    fn from_source(source: &str) -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let initial = crate::platform::kernel::tests::witness_snapshot();
        let created =
            GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
        let input = format!("request base={}\n{source}", created.current.head.revision);
        let request = decode_compact_change("literal-fixture.lkjc", input.as_bytes()).unwrap();
        let prepared = created
            .repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap();
        created.repository.publish(&prepared.publication).unwrap();
        let declaration = |name| match prepared.allocated[name] {
            OwnerKey::Declaration(id) => id,
            _ => panic!("fixture declaration"),
        };
        Self {
            _temporary: temporary,
            repository: created.repository,
            module: match prepared.allocated["$module"] {
                OwnerKey::Module(id) => id,
                _ => panic!("fixture module"),
            },
            function: declaration("$function"),
            neighbor: declaration("$neighbor"),
            marker: declaration("$marker"),
        }
    }

    fn view(&self) -> RepositoryView {
        self.repository.view_current().unwrap()
    }

    fn draft(&self) -> String {
        String::from_utf8(
            render_native_draft(
                &self.view(),
                &[OwnerKey::Declaration(self.function)],
                4 * 1_048_576,
                crate::platform::execution::ExecutionControl::uncancelled(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn edit(&self) -> NormalizedChangeRequest {
        let input = self
            .draft()
            .replace("(i64 7)", "(i64 8)")
            .replace("(bool false)", "(bool true)")
            .replace("(f64 -0.0)", "(f64 0.0)")
            .replace("old🌱", "new日本語")
            .replace("\"before\"", "\"after\"");
        decode_compact_change_in_repository("literal-edit.lkjc", input.as_bytes(), &self.repository)
            .unwrap()
    }

    fn body(&self, function: DeclarationId) -> ExpressionId {
        let OwnerRecord::Declaration(record) = self
            .view()
            .read_owner(OwnerKey::Declaration(function))
            .unwrap()
            .value
            .unwrap()
        else {
            panic!("function");
        };
        let DeclarationPayload::Function(function) = record.payload else {
            panic!("function payload");
        };
        function.body
    }
}

// This intentionally enumerates only the literal fixture's known syntax instead of sharing
// the production aggregation walker or native re-entry reader with the oracle.
fn inventory(view: &RepositoryView, function: DeclarationId) -> BTreeMap<OwnerKey, OwnerRecord> {
    let mut pending = vec![OwnerKey::Declaration(function)];
    let mut records = BTreeMap::new();
    while let Some(owner) = pending.pop() {
        let record = view.read_owner(owner).unwrap().value.unwrap();
        match &record {
            OwnerRecord::Declaration(record) => {
                let DeclarationPayload::Function(function) = &record.payload else {
                    panic!("function");
                };
                pending.push(OwnerKey::Expression(function.body));
            }
            OwnerRecord::Binding(binding) => {
                pending.push(OwnerKey::Expression(binding.value.unwrap()))
            }
            OwnerRecord::Expression(expression) => match &expression.operation {
                ExpressionOperation::Let { bindings, body } => {
                    pending.extend(bindings.iter().copied().map(OwnerKey::Binding));
                    pending.push(OwnerKey::Expression(*body));
                }
                ExpressionOperation::Record { fields, .. } => {
                    pending.extend(fields.iter().map(|f| OwnerKey::Expression(f.value)))
                }
                ExpressionOperation::Local { .. }
                | ExpressionOperation::Bool { .. }
                | ExpressionOperation::I64 { .. }
                | ExpressionOperation::F64 { .. }
                | ExpressionOperation::Text { .. }
                | ExpressionOperation::StaticText { .. } => {}
                _ => panic!("unexpected literal fixture syntax"),
            },
            _ => panic!("unexpected fixture owner"),
        }
        assert!(records.insert(owner, record).is_none());
    }
    records
}

fn literals(request: &AuthoredChangeSet) -> &[AuthoredLiteralUpdate] {
    let selected: Vec<_> = request
        .changes
        .iter()
        .filter_map(|change| match change {
            AuthoredChange::SetFunctionLiterals { literals, .. } => Some(literals.as_slice()),
            _ => None,
        })
        .collect();
    assert_eq!(selected.len(), 1, "{:#?}", request.changes);
    selected[0]
}

#[test]
fn literal_edit_preserves_every_other_owner_and_all_five_scalar_kinds() {
    let fixture = Fixture::new();
    let before = inventory(&fixture.view(), fixture.function);
    let neighbor = inventory(&fixture.view(), fixture.neighbor);
    let request = fixture.edit();
    let updates = literals(&request.semantic);
    assert_eq!(updates.len(), 5);
    assert!(
        updates
            .iter()
            .any(|u| matches!(&u.value, AuthoredLiteralValue::F64 { value } if value.bits() == 0))
    );
    assert!(
        updates.iter().any(
            |u| matches!(&u.value, AuthoredLiteralValue::Text { value } if value == "new日本語")
        )
    );
    let changed: BTreeSet<_> = updates
        .iter()
        .map(|u| OwnerKey::Expression(u.expression))
        .collect();
    let prepared = fixture
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap();
    assert!(prepared.logical_plan.allocations.is_empty());
    assert!(prepared.logical_plan.retirements.is_empty());
    assert!(
        prepared
            .logical_plan
            .semantically_checked
            .contains(&OwnerKey::Declaration(fixture.function))
    );
    fixture.repository.publish(&prepared.publication).unwrap();
    let after = inventory(&fixture.view(), fixture.function);
    assert_eq!(
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>()
    );
    for (owner, original) in &before {
        assert_eq!(
            original != &after[owner],
            changed.contains(owner),
            "{owner}"
        );
    }
    assert_eq!(neighbor, inventory(&fixture.view(), fixture.neighbor));
    let unchanged = decode_compact_change_in_repository(
        "unchanged.lkjc",
        fixture.draft().as_bytes(),
        &fixture.repository,
    )
    .unwrap();
    assert!(!unchanged.semantic.changes.iter().any(|c| matches!(
        c,
        AuthoredChange::SetFunctionLiterals { .. } | AuthoredChange::ReplaceFunctionBody { .. }
    )));
}

#[test]
fn literal_edit_core_rejects_foreign_duplicate_empty_nonliteral_and_wrong_kind() {
    let fixture = Fixture::new();
    let before = fixture.view().revision();
    let original = fixture.edit();
    let values = literals(&original.semantic).to_vec();
    let cases = [
        "foreign",
        "duplicate",
        "empty",
        "nonliteral",
        "kind",
        "function",
    ];
    for case in cases {
        let mut updates = values.clone();
        let mut function = fixture.function;
        match case {
            "foreign" => {
                updates[0].expression = fixture.body(fixture.neighbor);
                // Type-correct in the other function: only ownership can reject this edit.
                updates[0].value = AuthoredLiteralValue::Text {
                    value: "forged neighbor".into(),
                };
            }
            "duplicate" => updates.push(updates[0].clone()),
            "empty" => updates.clear(),
            "nonliteral" => updates[0].expression = fixture.body(fixture.function),
            "kind" => {
                updates[0].value = match updates[0].value {
                    AuthoredLiteralValue::I64 { .. } => AuthoredLiteralValue::Bool { value: true },
                    _ => AuthoredLiteralValue::I64 { value: 42 },
                }
            }
            "function" => function = fixture.marker,
            _ => unreachable!(),
        }
        let mut request = original.semantic.clone();
        request.changes = vec![AuthoredChange::SetFunctionLiterals {
            function: DeclarationSelector::Id {
                declaration: function,
            },
            literals: updates,
        }];
        let result = fixture
            .repository
            .prepare_authored_change(&request, Default::default());
        assert!(
            result.is_err(),
            "{case}: an invalid scalar selection was accepted"
        );
        let errors = result.unwrap_err();
        let code = match case {
            "nonliteral" => "kind",
            other => other,
        };
        assert!(
            errors
                .iter()
                .any(|e| e.code == format!("change_literal_{code}")),
            "{case}: {errors:#?}"
        );
        assert_eq!(fixture.view().revision(), before);
    }
}

#[test]
fn literal_edit_budgets_and_codec_bind_exact_values_owners_and_base() {
    let fixture = Fixture::new();
    let original = fixture.edit();
    let bytes =
        crate::platform::change::canonical_authored_intent_bytes(&original.semantic).unwrap();
    assert_eq!(&bytes[..8], b"LKJACR19");
    for case in ["value", "owner", "base"] {
        let mut request = original.semantic.clone();
        if case == "base" {
            request.base = RevisionId::from_digest([91; 32]);
        }
        for change in &mut request.changes {
            if let AuthoredChange::SetFunctionLiterals { literals, .. } = change {
                match case {
                    "value" => {
                        literals[0].value = AuthoredLiteralValue::Text {
                            value: "changed".into(),
                        }
                    }
                    "owner" => literals[0].expression = fixture.body(fixture.neighbor),
                    _ => {}
                }
            }
        }
        assert_ne!(
            bytes,
            crate::platform::change::canonical_authored_intent_bytes(&request).unwrap()
        );
    }
    for case in ["edits", "ownership", "reads"] {
        let mut request = original.semantic.clone();
        match case {
            "edits" => request.budget.canonical_edits.maximum_owner_edits = 0,
            "ownership" => request.budget.impact.maximum_ownership_steps = 0,
            "reads" => request.budget.canonical_reads.maximum_point_reads = 0,
            _ => unreachable!(),
        }
        let errors = fixture
            .repository
            .prepare_authored_change(&request, Default::default())
            .unwrap_err();
        assert!(
            errors.iter().any(|e| e.code.starts_with("change_budget_")),
            "{case}: {errors:#?}"
        );
        assert_eq!(fixture.view().revision(), original.semantic.base);
    }
}

#[test]
fn literal_edit_structure_scope_and_types_require_whole_body_validation() {
    let fixture = Fixture::new();
    let original = fixture.draft();
    for (from, to, decodes) in [
        ("(i64 7)", "(if (bool true) (i64 8) (i64 9))", true),
        ("(local saved)", "(i64 7)", true),
        ("(local saved)", "(local other)", true),
        ("(local saved)", "(local escaped)", false),
        ("(i64 7)", "(text \"wrong-type\")", true),
    ] {
        assert!(original.contains(from));
        // Also change a scalar so a shape-only or partial comparison cannot pass this test.
        let source = original.replace(from, to).replace("old🌱", "changed");
        let result = decode_compact_change_in_repository(
            "structural-edit.lkjc",
            source.as_bytes(),
            &fixture.repository,
        );
        if !decodes {
            assert!(result.is_err());
            continue;
        }
        let request = result.unwrap();
        assert!(
            !request
                .semantic
                .changes
                .iter()
                .any(|c| matches!(c, AuthoredChange::SetFunctionLiterals { .. }))
        );
        assert!(
            request
                .semantic
                .changes
                .iter()
                .any(|c| matches!(c, AuthoredChange::ReplaceFunctionBody { .. }))
        );
        let prepared = fixture
            .repository
            .prepare_authored_change(&request.semantic, request.options);
        if to.contains("wrong-type") {
            assert!(prepared.is_err());
        } else {
            assert!(!prepared.unwrap().logical_plan.retirements.is_empty());
        }
        assert_eq!(fixture.view().revision(), request.semantic.base);
    }
}
