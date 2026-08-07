/// Checked semantic traversal facts for an unbounded local structural return.
/// These observations do not grant or deny language validity.
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
