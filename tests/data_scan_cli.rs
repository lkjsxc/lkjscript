#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "public subprocess acceptance reports assertion failures"
)]

mod support;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lkjscript::platform::control::{CompactRecord, parse_records};
use lkjscript::platform::data::DataLimits;
use lkjscript::platform::stream::StreamLimits;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

struct Consumer {
    root: tempfile::TempDir,
    executable: PathBuf,
}

impl Consumer {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("lkjscript");
        let source = std::env::var_os("LKJSCRIPT_RELEASE_CANDIDATE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_lkjscript")));
        assert!(source.is_absolute());
        assert!(std::fs::symlink_metadata(&source).unwrap().is_file());
        support::copy_executable(&source, &executable);
        let consumer = Self { root, executable };
        let author = consumer.root.path().join("author");
        consumer.success(&[
            "new",
            path(&author),
            "--template",
            "command",
            "--name",
            "scan",
        ]);
        let status = consumer.success(&["--project", path(&author), "status"]);
        let revision = field(record(&status, "revision"), "id");
        let request = consumer.root.path().join("scan.request");
        std::fs::write(
            &request,
            format!(
                "request base={revision} idempotency=scan-consumer\n{}",
                include_str!("fixtures/data-scan.lkchg")
            ),
        )
        .unwrap();
        let plan = consumer.success(&[
            "--project",
            path(&author),
            "change",
            "plan",
            "--input-file",
            path(&request),
        ]);
        consumer.success(&[
            "--project",
            path(&author),
            "change",
            "apply",
            "--input-file",
            path(&request),
            "--plan",
            field(record(&plan, "plan"), "token"),
        ]);
        consumer.success(&["--project", path(&author), "check"]);
        let artifact = consumer.root.path().join("scan.lkja");
        consumer.success(&[
            "--project",
            path(&author),
            "build",
            "--output",
            path(&artifact),
        ]);
        for target in ["write", "page"] {
            let descriptor = json!({
                "artifact":"scan.lkja", "target":target, "listen":null,
                "http":null, "session":null, "worker":null,
                "streams":StreamLimits::default(), "configuration":{}, "secrets":[],
                "grants":[{
                    "requirement":"data", "sharing_domain":"scan-data",
                    "authority_revision":"82".repeat(32),
                    "adapter":{
                        "kind":"data", "root":"data", "namespace":"scan",
                        "limits":DataLimits{maximum_live_transactions:1,..Default::default()}
                    }
                }]
            });
            std::fs::write(
                consumer
                    .root
                    .path()
                    .join(format!("{target}.deployment.json")),
                serde_json::to_vec(&descriptor).unwrap(),
            )
            .unwrap();
        }
        consumer.success(&[
            "data",
            "initialize",
            "--root",
            path(&consumer.root.path().join("data")),
        ]);
        // Only this fixture's owned authoring checkout is removed. Every read and
        // write below uses the distributed artifact, never host-language storage.
        std::fs::remove_dir_all(author).unwrap();
        consumer
    }

    fn command(&self, arguments: &[&str], succeeds: bool) -> Vec<CompactRecord> {
        let output = support::output(
            Command::new(&self.executable)
                .args(arguments)
                .current_dir(self.root.path())
                .env_clear()
                .env("LANG", "C"),
        )
        .unwrap();
        assert!(
            output.status.code().is_some(),
            "child crashed: {arguments:?}"
        );
        assert_eq!(
            output.status.success(),
            succeeds,
            "{arguments:?}: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(output.stderr.is_empty(), "unexpected process-level error");
        parse_records("scan-cli", &output.stdout).unwrap()
    }

    fn success(&self, arguments: &[&str]) -> Vec<CompactRecord> {
        self.command(arguments, true)
    }

    fn invoke(&self, target: &str, arguments: Value, succeeds: bool) -> Vec<CompactRecord> {
        let descriptor = self.root.path().join(format!("{target}.deployment.json"));
        self.command(
            &[
                "run",
                "--deployment",
                path(&descriptor),
                "--arguments",
                &arguments.to_string(),
            ],
            succeeds,
        )
    }

    fn run(&self, target: &str, arguments: Value) -> Value {
        let records = self.invoke(target, arguments, true);
        let execution = record(&records, "execution");
        assert_eq!(field(execution, "execution-mode"), "production");
        assert_eq!(field(execution, "verification"), "not-performed");
        let cleanup: Value = serde_json::from_str(field(execution, "cleanup")).unwrap();
        assert_eq!(cleanup["remaining_tasks"], 0);
        assert_eq!(cleanup["cleanup_failures"], json!([]));
        let observed: Value =
            serde_json::from_str(field(execution, "production-observation")).unwrap();
        assert_eq!(observed["live_transactions_after"], 0);
        serde_json::from_str(field(execution, "value")).unwrap()
    }

    fn page(&self, prefix: &Value, direction: &str, cursor: &Value) -> Value {
        self.run(
            "page",
            json!([prefix, {"case":direction}, 3, 4096, 2, cursor]),
        )
    }
}

fn path(path: &Path) -> &str {
    path.to_str().unwrap()
}
fn record<'a>(records: &'a [CompactRecord], name: &str) -> &'a CompactRecord {
    records
        .iter()
        .find(|record| record.operation == name)
        .unwrap_or_else(|| panic!("missing {name}: {records:#?}"))
}
fn field<'a>(record: &'a CompactRecord, name: &str) -> &'a str {
    &record
        .fields
        .iter()
        .find(|field| field.name == name)
        .unwrap_or_else(|| panic!("missing {name}: {record:#?}"))
        .value
}
fn bytes(value: &[u8]) -> Value {
    json!({"$bytes":STANDARD.encode(value)})
}
fn part(case: &str, value: Value) -> Value {
    json!({"case":case, "value":value})
}

#[test]
fn copied_artifact_pages_persisted_typed_keys_across_processes_and_rejects_stale_tokens() {
    let consumer = Consumer::new();
    // Literal semantic order, independent of the implementation's comparator.
    let parts = vec![
        part("Bool", json!(false)),
        part("Bool", json!(true)),
        part("I64", json!(i64::MIN)),
        part("I64", json!(-1)),
        part("I64", json!(i64::MAX)),
        part("Text", json!("")),
        part("Text", json!("a")),
        part("Text", json!("a\0")),
        part("Text", json!("ab")),
        part("Text", json!("é")),
        part("Bytes", bytes(&[])),
        part("Bytes", bytes(&[0])),
        part("Bytes", bytes(&[255])),
    ];
    let mut expected = Vec::new();
    for first in &parts {
        for key in [json!([first]), json!([first, part("I64", json!(7))])] {
            let value = bytes(format!("row-{}", expected.len()).as_bytes());
            assert_eq!(consumer.run("write", json!([key, value])), true);
            expected.push((key, value));
        }
    }
    let mut all_forward = Vec::new();
    for prefix in [
        json!([]),
        json!([part("Text", json!("a"))]),
        json!([part("I64", json!(42))]),
    ] {
        let selected: Vec<_> = expected
            .iter()
            .filter(|(key, _)| {
                key.as_array()
                    .unwrap()
                    .starts_with(prefix.as_array().unwrap())
            })
            .cloned()
            .collect();
        for direction in ["Forward", "Reverse"] {
            let mut cursor = bytes(&[]);
            let mut actual = Vec::new();
            for _ in 0..=selected.len() {
                let page = consumer.page(&prefix, direction, &cursor);
                assert!(page["work"].as_u64().unwrap() <= 2);
                assert!(page["bytes"].as_u64().unwrap() <= 4096);
                let items = page["items"].as_array().unwrap();
                assert!(items.len() <= 2);
                for item in items {
                    assert_eq!(
                        STANDARD
                            .decode(item["revision"]["$bytes"].as_str().unwrap())
                            .unwrap()
                            .len(),
                        32
                    );
                    actual.push(item.clone());
                }
                cursor = page["continuation"].clone();
                if cursor == bytes(&[]) {
                    break;
                }
                assert!(!items.is_empty(), "continuation must make progress");
            }
            assert_eq!(cursor, bytes(&[]), "pagination failed to terminate");
            let mut oracle = selected.clone();
            if direction == "Reverse" {
                oracle.reverse();
            }
            assert_eq!(
                actual
                    .iter()
                    .map(|item| (item["key"].clone(), item["value"].clone()))
                    .collect::<Vec<_>>(),
                oracle
            );
            if prefix == json!([]) {
                if direction == "Forward" {
                    all_forward = actual;
                } else {
                    all_forward.reverse();
                    assert_eq!(actual, all_forward);
                }
            }
        }
    }
    let first = consumer.page(&json!([]), "Forward", &bytes(&[]));
    let token = first["continuation"].clone();
    assert_ne!(token, bytes(&[]));
    for (arguments, code) in [
        (
            json!([[], {"case":"Reverse"}, 3, 4096, 2, token]),
            "data_continuation_selector",
        ),
        (
            json!([[], {"case":"Forward"}, 4, 4096, 2, token]),
            "data_continuation_selector",
        ),
        (
            json!([[part("Text", json!("a"))], {"case":"Forward"}, 3, 4096, 2, token]),
            "data_continuation_selector",
        ),
        (
            json!([[], {"case":"Forward"}, 3, 4096, 2, bytes(b"truncated")]),
            "data_continuation_checksum",
        ),
        (
            json!([[], {"case":"Forward"}, 3, 1, 2, bytes(&[])]),
            "data_scan_item_bytes",
        ),
    ] {
        let failure = consumer.invoke("page", arguments, false);
        assert_eq!(field(record(&failure, "diagnostic"), "code"), code);
    }
    assert_eq!(consumer.page(&json!([]), "Forward", &bytes(&[])), first);
    // A committed intervening write invalidates the old revision-bound cursor;
    // rejected cursors must not mutate or corrupt the durable store.
    assert_eq!(
        consumer.run(
            "write",
            json!([[part("Text", json!("later"))], bytes(b"later")])
        ),
        true
    );
    let stale = consumer.invoke(
        "page",
        json!([[], {"case":"Forward"}, 3, 4096, 2, token]),
        false,
    );
    assert_eq!(
        field(record(&stale, "diagnostic"), "code"),
        "data_continuation_selector"
    );
    let later = consumer.page(
        &json!([part("Text", json!("later"))]),
        "Forward",
        &bytes(&[]),
    );
    assert_eq!(later["items"].as_array().unwrap().len(), 1);
    assert_eq!(later["items"][0]["value"], bytes(b"later"));
    assert_eq!(later["continuation"], bytes(&[]));
}
