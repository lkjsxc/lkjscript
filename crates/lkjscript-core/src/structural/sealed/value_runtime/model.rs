use super::super::StructuralValueKey;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralSealResult {
    pub owner: StructuralValueKey,
    pub zero_copy_adopted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralOwnerKind {
    Unique,
    Sealed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralDisposeReport {
    pub ownership: StructuralOwnerKind,
    pub final_release: bool,
    pub nodes_reclaimed: u64,
    pub bytes_reclaimed: u64,
    pub release_work: u64,
}
