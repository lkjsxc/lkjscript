#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "public admission fixtures report assertion failures"
)]

mod support;

use lkjscript::platform::control::{CompactRecord, parse_records};
use std::path::{Path, PathBuf};
use std::process::Command;

type Record<'a> = (&'a str, &'a str, &'a [i64]);
type Schema<'a> = (&'a str, &'a str);

// Independent wire construction deliberately preserves malformed order and empty
// keys. A valid envelope digest is not proof that its facts are canonical.
fn seal(domain: &'static str, mut bytes: Vec<u8>) -> Vec<u8> {
    let mut hash = blake3::Hasher::new_derive_key(domain);
    hash.update(&(bytes.len() as u64).to_be_bytes());
    hash.update(&bytes);
    bytes.extend_from_slice(hash.finalize().as_bytes());
    bytes
}

fn blob(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value);
}

fn snapshot(records: &[Record<'_>], schemas: &[Schema<'_>], parent: bool) -> Vec<u8> {
    let mut bytes = b"LKJDREV1".to_vec();
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&[0; 32]);
    bytes.push(u8::from(parent));
    if parent {
        bytes.extend_from_slice(&[0xa5; 32]);
    }
    bytes.extend_from_slice(&u32::try_from(schemas.len()).unwrap().to_be_bytes());
    for (namespace, space) in schemas {
        blob(&mut bytes, namespace.as_bytes());
        blob(&mut bytes, space.as_bytes());
        blob(&mut bytes, b"schema-v1");
        blob(&mut bytes, &[1; 32]);
    }
    bytes.extend_from_slice(&u32::try_from(records.len()).unwrap().to_be_bytes());
    for (namespace, space, key) in records {
        blob(&mut bytes, namespace.as_bytes());
        blob(&mut bytes, space.as_bytes());
        bytes.extend_from_slice(&u32::try_from(key.len()).unwrap().to_be_bytes());
        for part in *key {
            bytes.push(1);
            bytes.extend_from_slice(&part.to_be_bytes());
        }
        blob(&mut bytes, b"value");
        bytes.extend_from_slice(&[0xa5; 32]);
    }
    seal("lkjscript.data.revision-envelope.v1", bytes)
}

fn backup(snapshot: &[u8]) -> Vec<u8> {
    let mut bytes = b"LKJDBAK1".to_vec();
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&[0x5a; 32]);
    blob(&mut bytes, snapshot);
    seal("lkjscript.data.backup-envelope.v1", bytes)
}

struct Consumer {
    root: tempfile::TempDir,
    executable: PathBuf,
}

impl Consumer {
    fn new() -> Self {
        let root = tempfile::TempDir::new().unwrap();
        let executable = root.path().join("lkjscript");
        let source = std::env::var_os("LKJSCRIPT_RELEASE_CANDIDATE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_lkjscript")));
        assert!(source.is_absolute());
        support::copy_executable(&source, &executable);
        Self { root, executable }
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
            "child crashed: {arguments:?}: status={} stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert_eq!(
            output.status.success(),
            succeeds,
            "{arguments:?}: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(output.stderr.is_empty());
        parse_records("data-admission", &output.stdout).unwrap()
    }

    fn restore(&self, bytes: &[u8], name: &str, succeeds: bool) -> Vec<CompactRecord> {
        let input = self.root.path().join(format!("{name}.lkjd"));
        let destination = self.root.path().join(name);
        std::fs::write(&input, bytes).unwrap();
        let inventory = self.inventory();
        let result = self.command(
            &[
                "data",
                "restore",
                "--backup",
                path(&input),
                "--root",
                path(&destination),
            ],
            succeeds,
        );
        assert_eq!(
            std::fs::read(&input).unwrap(),
            bytes,
            "restore changed input"
        );
        if !succeeds {
            assert!(!destination.exists(), "invalid restore exposed a root");
            assert_eq!(self.inventory(), inventory, "invalid restore left staging");
        }
        result
    }

    fn inventory(&self) -> Vec<PathBuf> {
        let mut entries = std::fs::read_dir(self.root.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        entries
    }
}

fn path(value: &Path) -> &str {
    value.to_str().unwrap()
}
fn field<'a>(records: &'a [CompactRecord], operation: &str, name: &str) -> &'a str {
    &records
        .iter()
        .find(|record| record.operation == operation)
        .unwrap()
        .fields
        .iter()
        .find(|field| field.name == name)
        .unwrap()
        .value
}

#[test]
fn copied_restore_rejects_noncanonical_facts_before_destination_publication() {
    let consumer = Consumer::new();
    for (name, records, schemas, parent, code) in [
        (
            "empty-key",
            vec![("app", "facts", &[][..])],
            vec![],
            false,
            "data_key_empty",
        ),
        (
            "key-order",
            vec![("app", "facts", &[1][..]), ("app", "facts", &[-1][..])],
            vec![],
            false,
            "data_record_order",
        ),
        (
            "namespace-order",
            vec![("z", "facts", &[0][..]), ("a", "facts", &[0][..])],
            vec![],
            false,
            "data_record_order",
        ),
        (
            "space-order",
            vec![("app", "z", &[0][..]), ("app", "a", &[0][..])],
            vec![],
            false,
            "data_record_order",
        ),
        (
            "prefix-order",
            vec![("app", "facts", &[0, 1][..]), ("app", "facts", &[0][..])],
            vec![],
            false,
            "data_record_order",
        ),
        (
            "duplicate-record",
            vec![("app", "facts", &[1][..]), ("app", "facts", &[1][..])],
            vec![],
            false,
            "data_record_duplicate",
        ),
        (
            "schema-space-order",
            vec![],
            vec![("app", "z"), ("app", "a")],
            false,
            "data_schema_order",
        ),
        (
            "schema-namespace-order",
            vec![],
            vec![("z", "facts"), ("a", "facts")],
            false,
            "data_schema_order",
        ),
        (
            "duplicate-schema",
            vec![],
            vec![("app", "facts"), ("app", "facts")],
            false,
            "data_schema_duplicate",
        ),
        (
            "backup-parent",
            vec![("app", "facts", &[0][..])],
            vec![],
            true,
            "data_backup_parent",
        ),
    ] {
        let result = consumer.restore(&backup(&snapshot(&records, &schemas, parent)), name, false);
        assert_eq!(field(&result, "diagnostic", "code"), code, "{name}");
        assert_eq!(field(&result, "diagnostic", "class"), "corrupt", "{name}");
    }
}

#[test]
fn canonical_signed_and_prefix_order_round_trips_without_normalization() {
    let consumer = Consumer::new();
    let logical = snapshot(
        &[
            ("app", "facts", &[-1]),
            ("app", "facts", &[0]),
            ("app", "facts", &[0, -1]),
            ("app", "facts", &[1]),
            ("app", "facts0", &[0]),
            ("other", "facts", &[i64::MIN]),
        ],
        &[("app", "a"), ("app", "z"), ("other", "a")],
        false,
    );
    consumer.restore(&backup(&logical), "valid", true);
    let root = consumer.root.path().join("valid");
    let head = std::fs::read(root.join("HEAD")).unwrap();
    let verified = consumer.command(&["data", "verify", "--root", path(&root)], true);
    assert_eq!(field(&verified, "data", "records"), "6");
    assert_eq!(field(&verified, "data", "schemas"), "3");
    let output = consumer.root.path().join("roundtrip.lkjd");
    consumer.command(
        &[
            "data",
            "backup",
            "--root",
            path(&root),
            "--output",
            path(&output),
        ],
        true,
    );
    let produced = std::fs::read(&output).unwrap();
    // Outer source revision changes on restore; the canonical logical payload does not.
    assert_eq!(&produced[50..produced.len() - 32], &logical);
    assert_eq!(std::fs::read(root.join("HEAD")).unwrap(), head);
    let empty = backup(&snapshot(&[], &[], false));
    consumer.restore(&empty, "empty-store", true);
}

#[test]
fn empty_prefix_remains_valid_but_a_resealed_empty_resume_key_rejects() {
    use lkjscript::platform::data::{
        DataExpectation, DataKey, DataKeyPart, DataLimits, DataScanDirection, DataStore,
    };
    let temporary = tempfile::TempDir::new().unwrap();
    let root = temporary.path().join("data");
    DataStore::initialize(&root).unwrap();
    let store = DataStore::open(&root, "app", DataLimits::default()).unwrap();
    let mut write = store.begin().unwrap();
    for value in [-1, 1] {
        let key = DataKey::new(vec![DataKeyPart::I64(value)], &DataLimits::default()).unwrap();
        assert!(
            write
                .put("facts", &key, vec![7], DataExpectation::Missing)
                .unwrap()
        );
    }
    write.commit().unwrap();
    let head = std::fs::read(root.join("HEAD")).unwrap();
    let read = store.begin().unwrap();
    for direction in [DataScanDirection::Forward, DataScanDirection::Reverse] {
        let first = read
            .scan("facts", &[], direction, 1, 1024, 2, None)
            .unwrap();
        let token = first.continuation.unwrap();
        // The actual last key is exactly one I64: count(4), tag(1), signed value(8).
        let mut malformed = token[..token.len() - 32 - 13].to_vec();
        malformed.extend_from_slice(&0u32.to_be_bytes());
        let malformed = seal("lkjscript.data.scan-continuation.v1", malformed);
        assert_eq!(
            read.scan("facts", &[], direction, 1, 1024, 2, Some(&malformed))
                .expect_err("an exclusive resume must be a record key, not an empty prefix")
                .code,
            "data_key_empty"
        );
        let next = read
            .scan("facts", &[], direction, 1, 1024, 2, Some(&token))
            .unwrap();
        assert_eq!(next.items.len(), 1);
        assert!(next.continuation.is_none());
    }
    assert_eq!(std::fs::read(root.join("HEAD")).unwrap(), head);
}

fn digest(domain: &'static str, bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

#[test]
fn physical_verification_and_transaction_loading_independently_admit_record_keys() {
    use lkjscript::platform::data::{DataLimits, DataStore};
    let consumer = Consumer::new();
    for (name, records, code) in [
        (
            "physical-empty",
            vec![("app", "facts", &[][..])],
            "data_key_empty",
        ),
        (
            "physical-order",
            vec![("app", "facts", &[1][..]), ("app", "facts", &[-1][..])],
            "data_record_order",
        ),
    ] {
        consumer.restore(&backup(&snapshot(&[], &[], false)), name, true);
        let root = consumer.root.path().join(name);
        let format = std::fs::read(root.join("FORMAT")).unwrap();
        let store_id = &format[10..42];
        let logical = snapshot(&records, &[], false);
        let mut payload = logical[..logical.len() - 32].to_vec();
        payload[10..42].copy_from_slice(store_id);
        let invalid = seal("lkjscript.data.revision-envelope.v1", payload);
        let identity = digest("lkjscript.data.revision.v1", &invalid);
        let filename = identity
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let object = root.join("objects").join(format!("{filename}.lkjd"));
        std::fs::write(&object, &invalid).unwrap();
        let mut head = b"LKJDHEAD".to_vec();
        head.extend_from_slice(&1u16.to_be_bytes());
        head.extend_from_slice(store_id);
        head.extend_from_slice(&identity);
        let head = seal("lkjscript.data.head-envelope.v1", head);
        // Controlled malformed physical input in this test's disposable root only.
        std::fs::write(root.join("HEAD"), &head).unwrap();
        let result = consumer.command(&["data", "verify", "--root", path(&root)], false);
        assert_eq!(field(&result, "diagnostic", "code"), code);
        let store = DataStore::open(&root, "app", DataLimits::default()).unwrap();
        assert_eq!(
            store
                .begin()
                .expect_err("transaction loaded a malformed physical key")
                .code,
            code
        );
        assert_eq!(std::fs::read(&object).unwrap(), invalid);
        assert_eq!(std::fs::read(root.join("HEAD")).unwrap(), head);
    }
}
