use crate::source::{
    DeclarationKey, NodeId, RevisionId, SourceEdition, SourceIdentity, SourceTreeIdentity,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditionMigrationChange {
    pub(super) path: String,
    pub(super) insertion_byte: u64,
    pub(super) inserted_bytes: String,
    pub(super) conversion_count: u64,
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
    pub const fn conversion_count(&self) -> u64 {
        self.conversion_count
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
pub struct EditionMigrationDeclarationIdentity {
    pub(super) old_key: DeclarationKey,
    pub(super) new_key: DeclarationKey,
    pub(super) old_node: NodeId,
    pub(super) new_node: NodeId,
}

impl EditionMigrationDeclarationIdentity {
    pub fn old_key(&self) -> &DeclarationKey {
        &self.old_key
    }
    pub fn new_key(&self) -> &DeclarationKey {
        &self.new_key
    }
    pub const fn old_node(&self) -> NodeId {
        self.old_node
    }
    pub const fn new_node(&self) -> NodeId {
        self.new_node
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EditionMigrationNodeIdentity {
    pub(super) old: NodeId,
    pub(super) new: NodeId,
}

impl EditionMigrationNodeIdentity {
    pub const fn old_node(self) -> NodeId {
        self.old
    }
    pub const fn new_node(self) -> NodeId {
        self.new
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
    pub(super) declarations: Vec<EditionMigrationDeclarationIdentity>,
    pub(super) nodes: Vec<EditionMigrationNodeIdentity>,
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
    pub fn declarations(&self) -> &[EditionMigrationDeclarationIdentity] {
        &self.declarations
    }
    pub fn nodes(&self) -> &[EditionMigrationNodeIdentity] {
        &self.nodes
    }
    pub fn is_idempotent(&self) -> bool {
        self.changes.is_empty()
    }
}
