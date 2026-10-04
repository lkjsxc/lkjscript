//! Native sequence proposals retain accepted identity and admit their complete nested types.
use super::*;
use crate::platform::change::AuthoredLiteralValue;
use crate::platform::change::CanonicalBaseRead;
use crate::platform::kernel::{BindingKind, DeclarationPayload, ExpressionOperation};
use crate::platform::publication::{GraphRepository, RepositoryView};

const SOURCE: &str = r#"declarations.begin
(units (module create sequence_reentry (as $module)
  (function create empty (as $empty) (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (returns (owned-sequence T))
    (body (sequence-empty (type (owned-sequence T)))))
  (function create length (as $length) (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create values (type (owned-sequence T)) (use borrow)) (returns I64)
    (body (sequence-length (type (owned-sequence T)) (local values))))
  (function create push (as $push) (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create value (type T) (use consume))
    (parameter create values (type (owned-sequence T)) (use consume))
    (returns (owned-sequence T))
    (body (sequence-push (type (owned-sequence T)) (local value) (local values))))
  (function create pop (as $pop) (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create values (type (owned-sequence T)) (use consume))
    (returns (owned-choice (case empty (owned-sequence T))
      (case item (owned-product (field rest (owned-sequence T)) (field value T)))))
    (body (sequence-pop (type (owned-sequence T)) (local values))))
  (function create inspect (as $inspect) (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create values (type (owned-sequence (owned-sequence T))) (use borrow))
    (returns I64)
    (body (borrow-owned-item (type (owned-sequence (owned-sequence T))) (local values)
      (index (i64 3)) (binding values (type (owned-sequence T)))
      (in (sequence (i64 5) (sequence-length (type (owned-sequence T)) (local values)))))))))
declarations.end
"#;

struct Fixture {
    _temporary: tempfile::TempDir,
    repository: GraphRepository,
    module: OwnerKey,
    functions: BTreeMap<String, DeclarationId>,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let initial = crate::platform::kernel::tests::witness_snapshot();
        let created =
            GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
        let input = format!("request base={}\n{SOURCE}", created.current.head.revision);
        let request = decode_compact_change("sequence-reentry.lkjc", input.as_bytes()).unwrap();
        let prepared = created
            .repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap_or_else(|errors| panic!("generic sequence admission: {errors:#?}"));
        let functions = ["empty", "length", "push", "pop", "inspect"]
            .into_iter()
            .map(|name| {
                let OwnerKey::Declaration(declaration) = prepared.allocated[&format!("${name}")]
                else {
                    panic!("function allocation");
                };
                (name.to_owned(), declaration)
            })
            .collect();
        let module = prepared.allocated["$module"];
        created.repository.publish(&prepared.publication).unwrap();
        Self {
            _temporary: temporary,
            repository: created.repository,
            module,
            functions,
        }
    }

    fn view(&self) -> RepositoryView {
        self.repository.view_current().unwrap()
    }

    fn draft(&self) -> String {
        String::from_utf8(
            render_native_draft(
                &self.view(),
                &[self.module.into()],
                4 * 1_048_576,
                ExecutionControl::uncancelled(),
            )
            .unwrap(),
        )
        .unwrap()
    }
}

// Enumerate this fixture's accepted syntax independently of the production re-entry reader
// and child walker, so they cannot serve as their own identity-preservation oracle.
fn inventory(view: &RepositoryView, function: DeclarationId) -> BTreeMap<OwnerKey, OwnerRecord> {
    let mut pending = vec![OwnerKey::Declaration(function)];
    let mut records = BTreeMap::new();
    while let Some(owner) = pending.pop() {
        let record = view.read_owner(owner).unwrap().value.unwrap();
        match &record {
            OwnerRecord::Declaration(record) => {
                let DeclarationPayload::Function(function) = &record.payload else {
                    panic!("function payload");
                };
                pending.push(OwnerKey::Expression(function.body));
            }
            OwnerRecord::Binding(binding) => {
                assert_eq!(binding.kind, BindingKind::OwnedBorrow);
                assert!(binding.value.is_none());
                assert!(binding.declared_type.is_some());
            }
            OwnerRecord::Expression(expression) => match &expression.operation {
                ExpressionOperation::SequenceLength { source, .. }
                | ExpressionOperation::SequencePop { source, .. } => {
                    pending.push(OwnerKey::Expression(*source));
                }
                ExpressionOperation::SequencePush { value, source, .. } => {
                    pending.extend([OwnerKey::Expression(*value), OwnerKey::Expression(*source)]);
                }
                ExpressionOperation::BorrowOwnedItem {
                    source,
                    index,
                    binding,
                    body,
                    ..
                } => {
                    pending.extend([
                        OwnerKey::Expression(*index),
                        OwnerKey::Expression(*source),
                        OwnerKey::Binding(*binding),
                        OwnerKey::Expression(*body),
                    ]);
                }
                ExpressionOperation::Sequence { items } => {
                    pending.extend(items.iter().copied().map(OwnerKey::Expression));
                }
                ExpressionOperation::SequenceEmpty { .. }
                | ExpressionOperation::Local { .. }
                | ExpressionOperation::I64 { .. } => {}
                other => panic!("unexpected sequence fixture syntax: {other:?}"),
            },
            other => panic!("unexpected fixture owner: {other:?}"),
        }
        assert!(records.insert(owner, record).is_none());
    }
    records
}

#[test]
fn owned_sequence_native_drafts_and_scalar_edits_retain_exact_body_owners() {
    let fixture = Fixture::new();
    let original = fixture.draft();
    for form in [
        "(owned-sequence ",
        "(sequence-empty ",
        "(sequence-length ",
        "(sequence-push ",
        "(sequence-pop ",
        "(borrow-owned-item ",
        "(index (i64 3))",
    ] {
        assert!(original.contains(form), "{form}: {original}");
    }
    let unchanged = decode_compact_change_in_repository(
        "sequence-reentry-noop.lkjc",
        original.as_bytes(),
        &fixture.repository,
    )
    .unwrap();
    assert!(!unchanged.semantic.changes.iter().any(|change| matches!(
        change,
        AuthoredChange::SetFunctionLiterals { .. } | AuthoredChange::ReplaceFunctionBody { .. }
    )));
    let errors = fixture
        .repository
        .prepare_authored_change(&unchanged.semantic, unchanged.options)
        .unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!(errors[0].code, "publication_semantic_no_change");

    let before = fixture
        .functions
        .iter()
        .map(|(name, function)| (name.clone(), inventory(&fixture.view(), *function)))
        .collect::<BTreeMap<_, _>>();
    let changed = original
        .replace("(i64 3)", "(i64 4)")
        .replace("(i64 5)", "(i64 6)");
    let request = decode_compact_change_in_repository(
        "sequence-reentry-literals.lkjc",
        changed.as_bytes(),
        &fixture.repository,
    )
    .unwrap();
    assert!(
        !request
            .semantic
            .changes
            .iter()
            .any(|change| matches!(change, AuthoredChange::ReplaceFunctionBody { .. }))
    );
    let edits = request
        .semantic
        .changes
        .iter()
        .filter_map(|change| match change {
            AuthoredChange::SetFunctionLiterals { literals, .. } => Some(literals),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].len(), 2);
    assert_eq!(edits[0][0].value, AuthoredLiteralValue::I64 { value: 4 });
    assert_eq!(edits[0][1].value, AuthoredLiteralValue::I64 { value: 6 });
    let selected = edits[0]
        .iter()
        .map(|update| OwnerKey::Expression(update.expression))
        .collect::<BTreeSet<_>>();
    let prepared = fixture
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap();
    assert!(prepared.logical_plan.allocations.is_empty());
    assert!(prepared.logical_plan.retirements.is_empty());
    fixture.repository.publish(&prepared.publication).unwrap();
    for (name, function) in &fixture.functions {
        let after = inventory(&fixture.view(), *function);
        assert_eq!(
            before[name].keys().collect::<Vec<_>>(),
            after.keys().collect::<Vec<_>>()
        );
        for (owner, previous) in &before[name] {
            assert_eq!(
                previous != &after[owner],
                selected.contains(owner),
                "{name}/{owner}"
            );
        }
    }
    assert_eq!(
        fixture.draft(),
        changed.replacen(
            &request.semantic.base.to_string(),
            &fixture.view().revision().to_string(),
            1,
        ),
    );
}

#[test]
fn owned_sequence_native_structural_edits_use_complete_body_validation() {
    let fixture = Fixture::new();
    let changed = fixture.draft().replace(
        "(index (i64 3))",
        "(index (if (bool true) (i64 4) (i64 3)))",
    );
    let request = decode_compact_change_in_repository(
        "sequence-reentry-structure.lkjc",
        changed.as_bytes(),
        &fixture.repository,
    )
    .unwrap();
    assert!(
        !request
            .semantic
            .changes
            .iter()
            .any(|change| matches!(change, AuthoredChange::SetFunctionLiterals { .. }))
    );
    assert!(
        request
            .semantic
            .changes
            .iter()
            .any(|change| matches!(change, AuthoredChange::ReplaceFunctionBody { .. }))
    );
    fixture
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap();
}

#[test]
fn cached_owned_sequence_expansion_admits_nested_products_and_choices_completely() {
    let fixture = Fixture::new();
    let view = fixture.view();
    let mut reader = Reader::new(&view, ExecutionControl::uncancelled());
    let OwnerRecord::Declaration(record) = reader
        .owner(OwnerKey::Declaration(fixture.functions["pop"]))
        .unwrap()
    else {
        panic!("pop declaration");
    };
    let DeclarationPayload::Function(function) = record.payload else {
        panic!("pop function");
    };
    reader.ty(function.result).unwrap();
    reader.remaining_type_nodes = 3;
    assert_eq!(
        reader.ty(function.result).unwrap_err().code,
        "change_draft_capacity"
    );
}
