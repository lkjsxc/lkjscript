//! Current publication validation, immutable reads, and callable admission boundaries.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::platform::{Diagnostic, DiagnosticClass, cli, control::parse_records};
use std::{collections::BTreeMap, path::Path};

fn fixture(destination: &Path) {
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(destination, &logical, None).unwrap();
    let request = format!(
        "request base={} idempotency=current-fixture\n{}",
        created.current.head.revision,
        r#"
create.module as=$module name=probe
type.parameter as=@T parameter=$T
expression.bool as=$stop value=true
expression.i64 as=$seven value=7
expression.call as=$recursive function=$growing
type.argument parent=$recursive index=0 type=@T
expression.if as=$body condition=$stop when-true=$seven when-false=$recursive
create.function as=$growing module=$module name=growing visibility=public result=i64 effect=pure body=$body
add.type-parameter as=$T declaration=$growing name=T
"#
    );
    let token = plan(destination, &request);
    apply(destination, &request, &token);
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
    let records = parse_records("validation-response", bytes).unwrap();
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
fn same_epoch_persisted_witness_requires_a_current_process_proof() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("same-epoch");
    let mut logical = crate::platform::kernel::tests::witness_snapshot();
    logical.root.repository_id =
        crate::platform::semantic_id::RepositoryId::migrate(b"same-epoch-current-process-proof", 0);
    let created = GraphRepository::create(&root, &logical, None).unwrap();
    let before = inventory(&root);
    let repository = GraphRepository::open(&root).unwrap();
    let current = repository.current().unwrap();
    assert!(current.witness.contract_is_current());
    assert_eq!(
        current.witness.graph_contract_version,
        crate::platform::canonical_schema::SCHEMA_EPOCH
    );
    // Matching serialized tags cannot establish that this process ran validation.
    let unproven = RepositoryView::new(current, repository.object_store().unwrap());
    assert_eq!(
        unproven.require_current_validation().unwrap_err().code,
        "publication_current_validation"
    );
    let proven = unproven.reconcile_validation().unwrap();
    proven.require_current_validation().unwrap();
    assert_eq!(proven.validation_origin(), "rebuilt");
    assert!(proven.validation_work().1 > 0);
    assert_eq!(repository.current().unwrap().head, created.current.head);
    assert_eq!(inventory(&root), before);
    repository
        .view_current()
        .unwrap()
        .require_current_validation()
        .unwrap();
    assert_eq!(inventory(&root), before);
}

#[test]
fn cold_and_reused_process_proofs_produce_identical_candidate_receipts() {
    let memory = crate::platform::MemProjectStore::new(None).unwrap();
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    snapshot.root.repository_id =
        crate::platform::semantic_id::RepositoryId::migrate(b"cold-warm-candidate-receipt", 0);
    let created = GraphRepository::create_memory(&memory, &snapshot, None).unwrap();
    let cold = created.repository.view_current().unwrap();
    let warm = created.repository.view_current().unwrap();
    assert!(cold.validation_work().1 > 0);
    assert_eq!(warm.validation_work().1, 0);
    let original = snapshot
        .owners
        .values()
        .find(|owner| matches!(owner, crate::platform::kernel::OwnerRecord::Module(_)))
        .unwrap();
    let mut replacement = original.clone();
    if let crate::platform::kernel::OwnerRecord::Module(module) = &mut replacement {
        module.name = crate::platform::kernel::Name::new("same_candidate").unwrap();
    }
    let edits = vec![crate::platform::change::PrimitiveEdit::ReplaceOwner {
        expected: crate::platform::kernel::encode_owner(original).unwrap().0,
        record: replacement,
    }];
    let cold_plan = cold
        .prepare_change(edits.clone(), PublicationOptions::default())
        .unwrap();
    let warm_plan = warm
        .prepare_change(edits, PublicationOptions::default())
        .unwrap();
    assert_eq!(cold_plan.head_bytes, warm_plan.head_bytes);
    assert_eq!(cold_plan.receipt_bytes, warm_plan.receipt_bytes);
    assert_eq!(cold_plan.objects, warm_plan.objects);
    assert_eq!(cold_plan.budget_work, warm_plan.budget_work);
}

#[test]
fn current_validation_and_preparation_propagate_cancellation_without_writes() {
    use crate::platform::execution::ExecutionControl;
    let temporary = tempfile::tempdir().unwrap();
    {
        let root = temporary.path().join("current");
        fixture(&root);
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
        let errors = crate::platform::kernel::test_work::validate_full_checked(
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
fn independently_reconstructed_reference_rejects_before_expanding() {
    use crate::platform::execution::{
        ExecutionControl, ExecutionFailureClass, normalized::NormalizedReferenceSchema,
    };
    use crate::platform::kernel::{
        ExpressionOperation, OwnerKey, OwnerRecord, TypeForm, TypeObject, encode_type_object,
    };

    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("reference");
    fixture(&root);
    let repository = GraphRepository::open(&root).unwrap();
    let snapshot = repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    assert!(NormalizedReferenceSchema::reconstruct([&snapshot]).is_ok());
    let growing = snapshot
        .owners
        .values()
        .find_map(|owner| {
            let OwnerRecord::Declaration(declaration) = owner else {
                return None;
            };
            let OwnerKey::Declaration(id) = declaration.header.owner else {
                return None;
            };
            (declaration.name.as_str() == "growing").then_some(id)
        })
        .unwrap();
    let mut expanding = snapshot.clone();
    let recursive = expanding
        .owners
        .values_mut()
        .find_map(|owner| {
            let OwnerRecord::Expression(expression) = owner else {
                return None;
            };
            let ExpressionOperation::Call {
                function,
                type_arguments,
                ..
            } = &mut expression.operation
            else {
                return None;
            };
            (function.declaration == growing).then_some(type_arguments)
        })
        .unwrap();
    let list = TypeObject::new(TypeForm::List { item: recursive[0] }).unwrap();
    let (digest, _) = encode_type_object(&list).unwrap();
    recursive[0] = digest;
    expanding.types.insert(digest, list);
    let error = NormalizedReferenceSchema::reconstruct([&expanding]).unwrap_err();
    assert_eq!(error.code, "kernel_callable_expansion");
    assert_eq!(error.class, ExecutionFailureClass::Trap);

    let control = ExecutionControl::uncancelled();
    control.cancel();
    assert_eq!(
        NormalizedReferenceSchema::reconstruct_with_control([&snapshot], &control)
            .unwrap_err()
            .class,
        ExecutionFailureClass::Cancelled
    );
}

#[test]
fn accepted_validation_does_not_hide_corrupted_canonical_storage() {
    use crate::platform::storage::object::{ObjectDomain, ObjectKey};
    use std::io::{Read, Seek, SeekFrom, Write};

    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("cached");
    fixture(&root);
    let repository = GraphRepository::open(&root).unwrap();
    let view = repository.view_current().unwrap();
    view.require_current_validation().unwrap();
    let snapshot = view.reconstruct_full_oracle().unwrap().value;
    let record = snapshot
        .owners
        .values()
        .find(|owner| matches!(owner, crate::platform::kernel::OwnerRecord::Expression(_)))
        .unwrap();
    let owner = record.owner();
    let (digest, _) = crate::platform::kernel::encode_owner(record).unwrap();
    let head = std::fs::read(root.join("HEAD")).unwrap();
    let warm = repository.view_current().unwrap();
    warm.require_current_validation().unwrap();
    let (exported, _) = repository
        .view_current_with_source(&crate::platform::execution::ExecutionControl::uncancelled())
        .unwrap();
    let location = repository
        .object_store()
        .unwrap()
        .catalog_location(ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()))
        .unwrap()
        .unwrap();
    let pack = root.join("packs").join(location.pack.file_name());
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(pack)
        .unwrap();
    file.seek(SeekFrom::Start(location.offset)).unwrap();
    let mut byte = [0_u8; 1];
    file.read_exact(&mut byte).unwrap();
    byte[0] ^= 0x80;
    file.seek(SeekFrom::Start(location.offset)).unwrap();
    file.write_all(&byte).unwrap();
    file.sync_all().unwrap();

    // Opening rechecks the complete cached read set before exposing current proof.
    let error = repository.view_current().unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Corrupt, "{error:?}");
    let error = warm.owner(owner).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Corrupt, "{error:?}");
    let error = repository.recheck_exported_source(&exported).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Corrupt, "{error:?}");
    assert_eq!(std::fs::read(root.join("HEAD")).unwrap(), head);
}

#[test]
fn callable_body_edit_closes_cycle_through_unchanged_helper() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("flow");
    fixture(&root);
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
fn current_change_interruption_and_stale_preparation_preserve_head() {
    use crate::platform::control::decode_compact_change;
    for point in [
        PublicationPoint::BeforeObjectStage,
        PublicationPoint::AfterPacksSealed,
        PublicationPoint::AfterHeadFileSynced,
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("interrupted");
        fixture(&root);
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
