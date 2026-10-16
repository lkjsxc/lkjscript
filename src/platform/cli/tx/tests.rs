use super::*;
use crate::platform::kernel::NamespaceClass;
use crate::platform::witness::NamespaceKey;
use crate::platform::{MemProjectStore, create_memory_project};
use serde_json::Value;

const SQUARE: &str = r#"(tx (base @head)
  (upsert-module math
    (upsert-function square (visibility public)
      (param value I64) (returns I64) (effect pure)
      (body (call std::multiply (local value) (local value))))
    (upsert-test twelve-squared
      (actual (call math::square (i64 12)))
      (expected (i64 144)))))"#;

fn project() -> MemProjectStore {
    let project = MemProjectStore::new(128 * 1_048_576).unwrap();
    create_memory_project(&project, "transaction-test", "minimal").unwrap();
    project
}

fn standard_project() -> MemProjectStore {
    static BASE: std::sync::OnceLock<MemProjectStore> = std::sync::OnceLock::new();
    BASE.get_or_init(|| {
        let project = MemProjectStore::new(512 * 1_048_576).unwrap();
        create_memory_project(&project, "transaction-standard", "command").unwrap();
        project
    })
    .fork()
    .unwrap()
}

fn repository(project: &MemProjectStore) -> GraphRepository {
    GraphRepository::open_memory(project).unwrap()
}

fn tx(project: &MemProjectStore, input: &str) -> Result<Value, Vec<Diagnostic>> {
    let bytes = execute_in_memory(project, vec!["tx".into(), "--input".into(), input.into()])?;
    assert_eq!(bytes.iter().filter(|byte| **byte == b'\n').count(), 1);
    assert!(bytes.len() < 65_536);
    Ok(serde_json::from_slice(&bytes).unwrap())
}

fn owner(
    view: &RepositoryView,
    parent: Option<KernelOwnerKey>,
    class: NamespaceClass,
    name: &str,
) -> KernelOwnerKey {
    view.namespace(&NamespaceKey {
        parent,
        class,
        name: Name::new(name).unwrap(),
    })
    .unwrap()
    .value
    .unwrap()
}

fn apply_native(project: &MemProjectStore, units: &str) {
    let repository = repository(project);
    let view = repository.view_current().unwrap();
    let input = format!(
        "request base={}\ndeclarations.begin\n{units}\ndeclarations.end\n",
        view.revision()
    );
    let request = crate::platform::control::decode_compact_change_in_view(
        "oracle.lkjc",
        input.as_bytes(),
        &view,
    )
    .unwrap();
    let prepared = view
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap();
    assert!(matches!(
        repository.publish(&prepared.publication).unwrap(),
        GraphPublicationOutcome::Accepted { .. }
    ));
}

#[test]
fn symbolic_square_matches_independent_native_intent_and_records_verified_execution() {
    let project = standard_project();
    let repository = repository(&project);
    let view = repository.view_current().unwrap();
    let symbolic = crate::platform::control::decode_transaction(
        "square.lkjx",
        SQUARE.as_bytes(),
        &view,
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    let literal = format!(
        r#"request base={}
declarations.begin
(units (use std builtin)
  (module create math
    (function create square (visibility public)
      (parameter create value (type I64)) (returns I64) (effect pure)
      (body (call std::multiply (local value) (local value))))
    (test create twelve-squared (visibility private)
      (actual (call math::square (i64 12))) (expected (i64 144)))))
declarations.end
"#,
        view.revision()
    );
    let native = crate::platform::control::decode_compact_change_in_view(
        "oracle.lkjc",
        literal.as_bytes(),
        &view,
    )
    .unwrap();
    assert_eq!(symbolic.request_commitment, native.request_commitment);
    let result = tx(&project, SQUARE).unwrap();
    assert_eq!(result["tests_selected"], 1);
    assert_eq!(result["tests_passed"], 1);
    assert_eq!(result["differential"], "equal");
    let current = repository.current().unwrap();
    assert_eq!(current.receipt.validation.tests_selected, 1);
    assert_eq!(current.receipt.validation.tests_executed, 1);
    assert_eq!(current.receipt.validation.tests_passed, 1);
    assert_eq!(current.revision.publication.parents.len(), 1);
    assert_eq!(
        current.revision.publication.parents[0].revision,
        view.current().head.revision
    );
    assert_eq!(
        current.revision.publication.parents[0].record,
        view.current().head.record
    );
}

#[test]
fn updates_preserve_module_function_parameter_and_test_identities() {
    let project = standard_project();
    tx(&project, SQUARE).unwrap();
    let repository = repository(&project);
    let before = repository.view_current().unwrap();
    let module = owner(&before, None, NamespaceClass::Module, "math");
    let function = owner(&before, Some(module), NamespaceClass::Declaration, "square");
    let parameter = owner(&before, Some(function), NamespaceClass::Parameter, "value");
    let test = owner(
        &before,
        Some(module),
        NamespaceClass::Declaration,
        "twelve-squared",
    );
    let result = tx(&project, r#"(tx (base @head)
      (upsert-module @module("math")
        (upsert-function @fn("math::square") (param value I64) (returns I64) (effect pure)
          (body (local value)))
        (upsert-test twelve-squared (actual (call @fn("math::square") (i64 12))) (expected (i64 12)))))"#).unwrap();
    assert_eq!(result["tests_selected"], 1);
    let after = repository.view_current().unwrap();
    assert_eq!(owner(&after, None, NamespaceClass::Module, "math"), module);
    assert_eq!(
        owner(&after, Some(module), NamespaceClass::Declaration, "square"),
        function
    );
    assert_eq!(
        owner(&after, Some(function), NamespaceClass::Parameter, "value"),
        parameter
    );
    assert_eq!(
        owner(
            &after,
            Some(module),
            NamespaceClass::Declaration,
            "twelve-squared"
        ),
        test
    );
    let OwnerRecord::Declaration(declaration) = after.owner(function).unwrap().value.unwrap()
    else {
        panic!("function")
    };
    assert_eq!(declaration.visibility, DeclarationVisibility::Public);
}

#[test]
fn rejection_preserves_all_graph_readiness_compiler_and_data_bytes() {
    let project = standard_project();
    tx(&project, SQUARE).unwrap();
    let before = project.test_inventory().unwrap();
    let cases = [
        (SQUARE.replace("(i64 144)", "(i64 143)"), "normalized_test_failed"),
        (r#"(tx (base @head) (upsert-module @module("absent")))"#.into(), "transaction_selector_missing"),
        ("(tx (base @head) (upsert-module math (upsert-constant square (type I64) (value (i64 1)))))".into(), "transaction_selector_kind"),
        ("(tx (base @head) (upsert-module math (upsert-function square (returns I64) (effect pure) (body (bool true)))))".into(), ""),
        ("(tx (base @head) ())".into(), "transaction_unit"),
        ("(tx (base @head) (base @head) (upsert-module duplicate))".into(), "transaction_base"),
    ];
    for (input, code) in cases {
        let errors = tx(&project, &input).unwrap_err();
        assert!(
            code.is_empty() || errors.iter().any(|error| error.code == code),
            "{errors:#?}"
        );
        assert_eq!(project.test_inventory().unwrap(), before);
    }
}

#[test]
fn reverse_dependency_cone_selects_transitive_tests_and_skips_unrelated_failures() {
    let project = project();
    apply_native(
        &project,
        r#"(units
      (module create cone
        (function create leaf (visibility private) (returns I64) (effect pure) (body (i64 1)))
        (function create middle (visibility private) (returns I64) (effect pure) (body (call leaf)))
        (test create affected (visibility private) (actual (call middle)) (expected (i64 2))))
      (module create unrelated
        (test create failure (visibility private) (actual (i64 98)) (expected (i64 99)))))"#,
    );
    let result = tx(&project, "(tx (base @head) (upsert-module cone (upsert-function leaf (returns I64) (effect pure) (body (i64 2)))))").unwrap();
    assert_eq!(result["tests_selected"], 1);
    assert_eq!(result["tests_passed"], 1);
    let errors = execute_in_memory(&project, vec!["check".into()]).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "normalized_test_failed")
    );
}

#[test]
fn forward_names_untouched_references_and_quoted_text_keep_native_meaning() {
    let project = standard_project();
    tx(&project, SQUARE).unwrap();
    let result = tx(
        &project,
        r#"(tx (base @head) (upsert-module more
      (upsert-test forward (actual (call answer)) (expected (i64 144)))
      (upsert-function answer (returns I64) (effect pure) (body (call math::square (i64 12))))
      (upsert-test text (actual (text "@fn(\"math::square\") math::square"))
        (expected (text "@fn(\"math::square\") math::square")))))"#,
    )
    .unwrap();
    assert_eq!(result["tests_selected"], 2);
}

#[test]
fn exact_base_is_not_silently_reresolved_after_another_transaction() {
    let project = standard_project();
    let before = repository(&project).current().unwrap().head.revision;
    tx(&project, SQUARE).unwrap();
    let inventory = project.test_inventory().unwrap();
    let errors = tx(
        &project,
        &format!("(tx (base {before}) (upsert-module stale))"),
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "transaction_stale");
    assert_eq!(inventory, project.test_inventory().unwrap());
}

#[test]
fn pure_run_is_verified_before_commit_and_task_run_is_rejected_without_mutation() {
    let project = project();
    let input = r#"(tx (base @head) (upsert-module run
      (upsert-function answer (returns I64) (effect pure) (body (i64 42)))
      (upsert-component cli (upsert-port main (type (function () I64)) (function answer))))
      (upsert-target main (component run::cli) (runner command) (port run::cli::main)))"#;
    let bytes = execute_in_memory(
        &project,
        ["tx", "--input", input, "--run", "main"]
            .map(str::to_owned)
            .to_vec(),
    )
    .unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap()["run"], 42);
    let before = project.test_inventory().unwrap();
    let task = input
        .replace("(effect pure)", "(effect (task))")
        .replace("(function () I64)", "(task-function () I64 (row))");
    let errors = execute_in_memory(
        &project,
        ["tx", "--input", &task, "--run", "main"]
            .map(str::to_owned)
            .to_vec(),
    )
    .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "normalized_runner_grants_required"),
        "{errors:#?}"
    );
    assert_eq!(before, project.test_inventory().unwrap());
}

#[test]
fn unchanged_module_upserts_keep_all_authority_bytes_and_can_run_a_pure_target() {
    let project = standard_project();
    let input = "(tx (base @head) (upsert-module unchanged))";
    tx(&project, input).unwrap();
    let inventory = project.test_inventory().unwrap();
    let bytes = execute_in_memory(
        &project,
        ["tx", "--input", input, "--run", "main"]
            .map(str::to_owned)
            .to_vec(),
    )
    .unwrap();
    let result: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(result["status"], "unchanged");
    assert_eq!(result["tests_selected"], 0);
    assert_eq!(result["run"], "hello");
    assert_eq!(project.test_inventory().unwrap(), inventory);
}

#[test]
fn missing_standard_is_not_staged_and_local_std_names_are_not_supplier_aliases() {
    let project = project();
    let inventory = project.test_inventory().unwrap();
    let errors = tx(&project, SQUARE).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "change_reference_dependency_missing")
    );
    assert_eq!(project.test_inventory().unwrap(), inventory);
    tx(&project, "(tx (base @head) (upsert-module std (upsert-function value (visibility public) (returns I64) (effect pure) (body (i64 7)))))").unwrap();
    let result = tx(&project, "(tx (base @head) (upsert-module app (upsert-test local (actual (call std::value)) (expected (i64 7)))))").unwrap();
    assert_eq!(result["tests_passed"], 1);
}

#[test]
fn verified_candidates_bind_their_exact_source_and_only_one_base_can_publish() {
    let project = project();
    let repository = repository(&project);
    let control = ExecutionControl::uncancelled();
    let prepare = |name| {
        let view = repository.view_current().unwrap();
        let source = format!("(tx (base @head) (upsert-module {name}))");
        let request = crate::platform::control::decode_transaction(
            "race.lkjx",
            source.as_bytes(),
            &view,
            &control,
        )
        .unwrap();
        let mut prepared = view
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap();
        let preview = PreparedPreview::prepare(view, &prepared.publication, &control).unwrap();
        let proof = preview
            .check(&prepared.logical_plan.tests, &control)
            .unwrap();
        prepared
            .record_verified_tests(&proof, request.semantic.budget)
            .unwrap();
        (prepared, proof)
    };
    let (first, proof) = prepare("first");
    let (mut second, _) = prepare("second");
    assert_eq!(
        second
            .record_verified_tests(&proof, Default::default())
            .unwrap_err()
            .code,
        "transaction_test_evidence"
    );
    assert!(matches!(
        repository.publish(&first.publication).unwrap(),
        GraphPublicationOutcome::Accepted { .. }
    ));
    let inventory = project.test_inventory().unwrap();
    assert!(matches!(
        repository.publish(&second.publication).unwrap(),
        GraphPublicationOutcome::Stale { .. }
    ));
    assert_eq!(project.test_inventory().unwrap(), inventory);
}
