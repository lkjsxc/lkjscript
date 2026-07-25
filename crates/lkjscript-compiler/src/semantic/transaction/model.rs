use std::path::PathBuf;

use crate::semantic::schema::{ChangedSource, IdentityRelation};
use crate::source::ValidatedSourceTree;

pub(crate) enum ResolvedOperation {
    Rename {
        key: String,
        old_name: String,
        new_name: String,
        declaration_node: u32,
    },
    Replace {
        key: String,
        node: u32,
        path: Vec<usize>,
        replacement: crate::semantic::schema::Expression,
    },
}

pub(crate) struct StagedTransaction {
    pub tree: ValidatedSourceTree,
    pub sources: Vec<StagedSource>,
    pub changes: Vec<ChangedSource>,
    pub identities: Vec<IdentityRelation>,
}

pub(crate) struct StagedSource {
    pub logical_path: String,
    pub host_path: PathBuf,
    pub old_bytes: Vec<u8>,
    pub new_bytes: Vec<u8>,
}
