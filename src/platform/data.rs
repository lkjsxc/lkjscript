//! First-party ordered transactional application-data store.
//!
//! This physical authority is deliberately separate from the accepted program-meaning graph.
//! Accepted revisions are immutable complete snapshots and `HEAD` is the only visibility point.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use fs2::FileExt;
use rustix::fs::{CWD, RenameFlags, renameat_with};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

pub const DATA_STORE_CONTRACT_VERSION: u16 = 1;
pub const DATA_BACKUP_CONTRACT_VERSION: u16 = 1;
pub const DATA_STORE_CONTRACT_IDENTITY: &str = "lkjscript-data-store-1";
pub const DATA_BACKUP_CONTRACT_IDENTITY: &str = "lkjscript-data-backup-1";

pub const MAXIMUM_DATA_SPACE_NAME_BYTES: usize = 128;
pub const MAXIMUM_DATA_NAMESPACE_BYTES: usize = 128;
pub const MAXIMUM_DATA_KEY_PARTS: usize = 16;
pub const MAXIMUM_DATA_KEY_BYTES: usize = 4 * 1_024;
pub const MAXIMUM_DATA_VALUE_BYTES: usize = 4 * 1_048_576;
pub const MAXIMUM_DATA_TRANSACTION_MUTATIONS: usize = 4_096;
pub const MAXIMUM_DATA_TRANSACTION_BYTES: usize = 16 * 1_048_576;
pub const MAXIMUM_DATA_SCAN_ITEMS: usize = 10_000;
pub const MAXIMUM_DATA_SCAN_BYTES: usize = 16 * 1_048_576;
pub const MAXIMUM_DATA_SCAN_WORK: usize = 1_000_000;
pub const MAXIMUM_DATA_LIVE_TRANSACTIONS: usize = 1_024;
pub const MAXIMUM_DATA_REVISION_BYTES: usize = 1_073_741_824;
pub const MAXIMUM_DATA_BACKUP_BYTES: usize = 1_073_741_824;
pub const MAXIMUM_DATA_HISTORY_REVISIONS: usize = 1_000_000;
pub const MAXIMUM_DATA_STORE_OBJECTS: usize = 1_000_000;

const FORMAT_FILE: &str = "FORMAT";
const LOCK_FILE: &str = "LOCK";
const HEAD_FILE: &str = "HEAD";
const OBJECTS_DIRECTORY: &str = "objects";
const STAGING_DIRECTORY: &str = "staging";
const CATALOG_DIRECTORY: &str = "catalog";
const CATALOG_FILE: &str = "CURRENT";
const FORMAT_MAGIC: &[u8; 8] = b"LKJDATA1";
const REVISION_MAGIC: &[u8; 8] = b"LKJDREV1";
const HEAD_MAGIC: &[u8; 8] = b"LKJDHEAD";
const BACKUP_MAGIC: &[u8; 8] = b"LKJDBAK1";
const CATALOG_MAGIC: &[u8; 8] = b"LKJDCAT1";
const FORMAT_DIGEST_DOMAIN: &str = "lkjscript.data.format.v1";
const REVISION_DIGEST_DOMAIN: &str = "lkjscript.data.revision.v1";
const REVISION_ENVELOPE_DOMAIN: &str = "lkjscript.data.revision-envelope.v1";
const HEAD_ENVELOPE_DOMAIN: &str = "lkjscript.data.head-envelope.v1";
const BACKUP_ENVELOPE_DOMAIN: &str = "lkjscript.data.backup-envelope.v1";
const CATALOG_ENVELOPE_DOMAIN: &str = "lkjscript.data.catalog-envelope.v1";
const ENTRY_REVISION_DOMAIN: &str = "lkjscript.data.entry-revision.v1";
const CONTINUATION_DOMAIN: &str = "lkjscript.data.scan-continuation.v1";
const MAXIMUM_STAGING_LEFTOVERS: usize = 4_096;
const MAXIMUM_DATA_CATALOG_BYTES: usize = 64 * 1_048_576;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataLimits {
    pub maximum_space_name_bytes: usize,
    pub maximum_key_parts: usize,
    pub maximum_key_bytes: usize,
    pub maximum_value_bytes: usize,
    pub maximum_transaction_mutations: usize,
    pub maximum_transaction_bytes: usize,
    pub maximum_scan_items: usize,
    pub maximum_scan_bytes: usize,
    pub maximum_scan_work: usize,
    pub maximum_live_transactions: usize,
}

impl Default for DataLimits {
    fn default() -> Self {
        Self {
            maximum_space_name_bytes: MAXIMUM_DATA_SPACE_NAME_BYTES,
            maximum_key_parts: MAXIMUM_DATA_KEY_PARTS,
            maximum_key_bytes: MAXIMUM_DATA_KEY_BYTES,
            maximum_value_bytes: MAXIMUM_DATA_VALUE_BYTES,
            maximum_transaction_mutations: MAXIMUM_DATA_TRANSACTION_MUTATIONS,
            maximum_transaction_bytes: MAXIMUM_DATA_TRANSACTION_BYTES,
            maximum_scan_items: MAXIMUM_DATA_SCAN_ITEMS,
            maximum_scan_bytes: MAXIMUM_DATA_SCAN_BYTES,
            maximum_scan_work: MAXIMUM_DATA_SCAN_WORK,
            maximum_live_transactions: MAXIMUM_DATA_LIVE_TRANSACTIONS,
        }
    }
}

impl DataLimits {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        validate_limit(
            "maximum_space_name_bytes",
            self.maximum_space_name_bytes,
            MAXIMUM_DATA_SPACE_NAME_BYTES,
        )?;
        validate_limit(
            "maximum_key_parts",
            self.maximum_key_parts,
            MAXIMUM_DATA_KEY_PARTS,
        )?;
        validate_limit(
            "maximum_key_bytes",
            self.maximum_key_bytes,
            MAXIMUM_DATA_KEY_BYTES,
        )?;
        validate_limit(
            "maximum_value_bytes",
            self.maximum_value_bytes,
            MAXIMUM_DATA_VALUE_BYTES,
        )?;
        validate_limit(
            "maximum_transaction_mutations",
            self.maximum_transaction_mutations,
            MAXIMUM_DATA_TRANSACTION_MUTATIONS,
        )?;
        validate_limit(
            "maximum_transaction_bytes",
            self.maximum_transaction_bytes,
            MAXIMUM_DATA_TRANSACTION_BYTES,
        )?;
        validate_limit(
            "maximum_scan_items",
            self.maximum_scan_items,
            MAXIMUM_DATA_SCAN_ITEMS,
        )?;
        validate_limit(
            "maximum_scan_bytes",
            self.maximum_scan_bytes,
            MAXIMUM_DATA_SCAN_BYTES,
        )?;
        validate_limit(
            "maximum_scan_work",
            self.maximum_scan_work,
            MAXIMUM_DATA_SCAN_WORK,
        )?;
        validate_limit(
            "maximum_live_transactions",
            self.maximum_live_transactions,
            MAXIMUM_DATA_LIVE_TRANSACTIONS,
        )
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum DataKeyPart {
    Bool(bool),
    I64(i64),
    Text(String),
    Bytes(Vec<u8>),
}

impl Ord for DataKeyPart {
    fn cmp(&self, other: &Self) -> Ordering {
        let tag = |part: &Self| match part {
            Self::Bool(_) => 0_u8,
            Self::I64(_) => 1,
            Self::Text(_) => 2,
            Self::Bytes(_) => 3,
        };
        tag(self)
            .cmp(&tag(other))
            .then_with(|| match (self, other) {
                (Self::Bool(left), Self::Bool(right)) => left.cmp(right),
                (Self::I64(left), Self::I64(right)) => left.cmp(right),
                (Self::Text(left), Self::Text(right)) => left.as_bytes().cmp(right.as_bytes()),
                (Self::Bytes(left), Self::Bytes(right)) => left.cmp(right),
                _ => Ordering::Equal,
            })
    }
}

impl PartialOrd for DataKeyPart {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct DataKey(Vec<DataKeyPart>);

impl DataKey {
    pub fn new(parts: Vec<DataKeyPart>, limits: &DataLimits) -> Result<Self, Diagnostic> {
        if parts.is_empty() || parts.len() > limits.maximum_key_parts {
            return Err(data_error(
                DiagnosticClass::Resource,
                "data_key_parts",
                format!(
                    "data keys must contain 1 through {} parts",
                    limits.maximum_key_parts
                ),
            ));
        }
        let key = Self(parts);
        let bytes = encode_key(&key)?;
        if bytes.len() > limits.maximum_key_bytes {
            return Err(data_error(
                DiagnosticClass::Resource,
                "data_key_bytes",
                format!(
                    "encoded data key contains {} bytes; maximum is {}",
                    bytes.len(),
                    limits.maximum_key_bytes
                ),
            ));
        }
        Ok(key)
    }

    pub fn parts(&self) -> &[DataKeyPart] {
        &self.0
    }

    fn empty_prefix() -> Self {
        Self(Vec::new())
    }

    fn starts_with(&self, prefix: &Self) -> bool {
        self.0.starts_with(&prefix.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct DataEntryRevision([u8; 32]);

impl DataEntryRevision {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataEntry {
    pub value: Vec<u8>,
    pub revision: DataEntryRevision,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataExpectation {
    Missing,
    Exact(DataEntryRevision),
}

impl DataExpectation {
    fn matches(self, entry: Option<&DataEntry>) -> bool {
        match (self, entry) {
            (Self::Missing, None) => true,
            (Self::Exact(expected), Some(found)) => expected == found.revision,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataSchema {
    pub identity: String,
    pub digest: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataSchemaExpectation {
    Missing,
    Exact(DataSchema),
}

impl DataSchemaExpectation {
    fn matches(&self, schema: Option<&DataSchema>) -> bool {
        match (self, schema) {
            (Self::Missing, None) => true,
            (Self::Exact(expected), Some(found)) => expected == found,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataScanDirection {
    Forward,
    Reverse,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataScanItem {
    pub key: DataKey,
    pub value: Vec<u8>,
    pub revision: DataEntryRevision,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataScanPage {
    pub items: Vec<DataScanItem>,
    pub continuation: Option<Vec<u8>>,
    pub bytes: usize,
    pub work: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataInitializeOutcome {
    Created,
    Unchanged,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataInitializeReceipt {
    pub outcome: DataInitializeOutcome,
    pub store: String,
    pub revision: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataVerifyReceipt {
    pub store: String,
    pub revision: String,
    pub revisions: usize,
    pub objects: usize,
    pub schemas: usize,
    pub records: usize,
    pub staging_leftovers: usize,
    pub bytes_read: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataBackupReceipt {
    pub store: String,
    pub revision: String,
    pub digest: String,
    pub schemas: usize,
    pub records: usize,
    pub bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataCommitOutcome {
    Unchanged {
        revision: String,
    },
    Committed {
        revision: String,
        durable_bytes: usize,
        fsync_publications: usize,
    },
    Conflict {
        expected: String,
        actual: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct RecordKey {
    namespace: String,
    space: String,
    key: DataKey,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct SchemaKey {
    namespace: String,
    space: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Snapshot {
    parent: Option<[u8; 32]>,
    schemas: BTreeMap<SchemaKey, DataSchema>,
    records: BTreeMap<RecordKey, DataEntry>,
}

#[derive(Debug)]
struct DataStoreInner {
    root: PathBuf,
    store_id: [u8; 32],
    namespace: String,
    limits: DataLimits,
    live_transactions: AtomicUsize,
}

#[derive(Clone, Debug)]
pub struct DataStore {
    inner: Arc<DataStoreInner>,
}

#[derive(Debug)]
pub struct DataTransaction {
    store: Arc<DataStoreInner>,
    base: [u8; 32],
    snapshot: Snapshot,
    mutations: usize,
    mutation_bytes: usize,
    next_mutation: u64,
    changed: bool,
    expectation_failed: bool,
}

impl Drop for DataTransaction {
    fn drop(&mut self) {
        self.store
            .live_transactions
            .fetch_sub(1, AtomicOrdering::AcqRel);
    }
}

impl DataStore {
    pub fn initialize(root: &Path) -> Result<DataInitializeReceipt, Diagnostic> {
        if path_exists(root)? {
            let opened = Self::open(root, "lifecycle", DataLimits::default())?;
            let verified = opened.verify()?;
            return Ok(DataInitializeReceipt {
                outcome: DataInitializeOutcome::Unchanged,
                store: verified.store,
                revision: verified.revision,
            });
        }
        let empty = Snapshot {
            parent: None,
            schemas: BTreeMap::new(),
            records: BTreeMap::new(),
        };
        let (store_id, revision, created) = publish_new_root(root, &empty)?;
        if !created {
            let opened = Self::open(root, "lifecycle", DataLimits::default())?;
            let verified = opened.verify()?;
            return Ok(DataInitializeReceipt {
                outcome: DataInitializeOutcome::Unchanged,
                store: verified.store,
                revision: verified.revision,
            });
        }
        Ok(DataInitializeReceipt {
            outcome: DataInitializeOutcome::Created,
            store: format_store(store_id),
            revision: format_revision(revision),
        })
    }

    pub fn open(
        root: &Path,
        namespace: impl Into<String>,
        limits: DataLimits,
    ) -> Result<Self, Diagnostic> {
        limits.validate()?;
        let namespace = namespace.into();
        validate_name(&namespace, MAXIMUM_DATA_NAMESPACE_BYTES, "data_namespace")?;
        let metadata = match fs::symlink_metadata(root) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(data_error(
                    DiagnosticClass::Source,
                    "data_root_absent",
                    "data root does not exist; initialize or restore it first",
                ));
            }
            Err(error) => return Err(data_io("data_root_open", root, error)),
        };
        reject_symlinked_existing_path(root)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(data_error(
                DiagnosticClass::Source,
                "data_root_type",
                "data root is not an ordinary directory",
            ));
        }
        validate_root_inventory(root)?;
        let store_id = read_format(root)?;
        let _ = read_head(root, store_id)?;
        Ok(Self {
            inner: Arc::new(DataStoreInner {
                root: root.to_path_buf(),
                store_id,
                namespace,
                limits,
                live_transactions: AtomicUsize::new(0),
            }),
        })
    }

    pub fn store_identity(&self) -> String {
        format_store(self.inner.store_id)
    }

    pub fn namespace(&self) -> &str {
        &self.inner.namespace
    }

    pub fn limits(&self) -> &DataLimits {
        &self.inner.limits
    }

    pub fn current_revision(&self) -> Result<String, Diagnostic> {
        read_head(&self.inner.root, self.inner.store_id).map(format_revision)
    }

    pub fn begin(&self) -> Result<DataTransaction, Diagnostic> {
        reserve_live_transaction(&self.inner)?;
        let opened = (|| {
            let base = read_head(&self.inner.root, self.inner.store_id)?;
            let snapshot = read_revision(&self.inner.root, self.inner.store_id, base)?.0;
            Ok::<_, Diagnostic>((base, snapshot))
        })();
        let (base, snapshot) = match opened {
            Ok(value) => value,
            Err(error) => {
                self.inner
                    .live_transactions
                    .fetch_sub(1, AtomicOrdering::AcqRel);
                return Err(error);
            }
        };
        Ok(DataTransaction {
            store: self.inner.clone(),
            base,
            snapshot,
            mutations: 0,
            mutation_bytes: 0,
            next_mutation: 0,
            changed: false,
            expectation_failed: false,
        })
    }

    pub fn verify(&self) -> Result<DataVerifyReceipt, Diagnostic> {
        verify_root(&self.inner.root, self.inner.store_id)
    }

    pub fn backup(&self, output: &Path) -> Result<DataBackupReceipt, Diagnostic> {
        let head = read_head(&self.inner.root, self.inner.store_id)?;
        let snapshot = read_revision(&self.inner.root, self.inner.store_id, head)?.0;
        let bytes = encode_backup(head, &snapshot)?;
        publish_create_new_file(output, &bytes, MAXIMUM_DATA_BACKUP_BYTES, "data backup")?;
        let digest = digest(BACKUP_ENVELOPE_DOMAIN, &bytes);
        Ok(DataBackupReceipt {
            store: format_store(self.inner.store_id),
            revision: format_revision(head),
            digest: format!("data_backup_{}", encode_hex(&digest)),
            schemas: snapshot.schemas.len(),
            records: snapshot.records.len(),
            bytes: bytes.len(),
        })
    }

    pub fn restore(backup: &Path, root: &Path) -> Result<DataInitializeReceipt, Diagnostic> {
        if path_exists(root)? {
            return Err(data_error(
                DiagnosticClass::Source,
                "data_restore_destination_exists",
                "data restore destination must be absent",
            ));
        }
        let bytes = read_limited_regular(backup, MAXIMUM_DATA_BACKUP_BYTES, "data_backup_read")?;
        let (_source_revision, snapshot) = decode_backup(&bytes)?;
        let (store_id, revision, created) = publish_new_root(root, &snapshot)?;
        if !created {
            return Err(data_error(
                DiagnosticClass::Source,
                "data_restore_destination_exists",
                "data restore destination became visible concurrently",
            ));
        }
        let restored = Self::open(root, "lifecycle", DataLimits::default())?;
        let _ = restored.verify()?;
        Ok(DataInitializeReceipt {
            outcome: DataInitializeOutcome::Created,
            store: format_store(store_id),
            revision: format_revision(revision),
        })
    }
}

impl DataTransaction {
    pub fn base_revision(&self) -> String {
        format_revision(self.base)
    }

    pub fn schema_read(&self, space: &str) -> Result<Option<DataSchema>, Diagnostic> {
        self.validate_space(space)?;
        Ok(self
            .snapshot
            .schemas
            .get(&SchemaKey {
                namespace: self.store.namespace.clone(),
                space: space.to_owned(),
            })
            .cloned())
    }

    pub fn schema_set(
        &mut self,
        space: &str,
        expected: &DataSchemaExpectation,
        next: DataSchema,
    ) -> Result<bool, Diagnostic> {
        self.validate_space(space)?;
        validate_schema(&next)?;
        let key = SchemaKey {
            namespace: self.store.namespace.clone(),
            space: space.to_owned(),
        };
        if !expected.matches(self.snapshot.schemas.get(&key)) {
            self.expectation_failed = true;
            return Ok(false);
        }
        if self.snapshot.schemas.get(&key) == Some(&next) {
            return Ok(true);
        }
        let bytes = space
            .len()
            .checked_add(next.identity.len())
            .and_then(|value| value.checked_add(next.digest.len()))
            .ok_or_else(data_transaction_overflow)?;
        self.reserve_mutation(bytes)?;
        self.snapshot.schemas.insert(key, next);
        self.changed = true;
        Ok(true)
    }

    pub fn get(&self, space: &str, key: &DataKey) -> Result<Option<DataEntry>, Diagnostic> {
        self.validate_selector(space, key)?;
        Ok(self
            .snapshot
            .records
            .get(&RecordKey {
                namespace: self.store.namespace.clone(),
                space: space.to_owned(),
                key: key.clone(),
            })
            .cloned())
    }

    pub fn put(
        &mut self,
        space: &str,
        key: &DataKey,
        value: Vec<u8>,
        expected: DataExpectation,
    ) -> Result<bool, Diagnostic> {
        self.validate_selector(space, key)?;
        if value.len() > self.store.limits.maximum_value_bytes {
            return Err(data_error(
                DiagnosticClass::Resource,
                "data_value_bytes",
                format!(
                    "data value contains {} bytes; maximum is {}",
                    value.len(),
                    self.store.limits.maximum_value_bytes
                ),
            ));
        }
        let record_key = RecordKey {
            namespace: self.store.namespace.clone(),
            space: space.to_owned(),
            key: key.clone(),
        };
        if !expected.matches(self.snapshot.records.get(&record_key)) {
            self.expectation_failed = true;
            return Ok(false);
        }
        let key_bytes = encode_key(key)?;
        let bytes = key_bytes
            .len()
            .checked_add(value.len())
            .ok_or_else(data_transaction_overflow)?;
        self.reserve_mutation(bytes)?;
        let revision = next_entry_revision(
            self.store.store_id,
            self.base,
            self.next_mutation,
            &record_key,
            &value,
        )?;
        self.next_mutation = self.next_mutation.checked_add(1).ok_or_else(|| {
            data_error(
                DiagnosticClass::Resource,
                "data_transaction_mutation_counter",
                "data transaction mutation counter overflowed",
            )
        })?;
        self.snapshot
            .records
            .insert(record_key, DataEntry { value, revision });
        self.changed = true;
        Ok(true)
    }

    pub fn delete(
        &mut self,
        space: &str,
        key: &DataKey,
        expected: DataExpectation,
    ) -> Result<bool, Diagnostic> {
        self.validate_selector(space, key)?;
        let record_key = RecordKey {
            namespace: self.store.namespace.clone(),
            space: space.to_owned(),
            key: key.clone(),
        };
        if !expected.matches(self.snapshot.records.get(&record_key)) {
            self.expectation_failed = true;
            return Ok(false);
        }
        if !self.snapshot.records.contains_key(&record_key) {
            return Ok(true);
        }
        let bytes = encode_key(key)?.len();
        self.reserve_mutation(bytes)?;
        self.snapshot.records.remove(&record_key);
        self.next_mutation = self.next_mutation.checked_add(1).ok_or_else(|| {
            data_error(
                DiagnosticClass::Resource,
                "data_transaction_mutation_counter",
                "data transaction mutation counter overflowed",
            )
        })?;
        self.changed = true;
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn scan(
        &self,
        space: &str,
        prefix: &[DataKeyPart],
        direction: DataScanDirection,
        maximum_items: usize,
        maximum_bytes: usize,
        maximum_work: usize,
        continuation: Option<&[u8]>,
    ) -> Result<DataScanPage, Diagnostic> {
        self.validate_space(space)?;
        if prefix.len() > self.store.limits.maximum_key_parts {
            return Err(data_error(
                DiagnosticClass::Resource,
                "data_scan_prefix_parts",
                "data scan prefix exceeds the key-part limit",
            ));
        }
        let prefix = DataKey(prefix.to_vec());
        if encode_key(&prefix)?.len() > self.store.limits.maximum_key_bytes {
            return Err(data_error(
                DiagnosticClass::Resource,
                "data_scan_prefix_bytes",
                "data scan prefix exceeds the key-byte limit",
            ));
        }
        validate_scan_limit(
            "maximum_items",
            maximum_items,
            self.store.limits.maximum_scan_items,
        )?;
        validate_scan_limit(
            "maximum_bytes",
            maximum_bytes,
            self.store.limits.maximum_scan_bytes,
        )?;
        validate_scan_limit(
            "maximum_work",
            maximum_work,
            self.store.limits.maximum_scan_work,
        )?;
        let resume = continuation
            .map(|bytes| {
                decode_continuation(
                    bytes,
                    self.store.store_id,
                    self.base,
                    &self.store.namespace,
                    space,
                    &prefix,
                    direction,
                    maximum_items,
                    maximum_bytes,
                    maximum_work,
                )
            })
            .transpose()?;
        let lower = RecordKey {
            namespace: self.store.namespace.clone(),
            space: space.to_owned(),
            key: DataKey::empty_prefix(),
        };
        let upper = RecordKey {
            namespace: self.store.namespace.clone(),
            space: format!("{space}\0"),
            key: DataKey::empty_prefix(),
        };
        let range = self.snapshot.records.range(lower..upper);
        let selector = ScanSelector {
            store_id: self.store.store_id,
            revision: self.base,
            namespace: &self.store.namespace,
            space,
            prefix: &prefix,
            direction,
            maximum_items,
            maximum_bytes,
            maximum_work,
            resume: resume.as_ref(),
        };
        match direction {
            DataScanDirection::Forward => scan_iterator(range, selector),
            DataScanDirection::Reverse => scan_iterator(range.rev(), selector),
        }
    }

    pub fn commit(mut self) -> Result<DataCommitOutcome, Diagnostic> {
        if self.expectation_failed {
            return Ok(DataCommitOutcome::Unchanged {
                revision: format_revision(self.base),
            });
        }
        self.commit_inner(&mut |_| Ok(()))
    }

    fn commit_inner<F>(&mut self, hook: &mut F) -> Result<DataCommitOutcome, Diagnostic>
    where
        F: FnMut(CommitCheckpoint) -> Result<(), Diagnostic>,
    {
        if !self.changed {
            return Ok(DataCommitOutcome::Unchanged {
                revision: format_revision(self.base),
            });
        }
        let lock_path = self.store.root.join(LOCK_FILE);
        validate_regular_file(&lock_path, "data_lock_type")?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|error| data_io("data_lock_open", &lock_path, error))?;
        FileExt::lock_exclusive(&lock)
            .map_err(|error| data_io("data_lock_acquire", &lock_path, error))?;
        let actual = read_head(&self.store.root, self.store.store_id)?;
        if actual != self.base {
            return Ok(DataCommitOutcome::Conflict {
                expected: format_revision(self.base),
                actual: format_revision(actual),
            });
        }
        self.snapshot.parent = Some(self.base);
        let bytes = encode_revision(self.store.store_id, &self.snapshot)?;
        let revision = digest(REVISION_DIGEST_DOMAIN, &bytes);
        hook(CommitCheckpoint::BeforeRevisionStage)?;
        let stage_name = format!(".revision-stage-{}", random_hex()?);
        let stage_path = self.store.root.join(STAGING_DIRECTORY).join(&stage_name);
        let object_path = revision_path(&self.store.root, revision);
        let mut stage = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&stage_path)
            .map_err(|error| data_io("data_revision_stage_create", &stage_path, error))?;
        stage
            .write_all(&bytes)
            .map_err(|error| data_io("data_revision_stage_write", &stage_path, error))?;
        hook(CommitCheckpoint::RevisionBytesWritten)?;
        stage
            .sync_all()
            .map_err(|error| data_io("data_revision_stage_sync", &stage_path, error))?;
        drop(stage);
        hook(CommitCheckpoint::RevisionStageSynced)?;
        match fs::hard_link(&stage_path, &object_path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing = read_limited_regular(
                    &object_path,
                    MAXIMUM_DATA_REVISION_BYTES,
                    "data_revision_existing",
                )?;
                if existing != bytes {
                    return Err(data_error(
                        DiagnosticClass::Corrupt,
                        "data_revision_identity_collision",
                        "existing immutable data revision disagrees with its content identity",
                    ));
                }
            }
            Err(error) => return Err(data_io("data_revision_publish", &object_path, error)),
        }
        sync_directory(
            &self.store.root.join(OBJECTS_DIRECTORY),
            "data_objects_sync",
        )?;
        hook(CommitCheckpoint::RevisionPublished)?;
        let head_stage_name = format!(".head-stage-{}", random_hex()?);
        let head_stage = self
            .store
            .root
            .join(STAGING_DIRECTORY)
            .join(&head_stage_name);
        write_new_synced(
            &head_stage,
            &encode_head(self.store.store_id, revision)?,
            "data_head_stage",
        )?;
        hook(CommitCheckpoint::HeadStageSynced)?;
        fs::rename(&head_stage, self.store.root.join(HEAD_FILE)).map_err(|error| {
            data_error(
                DiagnosticClass::Infrastructure,
                "data_head_visibility_unknown",
                format!("data head publication failed or has indeterminate visibility: {error}"),
            )
        })?;
        hook(CommitCheckpoint::HeadPublished)?;
        sync_directory(&self.store.root, "data_head_durability_unknown")?;
        hook(CommitCheckpoint::RootSynced)?;
        let _ = fs::remove_file(&stage_path);
        let _ = sync_directory(
            &self.store.root.join(STAGING_DIRECTORY),
            "data_staging_cleanup_sync",
        );
        let catalog_publications =
            rebuild_catalog(&self.store.root, self.store.store_id).unwrap_or_default();
        Ok(DataCommitOutcome::Committed {
            revision: format_revision(revision),
            durable_bytes: bytes.len(),
            fsync_publications: 4_usize.saturating_add(catalog_publications),
        })
    }

    fn validate_space(&self, space: &str) -> Result<(), Diagnostic> {
        validate_name(
            space,
            self.store.limits.maximum_space_name_bytes,
            "data_space",
        )
    }

    fn validate_selector(&self, space: &str, key: &DataKey) -> Result<(), Diagnostic> {
        self.validate_space(space)?;
        let _ = DataKey::new(key.0.clone(), &self.store.limits)?;
        Ok(())
    }

    fn reserve_mutation(&mut self, bytes: usize) -> Result<(), Diagnostic> {
        let mutations = self
            .mutations
            .checked_add(1)
            .ok_or_else(data_transaction_overflow)?;
        let total = self
            .mutation_bytes
            .checked_add(bytes)
            .ok_or_else(data_transaction_overflow)?;
        if mutations > self.store.limits.maximum_transaction_mutations {
            return Err(data_error(
                DiagnosticClass::Resource,
                "data_transaction_mutations",
                "data transaction mutation limit exceeded",
            ));
        }
        if total > self.store.limits.maximum_transaction_bytes {
            return Err(data_error(
                DiagnosticClass::Resource,
                "data_transaction_bytes",
                "data transaction byte limit exceeded",
            ));
        }
        self.mutations = mutations;
        self.mutation_bytes = total;
        Ok(())
    }
}

struct ScanSelector<'a> {
    store_id: [u8; 32],
    revision: [u8; 32],
    namespace: &'a str,
    space: &'a str,
    prefix: &'a DataKey,
    direction: DataScanDirection,
    maximum_items: usize,
    maximum_bytes: usize,
    maximum_work: usize,
    resume: Option<&'a DataKey>,
}

fn scan_iterator<'a, I>(iterator: I, selector: ScanSelector<'_>) -> Result<DataScanPage, Diagnostic>
where
    I: Iterator<Item = (&'a RecordKey, &'a DataEntry)>,
{
    let mut items = Vec::new();
    let mut bytes = 0_usize;
    let mut work = 0_usize;
    let mut has_more = false;
    for (key, entry) in iterator {
        if !key.key.starts_with(selector.prefix) {
            continue;
        }
        if let Some(resume) = selector.resume {
            let excluded = match selector.direction {
                DataScanDirection::Forward => key.key <= *resume,
                DataScanDirection::Reverse => key.key >= *resume,
            };
            if excluded {
                continue;
            }
        }
        if work == selector.maximum_work {
            has_more = true;
            break;
        }
        work = work.checked_add(1).ok_or_else(data_scan_overflow)?;
        let item_bytes = encode_key(&key.key)?
            .len()
            .checked_add(entry.value.len())
            .and_then(|value| value.checked_add(32))
            .ok_or_else(data_scan_overflow)?;
        let next_bytes = bytes
            .checked_add(item_bytes)
            .ok_or_else(data_scan_overflow)?;
        if items.len() == selector.maximum_items || next_bytes > selector.maximum_bytes {
            if items.is_empty() {
                return Err(data_error(
                    DiagnosticClass::Resource,
                    "data_scan_item_bytes",
                    "one selected data record exceeds the requested scan byte limit",
                ));
            }
            has_more = true;
            break;
        }
        bytes = next_bytes;
        items.push(DataScanItem {
            key: key.key.clone(),
            value: entry.value.clone(),
            revision: entry.revision,
        });
    }
    let continuation = if has_more {
        let last = items.last().ok_or_else(|| {
            data_error(
                DiagnosticClass::Resource,
                "data_scan_work",
                "scan work limit was exhausted before one item could be returned",
            )
        })?;
        Some(encode_continuation(&selector, &last.key)?)
    } else {
        None
    };
    Ok(DataScanPage {
        items,
        continuation,
        bytes,
        work,
    })
}

fn publish_new_root(
    root: &Path,
    source_snapshot: &Snapshot,
) -> Result<([u8; 32], [u8; 32], bool), Diagnostic> {
    let (parent, name) = canonical_absent_destination(root)?;
    let stage_name = format!(".{name}.data-stage-{}", random_hex()?);
    let stage = parent.join(&stage_name);
    fs::create_dir(&stage).map_err(|error| data_io("data_stage_create", &stage, error))?;
    let result = (|| {
        fs::create_dir(stage.join(OBJECTS_DIRECTORY))
            .map_err(|error| data_io("data_objects_create", &stage, error))?;
        fs::create_dir(stage.join(STAGING_DIRECTORY))
            .map_err(|error| data_io("data_staging_create", &stage, error))?;
        fs::create_dir(stage.join(CATALOG_DIRECTORY))
            .map_err(|error| data_io("data_catalog_create", &stage, error))?;
        let store_id = random_identity()?;
        write_new_synced(
            &stage.join(FORMAT_FILE),
            &encode_format(store_id),
            "data_format_create",
        )?;
        write_new_synced(&stage.join(LOCK_FILE), &[], "data_lock_create")?;
        let snapshot = Snapshot {
            parent: None,
            schemas: source_snapshot.schemas.clone(),
            records: source_snapshot.records.clone(),
        };
        let revision_bytes = encode_revision(store_id, &snapshot)?;
        let revision = digest(REVISION_DIGEST_DOMAIN, &revision_bytes);
        write_new_synced(
            &revision_path(&stage, revision),
            &revision_bytes,
            "data_initial_revision_create",
        )?;
        sync_directory(&stage.join(OBJECTS_DIRECTORY), "data_objects_sync")?;
        write_new_synced(
            &stage.join(CATALOG_DIRECTORY).join(CATALOG_FILE),
            &encode_catalog(store_id, &[revision])?,
            "data_catalog_initial_create",
        )?;
        sync_directory(&stage.join(CATALOG_DIRECTORY), "data_catalog_sync")?;
        write_new_synced(
            &stage.join(HEAD_FILE),
            &encode_head(store_id, revision)?,
            "data_head_create",
        )?;
        sync_directory(&stage.join(STAGING_DIRECTORY), "data_staging_sync")?;
        sync_directory(&stage, "data_stage_sync")?;
        Ok::<_, Diagnostic>((store_id, revision))
    })();
    let (store_id, revision) = match result {
        Ok(value) => value,
        Err(error) => {
            remove_owned_stage(&stage);
            return Err(error);
        }
    };
    let destination = parent.join(&name);
    match renameat_with(CWD, &stage, CWD, &destination, RenameFlags::NOREPLACE) {
        Ok(()) => {
            sync_directory(&parent, "data_parent_sync")?;
            Ok((store_id, revision, true))
        }
        Err(error) if error == rustix::io::Errno::EXIST => {
            remove_owned_stage(&stage);
            Ok((store_id, revision, false))
        }
        Err(error) => {
            remove_owned_stage(&stage);
            Err(data_error(
                DiagnosticClass::Infrastructure,
                "data_root_publish",
                format!("data root could not be published: {error}"),
            ))
        }
    }
}

fn verify_root(root: &Path, store_id: [u8; 32]) -> Result<DataVerifyReceipt, Diagnostic> {
    validate_root_inventory(root)?;
    if read_format(root)? != store_id {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            "data_store_identity_changed",
            "data format identity changed while the root was open",
        ));
    }
    let head = read_head(root, store_id)?;
    let mut reachable = BTreeSet::new();
    let mut cursor = Some(head);
    let mut schemas = 0_usize;
    let mut records = 0_usize;
    let mut bytes_read = 0_u64;
    while let Some(revision) = cursor {
        if !reachable.insert(revision) {
            return Err(data_error(
                DiagnosticClass::Corrupt,
                "data_revision_cycle",
                "accepted data history contains a revision cycle",
            ));
        }
        if reachable.len() > MAXIMUM_DATA_HISTORY_REVISIONS {
            return Err(data_error(
                DiagnosticClass::Resource,
                "data_history_revisions",
                "accepted data history exceeds the verification revision bound",
            ));
        }
        let (snapshot, bytes) = read_revision(root, store_id, revision)?;
        bytes_read = bytes_read.checked_add(bytes as u64).ok_or_else(|| {
            data_error(
                DiagnosticClass::Resource,
                "data_verify_bytes",
                "data verification byte counter overflowed",
            )
        })?;
        if revision == head {
            schemas = snapshot.schemas.len();
            records = snapshot.records.len();
        }
        cursor = snapshot.parent;
    }
    let object_entries =
        read_directory_entries(&root.join(OBJECTS_DIRECTORY), "data_objects_scan")?;
    if object_entries.len() > MAXIMUM_DATA_STORE_OBJECTS {
        return Err(data_error(
            DiagnosticClass::Resource,
            "data_store_objects",
            "data store object count exceeds the verification bound",
        ));
    }
    for entry in &object_entries {
        let revision = parse_revision_name(entry)?;
        let (_, bytes) = read_revision(root, store_id, revision)?;
        bytes_read = bytes_read.checked_add(bytes as u64).ok_or_else(|| {
            data_error(
                DiagnosticClass::Resource,
                "data_verify_bytes",
                "data verification byte counter overflowed",
            )
        })?;
    }
    let staging = read_directory_entries(&root.join(STAGING_DIRECTORY), "data_staging_scan")?;
    if staging.len() > MAXIMUM_STAGING_LEFTOVERS {
        return Err(data_error(
            DiagnosticClass::Resource,
            "data_staging_entries",
            "data staging entry count exceeds the verification bound",
        ));
    }
    for name in &staging {
        if !(name.starts_with(".revision-stage-") || name.starts_with(".head-stage-")) {
            return Err(data_error(
                DiagnosticClass::Corrupt,
                "data_staging_name",
                format!("foreign data staging entry '{name}'"),
            ));
        }
        validate_regular_file(
            &root.join(STAGING_DIRECTORY).join(name),
            "data_staging_type",
        )?;
    }
    Ok(DataVerifyReceipt {
        store: format_store(store_id),
        revision: format_revision(head),
        revisions: reachable.len(),
        objects: object_entries.len(),
        schemas,
        records,
        staging_leftovers: staging.len(),
        bytes_read,
    })
}

fn encode_format(store_id: [u8; 32]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(FORMAT_MAGIC);
    push_u16(&mut bytes, DATA_STORE_CONTRACT_VERSION);
    bytes.extend_from_slice(&store_id);
    seal(FORMAT_DIGEST_DOMAIN, bytes)
}

fn read_format(root: &Path) -> Result<[u8; 32], Diagnostic> {
    let path = root.join(FORMAT_FILE);
    let bytes = read_limited_regular(&path, 128, "data_format_read")?;
    let payload = open_envelope(&bytes, FORMAT_DIGEST_DOMAIN, "data_format_checksum")?;
    let mut cursor = Cursor::new(payload);
    cursor.expect_magic(FORMAT_MAGIC, "data_format_magic")?;
    let version = cursor.u16("data_format_version")?;
    if version != DATA_STORE_CONTRACT_VERSION {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            "data_format_version",
            format!(
                "foreign data store format version {version}; expected {DATA_STORE_CONTRACT_VERSION}"
            ),
        ));
    }
    let store_id = cursor.array_32("data_format_identity")?;
    cursor.finish("data_format_trailing")?;
    Ok(store_id)
}

fn encode_head(store_id: [u8; 32], revision: [u8; 32]) -> Result<Vec<u8>, Diagnostic> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(HEAD_MAGIC);
    push_u16(&mut bytes, DATA_STORE_CONTRACT_VERSION);
    bytes.extend_from_slice(&store_id);
    bytes.extend_from_slice(&revision);
    Ok(seal(HEAD_ENVELOPE_DOMAIN, bytes))
}

fn read_head(root: &Path, store_id: [u8; 32]) -> Result<[u8; 32], Diagnostic> {
    let path = root.join(HEAD_FILE);
    let bytes = read_limited_regular(&path, 160, "data_head_read")?;
    let payload = open_envelope(&bytes, HEAD_ENVELOPE_DOMAIN, "data_head_checksum")?;
    let mut cursor = Cursor::new(payload);
    cursor.expect_magic(HEAD_MAGIC, "data_head_magic")?;
    let version = cursor.u16("data_head_version")?;
    if version != DATA_STORE_CONTRACT_VERSION {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            "data_head_version",
            "data head belongs to a foreign format version",
        ));
    }
    if cursor.array_32("data_head_store")? != store_id {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            "data_head_store",
            "data head belongs to a foreign physical store",
        ));
    }
    let revision = cursor.array_32("data_head_revision")?;
    cursor.finish("data_head_trailing")?;
    Ok(revision)
}

fn encode_catalog(store_id: [u8; 32], revisions: &[[u8; 32]]) -> Result<Vec<u8>, Diagnostic> {
    if revisions.len() > MAXIMUM_DATA_STORE_OBJECTS {
        return Err(data_error(
            DiagnosticClass::Resource,
            "data_catalog_objects",
            "derived data catalog exceeds the object-count bound",
        ));
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(CATALOG_MAGIC);
    push_u16(&mut bytes, DATA_STORE_CONTRACT_VERSION);
    bytes.extend_from_slice(&store_id);
    push_count(&mut bytes, revisions.len(), "data_catalog_objects")?;
    let mut previous = None;
    for revision in revisions {
        if previous.is_some_and(|prior| prior >= revision) {
            return Err(data_error(
                DiagnosticClass::Corrupt,
                "data_catalog_order",
                "derived data catalog objects are duplicate or not canonical",
            ));
        }
        bytes.extend_from_slice(revision);
        previous = Some(revision);
    }
    if bytes.len() > MAXIMUM_DATA_CATALOG_BYTES.saturating_sub(32) {
        return Err(data_error(
            DiagnosticClass::Resource,
            "data_catalog_bytes",
            "derived data catalog exceeds its byte bound",
        ));
    }
    Ok(seal(CATALOG_ENVELOPE_DOMAIN, bytes))
}

fn decode_catalog(bytes: &[u8], store_id: [u8; 32]) -> Result<Vec<[u8; 32]>, Diagnostic> {
    if bytes.len() > MAXIMUM_DATA_CATALOG_BYTES {
        return Err(data_error(
            DiagnosticClass::Resource,
            "data_catalog_bytes",
            "derived data catalog exceeds its byte bound",
        ));
    }
    let payload = open_envelope(bytes, CATALOG_ENVELOPE_DOMAIN, "data_catalog_checksum")?;
    let mut cursor = Cursor::new(payload);
    cursor.expect_magic(CATALOG_MAGIC, "data_catalog_magic")?;
    if cursor.u16("data_catalog_version")? != DATA_STORE_CONTRACT_VERSION {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            "data_catalog_version",
            "derived data catalog belongs to a foreign format",
        ));
    }
    if cursor.array_32("data_catalog_store")? != store_id {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            "data_catalog_store",
            "derived data catalog belongs to a foreign physical store",
        ));
    }
    let count = cursor.count("data_catalog_objects", MAXIMUM_DATA_STORE_OBJECTS)?;
    let mut revisions = Vec::with_capacity(count);
    let mut previous = None;
    for _ in 0..count {
        let revision = cursor.array_32("data_catalog_revision")?;
        if previous.is_some_and(|prior| prior >= revision) {
            return Err(data_error(
                DiagnosticClass::Corrupt,
                "data_catalog_order",
                "derived data catalog objects are duplicate or not canonical",
            ));
        }
        revisions.push(revision);
        previous = Some(revision);
    }
    cursor.finish("data_catalog_trailing")?;
    Ok(revisions)
}

fn rebuild_catalog(root: &Path, store_id: [u8; 32]) -> Result<usize, Diagnostic> {
    let object_names = read_directory_entries(&root.join(OBJECTS_DIRECTORY), "data_catalog_scan")?;
    if object_names.len() > MAXIMUM_DATA_STORE_OBJECTS {
        return Err(data_error(
            DiagnosticClass::Resource,
            "data_catalog_objects",
            "derived data catalog exceeds the object-count bound",
        ));
    }
    let mut revisions = object_names
        .iter()
        .map(|name| parse_revision_name(name))
        .collect::<Result<Vec<_>, _>>()?;
    revisions.sort_unstable();
    let bytes = encode_catalog(store_id, &revisions)?;
    let catalog = root.join(CATALOG_DIRECTORY);
    let mut publications = 0_usize;
    match fs::create_dir(&catalog) {
        Ok(()) => {
            sync_directory(root, "data_catalog_root_sync")?;
            publications = publications.saturating_add(1);
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            validate_directory(&catalog, "data_catalog_type")?;
        }
        Err(error) => return Err(data_io("data_catalog_create", &catalog, error)),
    }
    let current = catalog.join(CATALOG_FILE);
    if let Ok(existing) =
        read_limited_regular(&current, MAXIMUM_DATA_CATALOG_BYTES, "data_catalog_read")
        && let Ok(decoded) = decode_catalog(&existing, store_id)
        && decoded == revisions
    {
        return Ok(publications);
    }
    let stage = catalog.join(format!(".catalog-stage-{}", random_hex()?));
    let result = (|| {
        write_new_synced(&stage, &bytes, "data_catalog_stage")?;
        fs::rename(&stage, &current)
            .map_err(|error| data_io("data_catalog_publish", &catalog, error))?;
        sync_directory(&catalog, "data_catalog_sync")?;
        Ok::<_, Diagnostic>(2_usize)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&stage);
    }
    result.map(|value| publications.saturating_add(value))
}

fn encode_revision(store_id: [u8; 32], snapshot: &Snapshot) -> Result<Vec<u8>, Diagnostic> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(REVISION_MAGIC);
    push_u16(&mut bytes, DATA_STORE_CONTRACT_VERSION);
    bytes.extend_from_slice(&store_id);
    match snapshot.parent {
        Some(parent) => {
            push_u8(&mut bytes, 1);
            bytes.extend_from_slice(&parent);
        }
        None => push_u8(&mut bytes, 0),
    }
    push_count(&mut bytes, snapshot.schemas.len(), "data_schema_count")?;
    for (key, schema) in &snapshot.schemas {
        push_text(&mut bytes, &key.namespace, "data_namespace_bytes")?;
        push_text(&mut bytes, &key.space, "data_space_bytes")?;
        push_text(&mut bytes, &schema.identity, "data_schema_identity_bytes")?;
        push_blob(&mut bytes, &schema.digest, "data_schema_digest_bytes")?;
    }
    push_count(&mut bytes, snapshot.records.len(), "data_record_count")?;
    for (key, entry) in &snapshot.records {
        push_text(&mut bytes, &key.namespace, "data_namespace_bytes")?;
        push_text(&mut bytes, &key.space, "data_space_bytes")?;
        push_key(&mut bytes, &key.key)?;
        push_blob(&mut bytes, &entry.value, "data_value_bytes")?;
        bytes.extend_from_slice(&entry.revision.0);
        if bytes.len() > MAXIMUM_DATA_REVISION_BYTES {
            return Err(data_error(
                DiagnosticClass::Resource,
                "data_revision_bytes",
                "encoded data revision exceeds the physical revision bound",
            ));
        }
    }
    Ok(seal(REVISION_ENVELOPE_DOMAIN, bytes))
}

fn read_revision(
    root: &Path,
    store_id: [u8; 32],
    revision: [u8; 32],
) -> Result<(Snapshot, usize), Diagnostic> {
    let path = revision_path(root, revision);
    let bytes = read_limited_regular(&path, MAXIMUM_DATA_REVISION_BYTES, "data_revision_read")?;
    if digest(REVISION_DIGEST_DOMAIN, &bytes) != revision {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            "data_revision_digest",
            "immutable data revision content disagrees with its file identity",
        ));
    }
    let snapshot = decode_revision(&bytes, store_id)?;
    Ok((snapshot, bytes.len()))
}

fn decode_revision(bytes: &[u8], store_id: [u8; 32]) -> Result<Snapshot, Diagnostic> {
    let payload = open_envelope(bytes, REVISION_ENVELOPE_DOMAIN, "data_revision_checksum")?;
    let mut cursor = Cursor::new(payload);
    cursor.expect_magic(REVISION_MAGIC, "data_revision_magic")?;
    if cursor.u16("data_revision_version")? != DATA_STORE_CONTRACT_VERSION {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            "data_revision_version",
            "immutable data revision belongs to a foreign format",
        ));
    }
    if cursor.array_32("data_revision_store")? != store_id {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            "data_revision_store",
            "immutable data revision belongs to a foreign physical store",
        ));
    }
    let parent = match cursor.u8("data_revision_parent_tag")? {
        0 => None,
        1 => Some(cursor.array_32("data_revision_parent")?),
        _ => {
            return Err(data_error(
                DiagnosticClass::Corrupt,
                "data_revision_parent_tag",
                "immutable data revision has a noncanonical parent tag",
            ));
        }
    };
    let schema_count = cursor.count("data_schema_count", MAXIMUM_DATA_STORE_OBJECTS)?;
    let mut schemas = BTreeMap::new();
    for _ in 0..schema_count {
        let namespace = cursor.text("data_namespace", MAXIMUM_DATA_NAMESPACE_BYTES)?;
        validate_name(&namespace, MAXIMUM_DATA_NAMESPACE_BYTES, "data_namespace")?;
        let space = cursor.text("data_space", MAXIMUM_DATA_SPACE_NAME_BYTES)?;
        validate_name(&space, MAXIMUM_DATA_SPACE_NAME_BYTES, "data_space")?;
        let schema = DataSchema {
            identity: cursor.text("data_schema_identity", MAXIMUM_DATA_SPACE_NAME_BYTES)?,
            digest: cursor.blob("data_schema_digest", 128)?,
        };
        validate_schema(&schema)?;
        if schemas
            .insert(SchemaKey { namespace, space }, schema)
            .is_some()
        {
            return Err(data_error(
                DiagnosticClass::Corrupt,
                "data_schema_duplicate",
                "immutable data revision contains a duplicate schema key",
            ));
        }
    }
    let record_count = cursor.count("data_record_count", MAXIMUM_DATA_STORE_OBJECTS)?;
    let mut records = BTreeMap::new();
    let default_limits = DataLimits::default();
    for _ in 0..record_count {
        let namespace = cursor.text("data_namespace", MAXIMUM_DATA_NAMESPACE_BYTES)?;
        validate_name(&namespace, MAXIMUM_DATA_NAMESPACE_BYTES, "data_namespace")?;
        let space = cursor.text("data_space", MAXIMUM_DATA_SPACE_NAME_BYTES)?;
        validate_name(&space, MAXIMUM_DATA_SPACE_NAME_BYTES, "data_space")?;
        let key = cursor.key(&default_limits)?;
        let entry = DataEntry {
            value: cursor.blob("data_value", MAXIMUM_DATA_VALUE_BYTES)?,
            revision: DataEntryRevision(cursor.array_32("data_entry_revision")?),
        };
        if records
            .insert(
                RecordKey {
                    namespace,
                    space,
                    key,
                },
                entry,
            )
            .is_some()
        {
            return Err(data_error(
                DiagnosticClass::Corrupt,
                "data_record_duplicate",
                "immutable data revision contains a duplicate record key",
            ));
        }
    }
    cursor.finish("data_revision_trailing")?;
    Ok(Snapshot {
        parent,
        schemas,
        records,
    })
}

fn encode_backup(source_revision: [u8; 32], snapshot: &Snapshot) -> Result<Vec<u8>, Diagnostic> {
    let logical = Snapshot {
        parent: None,
        schemas: snapshot.schemas.clone(),
        records: snapshot.records.clone(),
    };
    let mut bytes = Vec::new();
    bytes.extend_from_slice(BACKUP_MAGIC);
    push_u16(&mut bytes, DATA_BACKUP_CONTRACT_VERSION);
    bytes.extend_from_slice(&source_revision);
    let logical_bytes = encode_logical_snapshot(&logical)?;
    push_blob(&mut bytes, &logical_bytes, "data_backup_logical_bytes")?;
    if bytes.len() > MAXIMUM_DATA_BACKUP_BYTES.saturating_sub(32) {
        return Err(data_error(
            DiagnosticClass::Resource,
            "data_backup_bytes",
            "logical data backup exceeds the backup byte limit",
        ));
    }
    Ok(seal(BACKUP_ENVELOPE_DOMAIN, bytes))
}

fn decode_backup(bytes: &[u8]) -> Result<([u8; 32], Snapshot), Diagnostic> {
    let payload = open_envelope(bytes, BACKUP_ENVELOPE_DOMAIN, "data_backup_checksum")?;
    let mut cursor = Cursor::new(payload);
    cursor.expect_magic(BACKUP_MAGIC, "data_backup_magic")?;
    if cursor.u16("data_backup_version")? != DATA_BACKUP_CONTRACT_VERSION {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            "data_backup_version",
            "logical data backup belongs to a foreign format",
        ));
    }
    let source = cursor.array_32("data_backup_source_revision")?;
    let logical = cursor.blob("data_backup_logical", MAXIMUM_DATA_BACKUP_BYTES)?;
    cursor.finish("data_backup_trailing")?;
    Ok((source, decode_logical_snapshot(&logical)?))
}

fn encode_logical_snapshot(snapshot: &Snapshot) -> Result<Vec<u8>, Diagnostic> {
    let zero_store = [0_u8; 32];
    encode_revision(zero_store, snapshot)
}

fn decode_logical_snapshot(bytes: &[u8]) -> Result<Snapshot, Diagnostic> {
    decode_revision(bytes, [0_u8; 32])
}

fn encode_continuation(
    selector: &ScanSelector<'_>,
    resume: &DataKey,
) -> Result<Vec<u8>, Diagnostic> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"LKJDSCN1");
    bytes.extend_from_slice(&selector.store_id);
    bytes.extend_from_slice(&selector.revision);
    push_text(
        &mut bytes,
        selector.namespace,
        "data_continuation_namespace",
    )?;
    push_text(&mut bytes, selector.space, "data_continuation_space")?;
    push_key(&mut bytes, selector.prefix)?;
    push_u8(
        &mut bytes,
        match selector.direction {
            DataScanDirection::Forward => 0,
            DataScanDirection::Reverse => 1,
        },
    );
    push_usize(
        &mut bytes,
        selector.maximum_items,
        "data_continuation_items",
    )?;
    push_usize(
        &mut bytes,
        selector.maximum_bytes,
        "data_continuation_bytes",
    )?;
    push_usize(&mut bytes, selector.maximum_work, "data_continuation_work")?;
    push_key(&mut bytes, resume)?;
    Ok(seal(CONTINUATION_DOMAIN, bytes))
}

#[allow(clippy::too_many_arguments)]
fn decode_continuation(
    bytes: &[u8],
    store_id: [u8; 32],
    revision: [u8; 32],
    namespace: &str,
    space: &str,
    prefix: &DataKey,
    direction: DataScanDirection,
    maximum_items: usize,
    maximum_bytes: usize,
    maximum_work: usize,
) -> Result<DataKey, Diagnostic> {
    if bytes.len() > 16 * 1_024 {
        return Err(data_error(
            DiagnosticClass::Resource,
            "data_continuation_bytes",
            "data scan continuation exceeds 16384 bytes",
        ));
    }
    let payload = open_envelope(bytes, CONTINUATION_DOMAIN, "data_continuation_checksum")?;
    let mut cursor = Cursor::new(payload);
    cursor.expect_magic(b"LKJDSCN1", "data_continuation_magic")?;
    let found_store = cursor.array_32("data_continuation_store")?;
    let found_revision = cursor.array_32("data_continuation_revision")?;
    let found_namespace =
        cursor.text("data_continuation_namespace", MAXIMUM_DATA_NAMESPACE_BYTES)?;
    let found_space = cursor.text("data_continuation_space", MAXIMUM_DATA_SPACE_NAME_BYTES)?;
    let found_prefix = cursor.key(&DataLimits::default())?;
    let found_direction = match cursor.u8("data_continuation_direction")? {
        0 => DataScanDirection::Forward,
        1 => DataScanDirection::Reverse,
        _ => {
            return Err(data_error(
                DiagnosticClass::Corrupt,
                "data_continuation_direction",
                "data scan continuation has a noncanonical direction",
            ));
        }
    };
    let found_items = cursor.usize("data_continuation_items")?;
    let found_bytes = cursor.usize("data_continuation_bytes")?;
    let found_work = cursor.usize("data_continuation_work")?;
    let resume = cursor.key(&DataLimits::default())?;
    cursor.finish("data_continuation_trailing")?;
    if found_store != store_id
        || found_revision != revision
        || found_namespace != namespace
        || found_space != space
        || found_prefix != *prefix
        || found_direction != direction
        || found_items != maximum_items
        || found_bytes != maximum_bytes
        || found_work != maximum_work
    {
        return Err(data_error(
            DiagnosticClass::Source,
            "data_continuation_selector",
            "data scan continuation does not match the store, revision, selector, direction, or limits",
        ));
    }
    Ok(resume)
}

fn next_entry_revision(
    store_id: [u8; 32],
    base: [u8; 32],
    mutation: u64,
    key: &RecordKey,
    value: &[u8],
) -> Result<DataEntryRevision, Diagnostic> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&store_id);
    bytes.extend_from_slice(&base);
    push_u64(&mut bytes, mutation);
    push_text(&mut bytes, &key.namespace, "data_entry_namespace")?;
    push_text(&mut bytes, &key.space, "data_entry_space")?;
    push_key(&mut bytes, &key.key)?;
    push_blob(&mut bytes, value, "data_entry_value")?;
    Ok(DataEntryRevision(digest(ENTRY_REVISION_DOMAIN, &bytes)))
}

fn encode_key(key: &DataKey) -> Result<Vec<u8>, Diagnostic> {
    let mut bytes = Vec::new();
    push_key(&mut bytes, key)?;
    Ok(bytes)
}

fn push_key(output: &mut Vec<u8>, key: &DataKey) -> Result<(), Diagnostic> {
    push_count(output, key.0.len(), "data_key_parts")?;
    for part in &key.0 {
        match part {
            DataKeyPart::Bool(value) => {
                push_u8(output, 0);
                push_u8(output, u8::from(*value));
            }
            DataKeyPart::I64(value) => {
                push_u8(output, 1);
                output.extend_from_slice(&value.to_be_bytes());
            }
            DataKeyPart::Text(value) => {
                push_u8(output, 2);
                push_text(output, value, "data_key_text")?;
            }
            DataKeyPart::Bytes(value) => {
                push_u8(output, 3);
                push_blob(output, value, "data_key_bytes")?;
            }
        }
    }
    Ok(())
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize, code: &'static str) -> Result<&'a [u8], Diagnostic> {
        let end = self.offset.checked_add(length).ok_or_else(|| {
            data_error(
                DiagnosticClass::Corrupt,
                code,
                "data input offset overflowed",
            )
        })?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| data_error(DiagnosticClass::Corrupt, code, "data input is truncated"))?;
        self.offset = end;
        Ok(value)
    }

    fn expect_magic(&mut self, magic: &[u8; 8], code: &'static str) -> Result<(), Diagnostic> {
        if self.take(8, code)? != magic {
            return Err(data_error(
                DiagnosticClass::Corrupt,
                code,
                "data input has a foreign magic value",
            ));
        }
        Ok(())
    }

    fn u8(&mut self, code: &'static str) -> Result<u8, Diagnostic> {
        self.take(1, code)?
            .first()
            .copied()
            .ok_or_else(|| data_error(DiagnosticClass::Corrupt, code, "data input is truncated"))
    }

    fn u16(&mut self, code: &'static str) -> Result<u16, Diagnostic> {
        let bytes = self.take(2, code)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self, code: &'static str) -> Result<u32, Diagnostic> {
        let bytes = self.take(4, code)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn u64(&mut self, code: &'static str) -> Result<u64, Diagnostic> {
        let bytes = self.take(8, code)?;
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn usize(&mut self, code: &'static str) -> Result<usize, Diagnostic> {
        usize::try_from(self.u64(code)?).map_err(|_| {
            data_error(
                DiagnosticClass::Resource,
                code,
                "data input count does not fit this platform",
            )
        })
    }

    fn count(&mut self, code: &'static str, maximum: usize) -> Result<usize, Diagnostic> {
        let count = usize::try_from(self.u32(code)?).map_err(|_| {
            data_error(
                DiagnosticClass::Resource,
                code,
                "data input count is unsupported",
            )
        })?;
        if count > maximum {
            return Err(data_error(
                DiagnosticClass::Resource,
                code,
                format!("data input count {count} exceeds maximum {maximum}"),
            ));
        }
        Ok(count)
    }

    fn array_32(&mut self, code: &'static str) -> Result<[u8; 32], Diagnostic> {
        self.take(32, code)?
            .try_into()
            .map_err(|_| data_error(DiagnosticClass::Corrupt, code, "data input is truncated"))
    }

    fn blob(&mut self, code: &'static str, maximum: usize) -> Result<Vec<u8>, Diagnostic> {
        let length = self.usize(code)?;
        if length > maximum {
            return Err(data_error(
                DiagnosticClass::Resource,
                code,
                format!("data input field contains {length} bytes; maximum is {maximum}"),
            ));
        }
        Ok(self.take(length, code)?.to_vec())
    }

    fn text(&mut self, code: &'static str, maximum: usize) -> Result<String, Diagnostic> {
        let bytes = self.blob(code, maximum)?;
        String::from_utf8(bytes).map_err(|_| {
            data_error(
                DiagnosticClass::Corrupt,
                code,
                "data text field is not valid UTF-8",
            )
        })
    }

    fn key(&mut self, limits: &DataLimits) -> Result<DataKey, Diagnostic> {
        let count = self.count("data_key_parts", limits.maximum_key_parts)?;
        let mut parts = Vec::with_capacity(count);
        for _ in 0..count {
            let part = match self.u8("data_key_tag")? {
                0 => match self.u8("data_key_bool")? {
                    0 => DataKeyPart::Bool(false),
                    1 => DataKeyPart::Bool(true),
                    _ => {
                        return Err(data_error(
                            DiagnosticClass::Corrupt,
                            "data_key_bool",
                            "data key contains a noncanonical boolean",
                        ));
                    }
                },
                1 => {
                    let bytes = self.take(8, "data_key_i64")?;
                    DataKeyPart::I64(i64::from_be_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]))
                }
                2 => DataKeyPart::Text(self.text("data_key_text", limits.maximum_key_bytes)?),
                3 => DataKeyPart::Bytes(self.blob("data_key_bytes", limits.maximum_key_bytes)?),
                _ => {
                    return Err(data_error(
                        DiagnosticClass::Corrupt,
                        "data_key_tag",
                        "data key contains a foreign part tag",
                    ));
                }
            };
            parts.push(part);
        }
        if count == 0 {
            Ok(DataKey::empty_prefix())
        } else {
            DataKey::new(parts, limits)
        }
    }

    fn finish(self, code: &'static str) -> Result<(), Diagnostic> {
        if self.offset != self.bytes.len() {
            return Err(data_error(
                DiagnosticClass::Corrupt,
                code,
                "data input contains trailing bytes",
            ));
        }
        Ok(())
    }
}

fn seal(domain: &'static str, mut bytes: Vec<u8>) -> Vec<u8> {
    let checksum = digest(domain, &bytes);
    bytes.extend_from_slice(&checksum);
    bytes
}

fn open_envelope<'a>(
    bytes: &'a [u8],
    domain: &'static str,
    code: &'static str,
) -> Result<&'a [u8], Diagnostic> {
    let split = bytes
        .len()
        .checked_sub(32)
        .ok_or_else(|| data_error(DiagnosticClass::Corrupt, code, "data envelope is truncated"))?;
    let (payload, checksum) = bytes.split_at(split);
    if digest(domain, payload).as_slice() != checksum {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            code,
            "data envelope checksum does not match its payload",
        ));
    }
    Ok(payload)
}

fn digest(domain: &'static str, bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

fn push_u8(output: &mut Vec<u8>, value: u8) {
    output.push(value);
}

fn push_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn push_count(output: &mut Vec<u8>, value: usize, code: &'static str) -> Result<(), Diagnostic> {
    let value = u32::try_from(value).map_err(|_| {
        data_error(
            DiagnosticClass::Resource,
            code,
            "data count exceeds the canonical u32 range",
        )
    })?;
    output.extend_from_slice(&value.to_be_bytes());
    Ok(())
}

fn push_usize(output: &mut Vec<u8>, value: usize, code: &'static str) -> Result<(), Diagnostic> {
    let value = u64::try_from(value).map_err(|_| {
        data_error(
            DiagnosticClass::Resource,
            code,
            "data count exceeds the canonical u64 range",
        )
    })?;
    push_u64(output, value);
    Ok(())
}

fn push_blob(output: &mut Vec<u8>, value: &[u8], code: &'static str) -> Result<(), Diagnostic> {
    push_usize(output, value.len(), code)?;
    output.extend_from_slice(value);
    Ok(())
}

fn push_text(output: &mut Vec<u8>, value: &str, code: &'static str) -> Result<(), Diagnostic> {
    push_blob(output, value.as_bytes(), code)
}

fn reserve_live_transaction(store: &Arc<DataStoreInner>) -> Result<(), Diagnostic> {
    let mut current = store.live_transactions.load(AtomicOrdering::Acquire);
    loop {
        if current >= store.limits.maximum_live_transactions {
            return Err(data_error(
                DiagnosticClass::Resource,
                "data_live_transactions",
                "data live-transaction limit is exhausted",
            ));
        }
        match store.live_transactions.compare_exchange_weak(
            current,
            current + 1,
            AtomicOrdering::AcqRel,
            AtomicOrdering::Acquire,
        ) {
            Ok(_) => return Ok(()),
            Err(actual) => current = actual,
        }
    }
}

fn validate_limit(name: &str, value: usize, maximum: usize) -> Result<(), Diagnostic> {
    if value == 0 || value > maximum {
        return Err(data_error(
            DiagnosticClass::Source,
            "data_limit",
            format!("{name} must be 1 through {maximum}"),
        ));
    }
    Ok(())
}

fn validate_scan_limit(name: &str, value: usize, maximum: usize) -> Result<(), Diagnostic> {
    if value == 0 || value > maximum {
        return Err(data_error(
            DiagnosticClass::Resource,
            "data_scan_limit",
            format!("{name} must be 1 through {maximum}"),
        ));
    }
    Ok(())
}

fn validate_name(value: &str, maximum: usize, code: &'static str) -> Result<(), Diagnostic> {
    if value.is_empty() || value.len() > maximum {
        return Err(data_error(
            DiagnosticClass::Source,
            code,
            format!("name must contain 1 through {maximum} UTF-8 bytes"),
        ));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(data_error(
            DiagnosticClass::Source,
            code,
            "name must use only ASCII letters, digits, '-', '_', '.', or ':'",
        ));
    }
    Ok(())
}

fn validate_schema(schema: &DataSchema) -> Result<(), Diagnostic> {
    validate_name(
        &schema.identity,
        MAXIMUM_DATA_SPACE_NAME_BYTES,
        "data_schema_identity",
    )?;
    if schema.digest.is_empty() || schema.digest.len() > 128 {
        return Err(data_error(
            DiagnosticClass::Source,
            "data_schema_digest",
            "data schema digest must contain 1 through 128 bytes",
        ));
    }
    Ok(())
}

fn canonical_absent_destination(root: &Path) -> Result<(PathBuf, String), Diagnostic> {
    let name = root
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            data_error(
                DiagnosticClass::Source,
                "data_root_name",
                "data root must have a portable UTF-8 directory name",
            )
        })?
        .to_owned();
    let parent = root
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    reject_symlinked_existing_path(parent)?;
    let metadata =
        fs::symlink_metadata(parent).map_err(|error| data_io("data_parent", parent, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(data_error(
            DiagnosticClass::Source,
            "data_parent_type",
            "data root parent is not an ordinary directory",
        ));
    }
    let canonical = parent
        .canonicalize()
        .map_err(|error| data_io("data_parent", parent, error))?;
    Ok((canonical, name))
}

fn reject_symlinked_existing_path(path: &Path) -> Result<(), Diagnostic> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| data_io("data_current_directory", path, error))?
            .join(path)
    };
    let mut checked = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(prefix) => checked.push(prefix.as_os_str()),
            Component::RootDir => checked.push(Path::new("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(data_error(
                    DiagnosticClass::Source,
                    "data_path_traversal",
                    "data path may not contain '..'",
                ));
            }
            Component::Normal(value) => {
                checked.push(value);
                let metadata = fs::symlink_metadata(&checked)
                    .map_err(|error| data_io("data_path_metadata", &checked, error))?;
                if metadata.file_type().is_symlink() {
                    return Err(data_error(
                        DiagnosticClass::Source,
                        "data_path_symlink",
                        format!("data path traverses symbolic link '{}'", checked.display()),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_root_inventory(root: &Path) -> Result<(), Diagnostic> {
    let expected = BTreeSet::from([
        FORMAT_FILE.to_owned(),
        LOCK_FILE.to_owned(),
        HEAD_FILE.to_owned(),
        OBJECTS_DIRECTORY.to_owned(),
        STAGING_DIRECTORY.to_owned(),
    ]);
    let mut found = read_directory_entries(root, "data_root_inventory")?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let has_catalog = found.remove(CATALOG_DIRECTORY);
    if found != expected {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            "data_root_inventory",
            "data root inventory is incomplete or contains foreign entries",
        ));
    }
    validate_regular_file(&root.join(FORMAT_FILE), "data_format_type")?;
    validate_regular_file(&root.join(LOCK_FILE), "data_lock_type")?;
    validate_regular_file(&root.join(HEAD_FILE), "data_head_type")?;
    validate_directory(&root.join(OBJECTS_DIRECTORY), "data_objects_type")?;
    validate_directory(&root.join(STAGING_DIRECTORY), "data_staging_type")?;
    if has_catalog {
        validate_directory(&root.join(CATALOG_DIRECTORY), "data_catalog_type")?;
    }
    Ok(())
}

fn validate_regular_file(path: &Path, code: &'static str) -> Result<(), Diagnostic> {
    let metadata = fs::symlink_metadata(path).map_err(|error| data_io(code, path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            code,
            format!("'{}' is not an ordinary regular file", path.display()),
        ));
    }
    Ok(())
}

fn validate_directory(path: &Path, code: &'static str) -> Result<(), Diagnostic> {
    let metadata = fs::symlink_metadata(path).map_err(|error| data_io(code, path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            code,
            format!("'{}' is not an ordinary directory", path.display()),
        ));
    }
    Ok(())
}

fn read_directory_entries(path: &Path, code: &'static str) -> Result<Vec<String>, Diagnostic> {
    validate_directory(path, code)?;
    let mut names = Vec::new();
    for entry in fs::read_dir(path).map_err(|error| data_io(code, path, error))? {
        let entry = entry.map_err(|error| data_io(code, path, error))?;
        let name = entry.file_name().into_string().map_err(|_| {
            data_error(
                DiagnosticClass::Corrupt,
                code,
                "data directory contains a non-UTF-8 entry name",
            )
        })?;
        names.push(name);
    }
    names.sort();
    Ok(names)
}

fn read_limited_regular(
    path: &Path,
    maximum: usize,
    code: &'static str,
) -> Result<Vec<u8>, Diagnostic> {
    validate_regular_file(path, code)?;
    let metadata = fs::metadata(path).map_err(|error| data_io(code, path, error))?;
    let size = usize::try_from(metadata.len()).map_err(|_| {
        data_error(
            DiagnosticClass::Resource,
            code,
            "data file length does not fit this platform",
        )
    })?;
    if size > maximum {
        return Err(data_error(
            DiagnosticClass::Resource,
            code,
            format!("data file contains {size} bytes; maximum is {maximum}"),
        ));
    }
    let mut file = File::open(path).map_err(|error| data_io(code, path, error))?;
    let mut bytes = Vec::with_capacity(size);
    file.read_to_end(&mut bytes)
        .map_err(|error| data_io(code, path, error))?;
    if bytes.len() != size {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            code,
            "data file changed while it was being read",
        ));
    }
    Ok(bytes)
}

fn write_new_synced(path: &Path, bytes: &[u8], code: &'static str) -> Result<(), Diagnostic> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| data_io(code, path, error))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| data_io(code, path, error))
}

fn publish_create_new_file(
    path: &Path,
    bytes: &[u8],
    maximum: usize,
    label: &str,
) -> Result<(), Diagnostic> {
    if bytes.len() > maximum {
        return Err(data_error(
            DiagnosticClass::Resource,
            "data_output_bytes",
            format!("{label} exceeds its {maximum}-byte bound"),
        ));
    }
    let (parent, name) = canonical_absent_destination(path)?;
    let stage = parent.join(format!(".{name}.data-output-stage-{}", random_hex()?));
    write_new_synced(&stage, bytes, "data_output_stage")?;
    let output = parent.join(name);
    let published = fs::hard_link(&stage, &output);
    if let Err(error) = published {
        let _ = fs::remove_file(&stage);
        let (class, code) = if error.kind() == std::io::ErrorKind::AlreadyExists {
            (DiagnosticClass::Source, "data_output_exists")
        } else {
            (DiagnosticClass::Infrastructure, "data_output_publish")
        };
        return Err(data_error(
            class,
            code,
            format!("{label} could not be published: {error}"),
        ));
    }
    sync_directory(&parent, "data_output_parent_sync")?;
    let _ = fs::remove_file(&stage);
    let _ = sync_directory(&parent, "data_output_cleanup_sync");
    Ok(())
}

fn sync_directory(path: &Path, code: &'static str) -> Result<(), Diagnostic> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| data_io(code, path, error))
}

fn revision_path(root: &Path, revision: [u8; 32]) -> PathBuf {
    root.join(OBJECTS_DIRECTORY)
        .join(format!("{}.lkjd", encode_hex(&revision)))
}

fn parse_revision_name(name: &str) -> Result<[u8; 32], Diagnostic> {
    let encoded = name.strip_suffix(".lkjd").ok_or_else(|| {
        data_error(
            DiagnosticClass::Corrupt,
            "data_object_name",
            format!("foreign immutable data object name '{name}'"),
        )
    })?;
    decode_hex_32(encoded, "data_object_name")
}

fn random_identity() -> Result<[u8; 32], Diagnostic> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| {
        data_error(
            DiagnosticClass::Infrastructure,
            "data_random",
            format!("operating-system randomness is unavailable: {error}"),
        )
    })?;
    Ok(bytes)
}

fn random_hex() -> Result<String, Diagnostic> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        data_error(
            DiagnosticClass::Infrastructure,
            "data_random",
            format!("operating-system randomness is unavailable: {error}"),
        )
    })?;
    Ok(encode_hex(&bytes))
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn decode_hex_32(value: &str, code: &'static str) -> Result<[u8; 32], Diagnostic> {
    if value.len() != 64 {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            code,
            "data identity must contain 64 lowercase hexadecimal characters",
        ));
    }
    let mut output = [0_u8; 32];
    let (pairs, remainder) = value.as_bytes().as_chunks::<2>();
    if !remainder.is_empty() {
        return Err(invalid_hex(code));
    }
    for (index, pair) in pairs.iter().enumerate() {
        let high = decode_hex_digit(pair[0]).ok_or_else(|| invalid_hex(code))?;
        let low = decode_hex_digit(pair[1]).ok_or_else(|| invalid_hex(code))?;
        output[index] = (high << 4) | low;
    }
    Ok(output)
}

fn decode_hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn invalid_hex(code: &'static str) -> Diagnostic {
    data_error(
        DiagnosticClass::Corrupt,
        code,
        "data identity is not canonical lowercase hexadecimal",
    )
}

fn format_store(store_id: [u8; 32]) -> String {
    format!("data_store_{}", encode_hex(&store_id))
}

fn format_revision(revision: [u8; 32]) -> String {
    format!("data_revision_{}", encode_hex(&revision))
}

fn path_exists(path: &Path) -> Result<bool, Diagnostic> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(data_io("data_path_metadata", path, error)),
    }
}

fn remove_owned_stage(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

fn data_transaction_overflow() -> Diagnostic {
    data_error(
        DiagnosticClass::Resource,
        "data_transaction_bytes",
        "data transaction accounting overflowed",
    )
}

fn data_scan_overflow() -> Diagnostic {
    data_error(
        DiagnosticClass::Resource,
        "data_scan_accounting",
        "data scan accounting overflowed",
    )
}

fn data_io(code: &'static str, path: &Path, error: std::io::Error) -> Diagnostic {
    data_error(
        DiagnosticClass::Infrastructure,
        code,
        format!("data path '{}' failed: {error}", path.display()),
    )
}

fn data_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommitCheckpoint {
    BeforeRevisionStage,
    RevisionBytesWritten,
    RevisionStageSynced,
    RevisionPublished,
    HeadStageSynced,
    HeadPublished,
    RootSynced,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::thread;
    use std::time::{Duration, Instant};

    fn key(parts: Vec<DataKeyPart>) -> DataKey {
        DataKey::new(parts, &DataLimits::default()).expect("valid test key")
    }

    fn opened(root: &Path, namespace: &str) -> DataStore {
        DataStore::open(root, namespace, DataLimits::default()).expect("open data store")
    }

    fn mixed_key(index: usize) -> DataKey {
        let discriminator = match index % 4 {
            0 => DataKeyPart::Bool(index.is_multiple_of(8)),
            1 => DataKeyPart::I64(i64::try_from(index).expect("small key index") - 16),
            2 => DataKeyPart::Text(format!("key-{index:02}")),
            _ => DataKeyPart::Bytes(vec![u8::try_from(index).expect("small key index")]),
        };
        key(vec![
            DataKeyPart::Text("group".to_owned()),
            discriminator,
            DataKeyPart::I64(i64::try_from(index).expect("small key index")),
        ])
    }

    #[derive(Clone, Copy)]
    struct ReferenceRandom(u64);

    impl ReferenceRandom {
        fn next(&mut self) -> u64 {
            let mut value = self.0;
            value ^= value << 13;
            value ^= value >> 7;
            value ^= value << 17;
            self.0 = value;
            value
        }

        fn index(&mut self, upper: usize) -> usize {
            let upper = u64::try_from(upper).expect("test bound fits u64");
            usize::try_from(self.next() % upper).expect("bounded index fits usize")
        }
    }

    #[test]
    fn key_order_is_explicit_and_prefix_ordered() {
        let mut keys = vec![
            key(vec![DataKeyPart::Bytes(vec![0])]),
            key(vec![DataKeyPart::Text("a".to_owned())]),
            key(vec![DataKeyPart::I64(-1)]),
            key(vec![DataKeyPart::Bool(true)]),
            key(vec![DataKeyPart::Bool(false), DataKeyPart::I64(0)]),
            key(vec![DataKeyPart::Bool(false)]),
        ];
        keys.sort();
        assert_eq!(
            keys,
            vec![
                key(vec![DataKeyPart::Bool(false)]),
                key(vec![DataKeyPart::Bool(false), DataKeyPart::I64(0)]),
                key(vec![DataKeyPart::Bool(true)]),
                key(vec![DataKeyPart::I64(-1)]),
                key(vec![DataKeyPart::Text("a".to_owned())]),
                key(vec![DataKeyPart::Bytes(vec![0])]),
            ]
        );
    }

    #[test]
    fn initialize_transactions_expectations_aba_and_reopen() {
        let temporary = tempfile::TempDir::new().expect("temporary root");
        let root = temporary.path().join("data");
        let initialized = DataStore::initialize(&root).expect("initialize");
        assert_eq!(initialized.outcome, DataInitializeOutcome::Created);
        assert_eq!(
            DataStore::initialize(&root)
                .expect("idempotent initialize")
                .outcome,
            DataInitializeOutcome::Unchanged
        );
        let store = opened(&root, "test");
        let record = key(vec![DataKeyPart::Text("one".to_owned())]);
        let mut first = store.begin().expect("first transaction");
        assert!(
            first
                .put(
                    "records",
                    &record,
                    b"first".to_vec(),
                    DataExpectation::Missing
                )
                .expect("put")
        );
        assert_eq!(
            first
                .get("records", &record)
                .expect("read staged")
                .expect("entry")
                .value,
            b"first"
        );
        assert!(matches!(
            first.commit().expect("commit"),
            DataCommitOutcome::Committed { .. }
        ));
        let mut delete = store.begin().expect("delete transaction");
        let old_revision = delete
            .get("records", &record)
            .expect("get")
            .expect("entry")
            .revision;
        assert!(
            delete
                .delete("records", &record, DataExpectation::Exact(old_revision))
                .expect("delete")
        );
        delete.commit().expect("commit delete");
        let mut recreate = store.begin().expect("recreate transaction");
        assert!(
            recreate
                .put(
                    "records",
                    &record,
                    b"second".to_vec(),
                    DataExpectation::Missing
                )
                .expect("recreate")
        );
        recreate.commit().expect("commit recreate");
        let reopened = opened(&root, "test");
        let current = reopened.begin().expect("current");
        let new_revision = current
            .get("records", &record)
            .expect("get")
            .expect("entry")
            .revision;
        assert_ne!(old_revision, new_revision);
        let mut stale = reopened.begin().expect("stale expectation transaction");
        assert!(
            !stale
                .put(
                    "records",
                    &record,
                    b"wrong".to_vec(),
                    DataExpectation::Exact(old_revision),
                )
                .expect("stale put")
        );
        assert!(matches!(
            stale.commit().expect("unchanged"),
            DataCommitOutcome::Unchanged { .. }
        ));
    }

    #[test]
    fn exact_base_conflict_has_no_visibility() {
        let temporary = tempfile::TempDir::new().expect("temporary root");
        let root = temporary.path().join("data");
        DataStore::initialize(&root).expect("initialize");
        let store = opened(&root, "test");
        let mut left = store.begin().expect("left");
        let mut right = store.begin().expect("right");
        let left_key = key(vec![DataKeyPart::Text("left".to_owned())]);
        let right_key = key(vec![DataKeyPart::Text("right".to_owned())]);
        left.put("records", &left_key, vec![1], DataExpectation::Missing)
            .expect("left put");
        right
            .put("records", &right_key, vec![2], DataExpectation::Missing)
            .expect("right put");
        left.commit().expect("left commit");
        assert!(matches!(
            right.commit().expect("right conflict"),
            DataCommitOutcome::Conflict { .. }
        ));
        let read = store.begin().expect("read");
        assert!(read.get("records", &left_key).expect("left get").is_some());
        assert!(
            read.get("records", &right_key)
                .expect("right get")
                .is_none()
        );
    }

    #[test]
    fn scans_are_snapshot_ordered_reverse_and_continuable() {
        let temporary = tempfile::TempDir::new().expect("temporary root");
        let root = temporary.path().join("data");
        DataStore::initialize(&root).expect("initialize");
        let store = opened(&root, "test");
        let mut write = store.begin().expect("write");
        for value in 0..5_i64 {
            write
                .put(
                    "records",
                    &key(vec![
                        DataKeyPart::Text("p".to_owned()),
                        DataKeyPart::I64(value),
                    ]),
                    vec![u8::try_from(value).expect("small value")],
                    DataExpectation::Missing,
                )
                .expect("put");
        }
        write.commit().expect("commit");
        let read = store.begin().expect("read");
        let first = read
            .scan(
                "records",
                &[DataKeyPart::Text("p".to_owned())],
                DataScanDirection::Forward,
                2,
                1024,
                100,
                None,
            )
            .expect("first page");
        assert_eq!(first.items.len(), 2);
        let continuation = first.continuation.expect("continuation");
        let second = read
            .scan(
                "records",
                &[DataKeyPart::Text("p".to_owned())],
                DataScanDirection::Forward,
                2,
                1024,
                100,
                Some(&continuation),
            )
            .expect("second page");
        assert_eq!(second.items.len(), 2);
        let reverse = read
            .scan(
                "records",
                &[DataKeyPart::Text("p".to_owned())],
                DataScanDirection::Reverse,
                5,
                4096,
                100,
                None,
            )
            .expect("reverse");
        assert_eq!(reverse.items[0].value, vec![4]);
        let mismatch = read.scan(
            "records",
            &[DataKeyPart::Text("p".to_owned())],
            DataScanDirection::Forward,
            3,
            1024,
            100,
            Some(&continuation),
        );
        assert_eq!(
            mismatch.expect_err("limits bind continuation").code,
            "data_continuation_selector"
        );

        let mut later = store.begin().expect("later write");
        later
            .put(
                "records",
                &key(vec![DataKeyPart::Text("later".to_owned())]),
                vec![9],
                DataExpectation::Missing,
            )
            .expect("later put");
        later.commit().expect("later commit");
        assert_eq!(
            store
                .begin()
                .expect("new snapshot")
                .scan(
                    "records",
                    &[DataKeyPart::Text("p".to_owned())],
                    DataScanDirection::Forward,
                    2,
                    1024,
                    100,
                    Some(&continuation),
                )
                .expect_err("revision binds continuation")
                .code,
            "data_continuation_selector"
        );
        let mut corrupt = continuation;
        corrupt[0] ^= 1;
        assert_eq!(
            read.scan(
                "records",
                &[DataKeyPart::Text("p".to_owned())],
                DataScanDirection::Forward,
                2,
                1024,
                100,
                Some(&corrupt),
            )
            .expect_err("corrupt continuation")
            .code,
            "data_continuation_checksum"
        );
    }

    #[test]
    fn randomized_transactions_match_an_independent_ordered_map_model() {
        let temporary = tempfile::TempDir::new().expect("temporary root");
        let root = temporary.path().join("data");
        DataStore::initialize(&root).expect("initialize");
        let store = opened(&root, "model");
        let mut model = BTreeMap::<DataKey, Vec<u8>>::new();
        let mut random = ReferenceRandom(0x8dd4_e231_6a77_091b);

        for round in 0..384_u64 {
            let mut transaction = store.begin().expect("model transaction");
            let mut candidate = model.clone();
            let mut failed_expectation = false;
            let actions = 1 + random.index(6);
            for action in 0..actions {
                let record_key = mixed_key(random.index(32));
                let observed = transaction.get("facts", &record_key).expect("model get");
                let inject_stale = (round + u64::try_from(action).expect("small action")) % 29 == 0;
                let expectation = if inject_stale {
                    match observed {
                        Some(_) => DataExpectation::Missing,
                        None => DataExpectation::Exact(DataEntryRevision::from_bytes([0xa5; 32])),
                    }
                } else {
                    observed.as_ref().map_or(DataExpectation::Missing, |entry| {
                        DataExpectation::Exact(entry.revision)
                    })
                };
                let delete = observed.is_some() && random.next() & 3 == 0;
                let accepted = if delete {
                    transaction
                        .delete("facts", &record_key, expectation)
                        .expect("model delete")
                } else {
                    let value = format!("round-{round}-action-{action}").into_bytes();
                    let accepted = transaction
                        .put("facts", &record_key, value.clone(), expectation)
                        .expect("model put");
                    if accepted {
                        candidate.insert(record_key.clone(), value);
                    }
                    accepted
                };
                if accepted && delete {
                    candidate.remove(&record_key);
                }
                failed_expectation |= !accepted;
            }
            let outcome = transaction.commit().expect("model commit");
            if failed_expectation {
                assert!(matches!(outcome, DataCommitOutcome::Unchanged { .. }));
            } else {
                model = candidate;
            }

            let observed = store
                .begin()
                .expect("model read")
                .scan(
                    "facts",
                    &[],
                    DataScanDirection::Forward,
                    1_000,
                    1_048_576,
                    10_000,
                    None,
                )
                .expect("model scan")
                .items
                .into_iter()
                .map(|item| (item.key, item.value))
                .collect::<BTreeMap<_, _>>();
            assert_eq!(observed, model, "model diverged after round {round}");
        }
    }

    #[test]
    fn insertion_order_does_not_change_canonical_scan_facts() {
        let temporary = tempfile::TempDir::new().expect("temporary root");
        let left_root = temporary.path().join("left");
        let right_root = temporary.path().join("right");
        DataStore::initialize(&left_root).expect("left initialize");
        DataStore::initialize(&right_root).expect("right initialize");
        let left = opened(&left_root, "order");
        let right = opened(&right_root, "order");
        let facts = (0..32_usize)
            .map(|index| (mixed_key(index), format!("value-{index}").into_bytes()))
            .collect::<Vec<_>>();

        let mut left_write = left.begin().expect("left transaction");
        for (record_key, value) in &facts {
            left_write
                .put("facts", record_key, value.clone(), DataExpectation::Missing)
                .expect("left put");
        }
        left_write.commit().expect("left commit");

        let mut right_write = right.begin().expect("right transaction");
        for (record_key, value) in facts.iter().rev() {
            right_write
                .put("facts", record_key, value.clone(), DataExpectation::Missing)
                .expect("right put");
        }
        right_write.commit().expect("right commit");

        let scan = |store: &DataStore| {
            store
                .begin()
                .expect("ordered read")
                .scan(
                    "facts",
                    &[],
                    DataScanDirection::Forward,
                    100,
                    1_048_576,
                    1_000,
                    None,
                )
                .expect("ordered scan")
                .items
                .into_iter()
                .map(|item| (item.key, item.value))
                .collect::<Vec<_>>()
        };
        assert_eq!(scan(&left), scan(&right));
    }

    #[test]
    fn schemas_resources_and_failed_dependent_writes_are_exact() {
        let temporary = tempfile::TempDir::new().expect("temporary root");
        let root = temporary.path().join("data");
        DataStore::initialize(&root).expect("initialize");
        let limits = DataLimits {
            maximum_live_transactions: 1,
            maximum_transaction_mutations: 2,
            ..DataLimits::default()
        };
        let store = DataStore::open(&root, "schema", limits).expect("bounded open");
        let first = store.begin().expect("reserved transaction");
        assert_eq!(
            store.begin().expect_err("live transaction limit").code,
            "data_live_transactions"
        );
        drop(first);

        let version_one = DataSchema {
            identity: "example.records.v1".to_owned(),
            digest: vec![1; 32],
        };
        let version_two = DataSchema {
            identity: "example.records.v2".to_owned(),
            digest: vec![2; 32],
        };
        let mut migration = store.begin().expect("first migration");
        assert!(
            migration
                .schema_set(
                    "records",
                    &DataSchemaExpectation::Missing,
                    version_one.clone()
                )
                .expect("set first schema")
        );
        assert_eq!(
            migration
                .schema_read("records")
                .expect("read staged schema"),
            Some(version_one.clone())
        );
        migration.commit().expect("commit first schema");

        let prior = store.current_revision().expect("prior revision");
        let mut retry = store.begin().expect("exact retry");
        assert!(
            retry
                .schema_set(
                    "records",
                    &DataSchemaExpectation::Exact(version_one.clone()),
                    version_one.clone(),
                )
                .expect("exact retry")
        );
        assert!(matches!(
            retry.commit().expect("retry commit"),
            DataCommitOutcome::Unchanged { .. }
        ));
        assert_eq!(store.current_revision().expect("unchanged revision"), prior);

        let dependent = mixed_key(3);
        let mut divergent = store.begin().expect("divergent migration");
        assert!(
            divergent
                .put(
                    "records",
                    &dependent,
                    b"must-roll-back".to_vec(),
                    DataExpectation::Missing,
                )
                .expect("dependent put")
        );
        assert!(
            !divergent
                .schema_set(
                    "records",
                    &DataSchemaExpectation::Missing,
                    version_two.clone(),
                )
                .expect("divergent marker")
        );
        assert!(matches!(
            divergent.commit().expect("divergent commit"),
            DataCommitOutcome::Unchanged { .. }
        ));
        assert!(
            store
                .begin()
                .expect("rollback read")
                .get("records", &dependent)
                .expect("rollback get")
                .is_none()
        );

        let mut next = store.begin().expect("next migration");
        assert!(
            next.schema_set(
                "records",
                &DataSchemaExpectation::Exact(version_one),
                version_two.clone(),
            )
            .expect("next schema")
        );
        next.commit().expect("commit next schema");
        assert_eq!(
            store
                .begin()
                .expect("next read")
                .schema_read("records")
                .expect("next schema read"),
            Some(version_two)
        );
    }

    #[test]
    fn readers_pin_immutable_snapshots_while_a_writer_commits() {
        let temporary = tempfile::TempDir::new().expect("temporary root");
        let root = temporary.path().join("data");
        DataStore::initialize(&root).expect("initialize");
        let store = opened(&root, "concurrent");
        let pinned = store.begin().expect("pinned reader");
        let writer_store = store.clone();
        let record = mixed_key(11);
        let writer_key = record.clone();
        thread::spawn(move || {
            let mut writer = writer_store.begin().expect("writer transaction");
            writer
                .put(
                    "facts",
                    &writer_key,
                    b"visible-later".to_vec(),
                    DataExpectation::Missing,
                )
                .expect("writer put");
            writer.commit().expect("writer commit");
        })
        .join()
        .expect("writer thread");
        assert!(pinned.get("facts", &record).expect("pinned get").is_none());
        assert_eq!(
            store
                .begin()
                .expect("new reader")
                .get("facts", &record)
                .expect("new get")
                .expect("new fact")
                .value,
            b"visible-later"
        );
    }

    #[test]
    fn separate_processes_serialize_writers_and_retry_exact_base_conflicts() {
        let temporary = tempfile::TempDir::new().expect("temporary root");
        let root = temporary.path().join("data");
        let barrier = temporary.path().join("barrier");
        fs::create_dir(&barrier).expect("barrier directory");
        DataStore::initialize(&root).expect("initialize");
        let executable = std::env::current_exe().expect("current test executable");
        let child = |identity: &str| {
            Command::new(&executable)
                .args([
                    "--ignored",
                    "--exact",
                    "platform::data::tests::cross_process_writer_helper",
                ])
                .env("LKJSCRIPT_DATA_TEST_ROOT", &root)
                .env("LKJSCRIPT_DATA_TEST_BARRIER", &barrier)
                .env("LKJSCRIPT_DATA_TEST_IDENTITY", identity)
                .spawn()
                .expect("spawn data writer child")
        };
        let mut left = child("left");
        let mut right = child("right");
        assert!(left.wait().expect("left child").success());
        assert!(right.wait().expect("right child").success());
        let read = opened(&root, "process").begin().expect("process read");
        for identity in ["left", "right"] {
            let record = key(vec![DataKeyPart::Text(identity.to_owned())]);
            assert_eq!(
                read.get("facts", &record)
                    .expect("process get")
                    .expect("process fact")
                    .value,
                identity.as_bytes()
            );
        }
    }

    #[test]
    #[ignore = "invoked as a controlled subprocess by the cross-process writer proof"]
    fn cross_process_writer_helper() {
        let Some(root) = std::env::var_os("LKJSCRIPT_DATA_TEST_ROOT").map(PathBuf::from) else {
            return;
        };
        let barrier = std::env::var_os("LKJSCRIPT_DATA_TEST_BARRIER")
            .map(PathBuf::from)
            .expect("child barrier");
        let identity = std::env::var("LKJSCRIPT_DATA_TEST_IDENTITY").expect("child identity");
        let record = key(vec![DataKeyPart::Text(identity.clone())]);
        for attempt in 0..32_u8 {
            let store = opened(&root, "process");
            let mut transaction = store.begin().expect("child transaction");
            if transaction
                .get("facts", &record)
                .expect("child get")
                .is_some()
            {
                return;
            }
            transaction
                .put(
                    "facts",
                    &record,
                    identity.as_bytes().to_vec(),
                    DataExpectation::Missing,
                )
                .expect("child put");
            if attempt == 0 {
                fs::write(barrier.join(&identity), b"ready").expect("child ready marker");
                let deadline = Instant::now() + Duration::from_secs(5);
                while fs::read_dir(&barrier).expect("read barrier").count() < 2 {
                    assert!(Instant::now() < deadline, "child barrier timed out");
                    thread::sleep(Duration::from_millis(5));
                }
            }
            match transaction.commit().expect("child commit") {
                DataCommitOutcome::Committed { .. } => return,
                DataCommitOutcome::Conflict { .. } => {}
                DataCommitOutcome::Unchanged { .. } => {
                    panic!("child write unexpectedly remained unchanged");
                }
            }
        }
        panic!("child exhausted exact-base conflict retries");
    }

    #[test]
    fn backup_restore_changes_physical_identity_and_preserves_facts() {
        let temporary = tempfile::TempDir::new().expect("temporary root");
        let root = temporary.path().join("data");
        let restored_root = temporary.path().join("restored");
        let backup = temporary.path().join("backup.lkjd");
        DataStore::initialize(&root).expect("initialize");
        let store = opened(&root, "app");
        let mut write = store.begin().expect("write");
        let record = key(vec![DataKeyPart::I64(7)]);
        write
            .put("facts", &record, b"fact".to_vec(), DataExpectation::Missing)
            .expect("put");
        write.commit().expect("commit");
        store.backup(&backup).expect("backup");
        DataStore::restore(&backup, &restored_root).expect("restore");
        let restored = opened(&restored_root, "app");
        assert_ne!(store.store_identity(), restored.store_identity());
        assert_eq!(
            restored
                .begin()
                .expect("read")
                .get("facts", &record)
                .expect("get")
                .expect("fact")
                .value,
            b"fact"
        );
        assert_eq!(restored.verify().expect("verify").records, 1);
    }

    #[test]
    fn derived_catalog_damage_is_ignored_and_rebuilt_from_immutable_objects() {
        let temporary = tempfile::TempDir::new().expect("temporary root");
        let root = temporary.path().join("data");
        DataStore::initialize(&root).expect("initialize");
        let store = opened(&root, "catalog");
        let catalog = root.join(CATALOG_DIRECTORY).join(CATALOG_FILE);
        let initial = read_limited_regular(
            &catalog,
            MAXIMUM_DATA_CATALOG_BYTES,
            "data_test_catalog_read",
        )
        .expect("read initial catalog");
        assert_eq!(
            decode_catalog(&initial, store.inner.store_id)
                .expect("decode initial catalog")
                .len(),
            1
        );

        fs::write(&catalog, b"corrupt-derived-catalog").expect("damage catalog");
        File::open(&catalog)
            .expect("open damaged catalog")
            .sync_all()
            .expect("sync damaged catalog");
        assert_eq!(
            store
                .verify()
                .expect("verify without catalog authority")
                .records,
            0
        );
        let mut first = store.begin().expect("first catalog rebuild");
        first
            .put(
                "facts",
                &mixed_key(20),
                b"first".to_vec(),
                DataExpectation::Missing,
            )
            .expect("first catalog put");
        first.commit().expect("first catalog commit");
        let rebuilt = read_limited_regular(
            &catalog,
            MAXIMUM_DATA_CATALOG_BYTES,
            "data_test_catalog_read",
        )
        .expect("read rebuilt catalog");
        assert_eq!(
            decode_catalog(&rebuilt, store.inner.store_id)
                .expect("decode rebuilt catalog")
                .len(),
            2
        );

        fs::remove_file(&catalog).expect("remove derived catalog file");
        fs::remove_dir(root.join(CATALOG_DIRECTORY)).expect("remove derived catalog directory");
        let reopened = opened(&root, "catalog");
        let mut second = reopened.begin().expect("missing catalog write");
        second
            .put(
                "facts",
                &mixed_key(21),
                b"second".to_vec(),
                DataExpectation::Missing,
            )
            .expect("second catalog put");
        second.commit().expect("second catalog commit");
        let rebuilt = read_limited_regular(
            &catalog,
            MAXIMUM_DATA_CATALOG_BYTES,
            "data_test_catalog_read",
        )
        .expect("read recreated catalog");
        assert_eq!(
            decode_catalog(&rebuilt, reopened.inner.store_id)
                .expect("decode recreated catalog")
                .len(),
            3
        );
    }

    #[test]
    fn corruption_and_symlink_roots_fail_closed() {
        let temporary = tempfile::TempDir::new().expect("temporary root");
        let root = temporary.path().join("data");
        DataStore::initialize(&root).expect("initialize");
        let head = root.join(HEAD_FILE);
        let mut bytes = fs::read(&head).expect("read head");
        bytes[0] ^= 1;
        fs::write(&head, bytes).expect("corrupt head");
        File::open(&head)
            .expect("open head")
            .sync_all()
            .expect("sync corruption");
        assert_eq!(
            DataStore::open(&root, "test", DataLimits::default())
                .expect_err("corrupt head")
                .class,
            DiagnosticClass::Corrupt
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let linked = temporary.path().join("linked");
            symlink(&root, &linked).expect("symlink root");
            assert_eq!(
                DataStore::open(&linked, "test", DataLimits::default())
                    .expect_err("symlink root")
                    .code,
                "data_path_symlink"
            );
        }
    }

    #[test]
    fn interruption_reopens_exactly_old_or_new_state() {
        for checkpoint in [
            CommitCheckpoint::BeforeRevisionStage,
            CommitCheckpoint::RevisionBytesWritten,
            CommitCheckpoint::RevisionStageSynced,
            CommitCheckpoint::RevisionPublished,
            CommitCheckpoint::HeadStageSynced,
            CommitCheckpoint::HeadPublished,
            CommitCheckpoint::RootSynced,
        ] {
            let temporary = tempfile::TempDir::new().expect("temporary root");
            let root = temporary.path().join("data");
            DataStore::initialize(&root).expect("initialize");
            let store = opened(&root, "test");
            let old = store.current_revision().expect("old head");
            let record = key(vec![DataKeyPart::Text("key".to_owned())]);
            let mut transaction = store.begin().expect("transaction");
            transaction
                .put(
                    "records",
                    &record,
                    b"new".to_vec(),
                    DataExpectation::Missing,
                )
                .expect("put");
            let result = transaction.commit_inner(&mut |point| {
                if point == checkpoint {
                    Err(data_error(
                        DiagnosticClass::Infrastructure,
                        "data_test_interruption",
                        "injected interruption",
                    ))
                } else {
                    Ok(())
                }
            });
            assert!(result.is_err());
            let reopened = opened(&root, "test");
            let new = reopened.current_revision().expect("reopened head");
            let visible = reopened
                .begin()
                .expect("read")
                .get("records", &record)
                .expect("get")
                .is_some();
            if matches!(
                checkpoint,
                CommitCheckpoint::HeadPublished | CommitCheckpoint::RootSynced
            ) {
                assert_ne!(new, old);
                assert!(visible);
            } else {
                assert_eq!(new, old);
                assert!(!visible);
            }
        }
    }
}
