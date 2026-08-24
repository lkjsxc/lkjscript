//! Stable-owner semantic diffs for Graph 5 accepted history.

use super::contract::{
    MAXIMUM_INLINE_HISTORY_EDITS, MAXIMUM_SEMANTIC_DIFF_BYTES, SEMANTIC_DIFF_CONTRACT_VERSION,
    SEMANTIC_DIFF_ENVELOPE_DOMAIN, SEMANTIC_DIFF_MAGIC,
};
use super::digest::SemanticDiffDigest;
use super::transaction::DigestEdit;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DependencyObjectDigest, OwnerKey, OwnerObjectDigest, PackageId, RetirementObjectDigest,
    SemanticRootDigest, TypeObjectDigest,
};
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Decode, Default, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SummaryDimensions {
    pub semantic_interface: bool,
    pub implementation: bool,
    pub type_digest: bool,
    pub effect: bool,
    pub capability: bool,
    pub relations: bool,
    pub presentation: bool,
    pub test: bool,
    pub validation_dependencies: bool,
}

impl SummaryDimensions {
    pub const fn any(self) -> bool {
        self.semantic_interface
            || self.implementation
            || self.type_digest
            || self.effect
            || self.capability
            || self.relations
            || self.presentation
            || self.test
            || self.validation_dependencies
    }

    pub const fn executable(self) -> bool {
        self.semantic_interface
            || self.implementation
            || self.type_digest
            || self.effect
            || self.capability
            || self.relations
            || self.test
    }
}

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum OwnerChangeClass {
    Created,
    Deleted,
    Renamed,
    Moved,
    VisibilityChanged,
    PresentationOnly,
    TestOnly,
    PrivateImplementation,
    PublicInterface,
    EffectOrCapability,
    RelationSet,
    SemanticPayloadChanged,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerDiffEntry {
    pub owner: OwnerKey,
    pub objects: DigestEdit<OwnerObjectDigest>,
    pub classes: Vec<OwnerChangeClass>,
    pub dimensions: SummaryDimensions,
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyDiffEntry {
    pub package: PackageId,
    pub objects: DigestEdit<DependencyObjectDigest>,
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetirementDiffEntry {
    pub owner: OwnerKey,
    pub objects: DigestEdit<RetirementObjectDigest>,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SemanticDiffBody {
    Bootstrap {
        result_root: SemanticRootDigest,
        owners: u64,
        dependencies: u64,
        retirements: u64,
    },
    Change {
        base: RevisionId,
        base_root: SemanticRootDigest,
        result_root: SemanticRootDigest,
        owners: Vec<OwnerDiffEntry>,
        type_additions: Vec<TypeObjectDigest>,
        dependencies: Vec<DependencyDiffEntry>,
        retirements: Vec<RetirementDiffEntry>,
    },
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticDiff {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub repository_id: RepositoryId,
    pub body: SemanticDiffBody,
}

impl SemanticDiff {
    pub fn encode(&self) -> Result<(SemanticDiffDigest, Vec<u8>), Diagnostic> {
        self.validate()?;
        let bytes = crate::platform::packed::encode(
            SEMANTIC_DIFF_MAGIC,
            SEMANTIC_DIFF_ENVELOPE_DOMAIN,
            self,
            MAXIMUM_SEMANTIC_DIFF_BYTES,
        )?;
        Ok((SemanticDiffDigest::of(&bytes), bytes))
    }

    pub fn decode(bytes: &[u8], expected: SemanticDiffDigest) -> Result<Self, Diagnostic> {
        if SemanticDiffDigest::of(bytes) != expected {
            return Err(diff_error(
                DiagnosticClass::Corrupt,
                "publication_diff_digest",
                "semantic diff bytes disagree with their object digest",
            ));
        }
        let value: Self = crate::platform::packed::decode(
            bytes,
            SEMANTIC_DIFF_MAGIC,
            SEMANTIC_DIFF_ENVELOPE_DOMAIN,
            MAXIMUM_SEMANTIC_DIFF_BYTES,
        )?;
        value.validate()?;
        if value.encode()?.1 != bytes {
            return Err(diff_error(
                DiagnosticClass::Corrupt,
                "publication_diff_canonical",
                "semantic diff object is not canonically encoded",
            ));
        }
        Ok(value)
    }

    pub fn result_root(&self) -> SemanticRootDigest {
        match &self.body {
            SemanticDiffBody::Bootstrap { result_root, .. }
            | SemanticDiffBody::Change { result_root, .. } => *result_root,
        }
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != SEMANTIC_DIFF_CONTRACT_VERSION
            || self.graph_contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
        {
            return Err(diff_error(
                DiagnosticClass::Source,
                "publication_diff_contract",
                "semantic diff uses a predecessor or foreign contract",
            ));
        }
        let SemanticDiffBody::Change {
            base_root,
            result_root,
            owners,
            type_additions,
            dependencies,
            retirements,
            ..
        } = &self.body
        else {
            return Ok(());
        };
        if base_root == result_root {
            return Err(diff_error(
                DiagnosticClass::Corrupt,
                "publication_diff_no_change",
                "accepted semantic diff has equal before and after roots",
            ));
        }
        let entries = owners
            .len()
            .checked_add(type_additions.len())
            .and_then(|count| count.checked_add(dependencies.len()))
            .and_then(|count| count.checked_add(retirements.len()))
            .ok_or_else(|| {
                diff_error(
                    DiagnosticClass::Resource,
                    "publication_diff_entry_count",
                    "semantic diff entry count overflows this platform",
                )
            })?;
        if entries == 0 || entries > MAXIMUM_INLINE_HISTORY_EDITS {
            return Err(diff_error(
                DiagnosticClass::Resource,
                "publication_diff_entry_count",
                format!(
                    "inline semantic diff contains {entries} entries; current bound is {MAXIMUM_INLINE_HISTORY_EDITS}"
                ),
            ));
        }
        validate_sorted(owners, |entry| entry.owner, "owner")?;
        validate_sorted(type_additions, |digest| *digest, "type addition")?;
        validate_sorted(dependencies, |entry| entry.package, "dependency")?;
        validate_sorted(retirements, |entry| entry.owner, "retirement")?;
        for entry in owners {
            validate_owner_entry(entry)?;
        }
        for entry in dependencies {
            validate_digest_edit(entry.objects, "dependency")?;
        }
        for entry in retirements {
            validate_digest_edit(entry.objects, "retirement")?;
        }
        Ok(())
    }
}

fn validate_owner_entry(entry: &OwnerDiffEntry) -> Result<(), Diagnostic> {
    if matches!((entry.objects.before, entry.objects.after), (None, None)) {
        return Err(diff_error(
            DiagnosticClass::Corrupt,
            "publication_diff_edit",
            "owner diff entry has neither a before nor an after binding",
        ));
    }
    if entry.classes.is_empty() || entry.classes.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(diff_error(
            DiagnosticClass::Corrupt,
            "publication_diff_owner_classes",
            "owner diff classes are empty, duplicated, or noncanonical",
        ));
    }
    let created = entry.objects.before.is_none();
    let deleted = entry.objects.after.is_none();
    if entry.classes.contains(&OwnerChangeClass::Created) != created
        || entry.classes.contains(&OwnerChangeClass::Deleted) != deleted
    {
        return Err(diff_error(
            DiagnosticClass::Corrupt,
            "publication_diff_owner_lifecycle",
            "owner lifecycle class disagrees with before and after bindings",
        ));
    }
    if (created || deleted)
        && entry.classes.iter().any(|class| {
            matches!(
                class,
                OwnerChangeClass::Renamed
                    | OwnerChangeClass::Moved
                    | OwnerChangeClass::VisibilityChanged
            )
        })
    {
        return Err(diff_error(
            DiagnosticClass::Corrupt,
            "publication_diff_owner_transition",
            "created or deleted owner claims a live-to-live rename, move, or visibility change",
        ));
    }
    for class in &entry.classes {
        let consistent = match class {
            OwnerChangeClass::Created | OwnerChangeClass::Deleted => true,
            OwnerChangeClass::Renamed => entry.dimensions.presentation,
            OwnerChangeClass::Moved | OwnerChangeClass::VisibilityChanged => !created && !deleted,
            OwnerChangeClass::PresentationOnly => {
                entry.dimensions.presentation && !entry.dimensions.executable()
            }
            OwnerChangeClass::TestOnly => {
                entry.dimensions.test
                    && !entry.dimensions.semantic_interface
                    && !entry.dimensions.implementation
                    && !entry.dimensions.type_digest
                    && !entry.dimensions.effect
                    && !entry.dimensions.capability
            }
            OwnerChangeClass::PrivateImplementation => {
                entry.dimensions.implementation && !entry.dimensions.semantic_interface
            }
            OwnerChangeClass::PublicInterface => entry.dimensions.semantic_interface,
            OwnerChangeClass::EffectOrCapability => {
                entry.dimensions.effect || entry.dimensions.capability
            }
            OwnerChangeClass::RelationSet => {
                entry.dimensions.relations || entry.dimensions.validation_dependencies
            }
            OwnerChangeClass::SemanticPayloadChanged => {
                entry.dimensions.executable() || created || deleted
            }
        };
        if !consistent {
            return Err(diff_error(
                DiagnosticClass::Corrupt,
                "publication_diff_owner_class_dimensions",
                "owner diff class disagrees with its exact summary dimensions",
            ));
        }
    }
    if !created
        && !deleted
        && entry.objects.before == entry.objects.after
        && !entry.dimensions.any()
        && !entry.classes.contains(&OwnerChangeClass::Moved)
    {
        return Err(diff_error(
            DiagnosticClass::Corrupt,
            "publication_diff_owner_empty",
            "unchanged owner binding has no derived summary or ownership change",
        ));
    }
    Ok(())
}

fn validate_digest_edit<D: Copy + Eq>(edit: DigestEdit<D>, label: &str) -> Result<(), Diagnostic> {
    if matches!((edit.before, edit.after), (None, None)) || edit.before == edit.after {
        return Err(diff_error(
            DiagnosticClass::Corrupt,
            "publication_diff_edit",
            format!("{label} diff entry is empty or unchanged"),
        ));
    }
    Ok(())
}

fn validate_sorted<T, K: Ord>(
    values: &[T],
    key: impl Fn(&T) -> K,
    label: &str,
) -> Result<(), Diagnostic> {
    if values.windows(2).any(|pair| key(&pair[0]) >= key(&pair[1])) {
        return Err(diff_error(
            DiagnosticClass::Corrupt,
            "publication_diff_order",
            format!("{label} diff entries are not unique and canonically ordered"),
        ));
    }
    Ok(())
}

fn diff_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
