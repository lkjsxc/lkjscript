//! Authentic predecessor bytes, current public requests, and publication continuity.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::platform::{Diagnostic, DiagnosticClass, cli, control::parse_records};
use std::{collections::BTreeMap, path::Path};

fn copy(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &destination.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), destination.join(entry.file_name())).unwrap();
        }
    }
}
fn fixture(name: &str, destination: &Path) {
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/finite-callable-predecessor")
            .join(name),
        destination,
    );
}
fn inventory(root: &Path) -> BTreeMap<std::path::PathBuf, Vec<u8>> {
    fn visit(root: &Path, relative: &Path, output: &mut BTreeMap<std::path::PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(root.join(relative)).unwrap() {
            let entry = entry.unwrap();
            let name = relative.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                visit(root, &name, output);
            } else {
                output.insert(name, std::fs::read(entry.path()).unwrap());
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, Path::new(""), &mut result);
    result
}
fn args(root: &Path, command: &[&str]) -> Vec<String> {
    let mut result = vec!["--project".into(), root.display().to_string()];
    result.extend(command.iter().map(|s| (*s).to_owned()));
    result
}
fn field(bytes: &[u8], record: &str, name: &str) -> String {
    let records = parse_records("upgrade-response", bytes).unwrap();
    records
        .iter()
        .find(|r| r.operation == record)
        .unwrap()
        .fields
        .iter()
        .find(|f| f.name == name)
        .unwrap()
        .value
        .clone()
}
fn plan(root: &Path, request: &str) -> String {
    field(
        &cli::execute_change(args(root, &["change", "plan", "--input", request])).unwrap(),
        "plan",
        "token",
    )
}
fn apply(root: &Path, request: &str, token: &str) -> Vec<u8> {
    cli::execute_change(args(
        root,
        &["change", "apply", "--input", request, "--plan", token],
    ))
    .unwrap()
}

#[test]
fn current_revalidation_and_repair_propagate_cancellation_without_writes() {
    use crate::platform::execution::ExecutionControl;
    let temporary = tempfile::tempdir().unwrap();
    for name in ["valid", "expanding"] {
        let root = temporary.path().join(name);
        fixture(name, &root);
        let before = inventory(&root);
        let repository = GraphRepository::open(&root).unwrap();
        let cancelled = ExecutionControl::uncancelled();
        cancelled.cancel();
        assert_eq!(
            repository
                .view_current_with_control(&cancelled)
                .unwrap_err()
                .class,
            DiagnosticClass::Cancelled
        );
        let control = ExecutionControl::uncancelled();
        let view = repository.view_current_with_control(&control).unwrap();
        let snapshot = view.reconstruct_full_oracle().unwrap().value;
        let checks = std::cell::Cell::new(0);
        let mut work = 0;
        let errors = crate::platform::kernel::validate_full_checked(
            &snapshot,
            crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK,
            &mut work,
            &|| {
                checks.set(checks.get() + 1);
                if checks.get() == 5 {
                    control.cancel();
                }
                control
                    .check()
                    .map_err(|e| Diagnostic::new(DiagnosticClass::Cancelled, e.code, e.message))
            },
        )
        .unwrap_err();
        assert!(work > 0 && errors.iter().all(|e| e.class == DiagnosticClass::Cancelled));
        let function = snapshot
            .owners
            .iter()
            .find_map(|(key, record)| match record {
                crate::platform::kernel::OwnerRecord::Declaration(declaration)
                    if matches!(
                        declaration.payload,
                        crate::platform::kernel::DeclarationPayload::Function(_)
                    ) =>
                {
                    Some((*key, record.clone()))
                }
                _ => None,
            })
            .unwrap();
        let mut after = function.1.clone();
        if let crate::platform::kernel::OwnerRecord::Declaration(declaration) = &mut after {
            declaration.name = crate::platform::kernel::Name::new("cancelled-rename").unwrap();
        }
        let errors = view
            .prepare_change(
                vec![crate::platform::change::PrimitiveEdit::ReplaceOwner {
                    expected: crate::platform::kernel::encode_owner(&function.1)
                        .unwrap()
                        .0,
                    record: after,
                }],
                PublicationOptions::default(),
            )
            .unwrap_err();
        assert!(errors.iter().any(|e| e.class == DiagnosticClass::Cancelled));
        assert_eq!(inventory(&root), before);
    }
}

#[test]
fn authentic_predecessor_validity_repair_history_and_retry() {
    let temporary = tempfile::tempdir().unwrap();
    for (name, valid) in [("valid", true), ("expanding", false)] {
        let root = temporary.path().join(name);
        fixture(name, &root);
        let before = inventory(&root);
        let repository = GraphRepository::open(&root).unwrap();
        let old = repository.current().unwrap();
        assert!(!old.witness.contract_is_current());
        let status = cli::execute_status(args(&root, &["status"])).unwrap();
        assert_eq!(
            field(&status, "current-validation", "status"),
            if valid { "valid" } else { "invalid" }
        );
        cli::execute_query(args(&root, &["query", "owners", "--kind", "pure_function"])).unwrap();
        assert_eq!(
            inventory(&root),
            before,
            "read/revalidation must leave HEAD, history, catalog, and caches untouched"
        );
        let view = repository.view_current().unwrap();
        assert_eq!(view.current().accepted, old.accepted);
        if valid {
            view.require_current_validation().unwrap();
            assert!(cli::execute_check(args(&root, &["check"])).is_ok());
            let request =
                include_str!("../../../tests/fixtures/finite-callable-predecessor/valid.request");
            let old_token = include_str!(
                "../../../tests/fixtures/finite-callable-predecessor/valid.plan-token"
            )
            .trim();
            let before_retry = inventory(&root);
            let errors = cli::execute_change(args(
                &root,
                &["change", "apply", "--input", request, "--plan", old_token],
            ))
            .unwrap_err();
            assert_eq!(errors[0].code, "change_prepared_plan_mismatch");
            assert_eq!(
                inventory(&root),
                before_retry,
                "an old prepared token requires re-planning without changing its historical result"
            );
            let token = plan(&root, request);
            assert_eq!(
                field(&apply(&root, request, &token), "result", "status"),
                "already-accepted"
            );
            assert_eq!(repository.current().unwrap().head, old.head);
        } else {
            assert_eq!(
                view.require_current_validation().unwrap_err().code,
                "kernel_callable_expansion"
            );
            assert_eq!(
                cli::execute_check(args(&root, &["check"]))
                    .unwrap_err()
                    .code,
                "kernel_callable_expansion"
            );
            assert!(view.export_package_transport().is_err());
            let request = format!(
                "request base={} idempotency=repair-test\nexpression.i64 as=$seven value=7\nreplace.body function=probe/growing body=$seven\n",
                old.head.revision
            );
            let token = plan(&root, &request);
            assert_eq!(repository.current().unwrap().head, old.head);
            let result = apply(&root, &request, &token);
            assert_eq!(field(&result, "result", "status"), "accepted");
            let repaired = repository.current().unwrap();
            assert_eq!(
                repaired.revision.publication.parents,
                vec![old.accepted.parent()]
            );
            assert!(repaired.witness.contract_is_current());
            assert_eq!(
                field(&apply(&root, &request, &token), "result", "status"),
                "already-accepted"
            );
            let later = format!(
                "request base={} idempotency=later-test\nexpression.i64 as=$eight value=8\nreplace.body function=probe/growing body=$eight\n",
                repaired.head.revision
            );
            let later_token = plan(&root, &later);
            apply(&root, &later, &later_token);
            let later_head = repository.current().unwrap().head;
            assert_eq!(
                field(&apply(&root, &request, &token), "result", "status"),
                "already-accepted"
            );
            assert_eq!(repository.current().unwrap().head, later_head);
            let original = include_str!(
                "../../../tests/fixtures/finite-callable-predecessor/expanding.request"
            );
            let errors = cli::execute_change(args(&root, &["change", "plan", "--input", original]))
                .unwrap_err();
            assert_eq!(errors[0].code, "change_historical_request_incompatible");
            assert!(errors[0].message.contains(&old.head.revision.to_string()));
            assert!(
                errors
                    .iter()
                    .any(|error| error.code == "kernel_callable_expansion")
            );
            assert_eq!(repository.current().unwrap().head, later_head);
        }
    }
}

#[test]
fn independently_reconstructed_reference_rejects_before_expanding() {
    use crate::platform::execution::{
        ExecutionControl, ExecutionFailureClass, normalized::NormalizedReferenceSchema,
    };
    let temporary = tempfile::tempdir().unwrap();
    for (name, expanding) in [("valid", false), ("expanding", true)] {
        let root = temporary.path().join(name);
        fixture(name, &root);
        let repository = GraphRepository::open(&root).unwrap();
        let snapshot = repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        let result = NormalizedReferenceSchema::reconstruct([&snapshot]);
        if expanding {
            let error = result.unwrap_err();
            assert_eq!(error.code, "kernel_callable_expansion");
            assert_eq!(error.class, ExecutionFailureClass::Trap);
        } else {
            assert!(result.is_ok());
        }
        let control = ExecutionControl::uncancelled();
        control.cancel();
        assert_eq!(
            NormalizedReferenceSchema::reconstruct_with_control([&snapshot], &control)
                .unwrap_err()
                .class,
            ExecutionFailureClass::Cancelled
        );
    }
}

#[test]
fn historical_invalid_supplier_can_be_replaced_in_a_current_repair() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("dependency");
    fixture("dependency", &root);
    let repository = GraphRepository::open(&root).unwrap();
    let original = repository.current().unwrap();
    let before = inventory(&root);
    let view = repository.view_current().unwrap();
    assert_eq!(
        view.require_current_validation().unwrap_err().code,
        "kernel_callable_expansion"
    );
    cli::execute_query(args(&root, &["query", "owners", "--kind", "pure_function"])).unwrap();
    assert_eq!(inventory(&root), before);
    let container = temporary.path().join("fixed.lkjp");
    std::fs::write(
        &container,
        include_bytes!("../../../tests/fixtures/finite-callable-predecessor/supplier-fixed.lkjp"),
    )
    .unwrap();
    cli::execute_package_builtin(args(
        &root,
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            "package_transport_acb029502f3457e889b0f7f9209daec87c3660f21f876771ca93a17addae756c",
            "--input-file",
            container.to_str().unwrap(),
        ],
    ))
    .unwrap();
    assert_eq!(repository.current().unwrap().head, original.head);
    let request = format!(
        "request base={} idempotency=repair-supplier\nreplace.dependency package=pkg_5e9b07f0d0a8a9608be2a4a16df31c7f semantic-revision=rev_4999cefa5a3000dd85f487feb527811dfab7571144361712c1baadf374fe8fa3 package-revision=package_revision_46181ebcc4884383391524939b049718a022051ba9f05ebb87db3c2a9ab27b05\n",
        original.head.revision
    );
    let token = plan(&root, &request);
    apply(&root, &request, &token);
    let result = repository.current().unwrap();
    assert!(result.witness.contract_is_current());
    assert_eq!(
        result.revision.publication.parents,
        vec![original.accepted.parent()]
    );
    cli::execute_check(args(&root, &["check"])).unwrap();
    assert_eq!(
        field(&apply(&root, &request, &token), "revision", "result"),
        result.head.revision.to_string()
    );
}

#[test]
fn reused_validation_cannot_hide_corrupted_canonical_storage() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("cached");
    fixture("valid", &root);
    let repository = GraphRepository::open(&root).unwrap();
    let view = repository.view_current().unwrap();
    view.require_current_validation().unwrap();
    let snapshot = view.reconstruct_full_oracle().unwrap().value;
    let record = snapshot
        .owners
        .values()
        .find(|owner| matches!(owner, crate::platform::kernel::OwnerRecord::Expression(_)))
        .unwrap();
    let (_, encoded) = crate::platform::kernel::encode_owner(record).unwrap();
    let head = std::fs::read(root.join("HEAD")).unwrap();
    let warm = repository.view_current().unwrap();
    warm.require_current_validation().unwrap();
    println!(
        "current-validation cold={:?} warm={:?} origin={}",
        view.validation_work(),
        warm.validation_work(),
        warm.validation_origin()
    );
    let mut damaged = false;
    for entry in std::fs::read_dir(root.join("packs")).unwrap() {
        let path = entry.unwrap().path();
        let mut bytes = std::fs::read(&path).unwrap();
        if let Some(offset) = bytes
            .windows(encoded.len())
            .position(|window| window == encoded)
        {
            bytes[offset + encoded.len() - 1] ^= 1;
            std::fs::write(path, bytes).unwrap();
            damaged = true;
            break;
        }
    }
    assert!(damaged);
    let error = repository.view_current().unwrap_err();
    assert_eq!(
        error.class,
        crate::platform::DiagnosticClass::Corrupt,
        "{error:?}"
    );
    assert_eq!(std::fs::read(root.join("HEAD")).unwrap(), head);
}

#[test]
fn callable_body_edit_closes_cycle_through_unchanged_helper() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("flow");
    fixture("valid", &root);
    let repository = GraphRepository::open(&root).unwrap();
    let initial = repository.current().unwrap();
    let request = format!(
        "request base={} idempotency=finite-forward\n{}",
        initial.head.revision,
        r#"
type.parameter as=@T parameter=$T
type.list as=@list item=@T
expression.call as=$fbody function=$g
type.argument parent=$fbody index=0 type=@list
create.function as=$f module=probe name=flow_f visibility=private result=i64 effect=pure body=$fbody
add.type-parameter as=$T declaration=$f name=T
expression.i64 as=$gbody value=7
create.function as=$g module=probe name=flow_g visibility=private result=i64 effect=pure body=$gbody
add.type-parameter as=$U declaration=$g name=T
"#
    );
    let token = plan(&root, &request);
    apply(&root, &request, &token);
    let before = inventory(&root);
    let head = repository.current().unwrap().head;
    for named in [false, true] {
        let application = if named { "function-value" } else { "call" };
        let request = format!(
            "request base={} idempotency=join-cycle\nreference.owner as=$module package=local class=module name=probe\nreference.owner as=$g package=local class=declaration parent=$module name=flow_g\nreference.owner as=$U package=local class=type-parameter parent=$g name=T\ntype.parameter as=@U parameter=$U\nexpression.{application} as=$call function=probe/flow_f\ntype.argument parent=$call index=0 type=@U\n{}replace.body function=probe/flow_g body=$body\n",
            head.revision,
            if named {
                "expression.i64 as=$seven value=7\nexpression.sequence as=$body\nexpression.argument parent=$body index=0 expression=$call\nexpression.argument parent=$body index=1 expression=$seven\n"
            } else {
                "expression.sequence as=$body\nexpression.argument parent=$body index=0 expression=$call\n"
            }
        );
        let errors =
            cli::execute_change(args(&root, &["change", "plan", "--input", &request])).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == "kernel_callable_expansion"),
            "{errors:?}"
        );
        assert_eq!(inventory(&root), before);
    }
}

#[test]
fn historical_repair_interruption_and_stale_preparation_preserve_head() {
    use crate::platform::control::decode_compact_change;
    for point in [
        PublicationPoint::BeforeObjectStage,
        PublicationPoint::AfterPacksSealed,
        PublicationPoint::AfterHeadFileSynced,
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("interrupted");
        fixture("expanding", &root);
        let repository = GraphRepository::open(&root).unwrap();
        let original = repository.current().unwrap().head;
        let input = format!(
            "request base={} idempotency=interrupted-repair\nexpression.i64 as=$body value=7\nreplace.body function=probe/growing body=$body\n",
            original.revision
        );
        let request = decode_compact_change("repair", input.as_bytes()).unwrap();
        let prepared = repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap();
        assert_eq!(
            repository
                .publish_with_fault(&prepared.publication, point)
                .unwrap_err()
                .code,
            "publication_repository_injected_interruption"
        );
        assert_eq!(
            GraphRepository::open(&root)
                .unwrap()
                .current()
                .unwrap()
                .head,
            original
        );
        let input = input.replace("value=7", "value=8").replace(
            "idempotency=interrupted-repair",
            "idempotency=competing-repair",
        );
        let request = decode_compact_change("competing", input.as_bytes()).unwrap();
        let competing = repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap();
        repository.publish(&competing.publication).unwrap();
        let accepted = repository.current().unwrap().head;
        assert!(matches!(
            repository.publish(&prepared.publication).unwrap(),
            PublicationOutcome::Stale { .. }
        ));
        assert_eq!(repository.current().unwrap().head, accepted);
    }
}
