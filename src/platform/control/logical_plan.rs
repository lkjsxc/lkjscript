//! Canonical review-bound logical change-plan token and compact-record codec.

use super::change::ChangeRequestCommitment;
use super::{CompactRecord, parse_records, render_record};
use crate::platform::change::{
    AuthoredAllocation, ChangeBudget, ImpactReason, ImpactReasonKind, LogicalChangePlanEvidence,
};
use crate::platform::contract::{capabilities_snapshot, registry_snapshot};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    ChangeDigest, DependencyObjectDigest, DependencyRecord, EncodedOwnerKey, ExactOwnerKey,
    IdentityKind, Name, OwnerKey, OwnerKind, OwnerObjectDigest, PackageId, PackageRevisionDigest,
    RelationEdge, RelationEndpoint, RelationKind, RetirementObjectDigest, RetirementRecord,
    SemanticStateDigest, TypeObjectDigest, encode_dependency, encode_retirement,
};
use crate::platform::publication::{
    DependencyDiffEntry, OwnerChangeClass, OwnerDiffEntry, PreparedAuthoredPublication,
    RetirementDiffEntry, SemanticDiffBody, SemanticDiffDigest, SummaryDimensions,
    TransactionDigest,
};
use crate::platform::semantic_id::{RepositoryId, RevisionId, encode_hex};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt;
use std::io::{BufRead, Read};
use std::str::FromStr;

pub const LOGICAL_CHANGE_PLAN_CONTRACT_IDENTITY: &str = "lkjscript-logical-change-plan-2";
pub const LOGICAL_CHANGE_PLAN_CONTRACT_VERSION: u16 = 2;
pub const PREPARED_CHANGE_PLAN_COMMITMENT_DOMAIN: &str = "lkjscript.logical-change-plan.v2";
const INTERNAL_PLAN_BINDING_LABEL: &[u8] = b"lkjscript.logical-change-plan.internal-registry\0";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LogicalPlanRecordDescriptor {
    pub name: &'static str,
    pub fields: &'static [&'static str],
}

macro_rules! plan_record {
    ($name:literal, [$($field:literal),* $(,)?]) => {
        LogicalPlanRecordDescriptor {
            name: $name,
            fields: &[$($field),*],
        }
    };
}

pub(crate) const LOGICAL_PLAN_RECORD_DESCRIPTORS: &[LogicalPlanRecordDescriptor] = &[
    plan_record!("logical-plan", ["product", "version"]),
    plan_record!("logical-plan.capabilities", ["digest"]),
    plan_record!(
        "logical-plan.authority",
        ["repository", "package", "base", "result", "semantic-state"]
    ),
    plan_record!("logical-plan.request", ["commitment"]),
    plan_record!(
        "logical-plan.budget-authored",
        [
            "operations",
            "preconditions",
            "allocated-identities",
            "type-nodes"
        ]
    ),
    plan_record!(
        "logical-plan.budget-canonical-edits",
        ["owners", "types", "dependencies", "retirements"]
    ),
    plan_record!(
        "logical-plan.budget-canonical-reads",
        [
            "point-reads",
            "map-pages",
            "map-entries",
            "catalog-lookups",
            "objects",
            "bytes",
            "decoded-records"
        ]
    ),
    plan_record!(
        "logical-plan.budget-canonical-map-update",
        ["pages", "bytes"]
    ),
    plan_record!(
        "logical-plan.budget-witness-reads",
        [
            "point-reads",
            "map-pages",
            "map-entries",
            "catalog-lookups",
            "objects",
            "bytes",
            "decoded-records"
        ]
    ),
    plan_record!(
        "logical-plan.budget-impact",
        [
            "affected-owners",
            "summary-owners",
            "summary-edits",
            "ownership-steps",
            "behavior-owners",
            "relation-edges",
            "relation-fanout"
        ]
    ),
    plan_record!(
        "logical-plan.budget-validation",
        [
            "owner-records",
            "ownership-entries",
            "type-objects",
            "expression-steps",
            "diagnostics"
        ]
    ),
    plan_record!(
        "logical-plan.budget-tests",
        [
            "selected",
            "dependencies-per-test",
            "ownership-steps",
            "owners-visited"
        ]
    ),
    plan_record!(
        "logical-plan.budget-witness-update",
        ["edits", "pages", "bytes"]
    ),
    plan_record!("logical-plan.budget-staging", ["objects", "bytes", "pages"]),
    plan_record!(
        "logical-plan.controls",
        [
            "idempotency-present",
            "idempotency",
            "intent-present",
            "intent"
        ]
    ),
    plan_record!("logical-plan.binding", ["transaction", "semantic-diff"]),
    plan_record!("logical-plan.allocation", ["domain", "ordinal", "owner"]),
    plan_record!(
        "logical-plan.owner",
        [
            "owner",
            "before-present",
            "before",
            "after-present",
            "after",
            "class-created",
            "class-deleted",
            "class-renamed",
            "class-moved",
            "class-visibility-changed",
            "class-presentation-only",
            "class-test-only",
            "class-private-implementation",
            "class-public-interface",
            "class-effect-or-capability",
            "class-relation-set",
            "class-semantic-payload-changed",
            "dimension-semantic-interface",
            "dimension-implementation",
            "dimension-type-digest",
            "dimension-effect",
            "dimension-capability",
            "dimension-relations",
            "dimension-presentation",
            "dimension-test",
            "dimension-validation-dependencies"
        ]
    ),
    plan_record!("logical-plan.type-addition", ["object"]),
    plan_record!(
        "logical-plan.dependency",
        [
            "package",
            "before-present",
            "before-object",
            "before-semantic-revision",
            "before-package-revision",
            "after-present",
            "after-object",
            "after-semantic-revision",
            "after-package-revision"
        ]
    ),
    plan_record!(
        "logical-plan.retirement",
        [
            "owner",
            "before-present",
            "before-object",
            "before-kind",
            "before-name-present",
            "before-name",
            "before-parent-present",
            "before-parent",
            "before-live-revision",
            "before-deletion-change",
            "after-present",
            "after-object",
            "after-kind",
            "after-name-present",
            "after-name",
            "after-parent-present",
            "after-parent",
            "after-live-revision",
            "after-deletion-change"
        ]
    ),
    plan_record!(
        "logical-plan.relation-removed",
        [
            "source-package",
            "source-owner-present",
            "source-owner",
            "kind",
            "target-package",
            "target-owner-present",
            "target-owner"
        ]
    ),
    plan_record!(
        "logical-plan.relation-added",
        [
            "source-package",
            "source-owner-present",
            "source-owner",
            "kind",
            "target-package",
            "target-owner-present",
            "target-owner"
        ]
    ),
    plan_record!("logical-plan.validation-structural", ["owner"]),
    plan_record!("logical-plan.validation-semantic", ["owner"]),
    plan_record!("logical-plan.test", ["owner"]),
    plan_record!(
        "logical-plan.reason",
        ["kind", "source", "target", "relation-present", "relation"]
    ),
    plan_record!(
        "logical-plan.counts",
        [
            "allocations",
            "owners",
            "types",
            "dependencies",
            "retirements",
            "relations-removed",
            "relations-added",
            "structural-owners",
            "semantic-owners",
            "tests",
            "reasons"
        ]
    ),
    plan_record!("logical-plan.digest", ["request", "prepared", "token"]),
];

const FIXED_LOGICAL_PLAN_RECORDS: u64 = 18;
const DEFAULT_ALLOCATIONS: u64 = 100_000;
const DEFAULT_OWNER_CHANGES: u64 = 100_000;
const DEFAULT_TYPE_ADDITIONS: u64 = 100_000;
const DEFAULT_DEPENDENCY_CHANGES: u64 = 10_000;
const DEFAULT_RETIREMENT_CHANGES: u64 = 100_000;
const DEFAULT_RELATION_CHANGES: u64 = 100_000;
const DEFAULT_STRUCTURAL_OWNERS: u64 = 100_000;
const DEFAULT_SEMANTIC_OWNERS: u64 = 10_000;
const DEFAULT_SELECTED_TESTS: u64 = 10_000;
const DEFAULT_IMPACT_REASONS: u64 = 110_000;

/// Fixed plan-file admission covering every logical record possible under the current default
/// change admissions. Requests may declare larger engine budgets, but their actual logical plan
/// must still fit this independent complete-review boundary.
pub const MAXIMUM_LOGICAL_PLAN_RECORDS: u64 = FIXED_LOGICAL_PLAN_RECORDS
    + DEFAULT_ALLOCATIONS
    + DEFAULT_OWNER_CHANGES
    + DEFAULT_TYPE_ADDITIONS
    + DEFAULT_DEPENDENCY_CHANGES
    + DEFAULT_RETIREMENT_CHANGES
    + DEFAULT_RELATION_CHANGES
    + DEFAULT_STRUCTURAL_OWNERS
    + DEFAULT_SEMANTIC_OWNERS
    + DEFAULT_SELECTED_TESTS
    + DEFAULT_IMPACT_REASONS;

// These are the exact canonical byte maxima, including newline framing, for current default
// admissions and current typed text forms. The fixed total includes every singleton/budget record
// and the maximally escaped 4,096-byte intent. A unit test renders each maximum and requires exact
// equality, so vocabulary or field-bound growth must deliberately revise this contract.
const MAXIMUM_FIXED_RECORDS_BYTES: u64 = 27_368;
const MAXIMUM_ALLOCATION_RECORD_BYTES: u64 = 111;
const MAXIMUM_OWNER_RECORD_BYTES: u64 = 857;
const MAXIMUM_TYPE_RECORD_BYTES: u64 = 111;
const MAXIMUM_DEPENDENCY_RECORD_BYTES: u64 = 699;
const MAXIMUM_RETIREMENT_RECORD_BYTES: u64 = 1_225;
const MAXIMUM_RELATION_RECORD_BYTES: u64 = 331;
const MAXIMUM_SELECTION_RECORD_BYTES: u64 = 85;
const MAXIMUM_REASON_RECORD_BYTES: u64 = 206;

pub const MAXIMUM_LOGICAL_PLAN_BYTES: u64 = MAXIMUM_FIXED_RECORDS_BYTES
    + DEFAULT_ALLOCATIONS * MAXIMUM_ALLOCATION_RECORD_BYTES
    + DEFAULT_OWNER_CHANGES * MAXIMUM_OWNER_RECORD_BYTES
    + DEFAULT_TYPE_ADDITIONS * MAXIMUM_TYPE_RECORD_BYTES
    + DEFAULT_DEPENDENCY_CHANGES * MAXIMUM_DEPENDENCY_RECORD_BYTES
    + DEFAULT_RETIREMENT_CHANGES * MAXIMUM_RETIREMENT_RECORD_BYTES
    + DEFAULT_RELATION_CHANGES * MAXIMUM_RELATION_RECORD_BYTES
    + (DEFAULT_STRUCTURAL_OWNERS + DEFAULT_SEMANTIC_OWNERS + DEFAULT_SELECTED_TESTS)
        * MAXIMUM_SELECTION_RECORD_BYTES
    + DEFAULT_IMPACT_REASONS * MAXIMUM_REASON_RECORD_BYTES;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct PreparedPlanCommitment([u8; 32]);

impl PreparedPlanCommitment {
    pub(crate) const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for PreparedPlanCommitment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("prepared_")?;
        formatter.write_str(&encode_hex(&self.0))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ChangePlanToken {
    pub request: ChangeRequestCommitment,
    pub prepared: PreparedPlanCommitment,
}

impl fmt::Display for ChangePlanToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("plan_")?;
        formatter.write_str(&encode_hex(&self.request.bytes()))?;
        formatter.write_str(&encode_hex(&self.prepared.bytes()))
    }
}

impl FromStr for ChangePlanToken {
    type Err = Diagnostic;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let encoded = value.strip_prefix("plan_").ok_or_else(|| {
            plan_source_error(
                "change_plan_domain",
                "reviewed plan token must start with 'plan_'",
            )
        })?;
        if encoded.len() != 128 {
            return Err(plan_source_error(
                "change_plan_length",
                "reviewed plan token must contain 128 lowercase hexadecimal characters",
            ));
        }
        let request = decode_hex_32(&encoded[..64])?;
        let prepared = decode_hex_32(&encoded[64..])?;
        Ok(Self {
            request: ChangeRequestCommitment::from_bytes(request),
            prepared: PreparedPlanCommitment::from_bytes(prepared),
        })
    }
}

/// One transport-neutral view over the exact prepared publication and authored-only review facts.
pub(crate) struct LogicalChangePlan<'a> {
    request: ChangeRequestCommitment,
    publication: &'a PreparedAuthoredPublication,
    owners: &'a [OwnerDiffEntry],
    types: &'a [TypeObjectDigest],
    dependencies: &'a [DependencyDiffEntry],
    retirements: &'a [RetirementDiffEntry],
    base: RevisionId,
}

impl<'a> LogicalChangePlan<'a> {
    pub(crate) fn new(
        request: ChangeRequestCommitment,
        publication: &'a PreparedAuthoredPublication,
    ) -> Result<Self, Diagnostic> {
        let prepared = &publication.publication;
        let [base] = prepared.receipt.bases.as_slice() else {
            return Err(plan_corrupt(
                "change_logical_plan_base",
                "prepared authored change does not bind one exact base",
            ));
        };
        let SemanticDiffBody::Change {
            base: diff_base,
            result_root,
            owners,
            type_additions,
            dependencies,
            retirements,
            ..
        } = &prepared.semantic_diff.body
        else {
            return Err(plan_corrupt(
                "change_logical_plan_diff",
                "prepared authored change has a non-change semantic diff",
            ));
        };
        if prepared.semantic_diff.repository_id != prepared.head.repository_id
            || prepared.authority.semantic.root.repository_id != prepared.head.repository_id
            || *diff_base != *base
            || *result_root != prepared.authority.semantic.digest
            || prepared.receipt.result != prepared.head.revision
            || prepared.accepted.semantic_state != prepared.authority.semantic.state
            || prepared.receipt.repository_id != prepared.head.repository_id
            || prepared.receipt.transaction != prepared.transaction_digest
            || prepared.receipt.semantic_diff != prepared.semantic_diff_digest
        {
            return Err(plan_corrupt(
                "change_logical_plan_authority",
                "prepared publication, semantic diff, and candidate authority disagree",
            ));
        }
        validate_evidence(
            &publication.logical_plan,
            dependencies,
            retirements,
            &prepared.receipt,
        )?;
        Ok(Self {
            request,
            publication,
            owners,
            types: type_additions,
            dependencies,
            retirements,
            base: *base,
        })
    }

    fn prepared(&self) -> &PreparedAuthoredPublication {
        self.publication
    }

    fn evidence(&self) -> &LogicalChangePlanEvidence {
        &self.publication.logical_plan
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LogicalPlanEncoding {
    pub token: ChangePlanToken,
    pub bytes: u64,
    pub records: u64,
}

/// Renders and hashes one canonical logical plan in a single traversal. `emit` may discard bytes,
/// stream them to a private file stage, or collect a small test fixture.
pub(crate) fn encode_logical_change_plan<F>(
    plan: &LogicalChangePlan<'_>,
    emit: F,
) -> Result<LogicalPlanEncoding, Diagnostic>
where
    F: FnMut(&[u8]) -> Result<(), Diagnostic>,
{
    encode_logical_change_plan_admitted(
        plan,
        emit,
        MAXIMUM_LOGICAL_PLAN_BYTES,
        MAXIMUM_LOGICAL_PLAN_RECORDS,
    )
}

#[cfg(test)]
pub(crate) fn encode_logical_change_plan_with_limits<F>(
    plan: &LogicalChangePlan<'_>,
    emit: F,
    maximum_bytes: u64,
    maximum_records: u64,
) -> Result<LogicalPlanEncoding, Diagnostic>
where
    F: FnMut(&[u8]) -> Result<(), Diagnostic>,
{
    encode_logical_change_plan_admitted(plan, emit, maximum_bytes, maximum_records)
}

fn encode_logical_change_plan_admitted<F>(
    plan: &LogicalChangePlan<'_>,
    mut emit: F,
    maximum_bytes: u64,
    maximum_records: u64,
) -> Result<LogicalPlanEncoding, Diagnostic>
where
    F: FnMut(&[u8]) -> Result<(), Diagnostic>,
{
    let mut encoder = PlanEncoder::new(&mut emit, maximum_bytes, maximum_records);
    encode_plan_body(plan, &mut encoder)?;
    let prepared = PreparedPlanCommitment(*encoder.hasher.finalize().as_bytes());
    let token = ChangePlanToken {
        request: plan.request,
        prepared,
    };
    encoder.append_trailer(
        "logical-plan.digest",
        &[
            ("request", plan.request.to_string()),
            ("prepared", prepared.to_string()),
            ("token", token.to_string()),
        ],
    )?;
    Ok(LogicalPlanEncoding {
        token,
        bytes: encoder.bytes,
        records: encoder.records,
    })
}

struct PlanEncoder<'a, F> {
    emit: &'a mut F,
    hasher: blake3::Hasher,
    bytes: u64,
    records: u64,
    maximum_bytes: u64,
    maximum_records: u64,
}

impl<F> PlanEncoder<'_, F>
where
    F: FnMut(&[u8]) -> Result<(), Diagnostic>,
{
    fn new(emit: &mut F, maximum_bytes: u64, maximum_records: u64) -> PlanEncoder<'_, F> {
        PlanEncoder {
            emit,
            hasher: blake3::Hasher::new_derive_key(PREPARED_CHANGE_PLAN_COMMITMENT_DOMAIN),
            bytes: 0,
            records: 0,
            maximum_bytes,
            maximum_records,
        }
    }

    fn append(&mut self, operation: &str, fields: &[(&str, String)]) -> Result<(), Diagnostic> {
        self.append_inner(operation, fields, true)
    }

    fn append_trailer(
        &mut self,
        operation: &str,
        fields: &[(&str, String)],
    ) -> Result<(), Diagnostic> {
        self.append_inner(operation, fields, false)
    }

    fn append_inner(
        &mut self,
        operation: &str,
        fields: &[(&str, String)],
        hashed: bool,
    ) -> Result<(), Diagnostic> {
        let borrowed = fields
            .iter()
            .map(|(name, value)| (*name, value.as_str()))
            .collect::<Vec<_>>();
        let record = render_record(operation, &borrowed)?;
        let additional = u64::try_from(record.len()).map_err(|_| {
            plan_resource_error(
                "change_plan_output_byte_budget",
                "logical plan record length exceeds the platform counter domain",
            )
        })?;
        let bytes = self.bytes.checked_add(additional).ok_or_else(|| {
            plan_resource_error(
                "change_plan_output_byte_budget",
                "logical plan byte accounting overflowed",
            )
        })?;
        let records = self.records.checked_add(1).ok_or_else(|| {
            plan_resource_error(
                "change_plan_output_record_budget",
                "logical plan record accounting overflowed",
            )
        })?;
        if bytes > self.maximum_bytes {
            return Err(plan_resource_error(
                "change_plan_output_byte_budget",
                format!(
                    "logical plan requires more than the {}-byte complete-review budget",
                    self.maximum_bytes
                ),
            ));
        }
        if records > self.maximum_records {
            return Err(plan_resource_error(
                "change_plan_output_record_budget",
                format!(
                    "logical plan requires more than the {}-record complete-review budget",
                    self.maximum_records
                ),
            ));
        }
        if hashed {
            self.hasher.update(record.as_bytes());
        }
        (self.emit)(record.as_bytes())?;
        self.bytes = bytes;
        self.records = records;
        Ok(())
    }
}

fn encode_plan_body<F>(
    plan: &LogicalChangePlan<'_>,
    encoder: &mut PlanEncoder<'_, F>,
) -> Result<(), Diagnostic>
where
    F: FnMut(&[u8]) -> Result<(), Diagnostic>,
{
    let prepared = &plan.prepared().publication;
    let evidence = plan.evidence();
    let capabilities = capabilities_snapshot().map_err(|message| {
        plan_corrupt(
            "capabilities_projection_invalid",
            format!("public capabilities could not be prepared: {message}"),
        )
    })?;
    let internal_registry = registry_snapshot().map_err(|message| {
        plan_corrupt(
            "capabilities_projection_invalid",
            format!("internal plan bindings could not be prepared: {message}"),
        )
    })?;
    encoder.append(
        "logical-plan",
        &[
            ("product", "lkjscript".to_owned()),
            ("version", crate::PRODUCT_VERSION.to_owned()),
        ],
    )?;
    encoder.append(
        "logical-plan.capabilities",
        &[("digest", capabilities.digest)],
    )?;
    bind_internal_registry(&mut encoder.hasher, &internal_registry.digest);
    encoder.append(
        "logical-plan.authority",
        &[
            ("repository", prepared.head.repository_id.to_string()),
            (
                "package",
                prepared.authority.semantic.root.package_id.to_string(),
            ),
            ("base", plan.base.to_string()),
            ("result", prepared.head.revision.to_string()),
            (
                "semantic-state",
                prepared.authority.semantic.state.to_string(),
            ),
        ],
    )?;
    encoder.append(
        "logical-plan.request",
        &[("commitment", plan.request.to_string())],
    )?;
    encode_budget(evidence.budget, encoder)?;
    encoder.append(
        "logical-plan.controls",
        &[
            (
                "idempotency-present",
                prepared.receipt.idempotency_key.is_some().to_string(),
            ),
            (
                "idempotency",
                prepared
                    .receipt
                    .idempotency_key
                    .as_deref()
                    .unwrap_or("")
                    .to_owned(),
            ),
            (
                "intent-present",
                prepared.receipt.intent.is_some().to_string(),
            ),
            (
                "intent",
                prepared.receipt.intent.as_deref().unwrap_or("").to_owned(),
            ),
        ],
    )?;
    encoder.append(
        "logical-plan.binding",
        &[
            ("transaction", prepared.transaction_digest.to_string()),
            ("semantic-diff", prepared.semantic_diff_digest.to_string()),
        ],
    )?;
    for allocation in &evidence.allocations {
        encoder.append(
            "logical-plan.allocation",
            &[
                ("domain", identity_kind_name(allocation.domain).to_owned()),
                ("ordinal", allocation.ordinal.to_string()),
                ("owner", allocation.owner.to_string()),
            ],
        )?;
    }
    for owner in plan.owners {
        encode_owner_change(owner, encoder)?;
    }
    for digest in plan.types {
        encoder.append(
            "logical-plan.type-addition",
            &[("object", digest.to_string())],
        )?;
    }
    for diff in plan.dependencies {
        let values = evidence.dependencies.get(&diff.package).ok_or_else(|| {
            plan_corrupt(
                "change_logical_plan_dependency",
                "semantic diff dependency has no logical value projection",
            )
        })?;
        encode_dependency_change(diff, values.before.as_ref(), values.after.as_ref(), encoder)?;
    }
    for diff in plan.retirements {
        let values = evidence.retirements.get(&diff.owner).ok_or_else(|| {
            plan_corrupt(
                "change_logical_plan_retirement",
                "semantic diff retirement has no logical value projection",
            )
        })?;
        encode_retirement_change(diff, values.before.as_ref(), values.after.as_ref(), encoder)?;
    }
    for edge in &evidence.relations.removed {
        encode_relation("logical-plan.relation-removed", *edge, encoder)?;
    }
    for edge in &evidence.relations.added {
        encode_relation("logical-plan.relation-added", *edge, encoder)?;
    }
    for owner in &evidence.structurally_checked {
        encoder.append(
            "logical-plan.validation-structural",
            &[("owner", owner.to_string())],
        )?;
    }
    for owner in &evidence.semantically_checked {
        encoder.append(
            "logical-plan.validation-semantic",
            &[("owner", owner.to_string())],
        )?;
    }
    for owner in &evidence.tests {
        encoder.append("logical-plan.test", &[("owner", owner.to_string())])?;
    }
    for reason in &evidence.reasons {
        encoder.append(
            "logical-plan.reason",
            &[
                ("kind", reason.kind.name().to_owned()),
                ("source", reason.source.to_string()),
                ("target", reason.target.to_string()),
                ("relation-present", reason.relation.is_some().to_string()),
                (
                    "relation",
                    reason
                        .relation
                        .map(RelationKind::name)
                        .unwrap_or("")
                        .to_owned(),
                ),
            ],
        )?;
    }
    encoder.append(
        "logical-plan.counts",
        &[
            ("allocations", count(evidence.allocations.len())?),
            ("owners", count(plan.owners.len())?),
            ("types", count(plan.types.len())?),
            ("dependencies", count(plan.dependencies.len())?),
            ("retirements", count(plan.retirements.len())?),
            (
                "relations-removed",
                count(evidence.relations.removed.len())?,
            ),
            ("relations-added", count(evidence.relations.added.len())?),
            (
                "structural-owners",
                count(evidence.structurally_checked.len())?,
            ),
            (
                "semantic-owners",
                count(evidence.semantically_checked.len())?,
            ),
            ("tests", count(evidence.tests.len())?),
            ("reasons", count(evidence.reasons.len())?),
        ],
    )?;
    Ok(())
}

fn encode_budget<F>(
    budget: ChangeBudget,
    encoder: &mut PlanEncoder<'_, F>,
) -> Result<(), Diagnostic>
where
    F: FnMut(&[u8]) -> Result<(), Diagnostic>,
{
    encoder.append(
        "logical-plan.budget-authored",
        &[
            ("operations", budget.authored.maximum_operations.to_string()),
            (
                "preconditions",
                budget.authored.maximum_preconditions.to_string(),
            ),
            (
                "allocated-identities",
                budget.authored.maximum_allocated_identities.to_string(),
            ),
            ("type-nodes", budget.authored.maximum_type_nodes.to_string()),
        ],
    )?;
    encoder.append(
        "logical-plan.budget-canonical-edits",
        &[
            (
                "owners",
                budget.canonical_edits.maximum_owner_edits.to_string(),
            ),
            (
                "types",
                budget.canonical_edits.maximum_type_edits.to_string(),
            ),
            (
                "dependencies",
                budget.canonical_edits.maximum_dependency_edits.to_string(),
            ),
            (
                "retirements",
                budget.canonical_edits.maximum_retirement_edits.to_string(),
            ),
        ],
    )?;
    encode_read_budget(
        "logical-plan.budget-canonical-reads",
        budget.canonical_reads.maximum_point_reads,
        budget.canonical_reads.maximum_map_pages,
        budget.canonical_reads.maximum_map_entries,
        budget.canonical_reads.maximum_catalog_lookups,
        budget.canonical_reads.maximum_objects,
        budget.canonical_reads.maximum_bytes,
        budget.canonical_reads.maximum_decoded_records,
        encoder,
    )?;
    encoder.append(
        "logical-plan.budget-canonical-map-update",
        &[
            (
                "pages",
                budget
                    .canonical_map_update
                    .maximum_pages_encoded
                    .to_string(),
            ),
            (
                "bytes",
                budget
                    .canonical_map_update
                    .maximum_bytes_encoded
                    .to_string(),
            ),
        ],
    )?;
    encode_read_budget(
        "logical-plan.budget-witness-reads",
        budget.witness_reads.maximum_point_reads,
        budget.witness_reads.maximum_map_pages,
        budget.witness_reads.maximum_map_entries,
        budget.witness_reads.maximum_catalog_lookups,
        budget.witness_reads.maximum_objects,
        budget.witness_reads.maximum_bytes,
        budget.witness_reads.maximum_decoded_records,
        encoder,
    )?;
    encoder.append(
        "logical-plan.budget-impact",
        &[
            (
                "affected-owners",
                budget.impact.maximum_affected_owners.to_string(),
            ),
            (
                "summary-owners",
                budget.impact.maximum_summary_owners.to_string(),
            ),
            (
                "summary-edits",
                budget.impact.maximum_summary_edits.to_string(),
            ),
            (
                "ownership-steps",
                budget.impact.maximum_ownership_steps.to_string(),
            ),
            (
                "behavior-owners",
                budget.impact.maximum_behavior_owners.to_string(),
            ),
            (
                "relation-edges",
                budget.impact.maximum_relation_edges.to_string(),
            ),
            (
                "relation-fanout",
                budget.impact.maximum_relation_fanout.to_string(),
            ),
        ],
    )?;
    encoder.append(
        "logical-plan.budget-validation",
        &[
            (
                "owner-records",
                budget.validation.maximum_owner_records.to_string(),
            ),
            (
                "ownership-entries",
                budget.validation.maximum_ownership_entries.to_string(),
            ),
            (
                "type-objects",
                budget.validation.maximum_type_objects.to_string(),
            ),
            (
                "expression-steps",
                budget.validation.maximum_expression_steps.to_string(),
            ),
            (
                "diagnostics",
                budget.validation.maximum_diagnostics.to_string(),
            ),
        ],
    )?;
    encoder.append(
        "logical-plan.budget-tests",
        &[
            ("selected", budget.tests.maximum_selected.to_string()),
            (
                "dependencies-per-test",
                budget.tests.maximum_dependencies_per_test.to_string(),
            ),
            (
                "ownership-steps",
                budget.tests.maximum_ownership_steps.to_string(),
            ),
            (
                "owners-visited",
                budget.tests.maximum_owners_visited.to_string(),
            ),
        ],
    )?;
    encoder.append(
        "logical-plan.budget-witness-update",
        &[
            ("edits", budget.witness_update.maximum_edits.to_string()),
            (
                "pages",
                budget.witness_update.maximum_pages_encoded.to_string(),
            ),
            (
                "bytes",
                budget.witness_update.maximum_bytes_encoded.to_string(),
            ),
        ],
    )?;
    encoder.append(
        "logical-plan.budget-staging",
        &[
            ("objects", budget.staging.maximum_objects.to_string()),
            ("bytes", budget.staging.maximum_bytes.to_string()),
            ("pages", budget.staging.maximum_pages.to_string()),
        ],
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "canonical and witness read groups share seven independently bounded dimensions"
)]
fn encode_read_budget<F>(
    operation: &str,
    point_reads: u64,
    map_pages: u64,
    map_entries: u64,
    catalog_lookups: u64,
    objects: u64,
    bytes: u64,
    decoded_records: u64,
    encoder: &mut PlanEncoder<'_, F>,
) -> Result<(), Diagnostic>
where
    F: FnMut(&[u8]) -> Result<(), Diagnostic>,
{
    encoder.append(
        operation,
        &[
            ("point-reads", point_reads.to_string()),
            ("map-pages", map_pages.to_string()),
            ("map-entries", map_entries.to_string()),
            ("catalog-lookups", catalog_lookups.to_string()),
            ("objects", objects.to_string()),
            ("bytes", bytes.to_string()),
            ("decoded-records", decoded_records.to_string()),
        ],
    )
}

fn encode_owner_change<F>(
    entry: &OwnerDiffEntry,
    encoder: &mut PlanEncoder<'_, F>,
) -> Result<(), Diagnostic>
where
    F: FnMut(&[u8]) -> Result<(), Diagnostic>,
{
    let classes = &entry.classes;
    encoder.append(
        "logical-plan.owner",
        &[
            ("owner", entry.owner.to_string()),
            ("before-present", entry.objects.before.is_some().to_string()),
            (
                "before",
                entry
                    .objects
                    .before
                    .map_or_else(String::new, |value| value.to_string()),
            ),
            ("after-present", entry.objects.after.is_some().to_string()),
            (
                "after",
                entry
                    .objects
                    .after
                    .map_or_else(String::new, |value| value.to_string()),
            ),
            (
                "class-created",
                has_class(classes, OwnerChangeClass::Created),
            ),
            (
                "class-deleted",
                has_class(classes, OwnerChangeClass::Deleted),
            ),
            (
                "class-renamed",
                has_class(classes, OwnerChangeClass::Renamed),
            ),
            ("class-moved", has_class(classes, OwnerChangeClass::Moved)),
            (
                "class-visibility-changed",
                has_class(classes, OwnerChangeClass::VisibilityChanged),
            ),
            (
                "class-presentation-only",
                has_class(classes, OwnerChangeClass::PresentationOnly),
            ),
            (
                "class-test-only",
                has_class(classes, OwnerChangeClass::TestOnly),
            ),
            (
                "class-private-implementation",
                has_class(classes, OwnerChangeClass::PrivateImplementation),
            ),
            (
                "class-public-interface",
                has_class(classes, OwnerChangeClass::PublicInterface),
            ),
            (
                "class-effect-or-capability",
                has_class(classes, OwnerChangeClass::EffectOrCapability),
            ),
            (
                "class-relation-set",
                has_class(classes, OwnerChangeClass::RelationSet),
            ),
            (
                "class-semantic-payload-changed",
                has_class(classes, OwnerChangeClass::SemanticPayloadChanged),
            ),
            (
                "dimension-semantic-interface",
                entry.dimensions.semantic_interface.to_string(),
            ),
            (
                "dimension-implementation",
                entry.dimensions.implementation.to_string(),
            ),
            (
                "dimension-type-digest",
                entry.dimensions.type_digest.to_string(),
            ),
            ("dimension-effect", entry.dimensions.effect.to_string()),
            (
                "dimension-capability",
                entry.dimensions.capability.to_string(),
            ),
            (
                "dimension-relations",
                entry.dimensions.relations.to_string(),
            ),
            (
                "dimension-presentation",
                entry.dimensions.presentation.to_string(),
            ),
            ("dimension-test", entry.dimensions.test.to_string()),
            (
                "dimension-validation-dependencies",
                entry.dimensions.validation_dependencies.to_string(),
            ),
        ],
    )
}

fn has_class(classes: &[OwnerChangeClass], class: OwnerChangeClass) -> String {
    classes.contains(&class).to_string()
}

fn encode_dependency_change<F>(
    diff: &DependencyDiffEntry,
    before: Option<&DependencyRecord>,
    after: Option<&DependencyRecord>,
    encoder: &mut PlanEncoder<'_, F>,
) -> Result<(), Diagnostic>
where
    F: FnMut(&[u8]) -> Result<(), Diagnostic>,
{
    encoder.append(
        "logical-plan.dependency",
        &[
            ("package", diff.package.to_string()),
            ("before-present", before.is_some().to_string()),
            (
                "before-object",
                diff.objects
                    .before
                    .map_or_else(String::new, |value| value.to_string()),
            ),
            (
                "before-semantic-revision",
                before.map_or_else(String::new, |value| value.semantic_revision.to_string()),
            ),
            (
                "before-package-revision",
                before.map_or_else(String::new, |value| value.package_revision.to_string()),
            ),
            ("after-present", after.is_some().to_string()),
            (
                "after-object",
                diff.objects
                    .after
                    .map_or_else(String::new, |value| value.to_string()),
            ),
            (
                "after-semantic-revision",
                after.map_or_else(String::new, |value| value.semantic_revision.to_string()),
            ),
            (
                "after-package-revision",
                after.map_or_else(String::new, |value| value.package_revision.to_string()),
            ),
        ],
    )
}

fn encode_retirement_change<F>(
    diff: &RetirementDiffEntry,
    before: Option<&RetirementRecord>,
    after: Option<&RetirementRecord>,
    encoder: &mut PlanEncoder<'_, F>,
) -> Result<(), Diagnostic>
where
    F: FnMut(&[u8]) -> Result<(), Diagnostic>,
{
    let before_fields = RetirementFields::new(before);
    let after_fields = RetirementFields::new(after);
    encoder.append(
        "logical-plan.retirement",
        &[
            ("owner", diff.owner.to_string()),
            ("before-present", before.is_some().to_string()),
            (
                "before-object",
                diff.objects
                    .before
                    .map_or_else(String::new, |value| value.to_string()),
            ),
            ("before-kind", before_fields.kind),
            ("before-name-present", before_fields.name_present),
            ("before-name", before_fields.name),
            ("before-parent-present", before_fields.parent_present),
            ("before-parent", before_fields.parent),
            ("before-live-revision", before_fields.live_revision),
            ("before-deletion-change", before_fields.deletion_change),
            ("after-present", after.is_some().to_string()),
            (
                "after-object",
                diff.objects
                    .after
                    .map_or_else(String::new, |value| value.to_string()),
            ),
            ("after-kind", after_fields.kind),
            ("after-name-present", after_fields.name_present),
            ("after-name", after_fields.name),
            ("after-parent-present", after_fields.parent_present),
            ("after-parent", after_fields.parent),
            ("after-live-revision", after_fields.live_revision),
            ("after-deletion-change", after_fields.deletion_change),
        ],
    )
}

struct RetirementFields {
    kind: String,
    name_present: String,
    name: String,
    parent_present: String,
    parent: String,
    live_revision: String,
    deletion_change: String,
}

impl RetirementFields {
    fn new(record: Option<&RetirementRecord>) -> Self {
        Self {
            kind: record.map_or_else(String::new, |value| value.last_kind.name().to_owned()),
            name_present: record
                .is_some_and(|value| value.last_name.is_some())
                .to_string(),
            name: record
                .and_then(|value| value.last_name.as_ref())
                .map_or_else(String::new, |value| value.as_str().to_owned()),
            parent_present: record
                .is_some_and(|value| value.last_parent.is_some())
                .to_string(),
            parent: record
                .and_then(|value| value.last_parent)
                .map_or_else(String::new, |value| value.to_string()),
            live_revision: record
                .map_or_else(String::new, |value| value.last_live_revision.to_string()),
            deletion_change: record
                .map_or_else(String::new, |value| value.deletion_change.to_string()),
        }
    }
}

fn encode_relation<F>(
    operation: &str,
    edge: RelationEdge,
    encoder: &mut PlanEncoder<'_, F>,
) -> Result<(), Diagnostic>
where
    F: FnMut(&[u8]) -> Result<(), Diagnostic>,
{
    let source = EndpointFields::new(edge.source);
    let target = EndpointFields::new(edge.target);
    encoder.append(
        operation,
        &[
            ("source-package", source.package),
            ("source-owner-present", source.owner_present),
            ("source-owner", source.owner),
            ("kind", edge.kind.name().to_owned()),
            ("target-package", target.package),
            ("target-owner-present", target.owner_present),
            ("target-owner", target.owner),
        ],
    )
}

struct EndpointFields {
    package: String,
    owner_present: String,
    owner: String,
}

impl EndpointFields {
    fn new(endpoint: RelationEndpoint) -> Self {
        match endpoint {
            RelationEndpoint::Package(package) => Self {
                package: package.to_string(),
                owner_present: false.to_string(),
                owner: String::new(),
            },
            RelationEndpoint::Owner(owner) => Self {
                package: owner.package.to_string(),
                owner_present: true.to_string(),
                owner: owner.owner.to_string(),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LogicalPlanCounts {
    pub allocations: u64,
    pub owners: u64,
    pub types: u64,
    pub dependencies: u64,
    pub retirements: u64,
    pub relations_removed: u64,
    pub relations_added: u64,
    pub structural_owners: u64,
    pub semantic_owners: u64,
    pub tests: u64,
    pub reasons: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedLogicalPlan {
    pub request_commitment: String,
    pub prepared_plan_commitment: String,
    pub token: String,
    pub counts: LogicalPlanCounts,
    pub bytes: u64,
    pub records: u64,
}

/// Strictly validates a bounded canonical plan stream without retaining its complete bytes.
pub fn decode_logical_change_plan<R: BufRead>(
    mut reader: R,
) -> Result<DecodedLogicalPlan, Diagnostic> {
    let mut decoder = PlanDecoder::new();
    let mut line = Vec::new();
    loop {
        line.clear();
        let mut bounded_record = (&mut reader).take(64 * 1_024 + 1);
        let read = bounded_record
            .read_until(b'\n', &mut line)
            .map_err(|error| {
                plan_source_error(
                    "change_plan_file_read",
                    format!("logical plan file could not be read: {error}"),
                )
            })?;
        if read == 0 {
            break;
        }
        if line.len() > 64 * 1_024 {
            return Err(plan_resource_error(
                "change_plan_file_record_bytes",
                "logical plan record exceeds the 65536-byte compact-record bound",
            ));
        }
        if line.last() != Some(&b'\n') {
            return Err(plan_source_error(
                "change_plan_file_truncated",
                "logical plan file ends inside a record",
            ));
        }
        decoder.observe_meter(line.len())?;
        let parsed = parse_records("<logical-plan>", &line).map_err(|diagnostics| {
            diagnostics.into_iter().next().unwrap_or_else(|| {
                plan_source_error(
                    "change_plan_file_record",
                    "logical plan record decoder returned no diagnostic",
                )
            })
        })?;
        let [record] = parsed.as_slice() else {
            return Err(plan_source_error(
                "change_plan_file_blank",
                "logical plan files contain no blank physical records",
            ));
        };
        validate_record_schema(record)?;
        let borrowed = record
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.value.as_str()))
            .collect::<Vec<_>>();
        let canonical = render_record(&record.operation, &borrowed)?;
        if canonical.as_bytes() != line {
            return Err(plan_source_error(
                "change_plan_file_canonical",
                "logical plan record is not canonically escaped or framed",
            ));
        }
        decoder.accept(record, &line)?;
    }
    decoder.finish()
}

struct PlanDecoder {
    hasher: blake3::Hasher,
    bytes: u64,
    records: u64,
    next_fixed: usize,
    phase: usize,
    trailer_seen: bool,
    request: Option<ChangeRequestCommitment>,
    prepared: Option<PreparedPlanCommitment>,
    token: Option<ChangePlanToken>,
    budget: ChangeBudget,
    counts: LogicalPlanCounts,
    declared_counts: Option<LogicalPlanCounts>,
    last_allocation: Option<AuthoredAllocation>,
    last_owner: Option<OwnerKey>,
    last_type: Option<TypeObjectDigest>,
    last_dependency: Option<PackageId>,
    last_retirement: Option<OwnerKey>,
    last_removed_relation: Option<RelationEdge>,
    last_added_relation: Option<RelationEdge>,
    removed_relations: BTreeSet<RelationEdge>,
    last_structural: Option<OwnerKey>,
    last_semantic: Option<OwnerKey>,
    last_test: Option<OwnerKey>,
    last_reason: Option<ImpactReason>,
}

impl PlanDecoder {
    fn new() -> Self {
        Self {
            hasher: blake3::Hasher::new_derive_key(PREPARED_CHANGE_PLAN_COMMITMENT_DOMAIN),
            bytes: 0,
            records: 0,
            next_fixed: 0,
            phase: 16,
            trailer_seen: false,
            request: None,
            prepared: None,
            token: None,
            budget: ChangeBudget::default(),
            counts: LogicalPlanCounts::default(),
            declared_counts: None,
            last_allocation: None,
            last_owner: None,
            last_type: None,
            last_dependency: None,
            last_retirement: None,
            last_removed_relation: None,
            last_added_relation: None,
            removed_relations: BTreeSet::new(),
            last_structural: None,
            last_semantic: None,
            last_test: None,
            last_reason: None,
        }
    }

    fn observe_meter(&mut self, bytes: usize) -> Result<(), Diagnostic> {
        self.bytes = self
            .bytes
            .checked_add(u64::try_from(bytes).unwrap_or(u64::MAX))
            .ok_or_else(|| {
                plan_resource_error(
                    "change_plan_file_byte_budget",
                    "logical plan decoder byte accounting overflowed",
                )
            })?;
        self.records = self.records.checked_add(1).ok_or_else(|| {
            plan_resource_error(
                "change_plan_file_record_budget",
                "logical plan decoder record accounting overflowed",
            )
        })?;
        if self.bytes > MAXIMUM_LOGICAL_PLAN_BYTES {
            return Err(plan_resource_error(
                "change_plan_file_byte_budget",
                format!(
                    "logical plan file exceeds the {MAXIMUM_LOGICAL_PLAN_BYTES}-byte decoder bound"
                ),
            ));
        }
        if self.records > MAXIMUM_LOGICAL_PLAN_RECORDS {
            return Err(plan_resource_error(
                "change_plan_file_record_budget",
                format!(
                    "logical plan file exceeds the {MAXIMUM_LOGICAL_PLAN_RECORDS}-record decoder bound"
                ),
            ));
        }
        Ok(())
    }

    fn accept(&mut self, record: &CompactRecord, line: &[u8]) -> Result<(), Diagnostic> {
        if self.trailer_seen {
            return Err(plan_source_error(
                "change_plan_file_trailing",
                "logical plan file contains records after its digest trailer",
            ));
        }
        let descriptor_index = LOGICAL_PLAN_RECORD_DESCRIPTORS
            .iter()
            .position(|descriptor| descriptor.name == record.operation)
            .ok_or_else(|| {
                plan_source_error(
                    "change_plan_file_record_unknown",
                    format!("unknown logical plan record '{}'", record.operation),
                )
            })?;
        if self.next_fixed < 16 {
            if descriptor_index != self.next_fixed {
                if descriptor_index < self.next_fixed {
                    return Err(plan_source_error(
                        "change_plan_file_singleton_duplicate",
                        format!(
                            "logical plan singleton '{}' is duplicated",
                            record.operation
                        ),
                    ));
                }
                return Err(plan_source_error(
                    "change_plan_file_order",
                    format!(
                        "logical plan expected '{}' before '{}'",
                        LOGICAL_PLAN_RECORD_DESCRIPTORS[self.next_fixed].name, record.operation
                    ),
                ));
            }
            self.next_fixed += 1;
        } else if (16..=26).contains(&descriptor_index) {
            if descriptor_index < self.phase || self.phase >= 27 {
                return Err(plan_source_error(
                    "change_plan_file_order",
                    format!(
                        "logical plan record '{}' is out of canonical order",
                        record.operation
                    ),
                ));
            }
            self.phase = descriptor_index;
        } else if descriptor_index == 27 {
            if self.phase >= 27 || self.declared_counts.is_some() {
                return Err(plan_source_error(
                    "change_plan_file_counts_duplicate",
                    "logical plan contains a duplicate or misplaced counts record",
                ));
            }
            self.phase = 27;
        } else if descriptor_index == 28 {
            if self.phase != 27 || self.declared_counts.is_none() {
                return Err(plan_source_error(
                    "change_plan_file_digest_order",
                    "logical plan digest must follow exactly one counts record",
                ));
            }
            self.phase = 28;
            self.trailer_seen = true;
        } else {
            return Err(plan_source_error(
                "change_plan_file_singleton_duplicate",
                format!(
                    "logical plan singleton '{}' is duplicated",
                    record.operation
                ),
            ));
        }

        if descriptor_index != 28 {
            self.hasher.update(line);
        }
        self.validate_typed(descriptor_index, record)
    }

    fn validate_typed(
        &mut self,
        descriptor_index: usize,
        record: &CompactRecord,
    ) -> Result<(), Diagnostic> {
        match descriptor_index {
            0 => validate_header(record),
            1 => {
                let internal_registry_digest = validate_capabilities(record)?;
                bind_internal_registry(&mut self.hasher, &internal_registry_digest);
                Ok(())
            }
            2 => validate_authority(record),
            3 => {
                self.request = Some(parse_request_commitment(field(record, 0))?);
                Ok(())
            }
            4 => decode_budget_authored(record, &mut self.budget),
            5 => decode_budget_canonical_edits(record, &mut self.budget),
            6 => decode_budget_canonical_reads(record, &mut self.budget),
            7 => decode_budget_canonical_map(record, &mut self.budget),
            8 => decode_budget_witness_reads(record, &mut self.budget),
            9 => decode_budget_impact(record, &mut self.budget),
            10 => decode_budget_validation(record, &mut self.budget),
            11 => decode_budget_tests(record, &mut self.budget),
            12 => decode_budget_witness_update(record, &mut self.budget),
            13 => decode_budget_staging(record, &mut self.budget),
            14 => validate_controls(record),
            15 => {
                field(record, 0).parse::<TransactionDigest>()?;
                field(record, 1).parse::<SemanticDiffDigest>()?;
                self.budget.validate()?;
                Ok(())
            }
            16 => self.decode_allocation(record),
            17 => self.decode_owner(record),
            18 => self.decode_type(record),
            19 => self.decode_dependency(record),
            20 => self.decode_retirement(record),
            21 => self.decode_relation(record, false),
            22 => self.decode_relation(record, true),
            23 => self.decode_selection(record, SelectionKind::Structural),
            24 => self.decode_selection(record, SelectionKind::Semantic),
            25 => self.decode_selection(record, SelectionKind::Test),
            26 => self.decode_reason(record),
            27 => {
                let counts = decode_counts(record)?;
                if counts != self.counts {
                    return Err(plan_source_error(
                        "change_plan_file_counts",
                        "logical plan counts disagree with the preceding exact records",
                    ));
                }
                self.declared_counts = Some(counts);
                Ok(())
            }
            28 => self.decode_trailer(record),
            _ => Err(plan_source_error(
                "change_plan_file_record_unknown",
                "logical plan descriptor has no typed decoder",
            )),
        }
    }

    fn decode_allocation(&mut self, record: &CompactRecord) -> Result<(), Diagnostic> {
        let domain = parse_identity_kind(field(record, 0))?;
        let ordinal = parse_u64(field(record, 1), "allocation ordinal")?;
        if ordinal == 0 {
            return Err(plan_source_error(
                "change_plan_file_allocation_ordinal",
                "logical plan allocation ordinals begin at one",
            ));
        }
        let owner = field(record, 2).parse::<OwnerKey>()?;
        let allocation = AuthoredAllocation {
            domain,
            ordinal,
            owner,
        };
        if domain != owner.identity_kind()
            || self.last_allocation.is_some_and(|previous| {
                allocation_key_cmp(&previous, &allocation) != Ordering::Less
            })
        {
            return Err(plan_source_error(
                "change_plan_file_allocation_order",
                "logical plan allocation has a foreign domain, duplicate ordinal, or noncanonical order",
            ));
        }
        self.last_allocation = Some(allocation);
        increment(&mut self.counts.allocations, "allocation")
    }

    fn decode_owner(&mut self, record: &CompactRecord) -> Result<(), Diagnostic> {
        let owner = field(record, 0).parse::<OwnerKey>()?;
        require_owner_order(self.last_owner, owner, "owner change")?;
        let before = parse_optional::<OwnerObjectDigest>(field(record, 1), field(record, 2))?;
        let after = parse_optional::<OwnerObjectDigest>(field(record, 3), field(record, 4))?;
        if before.is_none() && after.is_none() {
            return Err(plan_source_error(
                "change_plan_file_owner_empty",
                "logical owner change has neither a before nor after object",
            ));
        }
        let mut classes = Vec::new();
        for (index, class) in [
            OwnerChangeClass::Created,
            OwnerChangeClass::Deleted,
            OwnerChangeClass::Renamed,
            OwnerChangeClass::Moved,
            OwnerChangeClass::VisibilityChanged,
            OwnerChangeClass::PresentationOnly,
            OwnerChangeClass::TestOnly,
            OwnerChangeClass::PrivateImplementation,
            OwnerChangeClass::PublicInterface,
            OwnerChangeClass::EffectOrCapability,
            OwnerChangeClass::RelationSet,
            OwnerChangeClass::SemanticPayloadChanged,
        ]
        .into_iter()
        .enumerate()
        {
            if parse_bool(field(record, index + 5), "owner class")? {
                classes.push(class);
            }
        }
        let dimensions = SummaryDimensions {
            semantic_interface: parse_bool(field(record, 17), "owner dimension")?,
            implementation: parse_bool(field(record, 18), "owner dimension")?,
            type_digest: parse_bool(field(record, 19), "owner dimension")?,
            effect: parse_bool(field(record, 20), "owner dimension")?,
            capability: parse_bool(field(record, 21), "owner dimension")?,
            relations: parse_bool(field(record, 22), "owner dimension")?,
            presentation: parse_bool(field(record, 23), "owner dimension")?,
            test: parse_bool(field(record, 24), "owner dimension")?,
            validation_dependencies: parse_bool(field(record, 25), "owner dimension")?,
        };
        crate::platform::publication::validate_owner_entry(&OwnerDiffEntry {
            owner,
            objects: crate::platform::publication::DigestEdit { before, after },
            classes,
            dimensions,
        })
        .map_err(|_| {
            plan_source_error(
                "change_plan_file_owner_consistency",
                "logical owner classes or dimensions disagree with its exact binding change",
            )
        })?;
        self.last_owner = Some(owner);
        increment(&mut self.counts.owners, "owner change")
    }

    fn decode_type(&mut self, record: &CompactRecord) -> Result<(), Diagnostic> {
        let digest = field(record, 0).parse::<TypeObjectDigest>()?;
        if self.last_type.is_some_and(|previous| previous >= digest) {
            return Err(plan_source_error(
                "change_plan_file_type_order",
                "logical plan type additions are duplicated or out of canonical order",
            ));
        }
        self.last_type = Some(digest);
        increment(&mut self.counts.types, "type addition")
    }

    fn decode_dependency(&mut self, record: &CompactRecord) -> Result<(), Diagnostic> {
        let package = field(record, 0).parse::<PackageId>()?;
        if self
            .last_dependency
            .is_some_and(|previous| previous >= package)
        {
            return Err(plan_source_error(
                "change_plan_file_dependency_order",
                "logical plan dependencies are duplicated or out of canonical order",
            ));
        }
        let before = decode_dependency_side(record, package, 1)?;
        let after = decode_dependency_side(record, package, 5)?;
        if before.is_none() && after.is_none() {
            return Err(plan_source_error(
                "change_plan_file_dependency_empty",
                "logical dependency change has neither a before nor after binding",
            ));
        }
        if before == after {
            return Err(plan_source_error(
                "change_plan_file_dependency_unchanged",
                "logical dependency before and after bindings are equal",
            ));
        }
        self.last_dependency = Some(package);
        increment(&mut self.counts.dependencies, "dependency change")
    }

    fn decode_retirement(&mut self, record: &CompactRecord) -> Result<(), Diagnostic> {
        let owner = field(record, 0).parse::<OwnerKey>()?;
        require_owner_order(self.last_retirement, owner, "retirement change")?;
        let before = decode_retirement_side(record, owner, 1)?;
        let after = decode_retirement_side(record, owner, 10)?;
        if before.is_none() && after.is_none() {
            return Err(plan_source_error(
                "change_plan_file_retirement_empty",
                "logical retirement change has neither a before nor after binding",
            ));
        }
        if before == after {
            return Err(plan_source_error(
                "change_plan_file_retirement_unchanged",
                "logical retirement before and after bindings are equal",
            ));
        }
        self.last_retirement = Some(owner);
        increment(&mut self.counts.retirements, "retirement change")
    }

    fn decode_relation(&mut self, record: &CompactRecord, added: bool) -> Result<(), Diagnostic> {
        let edge = RelationEdge {
            source: decode_endpoint(record, 0)?,
            kind: RelationKind::parse(field(record, 3))?,
            target: decode_endpoint(record, 4)?,
        };
        let previous = if added {
            &mut self.last_added_relation
        } else {
            &mut self.last_removed_relation
        };
        if previous.is_some_and(|value| relation_cmp(&value, &edge) != Ordering::Less) {
            return Err(plan_source_error(
                "change_plan_file_relation_order",
                "logical plan relations are duplicated or out of canonical order",
            ));
        }
        if added && self.removed_relations.contains(&edge) {
            return Err(plan_source_error(
                "change_plan_file_relation_overlap",
                "one logical relation is both removed and added",
            ));
        }
        *previous = Some(edge);
        if added {
            increment(&mut self.counts.relations_added, "added relation")
        } else {
            self.removed_relations.insert(edge);
            increment(&mut self.counts.relations_removed, "removed relation")
        }
    }

    fn decode_selection(
        &mut self,
        record: &CompactRecord,
        kind: SelectionKind,
    ) -> Result<(), Diagnostic> {
        let owner = field(record, 0).parse::<OwnerKey>()?;
        let (last, count) = match kind {
            SelectionKind::Structural => (
                &mut self.last_structural,
                &mut self.counts.structural_owners,
            ),
            SelectionKind::Semantic => (&mut self.last_semantic, &mut self.counts.semantic_owners),
            SelectionKind::Test => (&mut self.last_test, &mut self.counts.tests),
        };
        require_owner_order(*last, owner, kind.name())?;
        if kind == SelectionKind::Test && !matches!(owner, OwnerKey::Declaration(_)) {
            return Err(plan_source_error(
                "change_plan_file_test_domain",
                "selected graph test is not a declaration owner",
            ));
        }
        *last = Some(owner);
        increment(count, kind.name())
    }

    fn decode_reason(&mut self, record: &CompactRecord) -> Result<(), Diagnostic> {
        let kind = ImpactReasonKind::parse(field(record, 0)).ok_or_else(|| {
            plan_source_error(
                "change_plan_file_reason_kind",
                format!("unknown logical impact reason kind '{}'", field(record, 0)),
            )
        })?;
        let source = field(record, 1).parse::<OwnerKey>()?;
        let target = field(record, 2).parse::<OwnerKey>()?;
        let relation = parse_optional::<RelationKindValue>(field(record, 3), field(record, 4))?
            .map(|value| value.0);
        if (kind == ImpactReasonKind::DirectSemanticChange) != relation.is_none() {
            return Err(plan_source_error(
                "change_plan_file_reason_relation",
                "logical impact reason has invalid relation optionality",
            ));
        }
        let reason = ImpactReason {
            kind,
            source,
            target,
            relation,
        };
        if self
            .last_reason
            .is_some_and(|previous| previous.logical_cmp(&reason) != Ordering::Less)
        {
            return Err(plan_source_error(
                "change_plan_file_reason_order",
                "logical impact reasons are duplicated or out of canonical order",
            ));
        }
        self.last_reason = Some(reason);
        increment(&mut self.counts.reasons, "impact reason")
    }

    fn decode_trailer(&mut self, record: &CompactRecord) -> Result<(), Diagnostic> {
        let request = parse_request_commitment(field(record, 0))?;
        let prepared = parse_prepared_commitment(field(record, 1))?;
        let token = field(record, 2).parse::<ChangePlanToken>()?;
        let observed = PreparedPlanCommitment(*self.hasher.finalize().as_bytes());
        if self.request != Some(request)
            || prepared != observed
            || token.request != request
            || token.prepared != prepared
        {
            return Err(plan_source_error(
                "change_plan_file_digest",
                "logical plan digest trailer disagrees with its body or token components",
            ));
        }
        self.prepared = Some(prepared);
        self.token = Some(token);
        Ok(())
    }

    fn finish(self) -> Result<DecodedLogicalPlan, Diagnostic> {
        if !self.trailer_seen || self.phase != 28 {
            return Err(plan_source_error(
                "change_plan_file_incomplete",
                "logical plan file is missing its counts or digest trailer",
            ));
        }
        let request = self.request.ok_or_else(|| {
            plan_source_error(
                "change_plan_file_request",
                "logical plan file is missing its request commitment",
            )
        })?;
        let prepared = self.prepared.ok_or_else(|| {
            plan_source_error(
                "change_plan_file_prepared",
                "logical plan file is missing its prepared-plan commitment",
            )
        })?;
        let token = self.token.ok_or_else(|| {
            plan_source_error(
                "change_plan_file_token",
                "logical plan file is missing its complete plan token",
            )
        })?;
        Ok(DecodedLogicalPlan {
            request_commitment: request.to_string(),
            prepared_plan_commitment: prepared.to_string(),
            token: token.to_string(),
            counts: self.counts,
            bytes: self.bytes,
            records: self.records,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectionKind {
    Structural,
    Semantic,
    Test,
}

impl SelectionKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Structural => "structural validation owner",
            Self::Semantic => "semantic validation owner",
            Self::Test => "selected test",
        }
    }
}

fn validate_record_schema(record: &CompactRecord) -> Result<(), Diagnostic> {
    let descriptor = LOGICAL_PLAN_RECORD_DESCRIPTORS
        .iter()
        .find(|descriptor| descriptor.name == record.operation)
        .ok_or_else(|| {
            plan_source_error(
                "change_plan_file_record_unknown",
                format!("unknown logical plan record '{}'", record.operation),
            )
        })?;
    if record.fields.len() != descriptor.fields.len()
        || record
            .fields
            .iter()
            .zip(descriptor.fields)
            .any(|(field, expected)| field.name != *expected)
    {
        return Err(plan_source_error(
            "change_plan_file_fields",
            format!(
                "logical plan record '{}' does not use its exact canonical field set and order",
                record.operation
            ),
        ));
    }
    Ok(())
}

fn field(record: &CompactRecord, index: usize) -> &str {
    record
        .fields
        .get(index)
        .map_or("", |field| field.value.as_str())
}

fn bind_internal_registry(hasher: &mut blake3::Hasher, digest: &str) {
    hasher.update(INTERNAL_PLAN_BINDING_LABEL);
    hasher.update(digest.as_bytes());
}

fn validate_header(record: &CompactRecord) -> Result<(), Diagnostic> {
    if field(record, 0) != "lkjscript" || field(record, 1) != crate::PRODUCT_VERSION {
        return Err(plan_source_error(
            "change_plan_file_product",
            "logical plan file names a predecessor or foreign product",
        ));
    }
    Ok(())
}

fn validate_capabilities(record: &CompactRecord) -> Result<String, Diagnostic> {
    let expected = capabilities_snapshot().map_err(|message| {
        plan_corrupt(
            "capabilities_projection_invalid",
            format!("public capabilities could not be prepared: {message}"),
        )
    })?;
    if field(record, 0) != expected.digest {
        return Err(plan_source_error(
            "change_plan_file_capabilities",
            "logical plan body binds a predecessor or foreign capabilities digest",
        ));
    }
    registry_snapshot()
        .map(|snapshot| snapshot.digest)
        .map_err(|message| {
            plan_corrupt(
                "capabilities_projection_invalid",
                format!("internal plan bindings could not be prepared: {message}"),
            )
        })
}

fn validate_authority(record: &CompactRecord) -> Result<(), Diagnostic> {
    field(record, 0).parse::<RepositoryId>()?;
    field(record, 1).parse::<PackageId>()?;
    let base = field(record, 2).parse::<RevisionId>()?;
    let result = field(record, 3).parse::<RevisionId>()?;
    if base == result {
        return Err(plan_source_error(
            "change_plan_file_revision",
            "logical plan base and predicted result revisions must differ",
        ));
    }
    field(record, 4).parse::<SemanticStateDigest>()?;
    Ok(())
}

fn decode_budget_authored(
    record: &CompactRecord,
    budget: &mut ChangeBudget,
) -> Result<(), Diagnostic> {
    budget.authored.maximum_operations = parse_u64(field(record, 0), "operations")?;
    budget.authored.maximum_preconditions = parse_u64(field(record, 1), "preconditions")?;
    budget.authored.maximum_allocated_identities =
        parse_u64(field(record, 2), "allocated identities")?;
    budget.authored.maximum_type_nodes = parse_u64(field(record, 3), "type nodes")?;
    Ok(())
}

fn decode_budget_canonical_edits(
    record: &CompactRecord,
    budget: &mut ChangeBudget,
) -> Result<(), Diagnostic> {
    budget.canonical_edits.maximum_owner_edits = parse_u64(field(record, 0), "owner edits")?;
    budget.canonical_edits.maximum_type_edits = parse_u64(field(record, 1), "type edits")?;
    budget.canonical_edits.maximum_dependency_edits =
        parse_u64(field(record, 2), "dependency edits")?;
    budget.canonical_edits.maximum_retirement_edits =
        parse_u64(field(record, 3), "retirement edits")?;
    Ok(())
}

fn decode_budget_canonical_reads(
    record: &CompactRecord,
    budget: &mut ChangeBudget,
) -> Result<(), Diagnostic> {
    budget.canonical_reads.maximum_point_reads = parse_u64(field(record, 0), "point reads")?;
    budget.canonical_reads.maximum_map_pages = parse_u64(field(record, 1), "map pages")?;
    budget.canonical_reads.maximum_map_entries = parse_u64(field(record, 2), "map entries")?;
    budget.canonical_reads.maximum_catalog_lookups =
        parse_u64(field(record, 3), "catalog lookups")?;
    budget.canonical_reads.maximum_objects = parse_u64(field(record, 4), "objects")?;
    budget.canonical_reads.maximum_bytes = parse_u64(field(record, 5), "bytes")?;
    budget.canonical_reads.maximum_decoded_records =
        parse_u64(field(record, 6), "decoded records")?;
    Ok(())
}

fn decode_budget_canonical_map(
    record: &CompactRecord,
    budget: &mut ChangeBudget,
) -> Result<(), Diagnostic> {
    budget.canonical_map_update.maximum_pages_encoded = parse_u64(field(record, 0), "pages")?;
    budget.canonical_map_update.maximum_bytes_encoded = parse_u64(field(record, 1), "bytes")?;
    Ok(())
}

fn decode_budget_witness_reads(
    record: &CompactRecord,
    budget: &mut ChangeBudget,
) -> Result<(), Diagnostic> {
    budget.witness_reads.maximum_point_reads = parse_u64(field(record, 0), "point reads")?;
    budget.witness_reads.maximum_map_pages = parse_u64(field(record, 1), "map pages")?;
    budget.witness_reads.maximum_map_entries = parse_u64(field(record, 2), "map entries")?;
    budget.witness_reads.maximum_catalog_lookups = parse_u64(field(record, 3), "catalog lookups")?;
    budget.witness_reads.maximum_objects = parse_u64(field(record, 4), "objects")?;
    budget.witness_reads.maximum_bytes = parse_u64(field(record, 5), "bytes")?;
    budget.witness_reads.maximum_decoded_records = parse_u64(field(record, 6), "decoded records")?;
    Ok(())
}

fn decode_budget_impact(
    record: &CompactRecord,
    budget: &mut ChangeBudget,
) -> Result<(), Diagnostic> {
    budget.impact.maximum_affected_owners = parse_u64(field(record, 0), "affected owners")?;
    budget.impact.maximum_summary_owners = parse_u64(field(record, 1), "summary owners")?;
    budget.impact.maximum_summary_edits = parse_u64(field(record, 2), "summary edits")?;
    budget.impact.maximum_ownership_steps = parse_u64(field(record, 3), "ownership steps")?;
    budget.impact.maximum_behavior_owners = parse_u64(field(record, 4), "behavior owners")?;
    budget.impact.maximum_relation_edges = parse_u64(field(record, 5), "relation edges")?;
    budget.impact.maximum_relation_fanout = parse_u64(field(record, 6), "relation fanout")?;
    Ok(())
}

fn decode_budget_validation(
    record: &CompactRecord,
    budget: &mut ChangeBudget,
) -> Result<(), Diagnostic> {
    budget.validation.maximum_owner_records = parse_u64(field(record, 0), "owner records")?;
    budget.validation.maximum_ownership_entries = parse_u64(field(record, 1), "ownership entries")?;
    budget.validation.maximum_type_objects = parse_u64(field(record, 2), "type objects")?;
    budget.validation.maximum_expression_steps = parse_u64(field(record, 3), "expression steps")?;
    budget.validation.maximum_diagnostics = parse_u64(field(record, 4), "diagnostics")?;
    Ok(())
}

fn decode_budget_tests(
    record: &CompactRecord,
    budget: &mut ChangeBudget,
) -> Result<(), Diagnostic> {
    budget.tests.maximum_selected = parse_u64(field(record, 0), "selected tests")?;
    budget.tests.maximum_dependencies_per_test =
        parse_u64(field(record, 1), "dependencies per test")?;
    budget.tests.maximum_ownership_steps = parse_u64(field(record, 2), "ownership steps")?;
    budget.tests.maximum_owners_visited = parse_u64(field(record, 3), "owners visited")?;
    Ok(())
}

fn decode_budget_witness_update(
    record: &CompactRecord,
    budget: &mut ChangeBudget,
) -> Result<(), Diagnostic> {
    budget.witness_update.maximum_edits = parse_u64(field(record, 0), "witness edits")?;
    budget.witness_update.maximum_pages_encoded = parse_u64(field(record, 1), "pages")?;
    budget.witness_update.maximum_bytes_encoded = parse_u64(field(record, 2), "bytes")?;
    Ok(())
}

fn decode_budget_staging(
    record: &CompactRecord,
    budget: &mut ChangeBudget,
) -> Result<(), Diagnostic> {
    budget.staging.maximum_objects = parse_u64(field(record, 0), "staged objects")?;
    budget.staging.maximum_bytes = parse_u64(field(record, 1), "staged bytes")?;
    budget.staging.maximum_pages = parse_u64(field(record, 2), "staged pages")?;
    Ok(())
}

fn validate_controls(record: &CompactRecord) -> Result<(), Diagnostic> {
    let idempotency_present = parse_bool(field(record, 0), "idempotency presence")?;
    let idempotency = field(record, 1);
    if idempotency_present != !idempotency.is_empty()
        || (idempotency_present
            && !crate::platform::publication::idempotency_key_is_valid(idempotency))
    {
        return Err(plan_source_error(
            "change_plan_file_idempotency",
            "logical plan idempotency presence or value is invalid",
        ));
    }
    let intent_present = parse_bool(field(record, 2), "intent presence")?;
    let intent = field(record, 3);
    if (!intent_present && !intent.is_empty())
        || intent.len() > crate::platform::publication::contract::MAXIMUM_INTENT_BYTES
    {
        return Err(plan_source_error(
            "change_plan_file_intent",
            "logical plan intent presence or byte bound is invalid",
        ));
    }
    Ok(())
}

fn decode_dependency_side(
    record: &CompactRecord,
    package: PackageId,
    offset: usize,
) -> Result<Option<DependencyRecord>, Diagnostic> {
    let present = parse_bool(field(record, offset), "dependency presence")?;
    if !present {
        require_empty(
            record,
            &[offset + 1, offset + 2, offset + 3],
            "dependency absence",
        )?;
        return Ok(None);
    }
    let object = field(record, offset + 1).parse::<DependencyObjectDigest>()?;
    let value = DependencyRecord {
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        package,
        semantic_revision: field(record, offset + 2).parse::<RevisionId>()?,
        package_revision: field(record, offset + 3).parse::<PackageRevisionDigest>()?,
    };
    let (observed, _) = encode_dependency(&value)?;
    if observed != object {
        return Err(plan_source_error(
            "change_plan_file_dependency_object",
            "logical dependency value disagrees with its exact semantic object identity",
        ));
    }
    Ok(Some(value))
}

fn decode_retirement_side(
    record: &CompactRecord,
    owner: OwnerKey,
    offset: usize,
) -> Result<Option<RetirementRecord>, Diagnostic> {
    let present = parse_bool(field(record, offset), "retirement presence")?;
    let name_present = parse_bool(field(record, offset + 3), "retirement name presence")?;
    let parent_present = parse_bool(field(record, offset + 5), "retirement parent presence")?;
    if !present {
        if name_present || parent_present {
            return Err(plan_source_error(
                "change_plan_file_retirement_optionality",
                "absent retirement has present nested logical values",
            ));
        }
        require_empty(
            record,
            &[
                offset + 1,
                offset + 2,
                offset + 4,
                offset + 6,
                offset + 7,
                offset + 8,
            ],
            "retirement absence",
        )?;
        return Ok(None);
    }
    let object = field(record, offset + 1).parse::<RetirementObjectDigest>()?;
    let last_kind = OwnerKind::parse(field(record, offset + 2))?;
    let last_name = if name_present {
        Some(Name::new(field(record, offset + 4))?)
    } else {
        if !field(record, offset + 4).is_empty() {
            return Err(plan_source_error(
                "change_plan_file_retirement_name",
                "absent retirement name has a nonempty value",
            ));
        }
        None
    };
    let last_parent = if parent_present {
        Some(field(record, offset + 6).parse::<OwnerKey>()?)
    } else {
        if !field(record, offset + 6).is_empty() {
            return Err(plan_source_error(
                "change_plan_file_retirement_parent",
                "absent retirement parent has a nonempty value",
            ));
        }
        None
    };
    let value = RetirementRecord {
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        owner,
        last_kind,
        last_name,
        last_parent,
        last_live_revision: field(record, offset + 7).parse::<RevisionId>()?,
        deletion_change: field(record, offset + 8).parse::<ChangeDigest>()?,
    };
    let (observed, _) = encode_retirement(&value)?;
    if observed != object {
        return Err(plan_source_error(
            "change_plan_file_retirement_object",
            "logical retirement value disagrees with its exact semantic object identity",
        ));
    }
    Ok(Some(value))
}

fn decode_endpoint(record: &CompactRecord, offset: usize) -> Result<RelationEndpoint, Diagnostic> {
    let package = field(record, offset).parse::<PackageId>()?;
    let owner_present = parse_bool(field(record, offset + 1), "relation owner presence")?;
    let owner_text = field(record, offset + 2);
    if owner_present {
        Ok(RelationEndpoint::Owner(ExactOwnerKey {
            package,
            owner: owner_text.parse::<OwnerKey>()?,
        }))
    } else if owner_text.is_empty() {
        Ok(RelationEndpoint::Package(package))
    } else {
        Err(plan_source_error(
            "change_plan_file_relation_endpoint",
            "package relation endpoint has a nonempty owner value",
        ))
    }
}

fn decode_counts(record: &CompactRecord) -> Result<LogicalPlanCounts, Diagnostic> {
    Ok(LogicalPlanCounts {
        allocations: parse_u64(field(record, 0), "allocation count")?,
        owners: parse_u64(field(record, 1), "owner count")?,
        types: parse_u64(field(record, 2), "type count")?,
        dependencies: parse_u64(field(record, 3), "dependency count")?,
        retirements: parse_u64(field(record, 4), "retirement count")?,
        relations_removed: parse_u64(field(record, 5), "removed relation count")?,
        relations_added: parse_u64(field(record, 6), "added relation count")?,
        structural_owners: parse_u64(field(record, 7), "structural owner count")?,
        semantic_owners: parse_u64(field(record, 8), "semantic owner count")?,
        tests: parse_u64(field(record, 9), "test count")?,
        reasons: parse_u64(field(record, 10), "reason count")?,
    })
}

fn parse_request_commitment(value: &str) -> Result<ChangeRequestCommitment, Diagnostic> {
    let encoded = value.strip_prefix("request_").ok_or_else(|| {
        plan_source_error(
            "change_plan_file_request_domain",
            "request commitment must start with 'request_'",
        )
    })?;
    if encoded.len() != 64 {
        return Err(plan_source_error(
            "change_plan_file_request_length",
            "request commitment must contain 64 lowercase hexadecimal characters",
        ));
    }
    Ok(ChangeRequestCommitment::from_bytes(decode_hex_32(encoded)?))
}

fn parse_prepared_commitment(value: &str) -> Result<PreparedPlanCommitment, Diagnostic> {
    let encoded = value.strip_prefix("prepared_").ok_or_else(|| {
        plan_source_error(
            "change_plan_file_prepared_domain",
            "prepared-plan commitment must start with 'prepared_'",
        )
    })?;
    if encoded.len() != 64 {
        return Err(plan_source_error(
            "change_plan_file_prepared_length",
            "prepared-plan commitment must contain 64 lowercase hexadecimal characters",
        ));
    }
    Ok(PreparedPlanCommitment::from_bytes(decode_hex_32(encoded)?))
}

fn parse_identity_kind(value: &str) -> Result<IdentityKind, Diagnostic> {
    let kinds = [
        IdentityKind::Module,
        IdentityKind::Declaration,
        IdentityKind::TypeParameter,
        IdentityKind::Field,
        IdentityKind::Case,
        IdentityKind::Operation,
        IdentityKind::Parameter,
        IdentityKind::Binding,
        IdentityKind::Expression,
        IdentityKind::Requirement,
        IdentityKind::Port,
        IdentityKind::Target,
        IdentityKind::Documentation,
        IdentityKind::Annotation,
    ];
    kinds
        .into_iter()
        .find(|kind| identity_kind_name(*kind) == value)
        .ok_or_else(|| {
            plan_source_error(
                "change_plan_file_allocation_domain",
                format!("unknown logical allocation identity domain '{value}'"),
            )
        })
}

fn parse_u64(value: &str, label: &str) -> Result<u64, Diagnostic> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(plan_source_error(
            "change_plan_file_integer",
            format!("logical plan {label} is not a canonical unsigned integer"),
        ));
    }
    value.parse::<u64>().map_err(|_| {
        plan_resource_error(
            "change_plan_file_integer",
            format!("logical plan {label} exceeds the u64 counter domain"),
        )
    })
}

fn parse_bool(value: &str, label: &str) -> Result<bool, Diagnostic> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(plan_source_error(
            "change_plan_file_boolean",
            format!("logical plan {label} must be exactly true or false"),
        )),
    }
}

fn parse_optional<T>(present: &str, value: &str) -> Result<Option<T>, Diagnostic>
where
    T: FromStr<Err = Diagnostic>,
{
    if parse_bool(present, "optional value presence")? {
        if value.is_empty() {
            return Err(plan_source_error(
                "change_plan_file_optional",
                "present logical plan value is empty",
            ));
        }
        value.parse::<T>().map(Some)
    } else if value.is_empty() {
        Ok(None)
    } else {
        Err(plan_source_error(
            "change_plan_file_optional",
            "absent logical plan value is nonempty",
        ))
    }
}

struct RelationKindValue(RelationKind);

impl FromStr for RelationKindValue {
    type Err = Diagnostic;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        RelationKind::parse(value).map(Self)
    }
}

fn require_empty(record: &CompactRecord, indexes: &[usize], label: &str) -> Result<(), Diagnostic> {
    if indexes
        .iter()
        .any(|index| !field(record, *index).is_empty())
    {
        return Err(plan_source_error(
            "change_plan_file_optional",
            format!("logical plan {label} carries nonempty absent fields"),
        ));
    }
    Ok(())
}

fn increment(value: &mut u64, label: &str) -> Result<(), Diagnostic> {
    *value = value.checked_add(1).ok_or_else(|| {
        plan_resource_error(
            "change_plan_file_count_overflow",
            format!("logical plan {label} count overflowed"),
        )
    })?;
    Ok(())
}

fn require_owner_order(
    previous: Option<OwnerKey>,
    owner: OwnerKey,
    label: &str,
) -> Result<(), Diagnostic> {
    if previous.is_some_and(|previous| canonical_owner_cmp(previous, owner) != Ordering::Less) {
        return Err(plan_source_error(
            "change_plan_file_owner_order",
            format!("logical plan {label} is duplicated or out of canonical owner order"),
        ));
    }
    Ok(())
}

fn canonical_owner_cmp(left: OwnerKey, right: OwnerKey) -> Ordering {
    EncodedOwnerKey::new(left)
        .bytes()
        .cmp(&EncodedOwnerKey::new(right).bytes())
}

fn relation_cmp(left: &RelationEdge, right: &RelationEdge) -> Ordering {
    endpoint_cmp(left.source, right.source)
        .then_with(|| relation_kind_order(left.kind).cmp(&relation_kind_order(right.kind)))
        .then_with(|| endpoint_cmp(left.target, right.target))
}

fn endpoint_cmp(left: RelationEndpoint, right: RelationEndpoint) -> Ordering {
    match (left, right) {
        (RelationEndpoint::Owner(left), RelationEndpoint::Owner(right)) => left
            .package
            .cmp(&right.package)
            .then_with(|| canonical_owner_cmp(left.owner, right.owner)),
        (RelationEndpoint::Package(left), RelationEndpoint::Package(right)) => left.cmp(&right),
        (RelationEndpoint::Owner(_), RelationEndpoint::Package(_)) => Ordering::Less,
        (RelationEndpoint::Package(_), RelationEndpoint::Owner(_)) => Ordering::Greater,
    }
}

fn relation_kind_order(kind: RelationKind) -> usize {
    RelationKind::ALL
        .iter()
        .position(|candidate| *candidate == kind)
        .unwrap_or(usize::MAX)
}

fn validate_evidence(
    evidence: &LogicalChangePlanEvidence,
    dependencies: &[DependencyDiffEntry],
    retirements: &[RetirementDiffEntry],
    receipt: &crate::platform::publication::PublicationReceipt,
) -> Result<(), Diagnostic> {
    evidence.budget.validate()?;
    if evidence
        .allocations
        .windows(2)
        .any(|pair| allocation_key_cmp(&pair[0], &pair[1]) != Ordering::Less)
        || evidence
            .allocations
            .iter()
            .any(|allocation| allocation.domain != allocation.owner.identity_kind())
    {
        return Err(plan_corrupt(
            "change_logical_plan_allocations",
            "logical plan allocations are not unique in typed ordinal order",
        ));
    }
    if evidence
        .reasons
        .windows(2)
        .any(|pair| pair[0].logical_cmp(&pair[1]) != Ordering::Less)
    {
        return Err(plan_corrupt(
            "change_logical_plan_reasons",
            "logical plan impact reasons are not unique in canonical order",
        ));
    }
    if evidence
        .relations
        .removed
        .iter()
        .any(|edge| evidence.relations.added.contains(edge))
    {
        return Err(plan_corrupt(
            "change_logical_plan_relations",
            "one semantic relation is both removed and added",
        ));
    }
    if !strict_relations(&evidence.relations.removed)
        || !strict_relations(&evidence.relations.added)
        || !strict_owners(&evidence.structurally_checked)
        || !strict_owners(&evidence.semantically_checked)
        || !strict_owners(&evidence.tests)
    {
        return Err(plan_corrupt(
            "change_logical_plan_order",
            "logical relations or validation/test owners do not use canonical logical order",
        ));
    }
    if dependencies.len() != evidence.dependencies.len()
        || retirements.len() != evidence.retirements.len()
        || dependencies
            .iter()
            .any(|entry| !evidence.dependencies.contains_key(&entry.package))
        || retirements
            .iter()
            .any(|entry| !evidence.retirements.contains_key(&entry.owner))
    {
        return Err(plan_corrupt(
            "change_logical_plan_value_sets",
            "semantic diff and logical dependency or retirement values disagree",
        ));
    }
    for entry in dependencies {
        let values = evidence.dependencies.get(&entry.package).ok_or_else(|| {
            plan_corrupt(
                "change_logical_plan_dependency",
                "semantic diff dependency has no logical value projection",
            )
        })?;
        let before = values
            .before
            .as_ref()
            .map(encode_dependency)
            .transpose()?
            .map(|(digest, _)| digest);
        let after = values
            .after
            .as_ref()
            .map(encode_dependency)
            .transpose()?
            .map(|(digest, _)| digest);
        if before != entry.objects.before
            || after != entry.objects.after
            || values
                .before
                .as_ref()
                .is_some_and(|record| record.package != entry.package)
            || values
                .after
                .as_ref()
                .is_some_and(|record| record.package != entry.package)
        {
            return Err(plan_corrupt(
                "change_logical_plan_dependency",
                "logical dependency values disagree with the semantic diff",
            ));
        }
    }
    for entry in retirements {
        let values = evidence.retirements.get(&entry.owner).ok_or_else(|| {
            plan_corrupt(
                "change_logical_plan_retirement",
                "semantic diff retirement has no logical value projection",
            )
        })?;
        let before = values
            .before
            .as_ref()
            .map(encode_retirement)
            .transpose()?
            .map(|(digest, _)| digest);
        let after = values
            .after
            .as_ref()
            .map(encode_retirement)
            .transpose()?
            .map(|(digest, _)| digest);
        if before != entry.objects.before
            || after != entry.objects.after
            || values
                .before
                .as_ref()
                .is_some_and(|record| record.owner != entry.owner)
            || values
                .after
                .as_ref()
                .is_some_and(|record| record.owner != entry.owner)
        {
            return Err(plan_corrupt(
                "change_logical_plan_retirement",
                "logical retirement values disagree with the semantic diff",
            ));
        }
    }
    if receipt.validation.structurally_checked
        != u64::try_from(evidence.structurally_checked.len()).unwrap_or(u64::MAX)
        || receipt.validation.semantically_checked
            != u64::try_from(evidence.semantically_checked.len()).unwrap_or(u64::MAX)
        || receipt.validation.tests_selected
            != u64::try_from(evidence.tests.len()).unwrap_or(u64::MAX)
    {
        return Err(plan_corrupt(
            "change_logical_plan_validation_counts",
            "publication receipt counts disagree with exported validation or test membership",
        ));
    }
    Ok(())
}

fn strict_relations(relations: &BTreeSet<RelationEdge>) -> bool {
    relations
        .iter()
        .zip(relations.iter().skip(1))
        .all(|(left, right)| relation_cmp(left, right) == Ordering::Less)
}

fn strict_owners(owners: &BTreeSet<OwnerKey>) -> bool {
    owners
        .iter()
        .zip(owners.iter().skip(1))
        .all(|(left, right)| canonical_owner_cmp(*left, *right) == Ordering::Less)
}

fn allocation_key_cmp(left: &AuthoredAllocation, right: &AuthoredAllocation) -> Ordering {
    identity_kind_order(left.domain)
        .cmp(&identity_kind_order(right.domain))
        .then_with(|| left.ordinal.cmp(&right.ordinal))
}

pub(crate) const fn identity_kind_name(kind: IdentityKind) -> &'static str {
    match kind {
        IdentityKind::Module => "module",
        IdentityKind::Declaration => "declaration",
        IdentityKind::TypeParameter => "type_parameter",
        IdentityKind::Field => "field",
        IdentityKind::Case => "case",
        IdentityKind::Operation => "operation",
        IdentityKind::Parameter => "parameter",
        IdentityKind::Binding => "binding",
        IdentityKind::Expression => "expression",
        IdentityKind::Requirement => "requirement",
        IdentityKind::Port => "port",
        IdentityKind::Target => "target",
        IdentityKind::Documentation => "documentation",
        IdentityKind::Annotation => "annotation",
    }
}

const fn identity_kind_order(kind: IdentityKind) -> u8 {
    match kind {
        IdentityKind::Module => 1,
        IdentityKind::Declaration => 2,
        IdentityKind::TypeParameter => 3,
        IdentityKind::Field => 4,
        IdentityKind::Case => 5,
        IdentityKind::Operation => 6,
        IdentityKind::Parameter => 7,
        IdentityKind::Binding => 8,
        IdentityKind::Expression => 9,
        IdentityKind::Requirement => 10,
        IdentityKind::Port => 11,
        IdentityKind::Target => 12,
        IdentityKind::Documentation => 13,
        IdentityKind::Annotation => 14,
    }
}

fn count(value: usize) -> Result<String, Diagnostic> {
    u64::try_from(value)
        .map(|value| value.to_string())
        .map_err(|_| {
            plan_resource_error(
                "change_plan_output_record_budget",
                "logical plan fact count exceeds its canonical counter domain",
            )
        })
}

fn decode_hex_32(value: &str) -> Result<[u8; 32], Diagnostic> {
    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let high = lower_hex(pair[0]).ok_or_else(|| invalid_plan_hex(value))?;
        let low = lower_hex(pair[1]).ok_or_else(|| invalid_plan_hex(value))?;
        bytes[index] = (high << 4) | low;
    }
    Ok(bytes)
}

const fn lower_hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn invalid_plan_hex(value: &str) -> Diagnostic {
    plan_source_error(
        "change_plan_hex",
        format!("reviewed plan token component '{value}' is not canonical lowercase hexadecimal"),
    )
}

fn plan_source_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

fn plan_resource_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Resource, code, message)
}

fn plan_corrupt(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviewed_change_plan_token_is_strict_current_and_golden() {
        let token = ChangePlanToken {
            request: ChangeRequestCommitment::from_bytes([1; 32]),
            prepared: PreparedPlanCommitment::from_bytes([2; 32]),
        };
        let expected = format!("plan_{}{}", "01".repeat(32), "02".repeat(32));
        assert_eq!(token.to_string(), expected);
        assert_eq!(expected.len(), 133);
        assert_eq!(expected.parse::<ChangePlanToken>().unwrap(), token);
        for rejected in [
            format!("request_{}{}", "01".repeat(32), "02".repeat(32)),
            format!("plan_{}", "01".repeat(32)),
            format!("plan_{}0", "01".repeat(64)),
            expected.to_uppercase(),
            format!("{expected}0"),
        ] {
            assert!(rejected.parse::<ChangePlanToken>().is_err(), "{rejected}");
        }
    }

    #[test]
    fn reviewed_change_plan_every_body_field_changes_the_prepared_commitment() {
        for descriptor in &LOGICAL_PLAN_RECORD_DESCRIPTORS[..28] {
            for changed in 0..descriptor.fields.len() {
                let baseline = descriptor
                    .fields
                    .iter()
                    .map(|field| (*field, "a".to_owned()))
                    .collect::<Vec<_>>();
                let mut modified = baseline.clone();
                modified[changed].1 = "b".to_owned();
                assert_ne!(
                    one_record_commitment(descriptor.name, &baseline),
                    one_record_commitment(descriptor.name, &modified),
                    "{}.{}",
                    descriptor.name,
                    descriptor.fields[changed]
                );
            }
        }
    }

    #[test]
    fn reviewed_change_plan_commitment_hides_but_binds_internal_registry() {
        let mut first = blake3::Hasher::new_derive_key(PREPARED_CHANGE_PLAN_COMMITMENT_DOMAIN);
        let mut second = first.clone();
        bind_internal_registry(&mut first, &"a".repeat(64));
        bind_internal_registry(&mut second, &"b".repeat(64));
        assert_ne!(first.finalize(), second.finalize());
    }

    #[test]
    fn reviewed_change_plan_limits_cover_default_admission_and_maximum_records() {
        let budget = ChangeBudget::default();
        assert_eq!(
            budget.authored.maximum_allocated_identities,
            DEFAULT_ALLOCATIONS
        );
        assert_eq!(
            budget.canonical_edits.maximum_owner_edits,
            DEFAULT_OWNER_CHANGES
        );
        assert_eq!(
            budget.canonical_edits.maximum_type_edits,
            DEFAULT_TYPE_ADDITIONS
        );
        assert_eq!(
            budget.canonical_edits.maximum_dependency_edits,
            DEFAULT_DEPENDENCY_CHANGES
        );
        assert_eq!(
            budget.canonical_edits.maximum_retirement_edits,
            DEFAULT_RETIREMENT_CHANGES
        );
        assert_eq!(
            budget.impact.maximum_relation_edges,
            DEFAULT_RELATION_CHANGES
        );
        assert_eq!(
            budget.validation.maximum_owner_records,
            DEFAULT_STRUCTURAL_OWNERS
        );
        assert_eq!(
            budget.impact.maximum_affected_owners,
            DEFAULT_SEMANTIC_OWNERS
        );
        assert_eq!(budget.tests.maximum_selected, DEFAULT_SELECTED_TESTS);
        assert_eq!(
            DEFAULT_IMPACT_REASONS,
            budget
                .impact
                .maximum_relation_edges
                .saturating_add(budget.impact.maximum_affected_owners)
        );
        assert_eq!(MAXIMUM_LOGICAL_PLAN_RECORDS, 740_018);
        assert_eq!(MAXIMUM_LOGICAL_PLAN_BYTES, 303_377_368);

        let owner = format!("annotation_{}", "f".repeat(32));
        let owner_object = format!("owner_object_{}", "f".repeat(64));
        assert_record_maximum(
            "logical-plan.allocation",
            &[
                ("domain", "type_parameter"),
                ("ordinal", "100000"),
                ("owner", &owner),
            ],
            MAXIMUM_ALLOCATION_RECORD_BYTES,
        );
        let mut owner_fields = LOGICAL_PLAN_RECORD_DESCRIPTORS[17]
            .fields
            .iter()
            .map(|field| (*field, "false"))
            .collect::<Vec<_>>();
        owner_fields[0].1 = &owner;
        owner_fields[1].1 = "true";
        owner_fields[2].1 = &owner_object;
        owner_fields[3].1 = "true";
        owner_fields[4].1 = &owner_object;
        assert_record_maximum(
            "logical-plan.owner",
            &owner_fields,
            MAXIMUM_OWNER_RECORD_BYTES,
        );
        assert_record_maximum(
            "logical-plan.type-addition",
            &[("object", &format!("type_object_{}", "f".repeat(64)))],
            MAXIMUM_TYPE_RECORD_BYTES,
        );
        let package = format!("pkg_{}", "f".repeat(32));
        let dependency_object = format!("dependency_object_{}", "f".repeat(64));
        let revision = format!("rev_{}", "f".repeat(64));
        let package_revision = format!("package_revision_{}", "f".repeat(64));
        assert_record_maximum(
            "logical-plan.dependency",
            &[
                ("package", &package),
                ("before-present", "true"),
                ("before-object", &dependency_object),
                ("before-semantic-revision", &revision),
                ("before-package-revision", &package_revision),
                ("after-present", "true"),
                ("after-object", &dependency_object),
                ("after-semantic-revision", &revision),
                ("after-package-revision", &package_revision),
            ],
            MAXIMUM_DEPENDENCY_RECORD_BYTES,
        );
        let retirement_object = format!("retirement_object_{}", "f".repeat(64));
        let change = format!("change_{}", "f".repeat(64));
        let name = "x".repeat(crate::platform::kernel::contract::MAXIMUM_NAME_BYTES);
        assert_record_maximum(
            "logical-plan.retirement",
            &[
                ("owner", &owner),
                ("before-present", "true"),
                ("before-object", &retirement_object),
                ("before-kind", "type_parameter"),
                ("before-name-present", "true"),
                ("before-name", &name),
                ("before-parent-present", "true"),
                ("before-parent", &owner),
                ("before-live-revision", &revision),
                ("before-deletion-change", &change),
                ("after-present", "true"),
                ("after-object", &retirement_object),
                ("after-kind", "type_parameter"),
                ("after-name-present", "true"),
                ("after-name", &name),
                ("after-parent-present", "true"),
                ("after-parent", &owner),
                ("after-live-revision", &revision),
                ("after-deletion-change", &change),
            ],
            MAXIMUM_RETIREMENT_RECORD_BYTES,
        );
        assert_record_maximum(
            "logical-plan.relation-removed",
            &[
                ("source-package", &package),
                ("source-owner-present", "true"),
                ("source-owner", &owner),
                ("kind", "test_execution_dependency"),
                ("target-package", &package),
                ("target-owner-present", "true"),
                ("target-owner", &owner),
            ],
            MAXIMUM_RELATION_RECORD_BYTES,
        );
        assert_record_maximum(
            "logical-plan.validation-structural",
            &[("owner", &owner)],
            MAXIMUM_SELECTION_RECORD_BYTES,
        );
        assert_record_maximum(
            "logical-plan.reason",
            &[
                ("kind", "validation_dependency"),
                ("source", &owner),
                ("target", &owner),
                ("relation-present", "true"),
                ("relation", "test_execution_dependency"),
            ],
            MAXIMUM_REASON_RECORD_BYTES,
        );
        let worst_controls =
            "\u{7f}".repeat(crate::platform::publication::contract::MAXIMUM_INTENT_BYTES);
        assert_record_maximum(
            "logical-plan.controls",
            &[
                ("idempotency-present", "true"),
                (
                    "idempotency",
                    &"x".repeat(
                        crate::platform::publication::contract::MAXIMUM_IDEMPOTENCY_KEY_BYTES,
                    ),
                ),
                ("intent-present", "true"),
                ("intent", &worst_controls),
            ],
            24_794,
        );
        assert_eq!(maximum_fixed_record_bytes(), MAXIMUM_FIXED_RECORDS_BYTES);
    }

    fn one_record_commitment(operation: &str, fields: &[(&str, String)]) -> [u8; 32] {
        let borrowed = fields
            .iter()
            .map(|(name, value)| (*name, value.as_str()))
            .collect::<Vec<_>>();
        let record = render_record(operation, &borrowed).unwrap();
        let mut hasher = blake3::Hasher::new_derive_key(PREPARED_CHANGE_PLAN_COMMITMENT_DOMAIN);
        hasher.update(record.as_bytes());
        *hasher.finalize().as_bytes()
    }

    fn assert_record_maximum(operation: &str, fields: &[(&str, &str)], maximum: u64) {
        let record = render_record(operation, fields).unwrap();
        assert_eq!(u64::try_from(record.len()).unwrap(), maximum, "{operation}");
    }

    fn maximum_fixed_record_bytes() -> u64 {
        let mut total = 0_u64;
        let request = format!("request_{}", "f".repeat(64));
        {
            let mut add = |operation: &str, fields: &[(&str, &str)]| {
                total += u64::try_from(render_record(operation, fields).unwrap().len()).unwrap();
            };
            add(
                "logical-plan",
                &[
                    ("product", "lkjscript"),
                    ("version", crate::PRODUCT_VERSION),
                ],
            );
            add("logical-plan.capabilities", &[("digest", &"f".repeat(64))]);
            let repository = format!("repo_{}", "f".repeat(32));
            let package = format!("pkg_{}", "f".repeat(32));
            let revision = format!("rev_{}", "f".repeat(64));
            let state = format!("semantic_state_{}", "f".repeat(64));
            add(
                "logical-plan.authority",
                &[
                    ("repository", &repository),
                    ("package", &package),
                    ("base", &revision),
                    ("result", &revision),
                    ("semantic-state", &state),
                ],
            );
            add("logical-plan.request", &[("commitment", &request)]);
        }

        let mut budget_bytes = 0_u64;
        {
            let mut emit = |bytes: &[u8]| {
                budget_bytes += u64::try_from(bytes.len()).unwrap();
                Ok(())
            };
            let mut encoder = PlanEncoder::new(
                &mut emit,
                MAXIMUM_LOGICAL_PLAN_BYTES,
                MAXIMUM_LOGICAL_PLAN_RECORDS,
            );
            encode_budget(ChangeBudget::default(), &mut encoder).unwrap();
        }
        total += budget_bytes;

        let worst_controls =
            "\u{7f}".repeat(crate::platform::publication::contract::MAXIMUM_INTENT_BYTES);
        let idempotency =
            "x".repeat(crate::platform::publication::contract::MAXIMUM_IDEMPOTENCY_KEY_BYTES);
        let mut add = |operation: &str, fields: &[(&str, &str)]| {
            total += u64::try_from(render_record(operation, fields).unwrap().len()).unwrap();
        };
        add(
            "logical-plan.controls",
            &[
                ("idempotency-present", "true"),
                ("idempotency", &idempotency),
                ("intent-present", "true"),
                ("intent", &worst_controls),
            ],
        );
        let transaction = format!("transaction_{}", "f".repeat(64));
        let semantic_diff = format!("semantic_diff_{}", "f".repeat(64));
        add(
            "logical-plan.binding",
            &[
                ("transaction", &transaction),
                ("semantic-diff", &semantic_diff),
            ],
        );
        let six_digits = "999999";
        add(
            "logical-plan.counts",
            &LOGICAL_PLAN_RECORD_DESCRIPTORS[27]
                .fields
                .iter()
                .map(|field| (*field, six_digits))
                .collect::<Vec<_>>(),
        );
        let prepared = format!("prepared_{}", "f".repeat(64));
        let token = format!("plan_{}", "f".repeat(128));
        add(
            "logical-plan.digest",
            &[
                ("request", &request),
                ("prepared", &prepared),
                ("token", &token),
            ],
        );
        total
    }
}
