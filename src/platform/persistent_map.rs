//! Canonical path-compressed Merkle radix map prototype.
//!
//! The logical map is a sorted set of bounded byte-string keys and bounded byte-string values.
//! Page shape is a pure function of that set: a subtree is one leaf when its canonical leaf
//! encoding fits the target size (or it contains one entry), and otherwise branches at the first
//! byte after the subtree's longest common prefix. Consequently, equal maps have equal roots
//! regardless of insertion history.
//!
//! Pages are immutable, domain-separated, content-addressed objects. The [`PageStore`] trait is
//! intentionally narrower than the repository store: it supports later staging, packing, and
//! publication without exposing physical coordinates as map identity.

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const PAGE_MAGIC: [u8; 8] = *b"LKJPMAP1";
const PAGE_CONTRACT_VERSION: u16 = 1;
const PAGE_DIGEST_DOMAIN: &str = "lkjscript.persistent-map-page.v1";
const PAGE_CHECKSUM_DOMAIN: &str = "lkjscript.persistent-map-checksum.v1";
const PAGE_HEADER_BYTES: usize = 8 + 2 + 1 + 1 + 8;
const PAGE_CHECKSUM_BYTES: usize = 32;
const PAGE_COMMON_PAYLOAD_BYTES: usize = 2 + 8 + 8;
const LEAF_COUNT_BYTES: usize = 4;
const LEAF_ENTRY_OVERHEAD_BYTES: usize = 2 + 4;
const BRANCH_CHILD_BYTES: usize = 1 + 32 + 8 + 8;

/// Key bytes are deliberately bounded at the storage boundary. Current semantic names and typed
/// stable IDs fit well below this value.
pub const MAXIMUM_KEY_BYTES: usize = 256;
/// Larger semantic values should be stored as separately chunked objects and referenced by digest.
pub const MAXIMUM_VALUE_BYTES: usize = 48 * 1024;
/// Multi-entry leaves larger than this are deterministically split.
pub const TARGET_LEAF_PAGE_BYTES: usize = 16 * 1024;
/// Hostile page inputs are rejected before length-directed allocation.
pub const MAXIMUM_PAGE_BYTES: usize = 64 * 1024;
const MAXIMUM_LEAF_RECORDS: usize = MAXIMUM_PAGE_BYTES / LEAF_ENTRY_OVERHEAD_BYTES;
const MAXIMUM_TREE_DEPTH: usize = MAXIMUM_KEY_BYTES + 1;

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct PageDigest([u8; 32]);

impl PageDigest {
    pub fn of(bytes: &[u8]) -> Self {
        Self(domain_digest(PAGE_DIGEST_DOMAIN, bytes))
    }

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for PageDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("map_page_")?;
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MapRoot {
    page: PageDigest,
    entries: u64,
}

impl MapRoot {
    pub const fn from_parts(page: PageDigest, entries: u64) -> Self {
        Self { page, entries }
    }

    pub const fn page(self) -> PageDigest {
        self.page
    }

    pub const fn entries(self) -> u64 {
        self.entries
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PersistentMap {
    root: MapRoot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapErrorClass {
    Input,
    Resource,
    Corrupt,
    Store,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapError {
    pub class: MapErrorClass,
    pub code: &'static str,
    pub message: String,
}

impl MapError {
    fn new(class: MapErrorClass, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            class,
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for MapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for MapError {}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MapWork {
    pub pages_read: u64,
    pub pages_decoded: u64,
    pub pages_encoded: u64,
    pub pages_written: u64,
    pub pages_reused: u64,
    pub bytes_read: u64,
    pub bytes_encoded: u64,
    pub bytes_written: u64,
    pub key_comparisons: u64,
    pub entries_visited: u64,
    pub differences_emitted: u64,
    pub subtrees_skipped: u64,
    pub entries_skipped: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageWrite {
    Inserted,
    Reused,
}

/// Immutable object interface. Implementations must bound reads to [`MAXIMUM_PAGE_BYTES`] before
/// allocation and reject a digest collision with foreign bytes.
pub trait PageStore {
    fn read_page(&self, digest: PageDigest) -> Result<Option<Vec<u8>>, MapError>;

    fn write_page(&mut self, digest: PageDigest, bytes: &[u8]) -> Result<PageWrite, MapError>;
}

#[derive(Clone, Debug, Default)]
pub struct MemoryPageStore {
    pages: BTreeMap<PageDigest, Vec<u8>>,
}

impl MemoryPageStore {
    pub fn object_count(&self) -> usize {
        self.pages.len()
    }

    pub fn stored_bytes(&self) -> usize {
        self.pages.values().map(Vec::len).sum()
    }

    pub fn objects(&self) -> impl Iterator<Item = (PageDigest, &[u8])> {
        self.pages
            .iter()
            .map(|(digest, bytes)| (*digest, bytes.as_slice()))
    }
}

/// Read-through staging store used by local path-copy updates. Every page produced by the mutation
/// is retained, including exact physical reuse, so final reachability can preserve newly produced
/// descendants below a reused ancestor. [`PageWrite::Reused`] remains accounting information.
pub struct OverlayPageStore<'a, S: PageStore + ?Sized> {
    base: &'a S,
    pages: MemoryPageStore,
}

impl<'a, S: PageStore + ?Sized> OverlayPageStore<'a, S> {
    pub const fn new(base: &'a S) -> Self {
        Self {
            base,
            pages: MemoryPageStore {
                pages: BTreeMap::new(),
            },
        }
    }

    pub fn into_pages(self) -> MemoryPageStore {
        self.pages
    }
}

impl<S: PageStore + ?Sized> PageStore for OverlayPageStore<'_, S> {
    fn read_page(&self, digest: PageDigest) -> Result<Option<Vec<u8>>, MapError> {
        match self.pages.read_page(digest)? {
            Some(bytes) => Ok(Some(bytes)),
            None => self.base.read_page(digest),
        }
    }

    fn write_page(&mut self, digest: PageDigest, bytes: &[u8]) -> Result<PageWrite, MapError> {
        if self.pages.read_page(digest)?.is_some() {
            return self.pages.write_page(digest, bytes);
        }
        match self.base.read_page(digest)? {
            Some(existing) if existing == bytes => {
                let _ = self.pages.write_page(digest, bytes)?;
                Ok(PageWrite::Reused)
            }
            Some(_) => Err(map_error(
                MapErrorClass::Corrupt,
                "persistent_map_overlay_collision",
                "base store binds one page digest to different immutable bytes",
            )),
            None => self.pages.write_page(digest, bytes),
        }
    }
}

impl PageStore for MemoryPageStore {
    fn read_page(&self, digest: PageDigest) -> Result<Option<Vec<u8>>, MapError> {
        Ok(self.pages.get(&digest).cloned())
    }

    fn write_page(&mut self, digest: PageDigest, bytes: &[u8]) -> Result<PageWrite, MapError> {
        if bytes.len() > MAXIMUM_PAGE_BYTES {
            return Err(map_error(
                MapErrorClass::Resource,
                "persistent_map_page_limit",
                format!("stored page exceeds {MAXIMUM_PAGE_BYTES} bytes"),
            ));
        }
        if PageDigest::of(bytes) != digest {
            return Err(map_error(
                MapErrorClass::Store,
                "persistent_map_store_digest",
                "page store was asked to bind bytes under a foreign digest",
            ));
        }
        match self.pages.get(&digest) {
            Some(existing) if existing == bytes => Ok(PageWrite::Reused),
            Some(_) => Err(map_error(
                MapErrorClass::Corrupt,
                "persistent_map_store_collision",
                "one page digest is already bound to different immutable bytes",
            )),
            None => {
                self.pages.insert(digest, bytes.to_vec());
                Ok(PageWrite::Inserted)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Entry {
    key: Vec<u8>,
    value: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ChildRef {
    edge: u8,
    digest: PageDigest,
    count: u64,
    logical_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Page {
    Leaf {
        prefix: Vec<u8>,
        count: u64,
        logical_bytes: u64,
        entries: Vec<Entry>,
    },
    Branch {
        prefix: Vec<u8>,
        count: u64,
        logical_bytes: u64,
        terminal: Option<Vec<u8>>,
        children: Vec<ChildRef>,
    },
}

impl Page {
    fn prefix(&self) -> &[u8] {
        match self {
            Self::Leaf { prefix, .. } | Self::Branch { prefix, .. } => prefix,
        }
    }

    const fn count(&self) -> u64 {
        match self {
            Self::Leaf { count, .. } | Self::Branch { count, .. } => *count,
        }
    }

    const fn logical_bytes(&self) -> u64 {
        match self {
            Self::Leaf { logical_bytes, .. } | Self::Branch { logical_bytes, .. } => *logical_bytes,
        }
    }
}

#[derive(Clone, Debug)]
struct NodeRef {
    digest: PageDigest,
    count: u64,
    logical_bytes: u64,
}

impl NodeRef {
    fn from_page(digest: PageDigest, page: &Page) -> Self {
        Self {
            digest,
            count: page.count(),
            logical_bytes: page.logical_bytes(),
        }
    }

    fn child(&self, edge: u8) -> ChildRef {
        ChildRef {
            edge,
            digest: self.digest,
            count: self.count,
            logical_bytes: self.logical_bytes,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InsertOutcome {
    Inserted,
    Replaced { previous: Vec<u8> },
    Unchanged,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RemoveOutcome {
    Removed { previous: Vec<u8> },
    Unchanged,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MapDifference {
    Added {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Removed {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Updated {
        key: Vec<u8>,
        before: Vec<u8>,
        after: Vec<u8>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerificationReport {
    pub pages: u64,
    pub entries: u64,
    pub logical_bytes: u64,
}

fn map_error(class: MapErrorClass, code: &'static str, message: impl Into<String>) -> MapError {
    MapError::new(class, code, message)
}

fn domain_digest(domain: &str, bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

fn add_counter(counter: &mut u64, amount: u64, code: &'static str) -> Result<(), MapError> {
    *counter = counter.checked_add(amount).ok_or_else(|| {
        map_error(
            MapErrorClass::Resource,
            code,
            "persistent-map work accounting overflowed",
        )
    })?;
    Ok(())
}

fn usize_to_u64(value: usize, code: &'static str) -> Result<u64, MapError> {
    u64::try_from(value).map_err(|_| {
        map_error(
            MapErrorClass::Resource,
            code,
            "persistent-map size cannot be represented",
        )
    })
}

fn u64_to_usize(value: u64, code: &'static str) -> Result<usize, MapError> {
    usize::try_from(value).map_err(|_| {
        map_error(
            MapErrorClass::Resource,
            code,
            "persistent-map size cannot be represented on this platform",
        )
    })
}

fn checked_add_usize(left: usize, right: usize, code: &'static str) -> Result<usize, MapError> {
    left.checked_add(right).ok_or_else(|| {
        map_error(
            MapErrorClass::Resource,
            code,
            "persistent-map byte length overflowed",
        )
    })
}

fn checked_mul_usize(left: usize, right: usize, code: &'static str) -> Result<usize, MapError> {
    left.checked_mul(right).ok_or_else(|| {
        map_error(
            MapErrorClass::Resource,
            code,
            "persistent-map byte length overflowed",
        )
    })
}

fn validate_key(key: &[u8]) -> Result<(), MapError> {
    if key.len() > MAXIMUM_KEY_BYTES {
        return Err(map_error(
            MapErrorClass::Input,
            "persistent_map_key_limit",
            format!("map key exceeds {MAXIMUM_KEY_BYTES} bytes"),
        ));
    }
    Ok(())
}

fn validate_value(value: &[u8]) -> Result<(), MapError> {
    if value.len() > MAXIMUM_VALUE_BYTES {
        return Err(map_error(
            MapErrorClass::Input,
            "persistent_map_value_limit",
            format!("map value exceeds {MAXIMUM_VALUE_BYTES} bytes"),
        ));
    }
    Ok(())
}

fn push_u16(bytes: &mut Vec<u8>, value: usize, code: &'static str) -> Result<(), MapError> {
    let value = u16::try_from(value).map_err(|_| {
        map_error(
            MapErrorClass::Resource,
            code,
            "persistent-map length exceeds its canonical encoding",
        )
    })?;
    bytes.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

fn push_u32(bytes: &mut Vec<u8>, value: usize, code: &'static str) -> Result<(), MapError> {
    let value = u32::try_from(value).map_err(|_| {
        map_error(
            MapErrorClass::Resource,
            code,
            "persistent-map length exceeds its canonical encoding",
        )
    })?;
    bytes.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

struct Decoder<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], MapError> {
        let end = self.position.checked_add(length).ok_or_else(|| {
            map_error(
                MapErrorClass::Corrupt,
                "persistent_map_page_length",
                "page field length overflowed",
            )
        })?;
        let value = self.bytes.get(self.position..end).ok_or_else(|| {
            map_error(
                MapErrorClass::Corrupt,
                "persistent_map_page_truncated",
                "page ends inside a declared field",
            )
        })?;
        self.position = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, MapError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, MapError> {
        let mut bytes = [0_u8; 2];
        bytes.copy_from_slice(self.take(2)?);
        Ok(u16::from_le_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, MapError> {
        let mut bytes = [0_u8; 4];
        bytes.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, MapError> {
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(bytes))
    }

    fn finish(self) -> Result<(), MapError> {
        if self.position != self.bytes.len() {
            return Err(map_error(
                MapErrorClass::Corrupt,
                "persistent_map_page_trailing",
                "page payload contains trailing data",
            ));
        }
        Ok(())
    }
}

fn longest_common_prefix(entries: &[Entry]) -> Vec<u8> {
    let Some(first) = entries.first() else {
        return Vec::new();
    };
    let Some(last) = entries.last() else {
        return Vec::new();
    };
    let length = first
        .key
        .iter()
        .zip(&last.key)
        .take_while(|(left, right)| left == right)
        .count();
    first.key[..length].to_vec()
}

fn common_prefix(left: &[u8], right: &[u8]) -> Vec<u8> {
    let length = left
        .iter()
        .zip(right)
        .take_while(|(left, right)| left == right)
        .count();
    left[..length].to_vec()
}

fn entry_logical_bytes(entries: &[Entry]) -> Result<u64, MapError> {
    entries.iter().try_fold(0_u64, |total, entry| {
        let key = usize_to_u64(entry.key.len(), "persistent_map_logical_key")?;
        let value = usize_to_u64(entry.value.len(), "persistent_map_logical_value")?;
        total
            .checked_add(key)
            .and_then(|sum| sum.checked_add(value))
            .ok_or_else(|| {
                map_error(
                    MapErrorClass::Resource,
                    "persistent_map_logical_bytes",
                    "persistent-map logical byte count overflowed",
                )
            })
    })
}

fn leaf_encoded_length(
    prefix_length: usize,
    count: u64,
    logical_bytes: u64,
) -> Result<usize, MapError> {
    let count = u64_to_usize(count, "persistent_map_leaf_count")?;
    let shared = checked_mul_usize(count, prefix_length, "persistent_map_leaf_shared")?;
    let logical = u64_to_usize(logical_bytes, "persistent_map_leaf_logical")?;
    let suffixes_and_values = logical.checked_sub(shared).ok_or_else(|| {
        map_error(
            MapErrorClass::Corrupt,
            "persistent_map_leaf_logical",
            "leaf logical bytes are smaller than its repeated prefix",
        )
    })?;
    let record_overhead = checked_mul_usize(
        count,
        LEAF_ENTRY_OVERHEAD_BYTES,
        "persistent_map_leaf_overhead",
    )?;
    let mut length = PAGE_HEADER_BYTES;
    length = checked_add_usize(
        length,
        PAGE_COMMON_PAYLOAD_BYTES,
        "persistent_map_leaf_size",
    )?;
    length = checked_add_usize(length, prefix_length, "persistent_map_leaf_size")?;
    length = checked_add_usize(length, LEAF_COUNT_BYTES, "persistent_map_leaf_size")?;
    length = checked_add_usize(length, record_overhead, "persistent_map_leaf_size")?;
    length = checked_add_usize(length, suffixes_and_values, "persistent_map_leaf_size")?;
    checked_add_usize(length, PAGE_CHECKSUM_BYTES, "persistent_map_leaf_size")
}

fn validate_page_local(page: &Page) -> Result<(), MapError> {
    validate_key(page.prefix())?;
    match page {
        Page::Leaf {
            prefix,
            count,
            logical_bytes,
            entries,
        } => {
            let encoded_count = usize_to_u64(entries.len(), "persistent_map_leaf_count")?;
            if *count != encoded_count || entries.len() > MAXIMUM_LEAF_RECORDS {
                return Err(map_error(
                    MapErrorClass::Corrupt,
                    "persistent_map_leaf_count",
                    "leaf count does not match its bounded entry vector",
                ));
            }
            if entries.is_empty() {
                if !prefix.is_empty() || *logical_bytes != 0 {
                    return Err(map_error(
                        MapErrorClass::Corrupt,
                        "persistent_map_empty_leaf",
                        "the canonical empty leaf has an empty prefix and zero logical bytes",
                    ));
                }
                return Ok(());
            }
            if longest_common_prefix(entries) != *prefix {
                return Err(map_error(
                    MapErrorClass::Corrupt,
                    "persistent_map_leaf_prefix",
                    "leaf prefix is not the longest common prefix of its keys",
                ));
            }
            for (index, entry) in entries.iter().enumerate() {
                validate_key(&entry.key)?;
                validate_value(&entry.value)?;
                if !entry.key.starts_with(prefix) {
                    return Err(map_error(
                        MapErrorClass::Corrupt,
                        "persistent_map_leaf_key_prefix",
                        "leaf key does not start with the page prefix",
                    ));
                }
                if index > 0 && entries[index - 1].key >= entry.key {
                    return Err(map_error(
                        MapErrorClass::Corrupt,
                        "persistent_map_leaf_order",
                        "leaf keys are not strictly ordered and unique",
                    ));
                }
            }
            if entry_logical_bytes(entries)? != *logical_bytes {
                return Err(map_error(
                    MapErrorClass::Corrupt,
                    "persistent_map_leaf_logical",
                    "leaf logical byte summary does not match its entries",
                ));
            }
            let encoded_length = leaf_encoded_length(prefix.len(), *count, *logical_bytes)?;
            if encoded_length > MAXIMUM_PAGE_BYTES {
                return Err(map_error(
                    MapErrorClass::Resource,
                    "persistent_map_leaf_page_limit",
                    "leaf exceeds the hostile page-size bound",
                ));
            }
            if entries.len() > 1 && encoded_length > TARGET_LEAF_PAGE_BYTES {
                return Err(map_error(
                    MapErrorClass::Corrupt,
                    "persistent_map_leaf_noncanonical",
                    "multi-entry leaf exceeds the canonical split threshold",
                ));
            }
        }
        Page::Branch {
            prefix,
            count,
            logical_bytes,
            terminal,
            children,
        } => {
            if *count < 2 || children.is_empty() || children.len() > 256 {
                return Err(map_error(
                    MapErrorClass::Corrupt,
                    "persistent_map_branch_count",
                    "canonical branch must contain at least two entries and one through 256 children",
                ));
            }
            if terminal.is_none() && children.len() < 2 {
                return Err(map_error(
                    MapErrorClass::Corrupt,
                    "persistent_map_branch_compression",
                    "branch without a terminal must have at least two child edges",
                ));
            }
            if let Some(value) = terminal {
                validate_value(value)?;
            }
            let mut computed_count = u64::from(terminal.is_some());
            let mut computed_logical = if let Some(value) = terminal {
                usize_to_u64(prefix.len(), "persistent_map_branch_prefix")?
                    .checked_add(usize_to_u64(value.len(), "persistent_map_branch_terminal")?)
                    .ok_or_else(|| {
                        map_error(
                            MapErrorClass::Resource,
                            "persistent_map_branch_logical",
                            "branch terminal byte count overflowed",
                        )
                    })?
            } else {
                0
            };
            for (index, child) in children.iter().enumerate() {
                if child.count == 0 {
                    return Err(map_error(
                        MapErrorClass::Corrupt,
                        "persistent_map_branch_child_count",
                        "branch child has an empty subtree",
                    ));
                }
                if index > 0 && children[index - 1].edge >= child.edge {
                    return Err(map_error(
                        MapErrorClass::Corrupt,
                        "persistent_map_branch_order",
                        "branch child edges are not strictly ordered and unique",
                    ));
                }
                let minimum_key_bytes = usize_to_u64(
                    prefix.len().saturating_add(1),
                    "persistent_map_branch_child_prefix",
                )?
                .checked_mul(child.count)
                .ok_or_else(|| {
                    map_error(
                        MapErrorClass::Resource,
                        "persistent_map_branch_child_logical",
                        "branch child minimum byte count overflowed",
                    )
                })?;
                if child.logical_bytes < minimum_key_bytes {
                    return Err(map_error(
                        MapErrorClass::Corrupt,
                        "persistent_map_branch_child_logical",
                        "branch child logical bytes cannot contain its required key prefixes",
                    ));
                }
                computed_count = computed_count.checked_add(child.count).ok_or_else(|| {
                    map_error(
                        MapErrorClass::Resource,
                        "persistent_map_branch_count",
                        "branch entry count overflowed",
                    )
                })?;
                computed_logical = computed_logical
                    .checked_add(child.logical_bytes)
                    .ok_or_else(|| {
                        map_error(
                            MapErrorClass::Resource,
                            "persistent_map_branch_logical",
                            "branch logical byte count overflowed",
                        )
                    })?;
            }
            if computed_count != *count || computed_logical != *logical_bytes {
                return Err(map_error(
                    MapErrorClass::Corrupt,
                    "persistent_map_branch_summary",
                    "branch summaries do not match its terminal and child references",
                ));
            }
            if leaf_encoded_length(prefix.len(), *count, *logical_bytes)? <= TARGET_LEAF_PAGE_BYTES
            {
                return Err(map_error(
                    MapErrorClass::Corrupt,
                    "persistent_map_branch_noncanonical",
                    "branch contents fit in one canonical leaf",
                ));
            }
        }
    }
    Ok(())
}

fn encode_page(page: &Page) -> Result<Vec<u8>, MapError> {
    validate_page_local(page)?;
    let mut payload = Vec::new();
    push_u16(
        &mut payload,
        page.prefix().len(),
        "persistent_map_prefix_length",
    )?;
    payload.extend_from_slice(page.prefix());
    push_u64(&mut payload, page.count());
    push_u64(&mut payload, page.logical_bytes());
    let kind = match page {
        Page::Leaf {
            prefix, entries, ..
        } => {
            push_u32(&mut payload, entries.len(), "persistent_map_leaf_count")?;
            for entry in entries {
                let suffix = &entry.key[prefix.len()..];
                push_u16(&mut payload, suffix.len(), "persistent_map_leaf_suffix")?;
                payload.extend_from_slice(suffix);
                push_u32(&mut payload, entry.value.len(), "persistent_map_leaf_value")?;
                payload.extend_from_slice(&entry.value);
            }
            0_u8
        }
        Page::Branch {
            terminal, children, ..
        } => {
            match terminal {
                Some(value) => {
                    payload.push(1);
                    push_u32(&mut payload, value.len(), "persistent_map_branch_terminal")?;
                    payload.extend_from_slice(value);
                }
                None => payload.push(0),
            }
            push_u16(
                &mut payload,
                children.len(),
                "persistent_map_branch_children",
            )?;
            for child in children {
                payload.push(child.edge);
                payload.extend_from_slice(&child.digest.bytes());
                push_u64(&mut payload, child.count);
                push_u64(&mut payload, child.logical_bytes);
            }
            1_u8
        }
    };
    let payload_length = usize_to_u64(payload.len(), "persistent_map_payload_length")?;
    let capacity = PAGE_HEADER_BYTES
        .checked_add(payload.len())
        .and_then(|length| length.checked_add(PAGE_CHECKSUM_BYTES))
        .ok_or_else(|| {
            map_error(
                MapErrorClass::Resource,
                "persistent_map_page_length",
                "encoded page length overflowed",
            )
        })?;
    if capacity > MAXIMUM_PAGE_BYTES {
        return Err(map_error(
            MapErrorClass::Resource,
            "persistent_map_page_limit",
            format!("encoded page exceeds {MAXIMUM_PAGE_BYTES} bytes"),
        ));
    }
    let mut bytes = Vec::with_capacity(capacity);
    bytes.extend_from_slice(&PAGE_MAGIC);
    bytes.extend_from_slice(&PAGE_CONTRACT_VERSION.to_le_bytes());
    bytes.push(kind);
    bytes.push(0);
    bytes.extend_from_slice(&payload_length.to_le_bytes());
    bytes.extend_from_slice(&payload);
    let checksum = domain_digest(PAGE_CHECKSUM_DOMAIN, &bytes);
    bytes.extend_from_slice(&checksum);
    Ok(bytes)
}

fn decode_page(bytes: &[u8]) -> Result<Page, MapError> {
    if bytes.len() > MAXIMUM_PAGE_BYTES {
        return Err(map_error(
            MapErrorClass::Resource,
            "persistent_map_page_limit",
            format!("page exceeds {MAXIMUM_PAGE_BYTES} bytes"),
        ));
    }
    let minimum = PAGE_HEADER_BYTES + PAGE_CHECKSUM_BYTES + PAGE_COMMON_PAYLOAD_BYTES;
    if bytes.len() < minimum {
        return Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_page_truncated",
            "page is shorter than its canonical envelope",
        ));
    }
    if bytes[..8] != PAGE_MAGIC {
        return Err(map_error(
            MapErrorClass::Input,
            "persistent_map_page_contract",
            "page uses an unknown persistent-map contract",
        ));
    }
    let mut version = [0_u8; 2];
    version.copy_from_slice(&bytes[8..10]);
    if u16::from_le_bytes(version) != PAGE_CONTRACT_VERSION {
        return Err(map_error(
            MapErrorClass::Input,
            "persistent_map_page_version",
            "page uses an unsupported persistent-map version",
        ));
    }
    let kind = bytes[10];
    if kind > 1 || bytes[11] != 0 {
        return Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_page_header",
            "page kind or reserved header flags are noncanonical",
        ));
    }
    let mut payload_length = [0_u8; 8];
    payload_length.copy_from_slice(&bytes[12..20]);
    let payload_length = u64_to_usize(
        u64::from_le_bytes(payload_length),
        "persistent_map_payload_length",
    )?;
    let expected_length = PAGE_HEADER_BYTES
        .checked_add(payload_length)
        .and_then(|length| length.checked_add(PAGE_CHECKSUM_BYTES))
        .ok_or_else(|| {
            map_error(
                MapErrorClass::Resource,
                "persistent_map_page_length",
                "declared page length overflowed",
            )
        })?;
    if expected_length != bytes.len() {
        return Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_page_length",
            "page length does not match its exact canonical envelope",
        ));
    }
    let checksum_start = PAGE_HEADER_BYTES + payload_length;
    let checksum = domain_digest(PAGE_CHECKSUM_DOMAIN, &bytes[..checksum_start]);
    if bytes[checksum_start..] != checksum {
        return Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_page_checksum",
            "page checksum does not match its domain-separated bytes",
        ));
    }
    let mut decoder = Decoder::new(&bytes[PAGE_HEADER_BYTES..checksum_start]);
    let prefix_length = usize::from(decoder.u16()?);
    if prefix_length > MAXIMUM_KEY_BYTES {
        return Err(map_error(
            MapErrorClass::Resource,
            "persistent_map_prefix_limit",
            "page prefix exceeds the hostile key bound",
        ));
    }
    let prefix = decoder.take(prefix_length)?.to_vec();
    let count = decoder.u64()?;
    let logical_bytes = decoder.u64()?;
    let page = if kind == 0 {
        let entry_count = usize::try_from(decoder.u32()?).map_err(|_| {
            map_error(
                MapErrorClass::Resource,
                "persistent_map_leaf_count",
                "leaf entry count cannot be represented",
            )
        })?;
        if entry_count > MAXIMUM_LEAF_RECORDS
            || usize_to_u64(entry_count, "persistent_map_leaf_count")? != count
        {
            return Err(map_error(
                MapErrorClass::Resource,
                "persistent_map_leaf_count",
                "leaf entry count exceeds its strict page-derived bound",
            ));
        }
        let mut entries = Vec::with_capacity(entry_count);
        for _ in 0..entry_count {
            let suffix_length = usize::from(decoder.u16()?);
            let key_length = prefix_length.checked_add(suffix_length).ok_or_else(|| {
                map_error(
                    MapErrorClass::Resource,
                    "persistent_map_key_length",
                    "decoded key length overflowed",
                )
            })?;
            if key_length > MAXIMUM_KEY_BYTES {
                return Err(map_error(
                    MapErrorClass::Resource,
                    "persistent_map_key_limit",
                    "decoded key exceeds the hostile key bound",
                ));
            }
            let suffix = decoder.take(suffix_length)?;
            let value_length = usize::try_from(decoder.u32()?).map_err(|_| {
                map_error(
                    MapErrorClass::Resource,
                    "persistent_map_value_length",
                    "decoded value length cannot be represented",
                )
            })?;
            if value_length > MAXIMUM_VALUE_BYTES {
                return Err(map_error(
                    MapErrorClass::Resource,
                    "persistent_map_value_limit",
                    "decoded value exceeds the hostile value bound",
                ));
            }
            let value = decoder.take(value_length)?.to_vec();
            let mut key = Vec::with_capacity(key_length);
            key.extend_from_slice(&prefix);
            key.extend_from_slice(suffix);
            entries.push(Entry { key, value });
        }
        Page::Leaf {
            prefix,
            count,
            logical_bytes,
            entries,
        }
    } else {
        let terminal = match decoder.u8()? {
            0 => None,
            1 => {
                let value_length = usize::try_from(decoder.u32()?).map_err(|_| {
                    map_error(
                        MapErrorClass::Resource,
                        "persistent_map_value_length",
                        "decoded terminal length cannot be represented",
                    )
                })?;
                if value_length > MAXIMUM_VALUE_BYTES {
                    return Err(map_error(
                        MapErrorClass::Resource,
                        "persistent_map_value_limit",
                        "decoded terminal exceeds the hostile value bound",
                    ));
                }
                Some(decoder.take(value_length)?.to_vec())
            }
            _ => {
                return Err(map_error(
                    MapErrorClass::Corrupt,
                    "persistent_map_branch_terminal",
                    "branch terminal discriminator is noncanonical",
                ));
            }
        };
        let child_count = usize::from(decoder.u16()?);
        if child_count == 0 || child_count > 256 {
            return Err(map_error(
                MapErrorClass::Resource,
                "persistent_map_branch_children",
                "branch child count exceeds its radix bound",
            ));
        }
        let child_bytes = checked_mul_usize(
            child_count,
            BRANCH_CHILD_BYTES,
            "persistent_map_branch_children",
        )?;
        if child_bytes > decoder.bytes.len().saturating_sub(decoder.position) {
            return Err(map_error(
                MapErrorClass::Corrupt,
                "persistent_map_page_truncated",
                "branch child vector exceeds the remaining payload",
            ));
        }
        let mut children = Vec::with_capacity(child_count);
        for _ in 0..child_count {
            let edge = decoder.u8()?;
            let mut digest = [0_u8; 32];
            digest.copy_from_slice(decoder.take(32)?);
            children.push(ChildRef {
                edge,
                digest: PageDigest::from_bytes(digest),
                count: decoder.u64()?,
                logical_bytes: decoder.u64()?,
            });
        }
        Page::Branch {
            prefix,
            count,
            logical_bytes,
            terminal,
            children,
        }
    };
    decoder.finish()?;
    validate_page_local(&page)?;
    if encode_page(&page)? != bytes {
        return Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_page_noncanonical",
            "decoded page does not reproduce its exact canonical bytes",
        ));
    }
    Ok(page)
}

fn load_page<S: PageStore + ?Sized>(
    store: &S,
    digest: PageDigest,
    work: &mut MapWork,
) -> Result<Page, MapError> {
    let bytes = store.read_page(digest)?.ok_or_else(|| {
        map_error(
            MapErrorClass::Corrupt,
            "persistent_map_page_missing",
            format!("referenced page {digest} is absent"),
        )
    })?;
    if bytes.len() > MAXIMUM_PAGE_BYTES {
        return Err(map_error(
            MapErrorClass::Resource,
            "persistent_map_page_limit",
            format!("stored page exceeds {MAXIMUM_PAGE_BYTES} bytes"),
        ));
    }
    add_counter(&mut work.pages_read, 1, "persistent_map_work_pages_read")?;
    add_counter(
        &mut work.bytes_read,
        usize_to_u64(bytes.len(), "persistent_map_work_bytes_read")?,
        "persistent_map_work_bytes_read",
    )?;
    if PageDigest::of(&bytes) != digest {
        return Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_page_digest",
            format!("referenced page {digest} has foreign bytes"),
        ));
    }
    let page = decode_page(&bytes)?;
    add_counter(
        &mut work.pages_decoded,
        1,
        "persistent_map_work_pages_decoded",
    )?;
    Ok(page)
}

fn write_page<S: PageStore + ?Sized>(
    store: &mut S,
    page: &Page,
    work: &mut MapWork,
) -> Result<NodeRef, MapError> {
    let bytes = encode_page(page)?;
    let digest = PageDigest::of(&bytes);
    add_counter(
        &mut work.pages_encoded,
        1,
        "persistent_map_work_pages_encoded",
    )?;
    add_counter(
        &mut work.bytes_encoded,
        usize_to_u64(bytes.len(), "persistent_map_work_bytes_encoded")?,
        "persistent_map_work_bytes_encoded",
    )?;
    match store.write_page(digest, &bytes)? {
        PageWrite::Inserted => {
            add_counter(
                &mut work.pages_written,
                1,
                "persistent_map_work_pages_written",
            )?;
            add_counter(
                &mut work.bytes_written,
                usize_to_u64(bytes.len(), "persistent_map_work_bytes_written")?,
                "persistent_map_work_bytes_written",
            )?;
        }
        PageWrite::Reused => add_counter(
            &mut work.pages_reused,
            1,
            "persistent_map_work_pages_reused",
        )?,
    }
    Ok(NodeRef::from_page(digest, page))
}

fn verify_root(root: MapRoot, page: &Page) -> Result<(), MapError> {
    if root.entries != page.count() {
        return Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_root_count",
            "map root entry count disagrees with its root page",
        ));
    }
    Ok(())
}

fn verify_child_link(
    parent_prefix: &[u8],
    reference: &ChildRef,
    child: &Page,
) -> Result<(), MapError> {
    verify_child_summary(parent_prefix, reference, &PageLinkSummary::from_page(child))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PageLinkSummary {
    prefix: Vec<u8>,
    count: u64,
    logical_bytes: u64,
}

impl PageLinkSummary {
    fn from_page(page: &Page) -> Self {
        Self {
            prefix: page.prefix().to_vec(),
            count: page.count(),
            logical_bytes: page.logical_bytes(),
        }
    }
}

fn verify_child_summary(
    parent_prefix: &[u8],
    reference: &ChildRef,
    child: &PageLinkSummary,
) -> Result<(), MapError> {
    if child.count != reference.count || child.logical_bytes != reference.logical_bytes {
        return Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_child_summary",
            "branch child reference disagrees with the referenced page summary",
        ));
    }
    if child.prefix.len() <= parent_prefix.len()
        || !child.prefix.starts_with(parent_prefix)
        || child.prefix[parent_prefix.len()] != reference.edge
    {
        return Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_child_prefix",
            "branch edge does not match the referenced child's canonical prefix",
        ));
    }
    Ok(())
}

fn search_entries(
    entries: &[Entry],
    key: &[u8],
    work: &mut MapWork,
) -> Result<Result<usize, usize>, MapError> {
    let mut low = 0;
    let mut high = entries.len();
    while low < high {
        let middle = low + (high - low) / 2;
        add_counter(
            &mut work.key_comparisons,
            1,
            "persistent_map_work_comparisons",
        )?;
        match entries[middle].key.as_slice().cmp(key) {
            std::cmp::Ordering::Less => low = middle + 1,
            std::cmp::Ordering::Greater => high = middle,
            std::cmp::Ordering::Equal => return Ok(Ok(middle)),
        }
    }
    Ok(Err(low))
}

fn search_children(
    children: &[ChildRef],
    edge: u8,
    work: &mut MapWork,
) -> Result<Result<usize, usize>, MapError> {
    let mut low = 0;
    let mut high = children.len();
    while low < high {
        let middle = low + (high - low) / 2;
        add_counter(
            &mut work.key_comparisons,
            1,
            "persistent_map_work_comparisons",
        )?;
        match children[middle].edge.cmp(&edge) {
            std::cmp::Ordering::Less => low = middle + 1,
            std::cmp::Ordering::Greater => high = middle,
            std::cmp::Ordering::Equal => return Ok(Ok(middle)),
        }
    }
    Ok(Err(low))
}

fn ensure_depth(depth: usize) -> Result<(), MapError> {
    if depth > MAXIMUM_TREE_DEPTH {
        return Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_tree_depth",
            "persistent-map tree exceeds the key-derived depth bound",
        ));
    }
    Ok(())
}

fn build_entries<S: PageStore + ?Sized>(
    store: &mut S,
    entries: &[Entry],
    depth: usize,
    work: &mut MapWork,
) -> Result<NodeRef, MapError> {
    ensure_depth(depth)?;
    if entries.is_empty() {
        return write_page(
            store,
            &Page::Leaf {
                prefix: Vec::new(),
                count: 0,
                logical_bytes: 0,
                entries: Vec::new(),
            },
            work,
        );
    }
    for (index, entry) in entries.iter().enumerate() {
        validate_key(&entry.key)?;
        validate_value(&entry.value)?;
        if index > 0 && entries[index - 1].key >= entry.key {
            return Err(map_error(
                MapErrorClass::Input,
                "persistent_map_build_order",
                "map builder input is not strictly ordered and unique",
            ));
        }
    }
    let prefix = longest_common_prefix(entries);
    let count = usize_to_u64(entries.len(), "persistent_map_build_count")?;
    let logical_bytes = entry_logical_bytes(entries)?;
    let leaf_length = leaf_encoded_length(prefix.len(), count, logical_bytes)?;
    if entries.len() == 1 || leaf_length <= TARGET_LEAF_PAGE_BYTES {
        return write_page(
            store,
            &Page::Leaf {
                prefix,
                count,
                logical_bytes,
                entries: entries.to_vec(),
            },
            work,
        );
    }

    let mut position = 0;
    let terminal = if entries[0].key.len() == prefix.len() {
        position = 1;
        Some(entries[0].value.clone())
    } else {
        None
    };
    let mut children = Vec::new();
    while position < entries.len() {
        let edge = *entries[position].key.get(prefix.len()).ok_or_else(|| {
            map_error(
                MapErrorClass::Corrupt,
                "persistent_map_build_prefix",
                "nonterminal branch entry ends at the branch prefix",
            )
        })?;
        let mut end = position + 1;
        while end < entries.len() && entries[end].key.get(prefix.len()) == Some(&edge) {
            end += 1;
        }
        let child = build_entries(store, &entries[position..end], depth + 1, work)?;
        children.push(child.child(edge));
        position = end;
    }
    write_page(
        store,
        &Page::Branch {
            prefix,
            count,
            logical_bytes,
            terminal,
            children,
        },
        work,
    )
}

fn visit_loaded<S, F>(
    store: &S,
    digest: PageDigest,
    page: &Page,
    depth: usize,
    work: &mut MapWork,
    visitor: &mut F,
) -> Result<u64, MapError>
where
    S: PageStore + ?Sized,
    F: FnMut(&[u8], &[u8], &mut MapWork) -> Result<(), MapError>,
{
    ensure_depth(depth)?;
    let mut visited = 0_u64;
    match page {
        Page::Leaf { entries, .. } => {
            for entry in entries {
                visitor(&entry.key, &entry.value, work)?;
                add_counter(
                    &mut work.entries_visited,
                    1,
                    "persistent_map_work_entries_visited",
                )?;
                visited = visited.checked_add(1).ok_or_else(|| {
                    map_error(
                        MapErrorClass::Resource,
                        "persistent_map_visit_count",
                        "visited entry count overflowed",
                    )
                })?;
            }
        }
        Page::Branch {
            prefix,
            terminal,
            children,
            ..
        } => {
            if let Some(value) = terminal {
                visitor(prefix, value, work)?;
                add_counter(
                    &mut work.entries_visited,
                    1,
                    "persistent_map_work_entries_visited",
                )?;
                visited = 1;
            }
            for child in children {
                let child_page = load_page(store, child.digest, work)?;
                verify_child_link(prefix, child, &child_page)?;
                visited = visited
                    .checked_add(visit_loaded(
                        store,
                        child.digest,
                        &child_page,
                        depth + 1,
                        work,
                        visitor,
                    )?)
                    .ok_or_else(|| {
                        map_error(
                            MapErrorClass::Resource,
                            "persistent_map_visit_count",
                            "visited entry count overflowed",
                        )
                    })?;
            }
        }
    }
    if visited != page.count() {
        return Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_visit_summary",
            format!("page {digest} traversal disagrees with its entry count"),
        ));
    }
    Ok(visited)
}

fn collect_loaded<S: PageStore + ?Sized>(
    store: &S,
    digest: PageDigest,
    page: &Page,
    depth: usize,
    work: &mut MapWork,
) -> Result<Vec<Entry>, MapError> {
    let capacity = u64_to_usize(page.count(), "persistent_map_collect_count")?;
    let mut entries = Vec::with_capacity(capacity);
    visit_loaded(
        store,
        digest,
        page,
        depth,
        work,
        &mut |key, value, _work| {
            entries.push(Entry {
                key: key.to_vec(),
                value: value.to_vec(),
            });
            Ok(())
        },
    )?;
    Ok(entries)
}

fn collect_child<S: PageStore + ?Sized>(
    store: &S,
    parent_prefix: &[u8],
    child: &ChildRef,
    depth: usize,
    work: &mut MapWork,
) -> Result<Vec<Entry>, MapError> {
    let page = load_page(store, child.digest, work)?;
    verify_child_link(parent_prefix, child, &page)?;
    collect_loaded(store, child.digest, &page, depth, work)
}

fn branch_summary(
    prefix: &[u8],
    terminal: Option<&[u8]>,
    children: &[ChildRef],
) -> Result<(u64, u64), MapError> {
    let mut count = u64::from(terminal.is_some());
    let mut logical_bytes = if let Some(value) = terminal {
        usize_to_u64(prefix.len(), "persistent_map_branch_prefix")?
            .checked_add(usize_to_u64(value.len(), "persistent_map_branch_terminal")?)
            .ok_or_else(|| {
                map_error(
                    MapErrorClass::Resource,
                    "persistent_map_branch_logical",
                    "branch terminal byte count overflowed",
                )
            })?
    } else {
        0
    };
    for child in children {
        count = count.checked_add(child.count).ok_or_else(|| {
            map_error(
                MapErrorClass::Resource,
                "persistent_map_branch_count",
                "branch entry count overflowed",
            )
        })?;
        logical_bytes = logical_bytes
            .checked_add(child.logical_bytes)
            .ok_or_else(|| {
                map_error(
                    MapErrorClass::Resource,
                    "persistent_map_branch_logical",
                    "branch logical byte count overflowed",
                )
            })?;
    }
    Ok((count, logical_bytes))
}

fn normalize_branch<S: PageStore + ?Sized>(
    store: &mut S,
    prefix: Vec<u8>,
    terminal: Option<Vec<u8>>,
    children: Vec<ChildRef>,
    depth: usize,
    work: &mut MapWork,
) -> Result<Option<NodeRef>, MapError> {
    ensure_depth(depth)?;
    let (count, logical_bytes) = branch_summary(&prefix, terminal.as_deref(), &children)?;
    if count == 0 {
        return Ok(None);
    }
    if terminal.is_none() && children.len() == 1 {
        let child = &children[0];
        let page = load_page(store, child.digest, work)?;
        verify_child_link(&prefix, child, &page)?;
        return Ok(Some(NodeRef::from_page(child.digest, &page)));
    }
    if count == 1 {
        let value = terminal.ok_or_else(|| {
            map_error(
                MapErrorClass::Corrupt,
                "persistent_map_branch_singleton",
                "singleton branch has no terminal entry",
            )
        })?;
        let entry = Entry { key: prefix, value };
        return build_entries(store, &[entry], depth, work).map(Some);
    }
    if leaf_encoded_length(prefix.len(), count, logical_bytes)? <= TARGET_LEAF_PAGE_BYTES {
        let capacity = u64_to_usize(count, "persistent_map_collapse_count")?;
        let mut entries = Vec::with_capacity(capacity);
        if let Some(value) = terminal {
            entries.push(Entry {
                key: prefix.clone(),
                value,
            });
        }
        for child in &children {
            entries.extend(collect_child(store, &prefix, child, depth + 1, work)?);
        }
        return build_entries(store, &entries, depth, work).map(Some);
    }
    write_page(
        store,
        &Page::Branch {
            prefix,
            count,
            logical_bytes,
            terminal,
            children,
        },
        work,
    )
    .map(Some)
}

fn insert_loaded<S: PageStore + ?Sized>(
    store: &mut S,
    digest: PageDigest,
    page: Page,
    key: &[u8],
    value: &[u8],
    depth: usize,
    work: &mut MapWork,
) -> Result<(NodeRef, Option<Vec<u8>>, bool), MapError> {
    ensure_depth(depth)?;
    let original = NodeRef::from_page(digest, &page);
    match page {
        Page::Leaf { mut entries, .. } => {
            let (previous, changed) = match search_entries(&entries, key, work)? {
                Ok(index) => {
                    let previous = entries[index].value.clone();
                    if previous == value {
                        return Ok((original, Some(previous), false));
                    }
                    entries[index].value = value.to_vec();
                    (Some(previous), true)
                }
                Err(index) => {
                    entries.insert(
                        index,
                        Entry {
                            key: key.to_vec(),
                            value: value.to_vec(),
                        },
                    );
                    (None, true)
                }
            };
            let replacement = build_entries(store, &entries, depth, work)?;
            Ok((replacement, previous, changed))
        }
        Page::Branch {
            prefix,
            mut terminal,
            mut children,
            ..
        } => {
            if key == prefix {
                let previous = terminal.replace(value.to_vec());
                if previous.as_deref() == Some(value) {
                    return Ok((original, previous, false));
                }
                let replacement = normalize_branch(store, prefix, terminal, children, depth, work)?
                    .ok_or_else(|| {
                        map_error(
                            MapErrorClass::Corrupt,
                            "persistent_map_insert_empty",
                            "insertion unexpectedly produced an empty subtree",
                        )
                    })?;
                return Ok((replacement, previous, true));
            }

            if key.starts_with(&prefix) && key.len() > prefix.len() {
                let edge = key[prefix.len()];
                match search_children(&children, edge, work)? {
                    Ok(index) => {
                        let child = children[index].clone();
                        let child_page = load_page(store, child.digest, work)?;
                        verify_child_link(&prefix, &child, &child_page)?;
                        let (replacement, previous, changed) = insert_loaded(
                            store,
                            child.digest,
                            child_page,
                            key,
                            value,
                            depth + 1,
                            work,
                        )?;
                        if !changed {
                            return Ok((original, previous, false));
                        }
                        children[index] = replacement.child(edge);
                        let replacement =
                            normalize_branch(store, prefix, terminal, children, depth, work)?
                                .ok_or_else(|| {
                                    map_error(
                                        MapErrorClass::Corrupt,
                                        "persistent_map_insert_empty",
                                        "insertion unexpectedly produced an empty subtree",
                                    )
                                })?;
                        return Ok((replacement, previous, true));
                    }
                    Err(index) => {
                        let entry = Entry {
                            key: key.to_vec(),
                            value: value.to_vec(),
                        };
                        let child = build_entries(store, &[entry], depth + 1, work)?;
                        children.insert(index, child.child(edge));
                        let replacement =
                            normalize_branch(store, prefix, terminal, children, depth, work)?
                                .ok_or_else(|| {
                                    map_error(
                                        MapErrorClass::Corrupt,
                                        "persistent_map_insert_empty",
                                        "insertion unexpectedly produced an empty subtree",
                                    )
                                })?;
                        return Ok((replacement, None, true));
                    }
                }
            }

            let parent_prefix = common_prefix(&prefix, key);
            let old_edge = *prefix.get(parent_prefix.len()).ok_or_else(|| {
                map_error(
                    MapErrorClass::Corrupt,
                    "persistent_map_insert_prefix",
                    "divergent existing subtree has no branch edge",
                )
            })?;
            let mut parent_terminal = None;
            let mut parent_children = vec![original.child(old_edge)];
            if key.len() == parent_prefix.len() {
                parent_terminal = Some(value.to_vec());
            } else {
                let new_edge = key[parent_prefix.len()];
                let entry = Entry {
                    key: key.to_vec(),
                    value: value.to_vec(),
                };
                let child = build_entries(store, &[entry], depth + 1, work)?;
                parent_children.push(child.child(new_edge));
                parent_children.sort_by_key(|child| child.edge);
            }
            let replacement = normalize_branch(
                store,
                parent_prefix,
                parent_terminal,
                parent_children,
                depth,
                work,
            )?
            .ok_or_else(|| {
                map_error(
                    MapErrorClass::Corrupt,
                    "persistent_map_insert_empty",
                    "insertion unexpectedly produced an empty subtree",
                )
            })?;
            Ok((replacement, None, true))
        }
    }
}

struct RemoveResult {
    replacement: Option<NodeRef>,
    previous: Option<Vec<u8>>,
    changed: bool,
}

fn remove_loaded<S: PageStore + ?Sized>(
    store: &mut S,
    digest: PageDigest,
    page: Page,
    key: &[u8],
    depth: usize,
    work: &mut MapWork,
) -> Result<RemoveResult, MapError> {
    ensure_depth(depth)?;
    let original = NodeRef::from_page(digest, &page);
    match page {
        Page::Leaf { mut entries, .. } => {
            let Ok(index) = search_entries(&entries, key, work)? else {
                return Ok(RemoveResult {
                    replacement: Some(original),
                    previous: None,
                    changed: false,
                });
            };
            let previous = entries.remove(index).value;
            if entries.is_empty() {
                return Ok(RemoveResult {
                    replacement: None,
                    previous: Some(previous),
                    changed: true,
                });
            }
            let replacement = build_entries(store, &entries, depth, work)?;
            Ok(RemoveResult {
                replacement: Some(replacement),
                previous: Some(previous),
                changed: true,
            })
        }
        Page::Branch {
            prefix,
            mut terminal,
            mut children,
            ..
        } => {
            let previous = if key == prefix {
                let Some(previous) = terminal.take() else {
                    return Ok(RemoveResult {
                        replacement: Some(original),
                        previous: None,
                        changed: false,
                    });
                };
                previous
            } else {
                if !key.starts_with(&prefix) || key.len() <= prefix.len() {
                    return Ok(RemoveResult {
                        replacement: Some(original),
                        previous: None,
                        changed: false,
                    });
                }
                let edge = key[prefix.len()];
                let Ok(index) = search_children(&children, edge, work)? else {
                    return Ok(RemoveResult {
                        replacement: Some(original),
                        previous: None,
                        changed: false,
                    });
                };
                let child = children[index].clone();
                let child_page = load_page(store, child.digest, work)?;
                verify_child_link(&prefix, &child, &child_page)?;
                let result = remove_loaded(store, child.digest, child_page, key, depth + 1, work)?;
                if !result.changed {
                    return Ok(RemoveResult {
                        replacement: Some(original),
                        previous: result.previous,
                        changed: false,
                    });
                }
                match result.replacement {
                    Some(replacement) => children[index] = replacement.child(edge),
                    None => {
                        children.remove(index);
                    }
                }
                result.previous.ok_or_else(|| {
                    map_error(
                        MapErrorClass::Corrupt,
                        "persistent_map_remove_previous",
                        "changed child removal omitted its previous value",
                    )
                })?
            };
            let replacement = normalize_branch(store, prefix, terminal, children, depth, work)?;
            Ok(RemoveResult {
                replacement,
                previous: Some(previous),
                changed: true,
            })
        }
    }
}

impl PersistentMap {
    /// Builds one canonical map directly from strictly ordered entries. This is the full-oracle
    /// and bootstrap path; ordinary mutation uses path-copy `insert` and `remove` below.
    pub fn from_sorted<S, I>(
        store: &mut S,
        entries: I,
        work: &mut MapWork,
    ) -> Result<Self, MapError>
    where
        S: PageStore + ?Sized,
        I: IntoIterator<Item = (Vec<u8>, Vec<u8>)>,
    {
        let mut canonical = Vec::new();
        for (key, value) in entries {
            validate_key(&key)?;
            validate_value(&value)?;
            if canonical
                .last()
                .is_some_and(|previous: &Entry| previous.key >= key)
            {
                return Err(map_error(
                    MapErrorClass::Input,
                    "persistent_map_input_order",
                    "bulk map entries must be strictly ordered by key",
                ));
            }
            canonical.push(Entry { key, value });
        }
        let root = build_entries(store, &canonical, 0, work)?;
        Ok(Self {
            root: MapRoot {
                page: root.digest,
                entries: root.count,
            },
        })
    }

    pub fn empty<S: PageStore + ?Sized>(
        store: &mut S,
        work: &mut MapWork,
    ) -> Result<Self, MapError> {
        let root = build_entries(store, &[], 0, work)?;
        Ok(Self {
            root: MapRoot {
                page: root.digest,
                entries: 0,
            },
        })
    }

    pub const fn from_root(root: MapRoot) -> Self {
        Self { root }
    }

    pub const fn root(self) -> MapRoot {
        self.root
    }

    pub const fn len(self) -> u64 {
        self.root.entries
    }

    pub const fn is_empty(self) -> bool {
        self.root.entries == 0
    }

    pub fn lookup<S: PageStore + ?Sized>(
        &self,
        store: &S,
        key: &[u8],
        work: &mut MapWork,
    ) -> Result<Option<Vec<u8>>, MapError> {
        validate_key(key)?;
        let mut digest = self.root.page;
        let mut page = load_page(store, digest, work)?;
        verify_root(self.root, &page)?;
        let mut depth = 0;
        loop {
            ensure_depth(depth)?;
            match page {
                Page::Leaf { entries, .. } => {
                    return match search_entries(&entries, key, work)? {
                        Ok(index) => Ok(Some(entries[index].value.clone())),
                        Err(_) => Ok(None),
                    };
                }
                Page::Branch {
                    prefix,
                    terminal,
                    children,
                    ..
                } => {
                    if key == prefix {
                        return Ok(terminal);
                    }
                    if !key.starts_with(&prefix) || key.len() <= prefix.len() {
                        return Ok(None);
                    }
                    let edge = key[prefix.len()];
                    let Ok(index) = search_children(&children, edge, work)? else {
                        return Ok(None);
                    };
                    let child = &children[index];
                    digest = child.digest;
                    page = load_page(store, digest, work)?;
                    verify_child_link(&prefix, child, &page)?;
                    depth += 1;
                }
            }
        }
    }

    /// Iterates in canonical byte-key order without allocating a complete result vector.
    pub fn for_each<S, F>(
        &self,
        store: &S,
        work: &mut MapWork,
        mut visitor: F,
    ) -> Result<(), MapError>
    where
        S: PageStore + ?Sized,
        F: FnMut(&[u8], &[u8]) -> Result<(), MapError>,
    {
        let page = load_page(store, self.root.page, work)?;
        verify_root(self.root, &page)?;
        let visited = visit_loaded(
            store,
            self.root.page,
            &page,
            0,
            work,
            &mut |key, value, _work| visitor(key, value),
        )?;
        if visited != self.root.entries {
            return Err(map_error(
                MapErrorClass::Corrupt,
                "persistent_map_iteration_count",
                "full iteration disagrees with the map root entry count",
            ));
        }
        Ok(())
    }

    pub fn insert<S: PageStore + ?Sized>(
        &self,
        store: &mut S,
        key: &[u8],
        value: &[u8],
        work: &mut MapWork,
    ) -> Result<(Self, InsertOutcome), MapError> {
        validate_key(key)?;
        validate_value(value)?;
        let page = load_page(store, self.root.page, work)?;
        verify_root(self.root, &page)?;
        let (replacement, previous, changed) =
            insert_loaded(store, self.root.page, page, key, value, 0, work)?;
        if !changed {
            return Ok((Self { root: self.root }, InsertOutcome::Unchanged));
        }
        let outcome = previous.map_or(InsertOutcome::Inserted, |previous| {
            InsertOutcome::Replaced { previous }
        });
        Ok((
            Self {
                root: MapRoot {
                    page: replacement.digest,
                    entries: replacement.count,
                },
            },
            outcome,
        ))
    }

    pub fn remove<S: PageStore + ?Sized>(
        &self,
        store: &mut S,
        key: &[u8],
        work: &mut MapWork,
    ) -> Result<(Self, RemoveOutcome), MapError> {
        validate_key(key)?;
        let page = load_page(store, self.root.page, work)?;
        verify_root(self.root, &page)?;
        let result = remove_loaded(store, self.root.page, page, key, 0, work)?;
        if !result.changed {
            return Ok((Self { root: self.root }, RemoveOutcome::Unchanged));
        }
        let previous = result.previous.ok_or_else(|| {
            map_error(
                MapErrorClass::Corrupt,
                "persistent_map_remove_previous",
                "successful removal omitted its previous value",
            )
        })?;
        let replacement = match result.replacement {
            Some(replacement) => replacement,
            None => build_entries(store, &[], 0, work)?,
        };
        Ok((
            Self {
                root: MapRoot {
                    page: replacement.digest,
                    entries: replacement.count,
                },
            },
            RemoveOutcome::Removed { previous },
        ))
    }

    /// Copies exactly the pages reachable from this map root into another immutable store.
    /// Intermediate pages left behind by a series of path-copy mutations are deliberately
    /// omitted. The copied root is exhaustively verified while traversed.
    pub fn copy_reachable<S, D>(
        &self,
        source: &S,
        destination: &mut D,
        work: &mut MapWork,
    ) -> Result<u64, MapError>
    where
        S: PageStore + ?Sized,
        D: PageStore + ?Sized,
    {
        let mut seen = BTreeMap::new();
        let summary = copy_reachable_page(source, destination, self.root.page, 0, &mut seen, work)?;
        if summary.count != self.root.entries {
            return Err(map_error(
                MapErrorClass::Corrupt,
                "persistent_map_copy_count",
                "copied page summaries disagree with the map root entry count",
            ));
        }
        u64::try_from(seen.len()).map_err(|_| {
            map_error(
                MapErrorClass::Resource,
                "persistent_map_copy_pages",
                "copied page count cannot be represented",
            )
        })
    }

    /// Copies only final pages staged by a path-copy mutation. [`OverlayPageStore`] records every
    /// generated page even when its bytes already exist physically, so a missing staged page is
    /// exactly an unchanged accepted-base subtree and can be reused without traversal.
    ///
    /// This is deliberately narrower than [`Self::copy_reachable`]: callers must retain the
    /// immutable base store until the returned pages are durably published. Full reconstruction,
    /// backup, and doctor must continue to use the exhaustive operation.
    pub fn copy_staged_reachable<D>(
        &self,
        staged: &MemoryPageStore,
        destination: &mut D,
        work: &mut MapWork,
    ) -> Result<u64, MapError>
    where
        D: PageStore + ?Sized,
    {
        let mut seen = BTreeMap::new();
        copy_staged_reachable_page(
            staged,
            destination,
            self.root.page,
            self.root.entries,
            None,
            0,
            &mut seen,
            work,
        )?;
        u64::try_from(seen.len()).map_err(|_| {
            map_error(
                MapErrorClass::Resource,
                "persistent_map_staged_pages",
                "staged reachable page count cannot be represented",
            )
        })
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the traversal carries explicit integrity summaries and bounded work state"
)]
fn copy_staged_reachable_page<D>(
    staged: &MemoryPageStore,
    destination: &mut D,
    digest: PageDigest,
    expected_count: u64,
    parent_link: Option<(&[u8], &ChildRef)>,
    depth: usize,
    seen: &mut BTreeMap<PageDigest, PageLinkSummary>,
    work: &mut MapWork,
) -> Result<(), MapError>
where
    D: PageStore + ?Sized,
{
    ensure_depth(depth)?;
    if let Some(summary) = seen.get(&digest) {
        if summary.count != expected_count {
            return Err(map_error(
                MapErrorClass::Corrupt,
                "persistent_map_staged_summary",
                "one staged page digest is referenced with conflicting subtree summaries",
            ));
        }
        if let Some((parent_prefix, child)) = parent_link {
            verify_child_summary(parent_prefix, child, summary)?;
        }
        return Ok(());
    }
    if !staged.pages.contains_key(&digest) {
        return Ok(());
    }

    let page = load_page(staged, digest, work)?;
    if page.count() != expected_count {
        return Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_staged_summary",
            "staged page content disagrees with its parent or map-root summary",
        ));
    }
    if let Some((parent_prefix, child)) = parent_link {
        verify_child_link(parent_prefix, child, &page)?;
    }
    let written = write_page(destination, &page, work)?;
    if written.digest != digest {
        return Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_staged_digest",
            "copied staged page changed its canonical digest",
        ));
    }
    seen.insert(digest, PageLinkSummary::from_page(&page));
    if let Page::Branch {
        prefix, children, ..
    } = &page
    {
        for child in children {
            copy_staged_reachable_page(
                staged,
                destination,
                child.digest,
                child.count,
                Some((prefix, child)),
                depth + 1,
                seen,
                work,
            )?;
        }
    }
    Ok(())
}

fn copy_reachable_page<S, D>(
    source: &S,
    destination: &mut D,
    digest: PageDigest,
    depth: usize,
    seen: &mut BTreeMap<PageDigest, PageLinkSummary>,
    work: &mut MapWork,
) -> Result<PageLinkSummary, MapError>
where
    S: PageStore + ?Sized,
    D: PageStore + ?Sized,
{
    ensure_depth(depth)?;
    if let Some(summary) = seen.get(&digest) {
        return Ok(summary.clone());
    }
    let page = load_page(source, digest, work)?;
    let written = write_page(destination, &page, work)?;
    if written.digest != digest {
        return Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_copy_digest",
            "copied canonical page changed its digest",
        ));
    }
    let summary = PageLinkSummary::from_page(&page);
    seen.insert(digest, summary.clone());
    if let Page::Branch {
        prefix,
        terminal,
        children,
        ..
    } = &page
    {
        let mut child_count = u64::from(terminal.is_some());
        for child in children {
            let observed =
                copy_reachable_page(source, destination, child.digest, depth + 1, seen, work)?;
            verify_child_summary(prefix, child, &observed)?;
            child_count = child_count.checked_add(observed.count).ok_or_else(|| {
                map_error(
                    MapErrorClass::Resource,
                    "persistent_map_copy_count",
                    "copied entry count overflowed",
                )
            })?;
        }
        if child_count != page.count() {
            return Err(map_error(
                MapErrorClass::Corrupt,
                "persistent_map_copy_count",
                "copied branch entry count disagrees with its page summary",
            ));
        }
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> (MemoryPageStore, PersistentMap) {
        let mut store = MemoryPageStore::default();
        let map = PersistentMap::empty(&mut store, &mut MapWork::default())
            .expect("empty map must be constructible");
        (store, map)
    }

    fn insert(
        store: &mut MemoryPageStore,
        map: PersistentMap,
        key: &[u8],
        value: &[u8],
    ) -> PersistentMap {
        map.insert(store, key, value, &mut MapWork::default())
            .expect("test insertion must succeed")
            .0
    }

    fn numbered_entry(number: u32) -> ([u8; 4], Vec<u8>) {
        let key = number.to_be_bytes();
        let mut value = vec![0_u8; 64];
        for (offset, byte) in value.iter_mut().enumerate() {
            *byte = number.wrapping_add(offset as u32) as u8;
        }
        (key, value)
    }

    fn numbered_map(
        store: &mut MemoryPageStore,
        order: impl IntoIterator<Item = u32>,
    ) -> PersistentMap {
        let mut map = PersistentMap::empty(store, &mut MapWork::default())
            .expect("empty numbered map must be constructible");
        for number in order {
            let (key, value) = numbered_entry(number);
            map = insert(store, map, &key, &value);
        }
        map
    }

    fn collect(store: &MemoryPageStore, map: PersistentMap) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut entries = Vec::new();
        map.for_each(store, &mut MapWork::default(), |key, value| {
            entries.push((key.to_vec(), value.to_vec()));
            Ok(())
        })
        .expect("test iteration must succeed");
        entries
    }

    fn reseal(bytes: &mut [u8]) {
        let checksum_start = bytes.len() - PAGE_CHECKSUM_BYTES;
        let checksum = domain_digest(PAGE_CHECKSUM_DOMAIN, &bytes[..checksum_start]);
        bytes[checksum_start..].copy_from_slice(&checksum);
    }

    #[test]
    fn small_map_is_one_leaf_and_iterates_in_key_order() {
        let (mut store, mut map) = empty();
        map = insert(&mut store, map, b"gamma", b"3");
        map = insert(&mut store, map, b"alpha", b"1");
        map = insert(&mut store, map, b"beta", b"2");

        let root_bytes = store
            .pages
            .get(&map.root().page())
            .expect("root page must exist");
        let root = decode_page(root_bytes).expect("root page must decode");
        assert!(matches!(root, Page::Leaf { .. }));
        assert_eq!(map.len(), 3);
        assert_eq!(
            map.lookup(&store, b"beta", &mut MapWork::default())
                .expect("lookup must succeed"),
            Some(b"2".to_vec())
        );
        assert_eq!(
            collect(&store, map),
            vec![
                (b"alpha".to_vec(), b"1".to_vec()),
                (b"beta".to_vec(), b"2".to_vec()),
                (b"gamma".to_vec(), b"3".to_vec()),
            ]
        );
        let report = map
            .verify(&store, &mut MapWork::default())
            .expect("small map must verify");
        assert_eq!(report.entries, 3);
        assert_eq!(report.pages, 1);
    }

    #[test]
    fn root_is_independent_of_insertion_order() {
        let mut forward_store = MemoryPageStore::default();
        let forward = numbered_map(&mut forward_store, 0..600);
        let mut reverse_store = MemoryPageStore::default();
        let reverse = numbered_map(&mut reverse_store, (0..600).rev());

        assert_eq!(forward.root(), reverse.root());
        assert_eq!(
            collect(&forward_store, forward),
            collect(&reverse_store, reverse)
        );
        assert!(matches!(
            decode_page(
                forward_store
                    .pages
                    .get(&forward.root().page())
                    .expect("forward root must exist")
            )
            .expect("forward root must decode"),
            Page::Branch { .. }
        ));

        let mut bulk_store = MemoryPageStore::default();
        let bulk = PersistentMap::from_sorted(
            &mut bulk_store,
            (0..600).map(|number| {
                let (key, value) = numbered_entry(number);
                (key.to_vec(), value)
            }),
            &mut MapWork::default(),
        )
        .expect("ordered bulk map must build");
        assert_eq!(bulk.root(), forward.root());

        assert_eq!(
            PersistentMap::from_sorted(
                &mut bulk_store,
                [(b"b".to_vec(), vec![2]), (b"a".to_vec(), vec![1])],
                &mut MapWork::default(),
            )
            .expect_err("unordered bulk map must reject")
            .code,
            "persistent_map_input_order"
        );
    }

    #[test]
    fn prefix_key_uses_terminal_and_collapses_after_removal() {
        let (mut store, map) = empty();
        let large = vec![7_u8; TARGET_LEAF_PAGE_BYTES + 100];
        let map = insert(&mut store, map, b"a", &large);
        let map = insert(&mut store, map, b"ab", b"child");
        let page = decode_page(
            store
                .pages
                .get(&map.root().page())
                .expect("branch root must exist"),
        )
        .expect("branch root must decode");
        assert!(matches!(
            page,
            Page::Branch {
                terminal: Some(_),
                ..
            }
        ));
        assert_eq!(
            map.lookup(&store, b"a", &mut MapWork::default())
                .expect("terminal lookup must succeed"),
            Some(large.clone())
        );

        let (map, outcome) = map
            .remove(&mut store, b"a", &mut MapWork::default())
            .expect("terminal removal must succeed");
        assert_eq!(outcome, RemoveOutcome::Removed { previous: large });
        let page = decode_page(
            store
                .pages
                .get(&map.root().page())
                .expect("collapsed root must exist"),
        )
        .expect("collapsed root must decode");
        assert!(matches!(page, Page::Leaf { .. }));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn path_update_is_local_and_insert_remove_restores_root() {
        let mut store = MemoryPageStore::default();
        let base = numbered_map(&mut store, 0..1_200);
        let object_count = store.object_count();
        let (key, mut value) = numbered_entry(600);
        value[0] ^= 0xff;
        let mut update_work = MapWork::default();
        let (updated, outcome) = base
            .insert(&mut store, &key, &value, &mut update_work)
            .expect("local replacement must succeed");
        assert!(matches!(outcome, InsertOutcome::Replaced { .. }));
        assert!(update_work.pages_read < object_count as u64);
        assert!(update_work.pages_written <= 4);

        let new_key = u32::MAX.to_be_bytes();
        let (extended, outcome) = updated
            .insert(&mut store, &new_key, b"temporary", &mut MapWork::default())
            .expect("temporary insertion must succeed");
        assert_eq!(outcome, InsertOutcome::Inserted);
        let (restored, outcome) = extended
            .remove(&mut store, &new_key, &mut MapWork::default())
            .expect("temporary removal must succeed");
        assert_eq!(
            outcome,
            RemoveOutcome::Removed {
                previous: b"temporary".to_vec()
            }
        );
        assert_eq!(restored.root(), updated.root());

        let (unchanged, outcome) = updated
            .insert(&mut store, &key, &value, &mut MapWork::default())
            .expect("idempotent insertion must succeed");
        assert_eq!(outcome, InsertOutcome::Unchanged);
        assert_eq!(unchanged.root(), updated.root());
    }

    #[test]
    fn diff_skips_equal_subtrees_and_reports_exact_changes() {
        let mut store = MemoryPageStore::default();
        let base = numbered_map(&mut store, 0..1_000);
        let (update_key, mut update_value) = numbered_entry(10);
        update_value[1] ^= 0x5a;
        let (changed, _) = base
            .insert(
                &mut store,
                &update_key,
                &update_value,
                &mut MapWork::default(),
            )
            .expect("diff update must succeed");
        let remove_key = 20_u32.to_be_bytes();
        let (changed, _) = changed
            .remove(&mut store, &remove_key, &mut MapWork::default())
            .expect("diff removal must succeed");
        let add_key = 10_000_u32.to_be_bytes();
        let (changed, _) = changed
            .insert(&mut store, &add_key, b"new", &mut MapWork::default())
            .expect("diff addition must succeed");

        let mut differences = Vec::new();
        let mut work = MapWork::default();
        base.diff(&changed, &store, &mut work, |difference| {
            differences.push(difference);
            Ok(())
        })
        .expect("diff must succeed");
        assert_eq!(differences.len(), 3);
        assert!(differences.iter().any(|difference| matches!(
            difference,
            MapDifference::Updated { key, .. } if key == &update_key
        )));
        assert!(differences.iter().any(|difference| matches!(
            difference,
            MapDifference::Removed { key, .. } if key == &remove_key
        )));
        assert!(differences.iter().any(|difference| matches!(
            difference,
            MapDifference::Added { key, .. } if key == &add_key
        )));
        assert!(work.subtrees_skipped > 0);
        assert!(work.entries_skipped > 0);

        let mut equal_work = MapWork::default();
        base.diff(&base, &store, &mut equal_work, |_difference| Ok(()))
            .expect("equal diff must succeed");
        assert_eq!(equal_work.pages_read, 0);
        assert_eq!(equal_work.subtrees_skipped, 1);
        assert_eq!(equal_work.entries_skipped, base.len());
    }

    #[test]
    fn randomized_mutations_match_btree_oracle() {
        let (mut store, mut map) = empty();
        let mut oracle = BTreeMap::<Vec<u8>, Vec<u8>>::new();
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        for step in 0..4_000_u32 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let number = (state % 1_000) as u16;
            let key = number.to_be_bytes().to_vec();
            if state.is_multiple_of(4) {
                let expected = oracle.remove(&key);
                let (next, outcome) = map
                    .remove(&mut store, &key, &mut MapWork::default())
                    .expect("random removal must succeed");
                match (expected, outcome) {
                    (None, RemoveOutcome::Unchanged) => {}
                    (Some(expected), RemoveOutcome::Removed { previous }) => {
                        assert_eq!(previous, expected);
                    }
                    other => panic!("remove outcome disagrees with oracle: {other:?}"),
                }
                map = next;
            } else {
                let value = [step.to_be_bytes().as_slice(), &state.to_be_bytes()].concat();
                let expected = oracle.insert(key.clone(), value.clone());
                let (next, outcome) = map
                    .insert(&mut store, &key, &value, &mut MapWork::default())
                    .expect("random insertion must succeed");
                match (expected, outcome) {
                    (None, InsertOutcome::Inserted) => {}
                    (Some(expected), InsertOutcome::Replaced { previous }) => {
                        assert_eq!(previous, expected);
                    }
                    (Some(expected), InsertOutcome::Unchanged) => {
                        assert_eq!(expected, value);
                    }
                    other => panic!("insert outcome disagrees with oracle: {other:?}"),
                }
                map = next;
            }
            if step % 101 == 0 {
                assert_eq!(
                    map.lookup(&store, &key, &mut MapWork::default())
                        .expect("random lookup must succeed"),
                    oracle.get(&key).cloned()
                );
            }
        }

        let expected = oracle.into_iter().collect::<Vec<_>>();
        assert_eq!(collect(&store, map), expected);
        map.verify(&store, &mut MapWork::default())
            .expect("randomized map must verify");

        let mut reverse_store = MemoryPageStore::default();
        let mut reverse = PersistentMap::empty(&mut reverse_store, &mut MapWork::default())
            .expect("reverse oracle map must start empty");
        for (key, value) in expected.iter().rev() {
            reverse = insert(&mut reverse_store, reverse, key, value);
        }
        assert_eq!(map.root(), reverse.root());
    }

    #[test]
    fn hostile_envelope_and_summary_corruption_fail_closed() {
        let (store, map) = empty();
        let valid = store
            .pages
            .get(&map.root().page())
            .expect("empty root page must exist")
            .clone();

        let truncated = &valid[..valid.len() - 1];
        assert!(decode_page(truncated).is_err());

        let mut bad_checksum = valid.clone();
        let last = bad_checksum.len() - 1;
        bad_checksum[last] ^= 1;
        assert_eq!(
            decode_page(&bad_checksum)
                .expect_err("checksum corruption must reject")
                .code,
            "persistent_map_page_checksum"
        );

        let mut trailing = valid.clone();
        trailing.push(0);
        assert_eq!(
            decode_page(&trailing)
                .expect_err("trailing data must reject")
                .code,
            "persistent_map_page_length"
        );

        let mut bad_count = valid.clone();
        let prefix_length = u16::from_le_bytes([bad_count[20], bad_count[21]]) as usize;
        let count_offset = 22 + prefix_length;
        bad_count[count_offset..count_offset + 8].copy_from_slice(&1_u64.to_le_bytes());
        reseal(&mut bad_count);
        assert!(decode_page(&bad_count).is_err());

        let mut branched_store = MemoryPageStore::default();
        let branched = numbered_map(&mut branched_store, 0..800);
        let root_bytes = branched_store
            .pages
            .get(&branched.root().page())
            .expect("branched root must exist");
        let mut root_page = decode_page(root_bytes).expect("branched root must decode");
        let Page::Branch {
            count, children, ..
        } = &mut root_page
        else {
            panic!("numbered test map must have a branch root");
        };
        children[0].count += 1;
        *count += 1;
        let corrupt_root_bytes =
            encode_page(&root_page).expect("locally coherent root must encode");
        let corrupt_digest = PageDigest::of(&corrupt_root_bytes);
        branched_store
            .write_page(corrupt_digest, &corrupt_root_bytes)
            .expect("corrupt test root must be stored");
        let corrupt = PersistentMap::from_root(MapRoot::from_parts(
            corrupt_digest,
            branched.root().entries() + 1,
        ));
        assert_eq!(
            corrupt
                .verify(&branched_store, &mut MapWork::default())
                .expect_err("child summary corruption must reject")
                .code,
            "persistent_map_child_summary"
        );
    }

    #[test]
    fn missing_and_foreign_page_bytes_fail_on_the_touched_path() {
        let mut store = MemoryPageStore::default();
        let map = numbered_map(&mut store, 0..800);
        let root_bytes = store
            .pages
            .get(&map.root().page())
            .expect("root must exist")
            .clone();
        let root = decode_page(&root_bytes).expect("root must decode");
        let Page::Branch { children, .. } = root else {
            panic!("numbered test map must branch");
        };
        let child = children[0].clone();
        let child_page = decode_page(
            store
                .pages
                .get(&child.digest)
                .expect("selected child must exist"),
        )
        .expect("selected child must decode");
        let key = match child_page {
            Page::Leaf { entries, .. } => entries[0].key.clone(),
            Page::Branch { prefix, .. } => prefix,
        };
        store.pages.remove(&child.digest);
        assert_eq!(
            map.lookup(&store, &key, &mut MapWork::default())
                .expect_err("missing child must reject")
                .code,
            "persistent_map_page_missing"
        );

        let mut corrupt_store = MemoryPageStore::default();
        let corrupt_map = numbered_map(&mut corrupt_store, 0..20);
        corrupt_store
            .pages
            .get_mut(&corrupt_map.root().page())
            .expect("corrupt root must exist")[0] ^= 1;
        assert_eq!(
            corrupt_map
                .lookup(
                    &corrupt_store,
                    &0_u32.to_be_bytes(),
                    &mut MapWork::default()
                )
                .expect_err("foreign root bytes must reject")
                .code,
            "persistent_map_page_digest"
        );
    }

    #[test]
    fn hostile_key_and_value_limits_reject_before_store_access() {
        let (mut store, map) = empty();
        let oversized_key = vec![0_u8; MAXIMUM_KEY_BYTES + 1];
        let mut work = MapWork::default();
        assert_eq!(
            map.insert(&mut store, &oversized_key, b"value", &mut work)
                .expect_err("oversized key must reject")
                .code,
            "persistent_map_key_limit"
        );
        assert_eq!(work.pages_read, 0);

        let oversized_value = vec![0_u8; MAXIMUM_VALUE_BYTES + 1];
        let mut work = MapWork::default();
        assert_eq!(
            map.insert(&mut store, b"key", &oversized_value, &mut work)
                .expect_err("oversized value must reject")
                .code,
            "persistent_map_value_limit"
        );
        assert_eq!(work.pages_read, 0);
    }

    #[test]
    fn reachable_copy_omits_superseded_path_pages() {
        let mut source = MemoryPageStore::default();
        let base = numbered_map(&mut source, 0..1_200);
        let (key, mut value) = numbered_entry(600);
        value[0] ^= 0xff;
        let (current, _) = base
            .insert(&mut source, &key, &value, &mut MapWork::default())
            .expect("local replacement must succeed");
        assert!(source.object_count() > 1);

        let mut destination = MemoryPageStore::default();
        let mut work = MapWork::default();
        let copied = current
            .copy_reachable(&source, &mut destination, &mut work)
            .expect("reachable copy must succeed");
        assert_eq!(copied as usize, destination.object_count());
        assert!(destination.object_count() < source.object_count());
        assert_eq!(collect(&source, current), collect(&destination, current));
        current
            .verify(&destination, &mut MapWork::default())
            .expect("copied map must verify independently");
    }

    #[test]
    fn overlay_reads_base_and_retains_generated_path_pages() {
        let mut base_store = MemoryPageStore::default();
        let base = numbered_map(&mut base_store, 0..1_200);
        let base_objects = base_store.object_count();
        let (key, mut value) = numbered_entry(600);
        value[0] ^= 0x7f;

        let mut overlay = OverlayPageStore::new(&base_store);
        let mut work = MapWork::default();
        let (updated, outcome) = base
            .insert(&mut overlay, &key, &value, &mut work)
            .expect("overlay update must succeed");
        assert!(matches!(outcome, InsertOutcome::Replaced { .. }));
        assert!(work.pages_read < base_objects as u64);
        let staged = overlay.into_pages();
        assert!(staged.object_count() <= 4);
        let mut retained = MemoryPageStore::default();
        let mut extraction_work = MapWork::default();
        let copied = updated
            .copy_staged_reachable(&staged, &mut retained, &mut extraction_work)
            .expect("staged reachability must succeed");
        assert_eq!(copied as usize, retained.object_count());
        assert!(retained.object_count() <= staged.object_count());
        assert!(extraction_work.pages_read <= staged.object_count() as u64);

        let mut published = base_store.clone();
        for (digest, bytes) in retained.objects() {
            published
                .write_page(digest, bytes)
                .expect("retained update page must publish");
        }
        updated
            .verify(&published, &mut MapWork::default())
            .expect("base plus retained staged pages must verify");

        let combined = OverlayPageStore::new(&base_store);
        // A fresh overlay does not contain the staged pages, proving they were not written into
        // the immutable base store before publication.
        assert_eq!(
            updated
                .lookup(&combined, &key, &mut MapWork::default())
                .expect_err("unpublished root must not be readable from the base store")
                .code,
            "persistent_map_page_missing"
        );
        assert_eq!(
            base.lookup(&base_store, &key, &mut MapWork::default())
                .expect("base lookup must remain valid"),
            Some(numbered_entry(600).1)
        );
    }

    #[test]
    fn reused_physical_ancestor_still_retains_new_staged_descendants() {
        let mut base_store = MemoryPageStore::default();
        let base = numbered_map(&mut base_store, 0..1_200);
        let (key, mut value) = numbered_entry(600);
        value[0] ^= 0x7f;

        let mut first_overlay = OverlayPageStore::new(&base_store);
        let (updated, outcome) = base
            .insert(&mut first_overlay, &key, &value, &mut MapWork::default())
            .expect("first path-copy update");
        assert!(matches!(outcome, InsertOutcome::Replaced { .. }));
        let first_staged = first_overlay.into_pages();
        let root_digest = updated.root().page();
        let root_bytes = first_staged
            .pages
            .get(&root_digest)
            .expect("updated root must be generated")
            .clone();
        assert!(
            first_staged
                .pages
                .keys()
                .any(|digest| *digest != root_digest && !base_store.pages.contains_key(digest)),
            "the fixture needs a new descendant below the updated root"
        );

        // Model an unreachable page left by an interrupted old publication: the final branch is
        // physically present, but one of its new children is not. Repeating the mutation must
        // stage the physically reused branch so reachability does not discard that child.
        let mut physical = base_store.clone();
        physical
            .write_page(root_digest, &root_bytes)
            .expect("install orphan root fixture");
        let mut second_overlay = OverlayPageStore::new(&physical);
        let mut second_work = MapWork::default();
        let (repeated, outcome) = base
            .insert(&mut second_overlay, &key, &value, &mut second_work)
            .expect("repeated path-copy update");
        assert_eq!(repeated, updated);
        assert!(matches!(outcome, InsertOutcome::Replaced { .. }));
        assert!(second_work.pages_reused > 0);
        let second_staged = second_overlay.into_pages();
        assert!(second_staged.pages.contains_key(&root_digest));

        let mut retained = MemoryPageStore::default();
        repeated
            .copy_staged_reachable(&second_staged, &mut retained, &mut MapWork::default())
            .expect("extract repeated staged update");
        let mut published = physical;
        for (digest, bytes) in retained.objects() {
            published
                .write_page(digest, bytes)
                .expect("publish retained repeated page");
        }
        repeated
            .verify(&published, &mut MapWork::default())
            .expect("reused branch and newly staged descendant must verify");
    }

    #[test]
    fn reachable_copies_reject_parent_child_prefix_mismatch() {
        let mut source = MemoryPageStore::default();
        let map = numbered_map(&mut source, 0..1_200);
        let root_bytes = source
            .pages
            .get(&map.root().page())
            .expect("root page")
            .clone();
        let mut root_page = decode_page(&root_bytes).expect("root page must decode");
        let Page::Branch { children, .. } = &mut root_page else {
            panic!("numbered map root must branch");
        };
        let child = children.last_mut().expect("root branch child");
        assert!(child.edge < u8::MAX, "fixture needs a larger unused edge");
        child.edge += 1;
        let corrupt_bytes = encode_page(&root_page).expect("locally valid corrupt root page");
        let corrupt_digest = PageDigest::of(&corrupt_bytes);
        source
            .write_page(corrupt_digest, &corrupt_bytes)
            .expect("install corrupt parent fixture");
        let corrupt =
            PersistentMap::from_root(MapRoot::from_parts(corrupt_digest, map.root().entries()));

        let mut exhaustive_destination = MemoryPageStore::default();
        assert_eq!(
            corrupt
                .copy_reachable(
                    &source,
                    &mut exhaustive_destination,
                    &mut MapWork::default(),
                )
                .expect_err("exhaustive copy must reject a foreign child edge")
                .code,
            "persistent_map_child_prefix"
        );

        let mut staged_destination = MemoryPageStore::default();
        assert_eq!(
            corrupt
                .copy_staged_reachable(&source, &mut staged_destination, &mut MapWork::default(),)
                .expect_err("staged copy must reject a foreign generated child edge")
                .code,
            "persistent_map_child_prefix"
        );
    }
}

#[derive(Clone, Copy)]
enum EmitKind {
    Added,
    Removed,
}

fn skip_subtree(work: &mut MapWork, entries: u64) -> Result<(), MapError> {
    add_counter(
        &mut work.subtrees_skipped,
        1,
        "persistent_map_work_subtrees_skipped",
    )?;
    add_counter(
        &mut work.entries_skipped,
        entries,
        "persistent_map_work_entries_skipped",
    )
}

fn emit_difference<F>(
    difference: MapDifference,
    work: &mut MapWork,
    visitor: &mut F,
) -> Result<(), MapError>
where
    F: FnMut(MapDifference) -> Result<(), MapError>,
{
    visitor(difference)?;
    add_counter(
        &mut work.differences_emitted,
        1,
        "persistent_map_work_differences",
    )
}

fn emit_value_difference<F>(
    key: &[u8],
    before: Option<&[u8]>,
    after: Option<&[u8]>,
    work: &mut MapWork,
    visitor: &mut F,
) -> Result<(), MapError>
where
    F: FnMut(MapDifference) -> Result<(), MapError>,
{
    match (before, after) {
        (None, Some(value)) => emit_difference(
            MapDifference::Added {
                key: key.to_vec(),
                value: value.to_vec(),
            },
            work,
            visitor,
        ),
        (Some(value), None) => emit_difference(
            MapDifference::Removed {
                key: key.to_vec(),
                value: value.to_vec(),
            },
            work,
            visitor,
        ),
        (Some(before), Some(after)) if before != after => emit_difference(
            MapDifference::Updated {
                key: key.to_vec(),
                before: before.to_vec(),
                after: after.to_vec(),
            },
            work,
            visitor,
        ),
        _ => Ok(()),
    }
}

fn emit_loaded<S, F>(
    store: &S,
    digest: PageDigest,
    page: &Page,
    kind: EmitKind,
    depth: usize,
    work: &mut MapWork,
    visitor: &mut F,
) -> Result<(), MapError>
where
    S: PageStore + ?Sized,
    F: FnMut(MapDifference) -> Result<(), MapError>,
{
    visit_loaded(store, digest, page, depth, work, &mut |key, value, work| {
        let difference = match kind {
            EmitKind::Added => MapDifference::Added {
                key: key.to_vec(),
                value: value.to_vec(),
            },
            EmitKind::Removed => MapDifference::Removed {
                key: key.to_vec(),
                value: value.to_vec(),
            },
        };
        emit_difference(difference, work, visitor)
    })?;
    Ok(())
}

fn emit_child<S, F>(
    store: &S,
    parent_prefix: &[u8],
    child: &ChildRef,
    kind: EmitKind,
    depth: usize,
    work: &mut MapWork,
    visitor: &mut F,
) -> Result<(), MapError>
where
    S: PageStore + ?Sized,
    F: FnMut(MapDifference) -> Result<(), MapError>,
{
    let page = load_page(store, child.digest, work)?;
    verify_child_link(parent_prefix, child, &page)?;
    emit_loaded(store, child.digest, &page, kind, depth, work, visitor)
}

fn diff_leaf_entries<F>(
    before: &[Entry],
    after: &[Entry],
    work: &mut MapWork,
    visitor: &mut F,
) -> Result<(), MapError>
where
    F: FnMut(MapDifference) -> Result<(), MapError>,
{
    let mut left = 0;
    let mut right = 0;
    while left < before.len() || right < after.len() {
        match (before.get(left), after.get(right)) {
            (Some(old), Some(new)) => match old.key.cmp(&new.key) {
                std::cmp::Ordering::Less => {
                    emit_value_difference(&old.key, Some(&old.value), None, work, visitor)?;
                    left += 1;
                }
                std::cmp::Ordering::Greater => {
                    emit_value_difference(&new.key, None, Some(&new.value), work, visitor)?;
                    right += 1;
                }
                std::cmp::Ordering::Equal => {
                    emit_value_difference(
                        &old.key,
                        Some(&old.value),
                        Some(&new.value),
                        work,
                        visitor,
                    )?;
                    left += 1;
                    right += 1;
                }
            },
            (Some(old), None) => {
                emit_value_difference(&old.key, Some(&old.value), None, work, visitor)?;
                left += 1;
            }
            (None, Some(new)) => {
                emit_value_difference(&new.key, None, Some(&new.value), work, visitor)?;
                right += 1;
            }
            (None, None) => break,
        }
    }
    Ok(())
}

fn diff_leaf_to_tree<S, F>(
    store: &S,
    before: &[Entry],
    after_digest: PageDigest,
    after: &Page,
    depth: usize,
    work: &mut MapWork,
    visitor: &mut F,
) -> Result<(), MapError>
where
    S: PageStore + ?Sized,
    F: FnMut(MapDifference) -> Result<(), MapError>,
{
    let mut remaining = before
        .iter()
        .map(|entry| (entry.key.clone(), entry.value.clone()))
        .collect::<BTreeMap<_, _>>();
    visit_loaded(
        store,
        after_digest,
        after,
        depth,
        work,
        &mut |key, value, work| {
            let old = remaining.remove(key);
            emit_value_difference(key, old.as_deref(), Some(value), work, visitor)
        },
    )?;
    for (key, value) in remaining {
        emit_value_difference(&key, Some(&value), None, work, visitor)?;
    }
    Ok(())
}

fn diff_tree_to_leaf<S, F>(
    store: &S,
    before_digest: PageDigest,
    before: &Page,
    after: &[Entry],
    depth: usize,
    work: &mut MapWork,
    visitor: &mut F,
) -> Result<(), MapError>
where
    S: PageStore + ?Sized,
    F: FnMut(MapDifference) -> Result<(), MapError>,
{
    let mut remaining = after
        .iter()
        .map(|entry| (entry.key.clone(), entry.value.clone()))
        .collect::<BTreeMap<_, _>>();
    visit_loaded(
        store,
        before_digest,
        before,
        depth,
        work,
        &mut |key, value, work| {
            let new = remaining.remove(key);
            emit_value_difference(key, Some(value), new.as_deref(), work, visitor)
        },
    )?;
    for (key, value) in remaining {
        emit_value_difference(&key, None, Some(&value), work, visitor)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn diff_child_refs<S, F>(
    store: &S,
    before_parent: &[u8],
    before: &ChildRef,
    after_parent: &[u8],
    after: &ChildRef,
    depth: usize,
    work: &mut MapWork,
    visitor: &mut F,
) -> Result<(), MapError>
where
    S: PageStore + ?Sized,
    F: FnMut(MapDifference) -> Result<(), MapError>,
{
    if before.digest == after.digest {
        if before.count != after.count || before.logical_bytes != after.logical_bytes {
            return Err(map_error(
                MapErrorClass::Corrupt,
                "persistent_map_diff_child_summary",
                "equal child digests carry different subtree summaries",
            ));
        }
        return skip_subtree(work, before.count);
    }
    let before_page = load_page(store, before.digest, work)?;
    verify_child_link(before_parent, before, &before_page)?;
    let after_page = load_page(store, after.digest, work)?;
    verify_child_link(after_parent, after, &after_page)?;
    diff_loaded(
        store,
        before.digest,
        &before_page,
        after.digest,
        &after_page,
        depth,
        work,
        visitor,
    )
}

#[allow(clippy::too_many_arguments)]
fn diff_page_to_child<S, F>(
    store: &S,
    before_digest: PageDigest,
    before: &Page,
    after_parent: &[u8],
    after: &ChildRef,
    depth: usize,
    work: &mut MapWork,
    visitor: &mut F,
) -> Result<(), MapError>
where
    S: PageStore + ?Sized,
    F: FnMut(MapDifference) -> Result<(), MapError>,
{
    if before_digest == after.digest {
        if before.count() != after.count || before.logical_bytes() != after.logical_bytes {
            return Err(map_error(
                MapErrorClass::Corrupt,
                "persistent_map_diff_child_summary",
                "equal page digest and child reference have different summaries",
            ));
        }
        verify_child_link(after_parent, after, before)?;
        return skip_subtree(work, before.count());
    }
    let after_page = load_page(store, after.digest, work)?;
    verify_child_link(after_parent, after, &after_page)?;
    diff_loaded(
        store,
        before_digest,
        before,
        after.digest,
        &after_page,
        depth,
        work,
        visitor,
    )
}

#[allow(clippy::too_many_arguments)]
fn diff_child_to_page<S, F>(
    store: &S,
    before_parent: &[u8],
    before: &ChildRef,
    after_digest: PageDigest,
    after: &Page,
    depth: usize,
    work: &mut MapWork,
    visitor: &mut F,
) -> Result<(), MapError>
where
    S: PageStore + ?Sized,
    F: FnMut(MapDifference) -> Result<(), MapError>,
{
    if before.digest == after_digest {
        if before.count != after.count() || before.logical_bytes != after.logical_bytes() {
            return Err(map_error(
                MapErrorClass::Corrupt,
                "persistent_map_diff_child_summary",
                "equal child reference and page digest have different summaries",
            ));
        }
        verify_child_link(before_parent, before, after)?;
        return skip_subtree(work, before.count);
    }
    let before_page = load_page(store, before.digest, work)?;
    verify_child_link(before_parent, before, &before_page)?;
    diff_loaded(
        store,
        before.digest,
        &before_page,
        after_digest,
        after,
        depth,
        work,
        visitor,
    )
}

#[allow(clippy::too_many_arguments)]
fn diff_branches<S, F>(
    store: &S,
    before_prefix: &[u8],
    before_terminal: Option<&[u8]>,
    before_children: &[ChildRef],
    after_prefix: &[u8],
    after_terminal: Option<&[u8]>,
    after_children: &[ChildRef],
    depth: usize,
    work: &mut MapWork,
    visitor: &mut F,
) -> Result<(), MapError>
where
    S: PageStore + ?Sized,
    F: FnMut(MapDifference) -> Result<(), MapError>,
{
    emit_value_difference(
        before_prefix,
        before_terminal,
        after_terminal,
        work,
        visitor,
    )?;
    let mut left = 0;
    let mut right = 0;
    while left < before_children.len() || right < after_children.len() {
        match (before_children.get(left), after_children.get(right)) {
            (Some(old), Some(new)) => match old.edge.cmp(&new.edge) {
                std::cmp::Ordering::Less => {
                    emit_child(
                        store,
                        before_prefix,
                        old,
                        EmitKind::Removed,
                        depth + 1,
                        work,
                        visitor,
                    )?;
                    left += 1;
                }
                std::cmp::Ordering::Greater => {
                    emit_child(
                        store,
                        after_prefix,
                        new,
                        EmitKind::Added,
                        depth + 1,
                        work,
                        visitor,
                    )?;
                    right += 1;
                }
                std::cmp::Ordering::Equal => {
                    diff_child_refs(
                        store,
                        before_prefix,
                        old,
                        after_prefix,
                        new,
                        depth + 1,
                        work,
                        visitor,
                    )?;
                    left += 1;
                    right += 1;
                }
            },
            (Some(old), None) => {
                emit_child(
                    store,
                    before_prefix,
                    old,
                    EmitKind::Removed,
                    depth + 1,
                    work,
                    visitor,
                )?;
                left += 1;
            }
            (None, Some(new)) => {
                emit_child(
                    store,
                    after_prefix,
                    new,
                    EmitKind::Added,
                    depth + 1,
                    work,
                    visitor,
                )?;
                right += 1;
            }
            (None, None) => break,
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn diff_descendant_to_branch<S, F>(
    store: &S,
    before_digest: PageDigest,
    before: &Page,
    after_prefix: &[u8],
    after_terminal: Option<&[u8]>,
    after_children: &[ChildRef],
    depth: usize,
    work: &mut MapWork,
    visitor: &mut F,
) -> Result<(), MapError>
where
    S: PageStore + ?Sized,
    F: FnMut(MapDifference) -> Result<(), MapError>,
{
    emit_value_difference(after_prefix, None, after_terminal, work, visitor)?;
    let edge = before.prefix()[after_prefix.len()];
    let mut compared = false;
    for child in after_children {
        if !compared && edge < child.edge {
            emit_loaded(
                store,
                before_digest,
                before,
                EmitKind::Removed,
                depth,
                work,
                visitor,
            )?;
            compared = true;
        }
        if child.edge == edge {
            diff_page_to_child(
                store,
                before_digest,
                before,
                after_prefix,
                child,
                depth + 1,
                work,
                visitor,
            )?;
            compared = true;
        } else {
            emit_child(
                store,
                after_prefix,
                child,
                EmitKind::Added,
                depth + 1,
                work,
                visitor,
            )?;
        }
    }
    if !compared {
        emit_loaded(
            store,
            before_digest,
            before,
            EmitKind::Removed,
            depth,
            work,
            visitor,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn diff_branch_to_descendant<S, F>(
    store: &S,
    before_prefix: &[u8],
    before_terminal: Option<&[u8]>,
    before_children: &[ChildRef],
    after_digest: PageDigest,
    after: &Page,
    depth: usize,
    work: &mut MapWork,
    visitor: &mut F,
) -> Result<(), MapError>
where
    S: PageStore + ?Sized,
    F: FnMut(MapDifference) -> Result<(), MapError>,
{
    emit_value_difference(before_prefix, before_terminal, None, work, visitor)?;
    let edge = after.prefix()[before_prefix.len()];
    let mut compared = false;
    for child in before_children {
        if !compared && edge < child.edge {
            emit_loaded(
                store,
                after_digest,
                after,
                EmitKind::Added,
                depth,
                work,
                visitor,
            )?;
            compared = true;
        }
        if child.edge == edge {
            diff_child_to_page(
                store,
                before_prefix,
                child,
                after_digest,
                after,
                depth + 1,
                work,
                visitor,
            )?;
            compared = true;
        } else {
            emit_child(
                store,
                before_prefix,
                child,
                EmitKind::Removed,
                depth + 1,
                work,
                visitor,
            )?;
        }
    }
    if !compared {
        emit_loaded(
            store,
            after_digest,
            after,
            EmitKind::Added,
            depth,
            work,
            visitor,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn diff_loaded<S, F>(
    store: &S,
    before_digest: PageDigest,
    before: &Page,
    after_digest: PageDigest,
    after: &Page,
    depth: usize,
    work: &mut MapWork,
    visitor: &mut F,
) -> Result<(), MapError>
where
    S: PageStore + ?Sized,
    F: FnMut(MapDifference) -> Result<(), MapError>,
{
    ensure_depth(depth)?;
    if before_digest == after_digest {
        if before.count() != after.count() || before.logical_bytes() != after.logical_bytes() {
            return Err(map_error(
                MapErrorClass::Corrupt,
                "persistent_map_diff_page_summary",
                "equal page digests have different in-memory summaries",
            ));
        }
        return skip_subtree(work, before.count());
    }

    if before.prefix() == after.prefix() {
        return match (before, after) {
            (Page::Leaf { entries: old, .. }, Page::Leaf { entries: new, .. }) => {
                diff_leaf_entries(old, new, work, visitor)
            }
            (
                Page::Branch {
                    prefix: old_prefix,
                    terminal: old_terminal,
                    children: old_children,
                    ..
                },
                Page::Branch {
                    prefix: new_prefix,
                    terminal: new_terminal,
                    children: new_children,
                    ..
                },
            ) => diff_branches(
                store,
                old_prefix,
                old_terminal.as_deref(),
                old_children,
                new_prefix,
                new_terminal.as_deref(),
                new_children,
                depth,
                work,
                visitor,
            ),
            (Page::Leaf { entries, .. }, _) => {
                diff_leaf_to_tree(store, entries, after_digest, after, depth, work, visitor)
            }
            (_, Page::Leaf { entries, .. }) => {
                diff_tree_to_leaf(store, before_digest, before, entries, depth, work, visitor)
            }
        };
    }

    if before.prefix().starts_with(after.prefix())
        && let Page::Branch {
            prefix,
            terminal,
            children,
            ..
        } = after
    {
        return diff_descendant_to_branch(
            store,
            before_digest,
            before,
            prefix,
            terminal.as_deref(),
            children,
            depth,
            work,
            visitor,
        );
    }
    if after.prefix().starts_with(before.prefix())
        && let Page::Branch {
            prefix,
            terminal,
            children,
            ..
        } = before
    {
        return diff_branch_to_descendant(
            store,
            prefix,
            terminal.as_deref(),
            children,
            after_digest,
            after,
            depth,
            work,
            visitor,
        );
    }

    match before.prefix().cmp(after.prefix()) {
        std::cmp::Ordering::Less => {
            emit_loaded(
                store,
                before_digest,
                before,
                EmitKind::Removed,
                depth,
                work,
                visitor,
            )?;
            emit_loaded(
                store,
                after_digest,
                after,
                EmitKind::Added,
                depth,
                work,
                visitor,
            )
        }
        std::cmp::Ordering::Greater => {
            emit_loaded(
                store,
                after_digest,
                after,
                EmitKind::Added,
                depth,
                work,
                visitor,
            )?;
            emit_loaded(
                store,
                before_digest,
                before,
                EmitKind::Removed,
                depth,
                work,
                visitor,
            )
        }
        std::cmp::Ordering::Equal => Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_diff_shape",
            "equal page prefixes reached the disjoint diff path",
        )),
    }
}

fn verify_loaded<S: PageStore + ?Sized>(
    store: &S,
    digest: PageDigest,
    page: &Page,
    depth: usize,
    seen: &mut BTreeSet<PageDigest>,
    pages: &mut u64,
    work: &mut MapWork,
) -> Result<(), MapError> {
    ensure_depth(depth)?;
    if !seen.insert(digest) {
        return Err(map_error(
            MapErrorClass::Corrupt,
            "persistent_map_page_reused_in_tree",
            "one page object is reachable through more than one map edge",
        ));
    }
    add_counter(pages, 1, "persistent_map_verify_pages")?;
    match page {
        Page::Leaf { entries, .. } => add_counter(
            &mut work.entries_visited,
            usize_to_u64(entries.len(), "persistent_map_verify_entries")?,
            "persistent_map_work_entries_visited",
        )?,
        Page::Branch {
            prefix,
            terminal,
            children,
            ..
        } => {
            if terminal.is_some() {
                add_counter(
                    &mut work.entries_visited,
                    1,
                    "persistent_map_work_entries_visited",
                )?;
            }
            for child in children {
                let child_page = load_page(store, child.digest, work)?;
                verify_child_link(prefix, child, &child_page)?;
                verify_loaded(
                    store,
                    child.digest,
                    &child_page,
                    depth + 1,
                    seen,
                    pages,
                    work,
                )?;
            }
        }
    }
    Ok(())
}

impl PersistentMap {
    /// Compares two roots in one content-addressed store. Equal child digests are skipped without
    /// opening their objects; differences are streamed to the caller.
    pub fn diff<S, F>(
        &self,
        other: &Self,
        store: &S,
        work: &mut MapWork,
        mut visitor: F,
    ) -> Result<(), MapError>
    where
        S: PageStore + ?Sized,
        F: FnMut(MapDifference) -> Result<(), MapError>,
    {
        if self.root.page == other.root.page {
            if self.root.entries != other.root.entries {
                return Err(map_error(
                    MapErrorClass::Corrupt,
                    "persistent_map_diff_root_summary",
                    "equal map roots carry different entry counts",
                ));
            }
            return skip_subtree(work, self.root.entries);
        }
        let before = load_page(store, self.root.page, work)?;
        verify_root(self.root, &before)?;
        let after = load_page(store, other.root.page, work)?;
        verify_root(other.root, &after)?;
        diff_loaded(
            store,
            self.root.page,
            &before,
            other.root.page,
            &after,
            0,
            work,
            &mut visitor,
        )
    }

    /// Exhaustively verifies every reachable page and parent/child summary. Ordinary lookup and
    /// mutation verify only the pages they touch.
    pub fn verify<S: PageStore + ?Sized>(
        &self,
        store: &S,
        work: &mut MapWork,
    ) -> Result<VerificationReport, MapError> {
        let entries_before = work.entries_visited;
        let page = load_page(store, self.root.page, work)?;
        verify_root(self.root, &page)?;
        let mut seen = BTreeSet::new();
        let mut pages = 0;
        verify_loaded(store, self.root.page, &page, 0, &mut seen, &mut pages, work)?;
        let entries_verified = work
            .entries_visited
            .checked_sub(entries_before)
            .ok_or_else(|| {
                map_error(
                    MapErrorClass::Resource,
                    "persistent_map_verify_entries",
                    "verified entry accounting moved backwards",
                )
            })?;
        if entries_verified != self.root.entries {
            return Err(map_error(
                MapErrorClass::Corrupt,
                "persistent_map_verify_entries",
                "verified entry count disagrees with the map root",
            ));
        }
        Ok(VerificationReport {
            pages,
            entries: self.root.entries,
            logical_bytes: page.logical_bytes(),
        })
    }
}
