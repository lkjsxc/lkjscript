//! Public foreground command admission and real data visibility after numeric output failure.

use super::*;
use crate::platform::binary64::Binary64;
use crate::platform::data::{DataKey, DataKeyPart, DataLimits, DataStore};
use crate::platform::deployment::{AdapterDescriptor, DeploymentGrant};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use std::path::{Path, PathBuf};

const ABSENT_SECRET: &str = "LKJSCRIPT_F64_EFFECT_TEST_ABSENT_SECRET";
const NAMESPACE: &str = "f64-effect-test";

struct Fixture {
    directory: tempfile::TempDir,
    program: NormalizedProgram,
    stored: TypeObjectDigest,
}

fn member(snapshot: &crate::platform::kernel::KernelSnapshot, parent: &str, name: &str) -> String {
    let declaration = declaration_named(snapshot, parent).declaration;
    let members = snapshot
        .owners
        .iter()
        .filter_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Operation(id), OwnerRecord::Operation(record))
                if record.declaration == declaration && record.name.as_str() == name =>
            {
                Some(format!("{}/{}", snapshot.root.package_id, id))
            }
            (OwnerKey::Case(id), OwnerRecord::Case(record))
                if record.declaration == declaration && record.name.as_str() == name =>
            {
                Some(format!("{}/{}", snapshot.root.package_id, id))
            }
            (OwnerKey::Field(id), OwnerRecord::Field(record))
                if record.declaration == declaration && record.name.as_str() == name =>
            {
                Some(format!("{}/{}", snapshot.root.package_id, id))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(members.len(), 1, "one exact {parent}.{name}");
    members[0].clone()
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("authoring");
        let standard =
            GraphRepository::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/standard"))
                .unwrap()
                .view_current()
                .unwrap()
                .reconstruct_full_oracle()
                .unwrap()
                .value;
        let repository = GraphRepository::create(&source_path, &standard, None)
            .unwrap()
            .repository;
        let base = repository.view_current().unwrap().revision();
        let outcome = crate::platform::kernel::TransactionOutcomeContract::standard().unwrap();
        let request = format!(
            r#"request base={base}
create.module as=$module name=f64-effects
create.component as=$component module=$module name=f64-effects visibility=public
add.requirement as=$data component=$component name=numeric-store interface={store}
requirement.limit parent=$data index=0 name=maximum_calls maximum=32 unit=calls
requirement.operation parent=$data index=0 operation={get}
requirement.operation parent=$data index=1 operation={put}
requirement.operation parent=$data index=2 operation={transaction}
effect.row as=@Data
effect.requirement parent=@Data index=0 requirement=$data
type.named as=@KeyPart declaration={key_part}
type.named as=@Entry declaration={entry}
create.record as=$Stored module=$module name=F64Stored visibility=public
add.field as=$measurement record=$Stored name=measurement type=f64
type.named as=@Stored declaration=$Stored
type.application as=@Outcome declaration={outcome_type}
type.argument parent=@Outcome index=0 type=bool

expression.static-text as=$read-space value=numeric
expression.text as=$read-key-text value=one
expression.variant as=$read-key-part case={key_text} payload=$read-key-text
expression.list as=$read-key item=@KeyPart
expression.argument parent=$read-key index=0 expression=$read-key-part
expression.capability-call as=$get requirement=$data operation={get}
expression.argument parent=$get index=0 expression=$read-space
expression.argument parent=$get index=1 expression=$read-key
expression.local as=$length-input value=$entries
expression.call as=$length function={length}
type.argument parent=$length index=0 type=@Entry
expression.argument parent=$length index=0 expression=$length-input
expression.i64 as=$zero-length value=0
expression.call as=$absent function={integer_equal}
expression.argument parent=$absent index=0 expression=$length
expression.argument parent=$absent index=1 expression=$zero-length
expression.variant as=$missing case={missing}
expression.local as=$entry-input value=$entries
expression.i64 as=$entry-index value=0
expression.call as=$entry function={first}
type.argument parent=$entry index=0 type=@Entry
expression.argument parent=$entry index=0 expression=$entry-input
expression.argument parent=$entry index=1 expression=$entry-index
expression.field as=$revision value=$entry field={entry_revision}
expression.variant as=$exact case={exact} payload=$revision
expression.if as=$expectation condition=$absent when-true=$missing when-false=$exact
expression.local as=$input-value value=$input
expression.record as=$stored type=$Stored
expression.record-field parent=$stored index=0 field=$measurement value=$input-value
expression.call as=$encoded function={encode}
type.argument parent=$encoded index=0 type=@Stored
expression.argument parent=$encoded index=0 expression=$stored
expression.static-text as=$write-space value=numeric
expression.text as=$write-key-text value=one
expression.variant as=$write-key-part case={key_text} payload=$write-key-text
expression.list as=$write-key item=@KeyPart
expression.argument parent=$write-key index=0 expression=$write-key-part
expression.capability-call as=$put requirement=$data operation={put}
expression.argument parent=$put index=0 expression=$write-space
expression.argument parent=$put index=1 expression=$write-key
expression.argument parent=$put index=2 expression=$encoded
expression.argument parent=$put index=3 expression=$expectation
expression.let as=$algorithm body=$put
expression.binding parent=$algorithm index=0 as=$entries name=entries value=$get
expression.transaction-outcome as=$write-result requirement=$data binding=$transaction-owner name=transaction type=bool outcome={outcome_type} abort-reason={reason} committed={committed} aborted={aborted} condition-failed={condition} conflict={conflict} body=$algorithm
create.function as=$write module=$module name=f64-write visibility=public result=@Outcome effect=task body=$write-result
effect.requirement parent=$write index=0 requirement=$data
add.parameter as=$input function=$write name=value type=f64
type.task-function as=@Write result=@Outcome effect=@Data
type.argument parent=@Write index=0 type=f64
add.port as=$write-port component=$component name=write type=@Write function=$write
create.target as=$write-target name=f64-write component=$component port=$write-port runner=command

expression.local as=$failure-input value=$failure-parameter
expression.call as=$failure-write function=$write
expression.argument parent=$failure-write index=0 expression=$failure-input
expression.f64 as=$infinity value=inf
expression.sequence as=$failed-output
expression.argument parent=$failed-output index=0 expression=$failure-write
expression.argument parent=$failed-output index=1 expression=$infinity
create.function as=$failure module=$module name=f64-fail-output visibility=public result=f64 effect=task body=$failed-output
effect.requirement parent=$failure index=0 requirement=$data
add.parameter as=$failure-parameter function=$failure name=value type=f64
type.task-function as=@Failure result=f64 effect=@Data
type.argument parent=@Failure index=0 type=f64
add.port as=$failure-port component=$component name=failure type=@Failure function=$failure
create.target as=$failure-target name=f64-fail-output component=$component port=$failure-port runner=command

expression.local as=$encode-input value=$encode-parameter
expression.call as=$encode-write function=$write
expression.argument parent=$encode-write index=0 expression=$encode-input
expression.f64 as=$encode-infinity value=inf
expression.call as=$invalid-json function={json_encode}
type.argument parent=$invalid-json index=0 type=f64
expression.argument parent=$invalid-json index=0 expression=$encode-infinity
expression.sequence as=$encode-body
expression.argument parent=$encode-body index=0 expression=$encode-write
expression.argument parent=$encode-body index=1 expression=$invalid-json
create.function as=$encode-failure module=$module name=f64-fail-encode visibility=public result=bytes effect=task body=$encode-body
effect.requirement parent=$encode-failure index=0 requirement=$data
add.parameter as=$encode-parameter function=$encode-failure name=value type=f64
type.task-function as=@EncodeFailure result=bytes effect=@Data
type.argument parent=@EncodeFailure index=0 type=f64
add.port as=$encode-failure-port component=$component name=encode-failure type=@EncodeFailure function=$encode-failure
create.target as=$encode-failure-target name=f64-fail-encode component=$component port=$encode-failure-port runner=command

expression.f64 as=$zero value=0
create.function as=$supplier module=$module name=f64-supplier visibility=private result=f64 effect=pure body=$zero
type.function as=@Supplier result=f64
expression.local as=$unsupported-input value=$unsupported-parameter
expression.call as=$unsupported-write function=$write
expression.argument parent=$unsupported-write index=0 expression=$unsupported-input
expression.function-value as=$returned-function function=$supplier
expression.sequence as=$unsupported-body
expression.argument parent=$unsupported-body index=0 expression=$unsupported-write
expression.argument parent=$unsupported-body index=1 expression=$returned-function
create.function as=$unsupported module=$module name=f64-unencodable visibility=public result=@Supplier effect=task body=$unsupported-body
effect.requirement parent=$unsupported index=0 requirement=$data
add.parameter as=$unsupported-parameter function=$unsupported name=value type=f64
type.task-function as=@Unsupported result=@Supplier effect=@Data
type.argument parent=@Unsupported index=0 type=f64
add.port as=$unsupported-port component=$component name=unsupported type=@Unsupported function=$unsupported
create.target as=$unsupported-target name=f64-unencodable component=$component port=$unsupported-port runner=command

expression.static-text as=$reload-space value=numeric
expression.text as=$reload-key-text value=one
expression.variant as=$reload-key-part case={key_text} payload=$reload-key-text
expression.list as=$reload-key item=@KeyPart
expression.argument parent=$reload-key index=0 expression=$reload-key-part
expression.capability-call as=$reload-get requirement=$data operation={get}
expression.argument parent=$reload-get index=0 expression=$reload-space
expression.argument parent=$reload-get index=1 expression=$reload-key
expression.i64 as=$reload-index value=0
expression.call as=$reload-entry function={first}
type.argument parent=$reload-entry index=0 type=@Entry
expression.argument parent=$reload-entry index=0 expression=$reload-get
expression.argument parent=$reload-entry index=1 expression=$reload-index
expression.field as=$reload-bytes value=$reload-entry field={entry_value}
expression.f64 as=$fallback-number value=0
expression.record as=$fallback type=$Stored
expression.record-field parent=$fallback index=0 field=$measurement value=$fallback-number
expression.call as=$reloaded function={decode}
type.argument parent=$reloaded index=0 type=@Stored
expression.argument parent=$reloaded index=0 expression=$reload-bytes
expression.argument parent=$reloaded index=1 expression=$fallback
create.function as=$reload module=$module name=f64-read visibility=public result=@Stored effect=task body=$reloaded
effect.requirement parent=$reload index=0 requirement=$data
type.task-function as=@Read result=@Stored effect=@Data
add.port as=$read-port component=$component name=read type=@Read function=$reload
create.target as=$read-target name=f64-read component=$component port=$read-port runner=command
"#,
            store = declaration_named(&standard, "DataStore").declaration,
            key_part = declaration_named(&standard, "DataKeyPart").declaration,
            entry = declaration_named(&standard, "DataEntry").declaration,
            get = member(&standard, "DataStore", "get"),
            put = member(&standard, "DataStore", "put"),
            transaction = member(&standard, "DataStore", "transaction"),
            key_text = member(&standard, "DataKeyPart", "Text"),
            missing = member(&standard, "DataExpectation", "Missing"),
            exact = member(&standard, "DataExpectation", "Exact"),
            entry_revision = member(&standard, "DataEntry", "revision"),
            entry_value = member(&standard, "DataEntry", "value"),
            length = declaration_named(&standard, "list-length").declaration,
            integer_equal = declaration_named(&standard, "i64-equal").declaration,
            first = declaration_named(&standard, "list-get").declaration,
            encode = declaration_named(&standard, "data-encode").declaration,
            decode = declaration_named(&standard, "data-decode-or").declaration,
            json_encode = declaration_named(&standard, "json-encode").declaration,
            outcome_type = outcome.outcome.declaration,
            reason = outcome.abort_reason.declaration,
            committed = format_args!("{}/{}", outcome.committed.package, outcome.committed.case),
            aborted = format_args!("{}/{}", outcome.aborted.package, outcome.aborted.case),
            condition = format_args!(
                "{}/{}",
                outcome.condition_failed.package, outcome.condition_failed.case
            ),
            conflict = format_args!("{}/{}", outcome.conflict.package, outcome.conflict.case),
        );
        let decoded =
            crate::platform::control::decode_compact_change("f64-effects", request.as_bytes())
                .unwrap();
        let prepared = repository
            .prepare_authored_change(&decoded.semantic, decoded.options)
            .unwrap();
        assert!(matches!(
            repository.publish(&prepared.publication).unwrap(),
            PublicationOutcome::Accepted { .. }
        ));
        let snapshot = repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        let stored_declaration = declaration_named(&snapshot, "F64Stored");
        let stored = encode_type_object(
            &TypeObject::new(TypeForm::Named {
                declaration: stored_declaration,
            })
            .unwrap(),
        )
        .unwrap()
        .0;
        let compilation =
            build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
        let linked = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
        std::fs::write(
            directory.path().join("numeric.lkja"),
            &linked.artifact.bytes,
        )
        .unwrap();
        let program =
            NormalizedProgram::prepare(load_artifact(&linked.artifact.bytes).unwrap()).unwrap();
        drop(repository);
        std::fs::remove_dir_all(source_path).unwrap();
        Self {
            directory,
            program,
            stored,
        }
    }

    fn descriptor(&self, target: &str, missing_secret: bool) -> PathBuf {
        let mut descriptor = crate::platform::deployment::starter_command_deployment();
        descriptor.artifact = "numeric.lkja".to_owned();
        descriptor.target = target.to_owned();
        descriptor.grants.push(DeploymentGrant {
            requirement: "numeric-store".to_owned(),
            sharing_domain: "f64-effects".to_owned(),
            authority_revision: "7777777777777777777777777777777777777777777777777777777777777777"
                .to_owned(),
            adapter: AdapterDescriptor::Data {
                root: "store".to_owned(),
                namespace: NAMESPACE.to_owned(),
                limits: DataLimits::default(),
            },
        });
        if missing_secret {
            assert!(std::env::var_os(ABSENT_SECRET).is_none());
            descriptor
                .secrets
                .push(crate::platform::secrets::EnvironmentSecretBinding {
                    name: "preflight-sentinel".to_owned(),
                    variable: ABSENT_SECRET.to_owned(),
                });
        }
        let path = self
            .directory
            .path()
            .join(format!("{target}-{missing_secret}.json"));
        std::fs::write(
            &path,
            crate::platform::deployment::encode_deployment(&descriptor).unwrap(),
        )
        .unwrap();
        path
    }

    fn open_store(&self) -> DataStore {
        DataStore::open(
            &self.directory.path().join("store"),
            NAMESPACE,
            DataLimits::default(),
        )
        .unwrap()
    }
}

async fn command(descriptor: &Path, arguments: &str) -> Result<Vec<u8>, Diagnostic> {
    let options = crate::platform::cli::parse_foreground_run(&[
        "run".to_owned(),
        "--deployment".to_owned(),
        descriptor.to_str().unwrap().to_owned(),
        "--arguments".to_owned(),
        arguments.to_owned(),
    ])?;
    crate::platform::cli::execute_foreground_run(options, std::future::pending()).await
}

fn value(output: &[u8]) -> String {
    crate::platform::control::parse_records("f64-effect-output", output)
        .unwrap()
        .into_iter()
        .find(|record| record.operation == "execution")
        .unwrap()
        .fields
        .into_iter()
        .find(|field| field.name == "value")
        .unwrap()
        .value
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn f64_foreground_rejects_bad_numbers_and_unencodable_results_before_secrets_or_adapters() {
    let fixture = Fixture::new();
    let descriptor = fixture.descriptor("f64-fail-output", true);
    for (arguments, code) in [
        ("[1e400]", "json_decode"),
        ("[nan]", "json_decode"),
        ("[1.]", "json_decode"),
        ("[\"1.25\"]", "normalized_json_type"),
        ("[null]", "normalized_json_type"),
    ] {
        assert_eq!(
            command(&descriptor, arguments).await.unwrap_err().code,
            code,
            "{arguments}"
        );
        assert!(!fixture.directory.path().join("store").exists());
    }
    assert_eq!(
        command(&descriptor, "[1.25]").await.unwrap_err().code,
        "secret_missing"
    );
    let unsupported = fixture.descriptor("f64-unencodable", true);
    assert_eq!(
        command(&unsupported, "[1.25]").await.unwrap_err().code,
        "normalized_json_type"
    );
    let ordinary = fixture.descriptor("f64-fail-output", false);
    assert_eq!(
        command(&ordinary, "[1.25]").await.unwrap_err().code,
        "normalized_deployment_directory_missing"
    );
    assert!(!fixture.directory.path().join("store").exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn f64_committed_data_survives_foreground_and_builtin_json_failure_without_replay() {
    let fixture = Fixture::new();
    DataStore::initialize(&fixture.directory.path().join("store")).unwrap();
    let initial = fixture.open_store().verify().unwrap();
    let failed = fixture.descriptor("f64-fail-output", false);
    let error = command(&failed, "[-0.0]").await.unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic);
    assert_eq!(error.code, "normalized_json_nonfinite");
    assert!(error.notes.iter().any(|note| note.contains(
        "earlier application effects may already be visible; automatic retry is not safe"
    )));
    assert!(
        error
            .notes
            .iter()
            .any(|note| note.contains("remaining-owned-tasks=0 failures=0"))
    );
    let store = fixture.open_store();
    let committed = store.verify().unwrap();
    assert_eq!(
        committed.revisions,
        initial.revisions + 1,
        "each invocation writes a new exact entry revision, so a retry cannot hide"
    );
    assert_eq!(committed.records, 1);
    assert_eq!(committed.staging_leftovers, 0);
    let key = DataKey::new(
        vec![DataKeyPart::Text("one".to_owned())],
        &DataLimits::default(),
    )
    .unwrap();
    let transaction = store.begin().unwrap();
    let stored = transaction.get("numeric", &key).unwrap().unwrap();
    let decoded =
        super::super::data_codec::decode_typed(&fixture.program, &stored.value, fixture.stored)
            .unwrap();
    let super::super::value::NormalizedValue::Record(
        super::super::value::NormalizedRecord::Nominal { fields, .. },
    ) = decoded
    else {
        panic!("nominal stored F64 value")
    };
    assert_eq!(
        fields.as_slice(),
        &[NormalizedValue::F64(
            Binary64::from_bits(0x8000_0000_0000_0000).unwrap()
        )]
    );
    drop(transaction);
    drop(store);
    let read = fixture.descriptor("f64-read", false);
    assert_eq!(
        value(&command(&read, "[]").await.unwrap()),
        "{\"measurement\":-0.0}"
    );
    assert_eq!(
        fixture.open_store().verify().unwrap().revision,
        committed.revision
    );
    let success = fixture.descriptor("f64-write", false);
    assert_eq!(
        value(&command(&success, "[1.25]").await.unwrap()),
        "{\"case\":\"Committed\",\"value\":true}"
    );
    assert_eq!(
        value(&command(&read, "[]").await.unwrap()),
        "{\"measurement\":1.25}"
    );
    let before_builtin = fixture.open_store().verify().unwrap();
    let builtin_failure = fixture.descriptor("f64-fail-encode", false);
    let error = command(&builtin_failure, "[2.5]").await.unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic);
    assert_eq!(error.code, "normalized_json_nonfinite");
    assert!(
        error
            .notes
            .iter()
            .any(|note| note.contains("remaining-owned-tasks=0 failures=0"))
    );
    assert!(
        error
            .notes
            .iter()
            .any(|note| note.contains("automatic retry is not safe"))
    );
    let after_builtin = fixture.open_store().verify().unwrap();
    assert_eq!(after_builtin.revisions, before_builtin.revisions + 1);
    assert_eq!(after_builtin.staging_leftovers, 0);
    assert_eq!(
        value(&command(&read, "[]").await.unwrap()),
        "{\"measurement\":2.5}"
    );
    let unsupported = fixture.descriptor("f64-unencodable", false);
    assert_eq!(
        command(&unsupported, "[3.5]").await.unwrap_err().code,
        "normalized_json_type"
    );
    assert_eq!(
        fixture.open_store().verify().unwrap().revision,
        after_builtin.revision
    );
}
