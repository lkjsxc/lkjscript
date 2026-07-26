use std::path::{Path, PathBuf};

use crate::source::{
    format, validate::validate_logical_source_path, DeclarationKey, DeclarationSummary, NodeId,
    NodeSummary, RevisionId, SourceEdition, SourceFile, SourceIdentity, SourceOrigin,
    SourceTreeIdentity, StaleNodeId,
};

/// Opaque, immutable validated source authority for one explicit edition.
///
/// Source origins, declarations, and nodes are exposed in deterministic
/// canonical logical-path and stable-key order. Raw forms and the mutable
/// builder remain private. This type does not claim the complete Semantic
/// Source schema, transaction protocol, JSON transport, or typed holes.
pub(crate) struct ValidatedSourceParts {
    pub(crate) edition: SourceEdition,
    pub(crate) identity: SourceTreeIdentity,
    pub(crate) revision: RevisionId,
    pub(crate) root: PathBuf,
    pub(crate) root_origin: SourceOrigin,
    pub(crate) files: Vec<SourceFile>,
    pub(crate) origins: Vec<SourceOrigin>,
    pub(crate) declarations: Vec<DeclarationSummary>,
    pub(crate) nodes: Vec<NodeSummary>,
}

#[derive(Clone, Debug)]
pub struct ValidatedSourceTree {
    edition: SourceEdition,
    identity: SourceTreeIdentity,
    revision: RevisionId,
    root: PathBuf,
    root_origin: SourceOrigin,
    files: Vec<SourceFile>,
    origins: Vec<SourceOrigin>,
    declarations: Vec<DeclarationSummary>,
    nodes: Vec<NodeSummary>,
}

impl ValidatedSourceTree {
    pub(crate) fn from_authority(parts: ValidatedSourceParts) -> Self {
        Self {
            edition: parts.edition,
            identity: parts.identity,
            revision: parts.revision,
            root: parts.root,
            root_origin: parts.root_origin,
            files: parts.files,
            origins: parts.origins,
            declarations: parts.declarations,
            nodes: parts.nodes,
        }
    }

    pub const fn edition(&self) -> SourceEdition {
        self.edition
    }

    pub const fn identity(&self) -> SourceTreeIdentity {
        self.identity
    }

    pub const fn revision(&self) -> RevisionId {
        self.revision
    }

    pub fn source_identity(&self, logical_path: &str) -> Option<SourceIdentity> {
        let origin = validate_logical_source_path(logical_path).ok()?;
        self.files
            .iter()
            .find(|file| file.origin.logical_path == origin.logical_path)
            .map(|file| file.identity)
    }

    pub fn root_origin(&self) -> &SourceOrigin {
        &self.root_origin
    }

    pub fn source_origins(&self) -> &[SourceOrigin] {
        &self.origins
    }

    pub fn declarations(&self) -> &[DeclarationSummary] {
        &self.declarations
    }

    pub fn declaration(&self, key: &DeclarationKey) -> Option<&DeclarationSummary> {
        self.declarations
            .iter()
            .find(|declaration| declaration.key() == key)
    }

    pub fn nodes(&self) -> &[NodeSummary] {
        &self.nodes
    }

    pub fn node(&self, id: NodeId) -> std::result::Result<Option<&NodeSummary>, StaleNodeId> {
        if id.revision != self.revision {
            return Err(StaleNodeId {
                expected: self.revision,
                actual: id.revision,
            });
        }
        Ok(self
            .nodes
            .get(id.index as usize)
            .filter(|node| node.id == id))
    }

    /// Format the source unit at `logical_path` from structural nodes.
    pub fn format_source(&self, logical_path: &str) -> Option<String> {
        let origin = validate_logical_source_path(logical_path).ok()?;
        self.files
            .iter()
            .find(|file| file.origin.logical_path == origin.logical_path)
            .map(format::format_file)
    }

    /// Format a single-unit validated tree.
    pub fn format_single_source(&self) -> Option<String> {
        if self.files.len() != 1 {
            return None;
        }
        self.files.first().map(format::format_file)
    }

    pub(crate) fn files(&self) -> &[SourceFile] {
        &self.files
    }

    pub(crate) fn root_path(&self) -> &Path {
        &self.root
    }
}
