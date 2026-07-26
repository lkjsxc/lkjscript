use crate::source::{RevisionId, SourceEdition, SourceIdentity, SourceTreeIdentity};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditionMigrationChange {
    pub(super) path: String,
    pub(super) insertion_byte: u64,
    pub(super) inserted_bytes: String,
    pub(super) old_bytes: u64,
    pub(super) new_bytes: u64,
    pub(super) old_identity: SourceIdentity,
    pub(super) new_identity: SourceIdentity,
    pub(super) replacement_source: String,
}

impl EditionMigrationChange {
    pub fn path(&self) -> &str {
        &self.path
    }
    pub const fn insertion_byte(&self) -> u64 {
        self.insertion_byte
    }
    pub fn inserted_bytes(&self) -> &str {
        &self.inserted_bytes
    }
    pub const fn old_bytes(&self) -> u64 {
        self.old_bytes
    }
    pub const fn new_bytes(&self) -> u64 {
        self.new_bytes
    }
    pub const fn old_identity(&self) -> SourceIdentity {
        self.old_identity
    }
    pub const fn new_identity(&self) -> SourceIdentity {
        self.new_identity
    }
    pub fn replacement_source(&self) -> &str {
        &self.replacement_source
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditionMigrationPlan {
    pub(super) old_edition: SourceEdition,
    pub(super) new_edition: SourceEdition,
    pub(super) old_revision: RevisionId,
    pub(super) new_revision: RevisionId,
    pub(super) old_tree_identity: SourceTreeIdentity,
    pub(super) new_tree_identity: SourceTreeIdentity,
    pub(super) old_bytes: u64,
    pub(super) new_bytes: u64,
    pub(super) changes: Vec<EditionMigrationChange>,
}

impl EditionMigrationPlan {
    pub const fn old_edition(&self) -> SourceEdition {
        self.old_edition
    }
    pub const fn new_edition(&self) -> SourceEdition {
        self.new_edition
    }
    pub const fn old_revision(&self) -> RevisionId {
        self.old_revision
    }
    pub const fn new_revision(&self) -> RevisionId {
        self.new_revision
    }
    pub const fn old_tree_identity(&self) -> SourceTreeIdentity {
        self.old_tree_identity
    }
    pub const fn new_tree_identity(&self) -> SourceTreeIdentity {
        self.new_tree_identity
    }
    pub const fn old_bytes(&self) -> u64 {
        self.old_bytes
    }
    pub const fn new_bytes(&self) -> u64 {
        self.new_bytes
    }
    pub fn changes(&self) -> &[EditionMigrationChange] {
        &self.changes
    }
    pub fn is_idempotent(&self) -> bool {
        self.changes.is_empty()
    }
}
