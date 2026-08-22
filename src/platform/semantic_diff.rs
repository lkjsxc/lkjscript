//! Stable-identity semantic diff over exact accepted revisions.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::repository::SemanticRepository;
use super::semantic_digest::{RootObjectDigest, SemanticDiffDigest};
use super::semantic_id::RevisionId;
use super::semantic_query::{OwnerKind, OwnerSummary, QueryBudget, SemanticQueryIndex};
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::collections::BTreeSet;

pub const SEMANTIC_DIFF_CONTRACT_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticChangeClass {
    Added,
    Removed,
    Renamed,
    Moved,
    Modified,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticOwnerState {
    pub kind: OwnerKind,
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_id: Option<super::semantic_id::ModuleId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

impl From<&OwnerSummary> for SemanticOwnerState {
    fn from(owner: &OwnerSummary) -> Self {
        Self {
            kind: owner.kind,
            id: owner.id.clone(),
            name: owner.name.clone(),
            module_id: owner.module_id,
            parent_id: owner.parent_id.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticOwnerChange {
    pub id: String,
    pub classes: Vec<SemanticChangeClass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<SemanticOwnerState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<SemanticOwnerState>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticDiffStatus {
    NoChange,
    Changed,
    Truncated,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticDiffReport {
    pub contract_version: u16,
    pub base_revision: RevisionId,
    pub result_revision: RevisionId,
    pub status: SemanticDiffStatus,
    pub semantic_diff: SemanticDiffDigest,
    pub total_changes: usize,
    pub returned_changes: usize,
    pub work: usize,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
    pub changes: Vec<SemanticOwnerChange>,
}

pub fn semantic_diff_digest(
    base: RootObjectDigest,
    result: RootObjectDigest,
) -> SemanticDiffDigest {
    let mut bytes = Vec::with_capacity(64);
    bytes.extend_from_slice(&base.bytes());
    bytes.extend_from_slice(&result.bytes());
    SemanticDiffDigest::of(&bytes)
}

pub fn diff_revisions(
    repository: &SemanticRepository,
    base_revision: RevisionId,
    result_revision: RevisionId,
    offset: usize,
    budget: QueryBudget,
) -> Result<SemanticDiffReport, Diagnostic> {
    let budget = budget.validate()?;
    let base = repository.reconstruct_revision(base_revision)?;
    let result = repository.reconstruct_revision(result_revision)?;
    if base.record.core.repository_id != result.record.core.repository_id
        || base.root.package_id != result.root.package_id
    {
        return Err(diff_error(
            DiagnosticClass::Source,
            "semantic_diff_foreign_authority",
            "semantic diff revisions do not belong to one repository and package authority",
        ));
    }
    let digest = semantic_diff_digest(base.record.core.root, result.record.core.root);
    let base_owners = SemanticQueryIndex::from_snapshot(base)?.owner_states()?;
    let result_owners = SemanticQueryIndex::from_snapshot(result)?.owner_states()?;
    let ids = base_owners
        .keys()
        .chain(result_owners.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    if ids.len() > budget.maximum_work {
        return Err(diff_error(
            DiagnosticClass::Resource,
            "semantic_diff_work_exhausted",
            "semantic diff exhausted its declared work budget",
        ));
    }

    let mut all_changes = Vec::new();
    for id in ids {
        let before = base_owners.get(&id);
        let after = result_owners.get(&id);
        let classes = classify_change(before, after);
        if !classes.is_empty() {
            all_changes.push(SemanticOwnerChange {
                id,
                classes,
                before: before.map(|(owner, _)| owner.into()),
                after: after.map(|(owner, _)| owner.into()),
            });
        }
    }
    if offset > all_changes.len() {
        return Err(diff_error(
            DiagnosticClass::Source,
            "semantic_diff_offset",
            "semantic diff offset exceeds the exact result length",
        ));
    }

    let total_changes = all_changes.len();
    let mut changes = Vec::new();
    let mut response_bytes = 0usize;
    for change in all_changes.into_iter().skip(offset) {
        if changes.len() >= budget.maximum_items {
            break;
        }
        let item_bytes = serde_json::to_vec(&change)
            .map_err(|error| diff_json_error("semantic_diff_encode", error))?
            .len();
        let next_bytes = response_bytes.checked_add(item_bytes).ok_or_else(|| {
            diff_error(
                DiagnosticClass::Resource,
                "semantic_diff_byte_overflow",
                "semantic diff response byte accounting overflowed",
            )
        })?;
        if next_bytes > budget.maximum_bytes {
            break;
        }
        response_bytes = next_bytes;
        changes.push(change);
    }
    let next = offset.checked_add(changes.len()).ok_or_else(|| {
        diff_error(
            DiagnosticClass::Resource,
            "semantic_diff_offset_overflow",
            "semantic diff continuation offset overflowed",
        )
    })?;
    let truncated = next < total_changes;
    let status = if truncated {
        SemanticDiffStatus::Truncated
    } else if total_changes == 0 {
        SemanticDiffStatus::NoChange
    } else {
        SemanticDiffStatus::Changed
    };
    Ok(SemanticDiffReport {
        contract_version: SEMANTIC_DIFF_CONTRACT_VERSION,
        base_revision,
        result_revision,
        status,
        semantic_diff: digest,
        total_changes,
        returned_changes: changes.len(),
        work: base_owners.len().saturating_add(result_owners.len()),
        truncated,
        next_offset: truncated.then_some(next),
        changes,
    })
}

fn classify_change(
    before: Option<&(OwnerSummary, JsonValue)>,
    after: Option<&(OwnerSummary, JsonValue)>,
) -> Vec<SemanticChangeClass> {
    match (before, after) {
        (None, Some(_)) => vec![SemanticChangeClass::Added],
        (Some(_), None) => vec![SemanticChangeClass::Removed],
        (Some((before_owner, before_semantic)), Some((after_owner, after_semantic))) => {
            let mut classes = Vec::new();
            if before_owner.name != after_owner.name {
                classes.push(SemanticChangeClass::Renamed);
            }
            if before_owner.module_id != after_owner.module_id
                || before_owner.parent_id != after_owner.parent_id
            {
                classes.push(SemanticChangeClass::Moved);
            }
            if before_owner.kind != after_owner.kind
                || normalized_semantic(before_owner.kind, before_semantic)
                    != normalized_semantic(after_owner.kind, after_semantic)
            {
                classes.push(SemanticChangeClass::Modified);
            }
            classes
        }
        (None, None) => Vec::new(),
    }
}

fn normalized_semantic(kind: OwnerKind, value: &JsonValue) -> JsonValue {
    let mut value = value.clone();
    match kind {
        OwnerKind::Record
        | OwnerKind::Variant
        | OwnerKind::Interface
        | OwnerKind::External
        | OwnerKind::PureFunction
        | OwnerKind::TaskFunction
        | OwnerKind::Constant
        | OwnerKind::Component
        | OwnerKind::Test => {
            if let Some(data) = value.get_mut("data").and_then(JsonValue::as_object_mut) {
                data.remove("name");
            }
        }
        OwnerKind::Field
        | OwnerKind::Case
        | OwnerKind::Operation
        | OwnerKind::Parameter
        | OwnerKind::Binding
        | OwnerKind::Requirement
        | OwnerKind::Port
        | OwnerKind::Target => {
            if let Some(object) = value.as_object_mut() {
                object.remove("name");
            }
        }
        _ => {}
    }
    value
}

fn diff_json_error(code: &str, error: serde_json::Error) -> Diagnostic {
    diff_error(
        DiagnosticClass::Infrastructure,
        code,
        format!("semantic diff JSON encoding failed: {error}"),
    )
}

fn diff_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
