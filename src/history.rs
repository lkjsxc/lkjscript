//! Canonical immutable development-revision records.

use crate::codec::{CodecError, CodecErrorKind, Reader, Writer};
use crate::diff::{ChangeKind, SemanticDiff};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::{ChangeDigest, NodeId, Revision, RevisionRecordDigest, SnapshotHash, WorkspaceId};
use crate::schema::NodeKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const REVISION_RECORD_VERSION: u16 = 1;
pub const REVISION_RECORD_MAGIC: [u8; 8] = *b"LKJREC01";
pub const MAXIMUM_REVISION_RECORD_BYTES: usize = 16 * 1024 * 1024;
const DIGEST_DOMAIN: &str = "lkjscript.development-revision-record.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RevisionPublicationOutcome {
    Genesis,
    Accepted,
    Restoration,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionRecord {
    pub version: u16,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_revision: Option<Revision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_snapshot: Option<SnapshotHash>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_record: Option<RevisionRecordDigest>,
    pub result_snapshot: SnapshotHash,
    pub accepted_change_set: ChangeDigest,
    pub semantic_diff: ChangeDigest,
    pub change_count: u64,
    pub created: Vec<NodeId>,
    pub deleted: Vec<NodeId>,
    pub modified: Vec<NodeId>,
    pub function_bodies_changed: Vec<NodeId>,
    pub target_definitions_changed: Vec<NodeId>,
    pub affected_targets: Vec<NodeId>,
    pub outcome: RevisionPublicationOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RestorationReceipt {
    pub contract_version: u16,
    pub workspace: WorkspaceId,
    pub source_revision: Revision,
    pub base_revision: Revision,
    pub revision: Revision,
    pub snapshot: SnapshotHash,
    pub published: bool,
    pub semantic_diff: SemanticDiff,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_record: Option<RevisionRecordInspection>,
}

impl RevisionRecord {
    pub fn genesis(snapshot: &Snapshot) -> Self {
        let empty = empty_change_digest(snapshot);
        Self {
            version: REVISION_RECORD_VERSION,
            workspace: snapshot.workspace(),
            revision: snapshot.revision(),
            parent_revision: None,
            parent_snapshot: None,
            parent_record: None,
            result_snapshot: snapshot.hash(),
            accepted_change_set: empty,
            semantic_diff: empty,
            change_count: 0,
            created: vec![snapshot.root()],
            deleted: Vec::new(),
            modified: Vec::new(),
            function_bodies_changed: Vec::new(),
            target_definitions_changed: Vec::new(),
            affected_targets: Vec::new(),
            outcome: RevisionPublicationOutcome::Genesis,
        }
    }

    pub fn transition(
        before: &Snapshot,
        after: &Snapshot,
        parent_record: RevisionRecordDigest,
        accepted_change_set: ChangeDigest,
        outcome: RevisionPublicationOutcome,
    ) -> Result<Self> {
        if before.workspace() != after.workspace()
            || before.revision().next() != Some(after.revision())
        {
            return Err(LkError::new(
                ErrorCode::RevisionConflict,
                "revision record transition is not one adjacent project change",
            )
            .for_workspace(after.workspace())
            .at_revision(after.revision()));
        }
        let semantic = crate::diff::between(before, after);
        let mut created = BTreeSet::new();
        let mut deleted = BTreeSet::new();
        let mut modified = BTreeSet::new();
        let mut function_bodies_changed = BTreeSet::new();
        let mut target_definitions_changed = BTreeSet::new();
        for change in &semantic.changes {
            if change.node.is_function_local() {
                continue;
            }
            match &change.kind {
                ChangeKind::Created { kind } => {
                    created.insert(change.node);
                    if *kind == NodeKind::BuildTarget {
                        target_definitions_changed.insert(change.node);
                    }
                }
                ChangeKind::Deleted { kind } => {
                    deleted.insert(change.node);
                    if *kind == NodeKind::BuildTarget {
                        target_definitions_changed.insert(change.node);
                    }
                }
                ChangeKind::FunctionBodyChanged { .. } => {
                    modified.insert(change.node);
                    function_bodies_changed.insert(change.node);
                }
                ChangeKind::BuildTargetChanged { .. } => {
                    modified.insert(change.node);
                    target_definitions_changed.insert(change.node);
                }
                _ => {
                    modified.insert(change.node);
                    if matches!(
                        before
                            .node(change.node)
                            .or_else(|_| after.node(change.node)),
                        Ok(crate::schema::Node::BuildTarget { .. })
                    ) {
                        target_definitions_changed.insert(change.node);
                    }
                }
            }
        }
        for id in created.iter().chain(&deleted) {
            modified.remove(id);
        }
        let affected_targets = affected_targets(before, after)?;
        Ok(Self {
            version: REVISION_RECORD_VERSION,
            workspace: after.workspace(),
            revision: after.revision(),
            parent_revision: Some(before.revision()),
            parent_snapshot: Some(before.hash()),
            parent_record: Some(parent_record),
            result_snapshot: after.hash(),
            accepted_change_set,
            semantic_diff: semantic.digest,
            change_count: semantic.change_count(),
            created: created.into_iter().collect(),
            deleted: deleted.into_iter().collect(),
            modified: modified.into_iter().collect(),
            function_bodies_changed: function_bodies_changed.into_iter().collect(),
            target_definitions_changed: target_definitions_changed.into_iter().collect(),
            affected_targets,
            outcome,
        })
    }

    pub fn validate_against(
        &self,
        before: Option<&Snapshot>,
        after: &Snapshot,
        expected_parent_record: Option<RevisionRecordDigest>,
    ) -> Result<()> {
        if self.version != REVISION_RECORD_VERSION
            || self.workspace != after.workspace()
            || self.revision != after.revision()
            || self.result_snapshot != after.hash()
            || self.parent_record != expected_parent_record
        {
            return Err(record_corrupt(
                self,
                "revision record identity or result is inconsistent",
            ));
        }
        match before {
            None => {
                if self.revision != Revision::INITIAL
                    || self.parent_revision.is_some()
                    || self.parent_snapshot.is_some()
                    || self.outcome != RevisionPublicationOutcome::Genesis
                {
                    return Err(record_corrupt(
                        self,
                        "genesis revision record is not canonical",
                    ));
                }
                if *self != Self::genesis(after) {
                    return Err(record_corrupt(
                        self,
                        "genesis revision record facts disagree with its snapshot",
                    ));
                }
            }
            Some(before) => {
                if self.parent_revision != Some(before.revision())
                    || self.parent_snapshot != Some(before.hash())
                    || self.outcome == RevisionPublicationOutcome::Genesis
                {
                    return Err(record_corrupt(
                        self,
                        "revision record parent is inconsistent",
                    ));
                }
                let semantic = crate::diff::between(before, after);
                if self.semantic_diff != semantic.digest
                    || self.change_count != semantic.change_count()
                {
                    return Err(record_corrupt(
                        self,
                        "revision record semantic diff is inconsistent",
                    ));
                }
                let expected = Self::transition(
                    before,
                    after,
                    expected_parent_record.ok_or_else(|| {
                        record_corrupt(self, "non-genesis revision record has no parent record")
                    })?,
                    self.accepted_change_set,
                    self.outcome,
                )?;
                if *self != expected {
                    return Err(record_corrupt(
                        self,
                        "revision record change facts disagree with its semantic diff",
                    ));
                }
            }
        }
        Ok(())
    }
}

fn affected_targets(before: &Snapshot, after: &Snapshot) -> Result<Vec<NodeId>> {
    let ids = crate::target::summaries(before)
        .into_iter()
        .map(|target| target.target)
        .chain(
            crate::target::summaries(after)
                .into_iter()
                .map(|target| target.target),
        )
        .collect::<BTreeSet<_>>();
    let mut affected = Vec::new();
    for id in ids {
        let old = crate::target::prepare(before, id).ok();
        let new = crate::target::prepare(after, id).ok();
        if old.as_ref().map(crate::target::PreparedTarget::bytes)
            != new.as_ref().map(crate::target::PreparedTarget::bytes)
        {
            affected.push(id);
        }
    }
    Ok(affected)
}

fn empty_change_digest(snapshot: &Snapshot) -> ChangeDigest {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.genesis-change-set.v1");
    hasher.update(&snapshot.workspace().as_bytes());
    hasher.update(&snapshot.hash().as_bytes());
    ChangeDigest::from_bytes(*hasher.finalize().as_bytes())
}

pub fn digest(record: &RevisionRecord) -> Result<RevisionRecordDigest> {
    let body = canonical_body(record)?;
    let mut hasher = blake3::Hasher::new_derive_key(DIGEST_DOMAIN);
    hasher.update(&body);
    Ok(RevisionRecordDigest::from_bytes(
        *hasher.finalize().as_bytes(),
    ))
}

pub fn encode(record: &RevisionRecord) -> Result<Vec<u8>> {
    let body = canonical_body(record)?;
    let body_len = u64::try_from(body.len()).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "revision record length overflows canonical representation",
        )
    })?;
    let record_digest = digest(record)?;
    let mut writer = Writer::with_capacity(
        REVISION_RECORD_MAGIC.len() + 2 + 8 + body.len() + RevisionRecordDigest::BYTE_LEN,
    );
    writer.fixed(&REVISION_RECORD_MAGIC);
    writer.u16(REVISION_RECORD_VERSION);
    writer.u64(body_len);
    writer.fixed(&body);
    writer.fixed(&record_digest.as_bytes());
    let bytes = writer.finish();
    if bytes.len() > MAXIMUM_REVISION_RECORD_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "revision record exceeds byte policy",
        ));
    }
    Ok(bytes)
}

pub fn decode(bytes: &[u8]) -> Result<(RevisionRecord, RevisionRecordDigest)> {
    if bytes.len() > MAXIMUM_REVISION_RECORD_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "revision record exceeds byte policy",
        ));
    }
    let mut reader = Reader::new(bytes);
    if reader
        .fixed(REVISION_RECORD_MAGIC.len())
        .map_err(record_codec)?
        != REVISION_RECORD_MAGIC
    {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "revision record magic is invalid",
        ));
    }
    if reader.u16().map_err(record_codec)? != REVISION_RECORD_VERSION {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "revision record version is unsupported",
        ));
    }
    let length = usize::try_from(reader.u64().map_err(record_codec)?).map_err(|_| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            "revision record body length overflows host indexes",
        )
    })?;
    if length > MAXIMUM_REVISION_RECORD_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "revision record body exceeds byte policy",
        ));
    }
    let body = reader.fixed(length).map_err(record_codec)?;
    let digest_bytes = reader
        .fixed(RevisionRecordDigest::BYTE_LEN)
        .map_err(record_codec)?;
    reader.finish().map_err(record_codec)?;
    let record: RevisionRecord = serde_json::from_slice(body).map_err(|error| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            format!("revision record JSON is malformed: {error}"),
        )
    })?;
    if canonical_body(&record)? != body {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "revision record JSON is not canonical",
        ));
    }
    let mut expected = [0_u8; RevisionRecordDigest::BYTE_LEN];
    expected.copy_from_slice(digest_bytes);
    let expected = RevisionRecordDigest::from_bytes(expected);
    if digest(&record)? != expected {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "revision record digest is invalid",
        ));
    }
    Ok((record, expected))
}

fn canonical_body(record: &RevisionRecord) -> Result<Vec<u8>> {
    serde_json::to_vec(record).map_err(|error| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            format!("cannot encode canonical revision record: {error}"),
        )
    })
}

fn record_codec(error: CodecError) -> LkError {
    LkError::new(
        if error.kind == CodecErrorKind::PolicyExceeded {
            ErrorCode::PolicyExceeded
        } else {
            ErrorCode::ArtifactCorrupt
        },
        format!("canonical revision-record decoding failed: {error}"),
    )
}

fn record_corrupt(record: &RevisionRecord, message: &str) -> LkError {
    LkError::new(ErrorCode::ArtifactCorrupt, message)
        .for_workspace(record.workspace)
        .at_revision(record.revision)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionRecordInspection {
    pub digest: RevisionRecordDigest,
    pub record: RevisionRecord,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionRecordSummary {
    pub digest: RevisionRecordDigest,
    pub version: u16,
    pub revision: Revision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_revision: Option<Revision>,
    pub result_snapshot: SnapshotHash,
    pub accepted_change_set: ChangeDigest,
    pub semantic_diff: ChangeDigest,
    pub change_count: u64,
    pub created_count: u64,
    pub deleted_count: u64,
    pub modified_count: u64,
    pub function_body_change_count: u64,
    pub target_definition_change_count: u64,
    pub affected_target_count: u64,
    pub outcome: RevisionPublicationOutcome,
}

impl From<&RevisionRecordInspection> for RevisionRecordSummary {
    fn from(value: &RevisionRecordInspection) -> Self {
        let record = &value.record;
        Self {
            digest: value.digest,
            version: record.version,
            revision: record.revision,
            parent_revision: record.parent_revision,
            result_snapshot: record.result_snapshot,
            accepted_change_set: record.accepted_change_set,
            semantic_diff: record.semantic_diff,
            change_count: record.change_count,
            created_count: record.created.len() as u64,
            deleted_count: record.deleted.len() as u64,
            modified_count: record.modified.len() as u64,
            function_body_change_count: record.function_bodies_changed.len() as u64,
            target_definition_change_count: record.target_definitions_changed.len() as u64,
            affected_target_count: record.affected_targets.len() as u64,
            outcome: record.outcome,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryPage {
    pub workspace: WorkspaceId,
    pub head: Revision,
    pub records: Vec<RevisionRecordSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_before: Option<Revision>,
}

pub fn semantic_diff(before: &Snapshot, after: &Snapshot) -> SemanticDiff {
    crate::diff::between(before, after)
}
