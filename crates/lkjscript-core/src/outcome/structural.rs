use crate::{Error, Result};

pub const MAX_STRUCTURAL_SNAPSHOT_DEPTH: u16 = 64;
pub const MAX_STRUCTURAL_SNAPSHOT_NODES: u32 = 4_096;
pub const MAX_STRUCTURAL_SNAPSHOT_FIELDS: u32 = 4_096;
pub const MAX_STRUCTURAL_SNAPSHOT_BYTES: u64 = 1_000_000;
pub const MAX_STRUCTURAL_SNAPSHOT_PATH_BYTES: usize = 4_095;
pub const MAX_STRUCTURAL_SNAPSHOT_WORK: u64 = MAX_STRUCTURAL_SNAPSHOT_BYTES
    + MAX_STRUCTURAL_SNAPSHOT_NODES as u64
    + MAX_STRUCTURAL_SNAPSHOT_FIELDS as u64;

/// Independent bounds for a key-free returned structural value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralSnapshotLimits {
    pub max_depth: u16,
    pub max_nodes: u32,
    pub max_fields: u32,
    pub max_aggregate_bytes: u64,
    pub max_string_bytes: u64,
    pub max_path_bytes: u64,
    pub max_encode_work: u64,
    pub max_decode_work: u64,
}

impl StructuralSnapshotLimits {
    pub const DEFAULT: Self = Self {
        max_depth: MAX_STRUCTURAL_SNAPSHOT_DEPTH,
        max_nodes: MAX_STRUCTURAL_SNAPSHOT_NODES,
        max_fields: MAX_STRUCTURAL_SNAPSHOT_FIELDS,
        max_aggregate_bytes: MAX_STRUCTURAL_SNAPSHOT_BYTES,
        max_string_bytes: MAX_STRUCTURAL_SNAPSHOT_BYTES,
        max_path_bytes: MAX_STRUCTURAL_SNAPSHOT_BYTES,
        max_encode_work: MAX_STRUCTURAL_SNAPSHOT_WORK,
        max_decode_work: MAX_STRUCTURAL_SNAPSHOT_WORK,
    };

    pub fn validate(self) -> Result<Self> {
        let bounded = self.max_depth <= MAX_STRUCTURAL_SNAPSHOT_DEPTH
            && self.max_nodes <= MAX_STRUCTURAL_SNAPSHOT_NODES
            && self.max_fields <= MAX_STRUCTURAL_SNAPSHOT_FIELDS
            && self.max_aggregate_bytes <= MAX_STRUCTURAL_SNAPSHOT_BYTES
            && self.max_string_bytes <= self.max_aggregate_bytes
            && self.max_path_bytes <= self.max_aggregate_bytes
            && self.max_encode_work <= MAX_STRUCTURAL_SNAPSHOT_WORK
            && self.max_decode_work <= MAX_STRUCTURAL_SNAPSHOT_WORK;
        let nonzero = self.max_depth != 0
            && self.max_nodes != 0
            && self.max_fields != 0
            && self.max_aggregate_bytes != 0
            && self.max_string_bytes != 0
            && self.max_path_bytes != 0
            && self.max_encode_work != 0
            && self.max_decode_work != 0;
        if bounded && nonzero {
            Ok(self)
        } else {
            Err(Error::msg("invalid structural snapshot limits"))
        }
    }
}

impl Default for StructuralSnapshotLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Deterministic semantic traversal facts. Encode and decode work each charge
/// one unit per node, aggregate field, and payload byte.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StructuralSnapshotMetrics {
    pub nodes: u32,
    pub fields: u32,
    pub aggregate_bytes: u64,
    pub string_bytes: u64,
    pub path_bytes: u64,
    pub encode_work: u64,
    pub decode_work: u64,
}
