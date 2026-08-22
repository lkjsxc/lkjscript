//! Content-bound, rebuildable semantic summaries for local invalidation.
//!
//! Module summaries are exactly reusable across revisions while their canonical module input and
//! validator contract remain unchanged. Reverse dependency indexes bind one accepted revision.
//! Neither record authorizes accepted meaning, and callers must rebuild derived state from
//! canonical modules after any contract or integrity mismatch.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::language::Declaration;
use super::meaning::{
    DeclarationIdentity, DeclarationKind, MeaningModule, RelationRole, RelationSource,
    RelationTarget,
};
use super::package::PackageId;
use super::packed;
use super::semantic_digest::ModuleObjectDigest;
use super::semantic_id::{DeclarationId, ModuleId, RevisionId};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

pub const SEMANTIC_SUMMARY_CONTRACT_VERSION: u16 = 2;
pub const SEMANTIC_SUMMARY_CONTRACT_IDENTITY: &str = "lkjscript-semantic-summary-2";
pub const SEMANTIC_VALIDATOR_CONTRACT_IDENTITY: &str = "lkjscript-semantic-validator-2";

// These are hostile-decoder and single-object implementation bounds, not semantic project limits.
// The reverse index must be physically sharded before these values constrain a legitimate graph.
const MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES: usize = 16 * 1_048_576;
const MAXIMUM_REVERSE_INDEX_PAYLOAD_BYTES: usize = 64 * 1_048_576;
pub const MAXIMUM_MODULE_SUMMARY_ENCODED_BYTES: usize =
    MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES + PACKED_ENVELOPE_BYTES;
pub const MAXIMUM_REVERSE_INDEX_ENCODED_BYTES: usize =
    MAXIMUM_REVERSE_INDEX_PAYLOAD_BYTES + PACKED_ENVELOPE_BYTES;
const MAXIMUM_SUMMARY_DECLARATIONS: usize = 100_000;
const MAXIMUM_SUMMARY_DEPENDENCIES: usize = 2_000_000;
const MAXIMUM_INDEX_MODULES: usize = 1_000_000;
const MAXIMUM_INDEX_ENTRIES: usize = 2_000_000;
const MAXIMUM_INDEX_DEPENDENTS: usize = 8_000_000;
const PACKED_ENVELOPE_BYTES: usize = 8 + 2 + 8 + 32;

const SUMMARY_MAGIC: [u8; 8] = *b"LKJSUM02";
const SUMMARY_ENVELOPE_DOMAIN: &str = "lkjscript.semantic-summary-envelope.v2";
const INDEX_MAGIC: [u8; 8] = *b"LKJRDI02";
const INDEX_ENVELOPE_DOMAIN: &str = "lkjscript.reverse-dependency-index-envelope.v2";
const VALIDATOR_DIGEST_DOMAIN: &str = "lkjscript.semantic-validator-contract.v2";
const SUMMARY_INPUT_DIGEST_DOMAIN: &str = "lkjscript.semantic-summary-input.v2";
const PUBLIC_SIGNATURE_DIGEST_DOMAIN: &str = "lkjscript.public-signature-summary.v2";
const DECLARATION_SIGNATURE_DIGEST_DOMAIN: &str = "lkjscript.declaration-signature.v2";
const DECLARATION_IMPLEMENTATION_DIGEST_DOMAIN: &str = "lkjscript.declaration-implementation.v2";
const DECLARATION_EFFECT_DIGEST_DOMAIN: &str = "lkjscript.declaration-effect.v2";
const MODULE_IMPLEMENTATION_DIGEST_DOMAIN: &str = "lkjscript.module-implementation.v2";
const DEPENDENCY_DIGEST_DOMAIN: &str = "lkjscript.semantic-summary-dependencies.v2";
const SUMMARY_RECORD_DIGEST_DOMAIN: &str = "lkjscript.semantic-summary-record.v2";
const INDEX_INPUT_DIGEST_DOMAIN: &str = "lkjscript.reverse-dependency-index-input.v2";
const CERTIFICATE_DIGEST_DOMAIN: &str = "lkjscript.semantic-certificate.v2";
const INDEX_RECORD_DIGEST_DOMAIN: &str = "lkjscript.reverse-dependency-index-record.v2";

#[derive(
    Decode, Encode, Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(transparent)]
pub struct SemanticSummaryDigest([u8; 32]);

impl SemanticSummaryDigest {
    const ZERO: Self = Self([0; 32]);

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for SemanticSummaryDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("summary_")?;
        formatter.write_str(&super::semantic_id::encode_hex(&self.0))
    }
}

#[derive(
    Decode, Encode, Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SummaryDependencyKind {
    Namespace,
    Type,
    Value,
    Call,
    Effect,
    Capability,
    Deployment,
    Test,
}

const ALL_DEPENDENCY_KINDS: [SummaryDependencyKind; 8] = [
    SummaryDependencyKind::Namespace,
    SummaryDependencyKind::Type,
    SummaryDependencyKind::Value,
    SummaryDependencyKind::Call,
    SummaryDependencyKind::Effect,
    SummaryDependencyKind::Capability,
    SummaryDependencyKind::Deployment,
    SummaryDependencyKind::Test,
];

const EXECUTION_DEPENDENCY_KINDS: [SummaryDependencyKind; 3] = [
    SummaryDependencyKind::Value,
    SummaryDependencyKind::Call,
    SummaryDependencyKind::Effect,
];
const NAMESPACE_DEPENDENCIES: [SummaryDependencyKind; 1] = [SummaryDependencyKind::Namespace];
const TYPE_DEPENDENCIES: [SummaryDependencyKind; 1] = [SummaryDependencyKind::Type];
const VALUE_DEPENDENCIES: [SummaryDependencyKind; 1] = [SummaryDependencyKind::Value];
const CALL_DEPENDENCIES: [SummaryDependencyKind; 2] =
    [SummaryDependencyKind::Call, SummaryDependencyKind::Effect];
const CAPABILITY_DEPENDENCIES: [SummaryDependencyKind; 2] = [
    SummaryDependencyKind::Effect,
    SummaryDependencyKind::Capability,
];
const DEPLOYMENT_DEPENDENCIES: [SummaryDependencyKind; 1] = [SummaryDependencyKind::Deployment];
const TEST_DEPENDENCIES: [SummaryDependencyKind; 1] = [SummaryDependencyKind::Test];

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SummaryOwner {
    pub module: ModuleId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration: Option<DeclarationId>,
}

impl SummaryOwner {
    pub const fn module(module: ModuleId) -> Self {
        Self {
            module,
            declaration: None,
        }
    }

    pub const fn declaration(module: ModuleId, declaration: DeclarationId) -> Self {
        Self {
            module,
            declaration: Some(declaration),
        }
    }

    fn target(&self, package: &PackageId) -> DependencyTarget {
        DependencyTarget {
            package: package.clone(),
            module: self.module,
            declaration: self.declaration,
        }
    }
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyTarget {
    pub package: PackageId,
    pub module: ModuleId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration: Option<DeclarationId>,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SummaryDependency {
    pub source: SummaryOwner,
    pub target: DependencyTarget,
    pub kind: SummaryDependencyKind,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeclarationSemanticSummary {
    pub declaration: DeclarationId,
    pub kind: DeclarationKind,
    pub exported: bool,
    pub signature: SemanticSummaryDigest,
    pub implementation: SemanticSummaryDigest,
    pub effect: SemanticSummaryDigest,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleSemanticSummary {
    pub contract_version: u16,
    pub validator_contract: SemanticSummaryDigest,
    pub package: PackageId,
    pub module: ModuleId,
    pub module_object: ModuleObjectDigest,
    pub input: SemanticSummaryDigest,
    pub public_signature: SemanticSummaryDigest,
    pub implementation: SemanticSummaryDigest,
    pub dependency_digest: SemanticSummaryDigest,
    pub declarations: Vec<DeclarationSemanticSummary>,
    pub dependencies: Vec<SummaryDependency>,
    pub digest: SemanticSummaryDigest,
}

#[derive(Encode)]
struct ModuleSummaryCore<'a> {
    contract_version: u16,
    validator_contract: SemanticSummaryDigest,
    package: &'a PackageId,
    module: ModuleId,
    module_object: ModuleObjectDigest,
    input: SemanticSummaryDigest,
    public_signature: SemanticSummaryDigest,
    implementation: SemanticSummaryDigest,
    dependency_digest: SemanticSummaryDigest,
    declarations: &'a [DeclarationSemanticSummary],
    dependencies: &'a [SummaryDependency],
}

impl<'a> From<&'a ModuleSemanticSummary> for ModuleSummaryCore<'a> {
    fn from(summary: &'a ModuleSemanticSummary) -> Self {
        Self {
            contract_version: summary.contract_version,
            validator_contract: summary.validator_contract,
            package: &summary.package,
            module: summary.module,
            module_object: summary.module_object,
            input: summary.input,
            public_signature: summary.public_signature,
            implementation: summary.implementation,
            dependency_digest: summary.dependency_digest,
            declarations: &summary.declarations,
            dependencies: &summary.dependencies,
        }
    }
}

impl ModuleSemanticSummary {
    pub fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate()?;
        packed::encode(
            SUMMARY_MAGIC,
            SUMMARY_ENVELOPE_DOMAIN,
            self,
            MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
        )
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        reject_oversized_envelope(bytes, MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES, "summary")?;
        let summary: Self = packed::decode(
            bytes,
            SUMMARY_MAGIC,
            SUMMARY_ENVELOPE_DOMAIN,
            MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
        )?;
        summary.validate()?;
        let canonical = packed::encode(
            SUMMARY_MAGIC,
            SUMMARY_ENVELOPE_DOMAIN,
            &summary,
            MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
        )?;
        if canonical != bytes {
            return Err(summary_error(
                DiagnosticClass::Corrupt,
                "semantic_summary_noncanonical",
                "semantic summary bytes are not canonical",
            ));
        }
        Ok(summary)
    }

    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != SEMANTIC_SUMMARY_CONTRACT_VERSION {
            return Err(summary_error(
                DiagnosticClass::Source,
                "semantic_summary_contract",
                format!(
                    "semantic summary contract {} is not current contract {SEMANTIC_SUMMARY_CONTRACT_VERSION}",
                    self.contract_version
                ),
            ));
        }
        if self.validator_contract != validator_contract_digest() {
            return Err(summary_error(
                DiagnosticClass::Source,
                "semantic_summary_validator_contract",
                "semantic summary belongs to a different validator contract",
            ));
        }
        if self.declarations.len() > MAXIMUM_SUMMARY_DECLARATIONS
            || self.dependencies.len() > MAXIMUM_SUMMARY_DEPENDENCIES
        {
            return Err(summary_error(
                DiagnosticClass::Resource,
                "semantic_summary_item_limit",
                "semantic summary exceeds its single-object decoder budget",
            ));
        }
        if !strictly_sorted_by(&self.declarations, |value| value.declaration) {
            return Err(summary_error(
                DiagnosticClass::Corrupt,
                "semantic_summary_declaration_order",
                "semantic summary declarations must be sorted and unique by stable identity",
            ));
        }
        if !strictly_sorted(&self.dependencies) {
            return Err(summary_error(
                DiagnosticClass::Corrupt,
                "semantic_summary_dependency_order",
                "semantic summary dependencies must be sorted and unique",
            ));
        }
        let declarations = self
            .declarations
            .iter()
            .map(|value| value.declaration)
            .collect::<BTreeSet<_>>();
        for dependency in &self.dependencies {
            if dependency.source.module != self.module
                || dependency
                    .source
                    .declaration
                    .is_some_and(|id| !declarations.contains(&id))
            {
                return Err(summary_error(
                    DiagnosticClass::Corrupt,
                    "semantic_summary_dependency_source",
                    "semantic summary dependency has a foreign or missing source owner",
                ));
            }
        }
        let expected_input = summary_input_digest(
            &self.package,
            self.module,
            self.module_object,
            self.validator_contract,
        )?;
        if self.input != expected_input {
            return Err(summary_error(
                DiagnosticClass::Corrupt,
                "semantic_summary_input_digest",
                "semantic summary input digest does not bind its exact inputs",
            ));
        }
        let expected_implementation = module_implementation_digest(self.module_object)?;
        if self.implementation != expected_implementation {
            return Err(summary_error(
                DiagnosticClass::Corrupt,
                "semantic_summary_implementation_digest",
                "semantic summary implementation digest is inconsistent",
            ));
        }
        let expected_public = module_public_signature_digest(self.module, &self.declarations)?;
        if self.public_signature != expected_public {
            return Err(summary_error(
                DiagnosticClass::Corrupt,
                "semantic_summary_public_digest",
                "semantic summary public-signature digest is inconsistent",
            ));
        }
        let expected_dependencies = hash_encoded(
            DEPENDENCY_DIGEST_DOMAIN,
            &self.dependencies,
            MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
            "semantic summary dependencies",
        )?;
        if self.dependency_digest != expected_dependencies {
            return Err(summary_error(
                DiagnosticClass::Corrupt,
                "semantic_summary_dependency_digest",
                "semantic summary dependency digest is inconsistent",
            ));
        }
        let expected_digest = hash_encoded(
            SUMMARY_RECORD_DIGEST_DOMAIN,
            &ModuleSummaryCore::from(self),
            MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
            "semantic summary record",
        )?;
        if self.digest != expected_digest {
            return Err(summary_error(
                DiagnosticClass::Corrupt,
                "semantic_summary_record_digest",
                "semantic summary record digest is inconsistent",
            ));
        }
        Ok(())
    }

    pub fn verify_against(
        &self,
        package: &PackageId,
        module: &MeaningModule,
    ) -> Result<(), Diagnostic> {
        let rebuilt = build_module_summary(package, module)?;
        if &rebuilt != self {
            return Err(summary_error(
                DiagnosticClass::Corrupt,
                "semantic_summary_rebuild_mismatch",
                "semantic summary does not equal an independent rebuild from canonical meaning",
            ));
        }
        Ok(())
    }

    pub fn dependencies_of_kind(
        &self,
        kind: SummaryDependencyKind,
    ) -> impl Iterator<Item = &SummaryDependency> {
        self.dependencies
            .iter()
            .filter(move |dependency| dependency.kind == kind)
    }
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleSummaryBinding {
    pub module: ModuleId,
    pub input: SemanticSummaryDigest,
    pub summary: SemanticSummaryDigest,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReverseDependencyEntry {
    pub target: DependencyTarget,
    pub kind: SummaryDependencyKind,
    pub dependents: Vec<SummaryOwner>,
}

#[derive(Decode, Encode, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReverseDependencyIndex {
    pub contract_version: u16,
    pub validator_contract: SemanticSummaryDigest,
    pub package: PackageId,
    pub revision: RevisionId,
    pub summaries: Vec<ModuleSummaryBinding>,
    pub tests: Vec<SummaryOwner>,
    pub entries: Vec<ReverseDependencyEntry>,
    /// Revision-independent digest of the exact validated summary facts. Accepted revision
    /// records bind this value so disposable cache bytes can be authenticated without becoming
    /// program authority.
    pub certificate: SemanticSummaryDigest,
    pub input: SemanticSummaryDigest,
    pub digest: SemanticSummaryDigest,
}

#[derive(Encode)]
struct SemanticCertificateCore<'a> {
    contract_version: u16,
    validator_contract: SemanticSummaryDigest,
    package: &'a PackageId,
    summaries: &'a [ModuleSummaryBinding],
    tests: &'a [SummaryOwner],
    entries: &'a [ReverseDependencyEntry],
}

#[derive(Encode)]
struct ReverseIndexCore<'a> {
    contract_version: u16,
    validator_contract: SemanticSummaryDigest,
    package: &'a PackageId,
    revision: RevisionId,
    summaries: &'a [ModuleSummaryBinding],
    tests: &'a [SummaryOwner],
    entries: &'a [ReverseDependencyEntry],
    certificate: SemanticSummaryDigest,
    input: SemanticSummaryDigest,
}

impl<'a> From<&'a ReverseDependencyIndex> for ReverseIndexCore<'a> {
    fn from(index: &'a ReverseDependencyIndex) -> Self {
        Self {
            contract_version: index.contract_version,
            validator_contract: index.validator_contract,
            package: &index.package,
            revision: index.revision,
            summaries: &index.summaries,
            tests: &index.tests,
            entries: &index.entries,
            certificate: index.certificate,
            input: index.input,
        }
    }
}

impl ReverseDependencyIndex {
    pub fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate()?;
        packed::encode(
            INDEX_MAGIC,
            INDEX_ENVELOPE_DOMAIN,
            self,
            MAXIMUM_REVERSE_INDEX_PAYLOAD_BYTES,
        )
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        reject_oversized_envelope(bytes, MAXIMUM_REVERSE_INDEX_PAYLOAD_BYTES, "index")?;
        let index: Self = packed::decode(
            bytes,
            INDEX_MAGIC,
            INDEX_ENVELOPE_DOMAIN,
            MAXIMUM_REVERSE_INDEX_PAYLOAD_BYTES,
        )?;
        index.validate()?;
        let canonical = packed::encode(
            INDEX_MAGIC,
            INDEX_ENVELOPE_DOMAIN,
            &index,
            MAXIMUM_REVERSE_INDEX_PAYLOAD_BYTES,
        )?;
        if canonical != bytes {
            return Err(summary_error(
                DiagnosticClass::Corrupt,
                "semantic_summary_index_noncanonical",
                "reverse dependency index bytes are not canonical",
            ));
        }
        Ok(index)
    }

    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != SEMANTIC_SUMMARY_CONTRACT_VERSION {
            return Err(summary_error(
                DiagnosticClass::Source,
                "semantic_summary_index_contract",
                "reverse dependency index belongs to a different summary contract",
            ));
        }
        if self.validator_contract != validator_contract_digest() {
            return Err(summary_error(
                DiagnosticClass::Source,
                "semantic_summary_index_validator",
                "reverse dependency index belongs to a different validator contract",
            ));
        }
        if self.summaries.is_empty() || self.summaries.len() > MAXIMUM_INDEX_MODULES {
            return Err(summary_error(
                DiagnosticClass::Resource,
                "semantic_summary_index_module_limit",
                "reverse dependency index module count is outside its single-object budget",
            ));
        }
        if self.entries.len() > MAXIMUM_INDEX_ENTRIES {
            return Err(summary_error(
                DiagnosticClass::Resource,
                "semantic_summary_index_entry_limit",
                "reverse dependency index exceeds its single-object entry budget",
            ));
        }
        if !strictly_sorted_by(&self.summaries, |value| value.module)
            || !strictly_sorted(&self.tests)
        {
            return Err(summary_error(
                DiagnosticClass::Corrupt,
                "semantic_summary_index_owner_order",
                "reverse dependency index owners must be sorted and unique",
            ));
        }
        let modules = self
            .summaries
            .iter()
            .map(|binding| binding.module)
            .collect::<BTreeSet<_>>();
        for test in &self.tests {
            if !modules.contains(&test.module) || test.declaration.is_none() {
                return Err(summary_error(
                    DiagnosticClass::Corrupt,
                    "semantic_summary_index_test_owner",
                    "reverse dependency index has a foreign or module-level test owner",
                ));
            }
        }
        let mut previous_key: Option<(&DependencyTarget, SummaryDependencyKind)> = None;
        let mut dependent_count = 0usize;
        for entry in &self.entries {
            let key = (&entry.target, entry.kind);
            if previous_key.is_some_and(|previous| previous >= key) {
                return Err(summary_error(
                    DiagnosticClass::Corrupt,
                    "semantic_summary_index_entry_order",
                    "reverse dependency entries must be sorted and unique by target and role",
                ));
            }
            previous_key = Some(key);
            if entry.dependents.is_empty() || !strictly_sorted(&entry.dependents) {
                return Err(summary_error(
                    DiagnosticClass::Corrupt,
                    "semantic_summary_index_dependent_order",
                    "reverse dependency entry dependents must be nonempty, sorted, and unique",
                ));
            }
            dependent_count = dependent_count
                .checked_add(entry.dependents.len())
                .ok_or_else(index_work_exhausted)?;
            if dependent_count > MAXIMUM_INDEX_DEPENDENTS {
                return Err(index_work_exhausted());
            }
            if entry
                .dependents
                .iter()
                .any(|owner| !modules.contains(&owner.module))
            {
                return Err(summary_error(
                    DiagnosticClass::Corrupt,
                    "semantic_summary_index_dependent_owner",
                    "reverse dependency entry has a dependent outside the indexed modules",
                ));
            }
        }
        let expected_certificate = semantic_certificate_digest(
            &self.package,
            self.validator_contract,
            &self.summaries,
            &self.tests,
            &self.entries,
        )?;
        if self.certificate != expected_certificate {
            return Err(summary_error(
                DiagnosticClass::Corrupt,
                "semantic_summary_certificate_digest",
                "semantic certificate does not bind its exact summary facts",
            ));
        }
        let expected_input = reverse_index_input_digest(
            &self.package,
            self.revision,
            self.validator_contract,
            &self.summaries,
        )?;
        if self.input != expected_input {
            return Err(summary_error(
                DiagnosticClass::Corrupt,
                "semantic_summary_index_input_digest",
                "reverse dependency index does not bind its exact summary inputs",
            ));
        }
        let expected_digest = hash_encoded(
            INDEX_RECORD_DIGEST_DOMAIN,
            &ReverseIndexCore::from(self),
            MAXIMUM_REVERSE_INDEX_PAYLOAD_BYTES,
            "reverse dependency index",
        )?;
        if self.digest != expected_digest {
            return Err(summary_error(
                DiagnosticClass::Corrupt,
                "semantic_summary_index_record_digest",
                "reverse dependency index record digest is inconsistent",
            ));
        }
        Ok(())
    }

    pub fn dependents(
        &self,
        target: &DependencyTarget,
        kind: SummaryDependencyKind,
    ) -> &[SummaryOwner] {
        match self
            .entries
            .binary_search_by(|entry| (&entry.target, entry.kind).cmp(&(target, kind)))
        {
            Ok(index) => &self.entries[index].dependents,
            Err(_) => &[],
        }
    }

    pub fn contains_test(&self, owner: &SummaryOwner) -> bool {
        self.tests.binary_search(owner).is_ok()
    }

    pub fn modules(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.summaries.iter().map(|binding| binding.module)
    }
}

#[derive(Decode, Encode, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleInvalidationClass {
    Unchanged,
    PrivateImplementation,
    PublicSignature,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvalidationFrontier {
    pub class: ModuleInvalidationClass,
    pub changed_declarations: Vec<DeclarationId>,
    pub validate_modules: Vec<ModuleId>,
    pub retest_modules: Vec<ModuleId>,
    pub traversed_edges: u64,
}

pub fn validator_contract_digest() -> SemanticSummaryDigest {
    let mut hasher = blake3::Hasher::new_derive_key(VALIDATOR_DIGEST_DOMAIN);
    hash_part(&mut hasher, SEMANTIC_VALIDATOR_CONTRACT_IDENTITY.as_bytes());
    hash_part(&mut hasher, SEMANTIC_SUMMARY_CONTRACT_IDENTITY.as_bytes());
    hash_part(
        &mut hasher,
        super::meaning::GRAPH_CONTRACT_IDENTITY.as_bytes(),
    );
    SemanticSummaryDigest(*hasher.finalize().as_bytes())
}

pub fn build_module_summary(
    package: &PackageId,
    module: &MeaningModule,
) -> Result<ModuleSemanticSummary, Diagnostic> {
    module.validate_identity_shape()?;
    if module.declarations.len() > MAXIMUM_SUMMARY_DECLARATIONS {
        return Err(summary_error(
            DiagnosticClass::Resource,
            "semantic_summary_declaration_limit",
            "module has too many declarations for one summary object",
        ));
    }
    let module_bytes = module.encode()?;
    let module_object = ModuleObjectDigest::of(&module_bytes);
    let validator_contract = validator_contract_digest();
    let source_owners = source_owner_catalog(module)?;
    let mut dependencies = Vec::new();
    for relation in &module.relations {
        let source = source_owners
            .get(&relation.source)
            .cloned()
            .ok_or_else(|| {
                summary_error(
                    DiagnosticClass::Corrupt,
                    "semantic_summary_relation_source",
                    "semantic relation source has no owning declaration in its module",
                )
            })?;
        let target = dependency_target(&relation.target);
        for &kind in dependency_kinds(relation.role) {
            dependencies.push(SummaryDependency {
                source: source.clone(),
                target: target.clone(),
                kind,
            });
            if dependencies.len() > MAXIMUM_SUMMARY_DEPENDENCIES {
                return Err(summary_error(
                    DiagnosticClass::Resource,
                    "semantic_summary_dependency_limit",
                    "module has too many dependency facts for one summary object",
                ));
            }
        }
    }
    dependencies.sort();
    dependencies.dedup();

    let mut effect_dependencies = BTreeMap::<DeclarationId, Vec<&SummaryDependency>>::new();
    for dependency in &dependencies {
        if matches!(
            dependency.kind,
            SummaryDependencyKind::Effect | SummaryDependencyKind::Capability
        ) && let Some(declaration) = dependency.source.declaration
        {
            effect_dependencies
                .entry(declaration)
                .or_default()
                .push(dependency);
        }
    }

    let exports = module
        .module
        .exports
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if exports.len() != module.module.exports.len()
        || exports
            .iter()
            .any(|declaration| module.declaration(*declaration).is_none())
    {
        return Err(summary_error(
            DiagnosticClass::Corrupt,
            "semantic_summary_export_shape",
            "module exports must be unique and name existing declarations",
        ));
    }

    let mut declarations = Vec::with_capacity(module.declarations.len());
    for (identity, declaration) in module.declarations.iter().zip(&module.module.declarations) {
        let declared_effect = match declaration {
            Declaration::Function(function) => Some(function.effect.clone()),
            _ => None,
        };
        declarations.push(DeclarationSemanticSummary {
            declaration: identity.id,
            kind: identity.kind,
            exported: exports.contains(&identity.id),
            signature: declaration_signature_digest(identity, declaration)?,
            implementation: hash_encoded(
                DECLARATION_IMPLEMENTATION_DIGEST_DOMAIN,
                &(identity, declaration),
                MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
                "declaration implementation",
            )?,
            effect: hash_encoded(
                DECLARATION_EFFECT_DIGEST_DOMAIN,
                &(
                    identity.id,
                    declared_effect,
                    effect_dependencies
                        .get(&identity.id)
                        .map(Vec::as_slice)
                        .unwrap_or_default(),
                ),
                MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
                "declaration effect",
            )?,
        });
    }
    declarations.sort_by_key(|value| value.declaration);

    let input = summary_input_digest(package, module.module_id, module_object, validator_contract)?;
    let public_signature = module_public_signature_digest(module.module_id, &declarations)?;
    let implementation = module_implementation_digest(module_object)?;
    let dependency_digest = hash_encoded(
        DEPENDENCY_DIGEST_DOMAIN,
        &dependencies,
        MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
        "semantic summary dependencies",
    )?;
    let mut summary = ModuleSemanticSummary {
        contract_version: SEMANTIC_SUMMARY_CONTRACT_VERSION,
        validator_contract,
        package: package.clone(),
        module: module.module_id,
        module_object,
        input,
        public_signature,
        implementation,
        dependency_digest,
        declarations,
        dependencies,
        digest: SemanticSummaryDigest::ZERO,
    };
    summary.digest = hash_encoded(
        SUMMARY_RECORD_DIGEST_DOMAIN,
        &ModuleSummaryCore::from(&summary),
        MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
        "semantic summary record",
    )?;
    summary.validate()?;
    Ok(summary)
}

pub fn build_reverse_dependency_index(
    revision: RevisionId,
    summaries: &[ModuleSemanticSummary],
) -> Result<ReverseDependencyIndex, Diagnostic> {
    if summaries.is_empty() || summaries.len() > MAXIMUM_INDEX_MODULES {
        return Err(summary_error(
            DiagnosticClass::Resource,
            "semantic_summary_index_module_limit",
            "reverse dependency index needs a bounded nonempty summary set",
        ));
    }
    for summary in summaries {
        summary.validate()?;
    }
    let mut ordered = summaries.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|summary| summary.module);
    if ordered
        .windows(2)
        .any(|pair| pair[0].module == pair[1].module)
    {
        return Err(summary_error(
            DiagnosticClass::Source,
            "semantic_summary_index_duplicate_module",
            "one module has more than one summary in an index generation",
        ));
    }
    let package = ordered[0].package.clone();
    let validator_contract = ordered[0].validator_contract;
    if ordered.iter().any(|summary| {
        summary.package != package || summary.validator_contract != validator_contract
    }) {
        return Err(summary_error(
            DiagnosticClass::Source,
            "semantic_summary_index_generation",
            "reverse dependency index summaries must bind one package and validator contract",
        ));
    }

    let summary_bindings = ordered
        .iter()
        .map(|summary| ModuleSummaryBinding {
            module: summary.module,
            input: summary.input,
            summary: summary.digest,
        })
        .collect::<Vec<_>>();
    let mut tests = ordered
        .iter()
        .flat_map(|summary| {
            summary
                .declarations
                .iter()
                .filter(|declaration| declaration.kind == DeclarationKind::Test)
                .map(|declaration| {
                    SummaryOwner::declaration(summary.module, declaration.declaration)
                })
        })
        .collect::<Vec<_>>();
    tests.sort();
    tests.dedup();

    let mut reverse =
        BTreeMap::<(DependencyTarget, SummaryDependencyKind), BTreeSet<SummaryOwner>>::new();
    for summary in &ordered {
        for dependency in &summary.dependencies {
            reverse
                .entry((dependency.target.clone(), dependency.kind))
                .or_default()
                .insert(dependency.source.clone());
        }
    }
    if reverse.len() > MAXIMUM_INDEX_ENTRIES {
        return Err(index_work_exhausted());
    }
    let entries = reverse
        .into_iter()
        .map(|((target, kind), dependents)| ReverseDependencyEntry {
            target,
            kind,
            dependents: dependents.into_iter().collect(),
        })
        .collect::<Vec<_>>();
    let dependent_count = entries.iter().try_fold(0usize, |count, entry| {
        count
            .checked_add(entry.dependents.len())
            .ok_or_else(index_work_exhausted)
    })?;
    if dependent_count > MAXIMUM_INDEX_DEPENDENTS {
        return Err(index_work_exhausted());
    }
    let input =
        reverse_index_input_digest(&package, revision, validator_contract, &summary_bindings)?;
    let certificate = semantic_certificate_digest(
        &package,
        validator_contract,
        &summary_bindings,
        &tests,
        &entries,
    )?;
    let mut index = ReverseDependencyIndex {
        contract_version: SEMANTIC_SUMMARY_CONTRACT_VERSION,
        validator_contract,
        package,
        revision,
        summaries: summary_bindings,
        tests,
        entries,
        certificate,
        input,
        digest: SemanticSummaryDigest::ZERO,
    };
    index.digest = hash_encoded(
        INDEX_RECORD_DIGEST_DOMAIN,
        &ReverseIndexCore::from(&index),
        MAXIMUM_REVERSE_INDEX_PAYLOAD_BYTES,
        "reverse dependency index",
    )?;
    index.validate()?;
    Ok(index)
}

/// Applies exact module-summary replacements, additions, and removals to a revision-bound reverse
/// dependency index without requiring summaries for unchanged modules.
///
/// A summary whose module is already bound by `base` replaces that module; otherwise it adds the
/// module. Every removed module must exist in `base`, and a module cannot be both replaced and
/// removed. The result is canonical regardless of replacement or removal input order.
pub fn update_reverse_dependency_index(
    revision: RevisionId,
    base: &ReverseDependencyIndex,
    replacements: &[ModuleSemanticSummary],
    removed_modules: &[ModuleId],
) -> Result<ReverseDependencyIndex, Diagnostic> {
    base.validate()?;
    if replacements.len() > MAXIMUM_INDEX_MODULES || removed_modules.len() > MAXIMUM_INDEX_MODULES {
        return Err(index_work_exhausted());
    }

    let mut replacement_modules = BTreeSet::new();
    for summary in replacements {
        summary.validate()?;
        if summary.package != base.package || summary.validator_contract != base.validator_contract
        {
            return Err(summary_error(
                DiagnosticClass::Source,
                "semantic_summary_delta_generation",
                "replacement summaries must bind the base index package and validator contract",
            ));
        }
        if !replacement_modules.insert(summary.module) {
            return Err(summary_error(
                DiagnosticClass::Source,
                "semantic_summary_delta_duplicate_replacement",
                "a module has more than one replacement summary",
            ));
        }
    }

    let mut removals = BTreeSet::new();
    for &module in removed_modules {
        if !removals.insert(module) {
            return Err(summary_error(
                DiagnosticClass::Source,
                "semantic_summary_delta_duplicate_removal",
                "a module is removed more than once",
            ));
        }
        if replacement_modules.contains(&module) {
            return Err(summary_error(
                DiagnosticClass::Source,
                "semantic_summary_delta_owner_overlap",
                "a module cannot be both replaced and removed",
            ));
        }
    }

    let base_modules = base.modules().collect::<BTreeSet<_>>();
    if removals.iter().any(|module| !base_modules.contains(module)) {
        return Err(summary_error(
            DiagnosticClass::Source,
            "semantic_summary_delta_missing_removal",
            "a removed module is not bound by the base index",
        ));
    }
    let changed_modules = replacement_modules
        .union(&removals)
        .copied()
        .collect::<BTreeSet<_>>();

    let mut summary_bindings = base
        .summaries
        .iter()
        .filter(|binding| !changed_modules.contains(&binding.module))
        .cloned()
        .collect::<Vec<_>>();
    summary_bindings.extend(replacements.iter().map(|summary| ModuleSummaryBinding {
        module: summary.module,
        input: summary.input,
        summary: summary.digest,
    }));
    summary_bindings.sort_by_key(|binding| binding.module);
    if summary_bindings.is_empty() || summary_bindings.len() > MAXIMUM_INDEX_MODULES {
        return Err(summary_error(
            DiagnosticClass::Resource,
            "semantic_summary_index_module_limit",
            "reverse dependency index module count is outside its single-object budget",
        ));
    }

    let mut tests = base
        .tests
        .iter()
        .filter(|owner| !changed_modules.contains(&owner.module))
        .cloned()
        .collect::<BTreeSet<_>>();
    for summary in replacements {
        for declaration in &summary.declarations {
            if declaration.kind == DeclarationKind::Test {
                tests.insert(SummaryOwner::declaration(
                    summary.module,
                    declaration.declaration,
                ));
                if tests.len() > MAXIMUM_INDEX_DEPENDENTS {
                    return Err(index_work_exhausted());
                }
            }
        }
    }

    let mut reverse =
        BTreeMap::<(DependencyTarget, SummaryDependencyKind), BTreeSet<SummaryOwner>>::new();
    let mut dependent_count = 0usize;
    for entry in &base.entries {
        for dependent in &entry.dependents {
            if !changed_modules.contains(&dependent.module) {
                reverse
                    .entry((entry.target.clone(), entry.kind))
                    .or_default()
                    .insert(dependent.clone());
                dependent_count = dependent_count
                    .checked_add(1)
                    .ok_or_else(index_work_exhausted)?;
            }
        }
    }
    for summary in replacements {
        for dependency in &summary.dependencies {
            let inserted = reverse
                .entry((dependency.target.clone(), dependency.kind))
                .or_default()
                .insert(dependency.source.clone());
            if inserted {
                dependent_count = dependent_count
                    .checked_add(1)
                    .ok_or_else(index_work_exhausted)?;
                if dependent_count > MAXIMUM_INDEX_DEPENDENTS {
                    return Err(index_work_exhausted());
                }
            }
            if reverse.len() > MAXIMUM_INDEX_ENTRIES {
                return Err(index_work_exhausted());
            }
        }
    }
    if dependent_count > MAXIMUM_INDEX_DEPENDENTS {
        return Err(index_work_exhausted());
    }
    let entries = reverse
        .into_iter()
        .map(|((target, kind), dependents)| ReverseDependencyEntry {
            target,
            kind,
            dependents: dependents.into_iter().collect(),
        })
        .collect::<Vec<_>>();

    let input = reverse_index_input_digest(
        &base.package,
        revision,
        base.validator_contract,
        &summary_bindings,
    )?;
    let tests = tests.into_iter().collect::<Vec<_>>();
    let certificate = semantic_certificate_digest(
        &base.package,
        base.validator_contract,
        &summary_bindings,
        &tests,
        &entries,
    )?;
    let mut index = ReverseDependencyIndex {
        contract_version: SEMANTIC_SUMMARY_CONTRACT_VERSION,
        validator_contract: base.validator_contract,
        package: base.package.clone(),
        revision,
        summaries: summary_bindings,
        tests,
        entries,
        certificate,
        input,
        digest: SemanticSummaryDigest::ZERO,
    };
    index.digest = hash_encoded(
        INDEX_RECORD_DIGEST_DOMAIN,
        &ReverseIndexCore::from(&index),
        MAXIMUM_REVERSE_INDEX_PAYLOAD_BYTES,
        "reverse dependency index",
    )?;
    index.validate()?;
    Ok(index)
}

/// Computes the exact revision-independent semantic certificate for a complete module-summary
/// set. The returned digest is suitable for binding in an accepted revision core. The reverse
/// index itself remains disposable and is bound to a concrete revision separately.
pub fn build_semantic_certificate(
    summaries: &[ModuleSemanticSummary],
) -> Result<SemanticSummaryDigest, Diagnostic> {
    Ok(build_reverse_dependency_index(RevisionId::from_digest([0; 32]), summaries)?.certificate)
}

/// Rebinds an already validated exact fact set to a new revision without reconstructing unchanged
/// summaries. This changes the disposable index generation but preserves its certificate.
pub fn rebind_reverse_dependency_index(
    revision: RevisionId,
    index: &ReverseDependencyIndex,
) -> Result<ReverseDependencyIndex, Diagnostic> {
    update_reverse_dependency_index(revision, index, &[], &[])
}

pub fn classify_module_change(
    before: &ModuleSemanticSummary,
    after: &ModuleSemanticSummary,
) -> Result<ModuleInvalidationClass, Diagnostic> {
    before.validate()?;
    after.validate()?;
    require_same_summary_owner(before, after)?;
    if before.public_signature != after.public_signature {
        Ok(ModuleInvalidationClass::PublicSignature)
    } else if before.input != after.input
        || before.implementation != after.implementation
        || before.dependency_digest != after.dependency_digest
        || before.declarations != after.declarations
    {
        Ok(ModuleInvalidationClass::PrivateImplementation)
    } else {
        Ok(ModuleInvalidationClass::Unchanged)
    }
}

pub fn invalidation_frontier(
    base_revision: RevisionId,
    before: &ModuleSemanticSummary,
    after: &ModuleSemanticSummary,
    reverse: &ReverseDependencyIndex,
) -> Result<InvalidationFrontier, Diagnostic> {
    let class = classify_module_change(before, after)?;
    reverse.validate()?;
    if reverse.package != before.package
        || reverse.revision != base_revision
        || reverse.validator_contract != before.validator_contract
    {
        return Err(summary_error(
            DiagnosticClass::Source,
            "semantic_summary_index_stale",
            "invalidation requires the exact base revision reverse dependency index",
        ));
    }
    let changed_declarations = changed_declarations(before, after);
    if class == ModuleInvalidationClass::Unchanged {
        return Ok(InvalidationFrontier {
            class,
            changed_declarations,
            validate_modules: Vec::new(),
            retest_modules: Vec::new(),
            traversed_edges: 0,
        });
    }

    let mut validate_modules = BTreeSet::from([before.module]);
    let mut retest_modules = reverse
        .tests
        .iter()
        .filter(|owner| owner.module == before.module)
        .map(|owner| owner.module)
        .collect::<BTreeSet<_>>();
    let initial_owners = if changed_declarations.is_empty() {
        vec![SummaryOwner::module(before.module)]
    } else {
        changed_declarations
            .iter()
            .copied()
            .map(|declaration| SummaryOwner::declaration(before.module, declaration))
            .collect()
    };
    let mut queue = initial_owners
        .iter()
        .map(|owner| owner.target(&before.package))
        .collect::<VecDeque<_>>();
    let mut seen = queue.iter().cloned().collect::<BTreeSet<_>>();
    let mut traversed_edges = 0u64;

    while let Some(target) = queue.pop_front() {
        let propagation: &[SummaryDependencyKind] = match class {
            ModuleInvalidationClass::PublicSignature => &ALL_DEPENDENCY_KINDS,
            ModuleInvalidationClass::PrivateImplementation => &EXECUTION_DEPENDENCY_KINDS,
            ModuleInvalidationClass::Unchanged => &[],
        };
        for &kind in propagation {
            for dependent in reverse.dependents(&target, kind) {
                traversed_edges = traversed_edges
                    .checked_add(1)
                    .ok_or_else(index_work_exhausted)?;
                if class == ModuleInvalidationClass::PublicSignature {
                    validate_modules.insert(dependent.module);
                }
                if reverse.contains_test(dependent) {
                    retest_modules.insert(dependent.module);
                }
                let dependent_target = dependent.target(&before.package);
                if seen.insert(dependent_target.clone()) {
                    queue.push_back(dependent_target);
                }
            }
        }
        for dependent in reverse.dependents(&target, SummaryDependencyKind::Test) {
            traversed_edges = traversed_edges
                .checked_add(1)
                .ok_or_else(index_work_exhausted)?;
            retest_modules.insert(dependent.module);
            if class == ModuleInvalidationClass::PublicSignature {
                validate_modules.insert(dependent.module);
            }
        }
    }

    Ok(InvalidationFrontier {
        class,
        changed_declarations,
        validate_modules: validate_modules.into_iter().collect(),
        retest_modules: retest_modules.into_iter().collect(),
        traversed_edges,
    })
}

fn require_same_summary_owner(
    before: &ModuleSemanticSummary,
    after: &ModuleSemanticSummary,
) -> Result<(), Diagnostic> {
    if before.package != after.package || before.module != after.module {
        return Err(summary_error(
            DiagnosticClass::Source,
            "semantic_summary_foreign_owner",
            "module summaries belong to different stable package or module identities",
        ));
    }
    if before.validator_contract != after.validator_contract {
        return Err(summary_error(
            DiagnosticClass::Source,
            "semantic_summary_validator_change",
            "validator contract changes require complete summary and certificate invalidation",
        ));
    }
    Ok(())
}

fn changed_declarations(
    before: &ModuleSemanticSummary,
    after: &ModuleSemanticSummary,
) -> Vec<DeclarationId> {
    let before = before
        .declarations
        .iter()
        .map(|summary| (summary.declaration, summary))
        .collect::<BTreeMap<_, _>>();
    let after = after
        .declarations
        .iter()
        .map(|summary| (summary.declaration, summary))
        .collect::<BTreeMap<_, _>>();
    before
        .keys()
        .chain(after.keys())
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|declaration| before.get(declaration) != after.get(declaration))
        .collect()
}

fn source_owner_catalog(
    module: &MeaningModule,
) -> Result<BTreeMap<RelationSource, SummaryOwner>, Diagnostic> {
    let mut owners = BTreeMap::new();
    insert_source_owner(
        &mut owners,
        RelationSource::Module(module.module_id),
        SummaryOwner::module(module.module_id),
    )?;
    for identity in &module.declarations {
        let owner = SummaryOwner::declaration(module.module_id, identity.id);
        insert_source_owner(
            &mut owners,
            RelationSource::Declaration(identity.id),
            owner.clone(),
        )?;
        for member in &identity.members {
            let source = match member {
                super::meaning::MemberIdentity::TypeParameter { .. } => None,
                super::meaning::MemberIdentity::Field { id, .. } => {
                    Some(RelationSource::Field(*id))
                }
                super::meaning::MemberIdentity::Case { id, .. } => Some(RelationSource::Case(*id)),
                super::meaning::MemberIdentity::Operation { id, .. } => {
                    Some(RelationSource::Operation(*id))
                }
                super::meaning::MemberIdentity::Parameter { id, .. } => {
                    Some(RelationSource::Parameter(*id))
                }
                super::meaning::MemberIdentity::TaskRequirement { id, .. }
                | super::meaning::MemberIdentity::ComponentRequirement { id, .. } => {
                    Some(RelationSource::Requirement(*id))
                }
                super::meaning::MemberIdentity::Port { id, .. } => Some(RelationSource::Port(*id)),
            };
            if let Some(source) = source {
                insert_source_owner(&mut owners, source, owner.clone())?;
            }
        }
        for binding in &identity.bindings {
            insert_source_owner(
                &mut owners,
                RelationSource::Binding(binding.id),
                owner.clone(),
            )?;
        }
        for expression in &identity.expressions {
            insert_source_owner(
                &mut owners,
                RelationSource::Expression(expression.id),
                owner.clone(),
            )?;
        }
    }
    // Targets are package-root owners. A module shard can conservatively attribute a retained
    // target relation to the module without confusing the target's identity with a declaration.
    for relation in &module.relations {
        if let RelationSource::Target(id) = relation.source {
            insert_source_owner(
                &mut owners,
                RelationSource::Target(id),
                SummaryOwner::module(module.module_id),
            )?;
        }
    }
    Ok(owners)
}

fn insert_source_owner(
    owners: &mut BTreeMap<RelationSource, SummaryOwner>,
    source: RelationSource,
    owner: SummaryOwner,
) -> Result<(), Diagnostic> {
    if owners.insert(source, owner).is_some() {
        return Err(summary_error(
            DiagnosticClass::Corrupt,
            "semantic_summary_source_duplicate",
            "two semantic identities claim one relation source",
        ));
    }
    Ok(())
}

fn dependency_target(target: &RelationTarget) -> DependencyTarget {
    match target {
        RelationTarget::Module(owner) => DependencyTarget {
            package: owner.package.clone(),
            module: owner.module,
            declaration: None,
        },
        RelationTarget::Declaration(owner)
        | RelationTarget::Field { owner, .. }
        | RelationTarget::Case { owner, .. }
        | RelationTarget::Operation { owner, .. }
        | RelationTarget::TypeParameter { owner, .. }
        | RelationTarget::Parameter { owner, .. }
        | RelationTarget::Binding { owner, .. }
        | RelationTarget::Requirement { owner, .. }
        | RelationTarget::Port { owner, .. } => DependencyTarget {
            package: owner.package.clone(),
            module: owner.module,
            declaration: Some(owner.declaration),
        },
    }
}

fn dependency_kinds(role: RelationRole) -> &'static [SummaryDependencyKind] {
    match role {
        RelationRole::Import | RelationRole::Export => &NAMESPACE_DEPENDENCIES,
        RelationRole::TypeUse => &TYPE_DEPENDENCIES,
        RelationRole::ValueReference
        | RelationRole::FieldUse
        | RelationRole::VariantConstruction
        | RelationRole::VariantPattern => &VALUE_DEPENDENCIES,
        RelationRole::Call | RelationRole::ComponentPortFunction => &CALL_DEPENDENCIES,
        RelationRole::CapabilityInterface | RelationRole::CapabilityOperation => {
            &CAPABILITY_DEPENDENCIES
        }
        RelationRole::TargetComponent | RelationRole::TargetPort => &DEPLOYMENT_DEPENDENCIES,
        RelationRole::TestDependency => &TEST_DEPENDENCIES,
    }
}

fn declaration_signature_digest(
    identity: &DeclarationIdentity,
    declaration: &Declaration,
) -> Result<SemanticSummaryDigest, Diagnostic> {
    match declaration {
        Declaration::Record(value) => hash_encoded(
            DECLARATION_SIGNATURE_DIGEST_DOMAIN,
            &(identity.id, identity.kind, &identity.members, &value.fields),
            MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
            "record signature",
        ),
        Declaration::Variant(value) => hash_encoded(
            DECLARATION_SIGNATURE_DIGEST_DOMAIN,
            &(identity.id, identity.kind, &identity.members, &value.cases),
            MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
            "variant signature",
        ),
        Declaration::Interface(value) => hash_encoded(
            DECLARATION_SIGNATURE_DIGEST_DOMAIN,
            &(
                identity.id,
                identity.kind,
                &identity.members,
                &value.operations,
            ),
            MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
            "interface signature",
        ),
        Declaration::External(value) => hash_encoded(
            DECLARATION_SIGNATURE_DIGEST_DOMAIN,
            &(
                identity.id,
                identity.kind,
                &identity.members,
                &value.type_parameters,
                &value.parameters,
                &value.result,
            ),
            MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
            "external function signature",
        ),
        Declaration::Function(value) => hash_encoded(
            DECLARATION_SIGNATURE_DIGEST_DOMAIN,
            &(
                identity.id,
                identity.kind,
                &identity.members,
                &value.type_parameters,
                &value.parameters,
                &value.result,
                &value.effect,
            ),
            MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
            "function signature",
        ),
        Declaration::Constant(value) => hash_encoded(
            DECLARATION_SIGNATURE_DIGEST_DOMAIN,
            &(identity.id, identity.kind, &identity.members, &value.ty),
            MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
            "constant signature",
        ),
        Declaration::Component(value) => {
            let port_types = value.ports.iter().map(|port| &port.ty).collect::<Vec<_>>();
            hash_encoded(
                DECLARATION_SIGNATURE_DIGEST_DOMAIN,
                &(
                    identity.id,
                    identity.kind,
                    &identity.members,
                    &value.requirements,
                    port_types,
                ),
                MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
                "component signature",
            )
        }
        Declaration::Test(_) => hash_encoded(
            DECLARATION_SIGNATURE_DIGEST_DOMAIN,
            &(identity.id, identity.kind),
            128,
            "test signature",
        ),
    }
}

fn summary_input_digest(
    package: &PackageId,
    module: ModuleId,
    module_object: ModuleObjectDigest,
    validator: SemanticSummaryDigest,
) -> Result<SemanticSummaryDigest, Diagnostic> {
    hash_encoded(
        SUMMARY_INPUT_DIGEST_DOMAIN,
        &(package, module, module_object, validator),
        1024,
        "semantic summary input",
    )
}

fn module_implementation_digest(
    module_object: ModuleObjectDigest,
) -> Result<SemanticSummaryDigest, Diagnostic> {
    hash_encoded(
        MODULE_IMPLEMENTATION_DIGEST_DOMAIN,
        &module_object,
        128,
        "module implementation",
    )
}

fn module_public_signature_digest(
    module: ModuleId,
    declarations: &[DeclarationSemanticSummary],
) -> Result<SemanticSummaryDigest, Diagnostic> {
    let exported = declarations
        .iter()
        .filter(|declaration| declaration.exported)
        .map(|declaration| {
            (
                declaration.declaration,
                declaration.kind,
                declaration.signature,
                declaration.effect,
            )
        })
        .collect::<Vec<_>>();
    hash_encoded(
        PUBLIC_SIGNATURE_DIGEST_DOMAIN,
        &(module, exported),
        MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
        "module public signature",
    )
}

fn reverse_index_input_digest(
    package: &PackageId,
    revision: RevisionId,
    validator: SemanticSummaryDigest,
    summaries: &[ModuleSummaryBinding],
) -> Result<SemanticSummaryDigest, Diagnostic> {
    hash_encoded(
        INDEX_INPUT_DIGEST_DOMAIN,
        &(package, revision, validator, summaries),
        MAXIMUM_REVERSE_INDEX_PAYLOAD_BYTES,
        "reverse dependency index input",
    )
}

fn semantic_certificate_digest(
    package: &PackageId,
    validator_contract: SemanticSummaryDigest,
    summaries: &[ModuleSummaryBinding],
    tests: &[SummaryOwner],
    entries: &[ReverseDependencyEntry],
) -> Result<SemanticSummaryDigest, Diagnostic> {
    hash_encoded(
        CERTIFICATE_DIGEST_DOMAIN,
        &SemanticCertificateCore {
            contract_version: SEMANTIC_SUMMARY_CONTRACT_VERSION,
            validator_contract,
            package,
            summaries,
            tests,
            entries,
        },
        MAXIMUM_REVERSE_INDEX_PAYLOAD_BYTES,
        "semantic certificate",
    )
}

fn hash_encoded<T: Encode>(
    domain: &str,
    value: &T,
    maximum_bytes: usize,
    description: &str,
) -> Result<SemanticSummaryDigest, Diagnostic> {
    let configuration = bincode::config::standard()
        .with_little_endian()
        .with_variable_int_encoding()
        .with_limit::<{ packed::MAXIMUM_PACKED_PAYLOAD_BYTES }>();
    let bytes = bincode::encode_to_vec(value, configuration).map_err(|error| {
        summary_error(
            DiagnosticClass::Infrastructure,
            "semantic_summary_digest_encode",
            format!("{description} encoding failed: {error}"),
        )
    })?;
    if bytes.len() > maximum_bytes {
        return Err(summary_error(
            DiagnosticClass::Resource,
            "semantic_summary_digest_limit",
            format!("{description} exceeds its bounded digest input"),
        ));
    }
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hash_part(&mut hasher, &bytes);
    Ok(SemanticSummaryDigest(*hasher.finalize().as_bytes()))
}

fn hash_part(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn reject_oversized_envelope(
    bytes: &[u8],
    maximum_payload: usize,
    description: &str,
) -> Result<(), Diagnostic> {
    let maximum = maximum_payload
        .checked_add(PACKED_ENVELOPE_BYTES)
        .ok_or_else(index_work_exhausted)?;
    if bytes.len() > maximum {
        return Err(summary_error(
            DiagnosticClass::Resource,
            "semantic_summary_envelope_limit",
            format!("{description} exceeds its hostile-decoder byte budget"),
        ));
    }
    Ok(())
}

fn strictly_sorted<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn strictly_sorted_by<T, K: Ord + Copy>(values: &[T], key: impl Fn(&T) -> K) -> bool {
    values.windows(2).all(|pair| key(&pair[0]) < key(&pair[1]))
}

fn index_work_exhausted() -> Diagnostic {
    summary_error(
        DiagnosticClass::Resource,
        "semantic_summary_index_work",
        "reverse dependency index work exceeds its checked single-object budget",
    )
}

fn summary_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::super::language::{Effect, Expression, Function, Module, TestCase, Type};
    use super::super::meaning::{DeclarationReference, RequestIdentityAllocator, SemanticRelation};
    use super::super::syntax::SourceSpan;
    use super::*;

    fn span() -> SourceSpan {
        SourceSpan {
            byte_start: 0,
            byte_end: 0,
            line: 0,
            column: 0,
        }
    }

    fn pure_function(name: &str, result: Type, value: i64) -> Declaration {
        Declaration::Function(Function {
            name: name.to_owned(),
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            result,
            effect: Effect::Pure,
            body: Expression::I64(value, span()),
            span: span(),
        })
    }

    fn test_case(name: &str) -> Declaration {
        Declaration::Test(TestCase {
            name: name.to_owned(),
            actual: Expression::I64(1, span()),
            expected: Expression::I64(1, span()),
            span: span(),
        })
    }

    fn meaning_module(
        seed: &[u8],
        name: &str,
        exports: Vec<&str>,
        declarations: Vec<Declaration>,
    ) -> MeaningModule {
        let module = Module {
            name: name.to_owned(),
            imports: Vec::new(),
            exports: Vec::new(),
            declarations,
        };
        let mut module = MeaningModule::create(module, &mut RequestIdentityAllocator::new(seed))
            .expect("meaning module");
        module.module.exports = exports
            .into_iter()
            .map(|name| {
                module
                    .declaration_by_name(name)
                    .expect("exported declaration")
                    .0
                    .id
            })
            .collect();
        module.module.exports.sort();
        module
    }

    fn package() -> PackageId {
        PackageId::parse("11111111111111111111111111111111").expect("package")
    }

    fn revision(byte: u8) -> RevisionId {
        RevisionId::from_digest([byte; 32])
    }

    fn declaration_target(
        package: &PackageId,
        module: &MeaningModule,
        declaration: DeclarationId,
    ) -> RelationTarget {
        RelationTarget::Declaration(DeclarationReference {
            package: package.clone(),
            module: module.module_id,
            declaration,
        })
    }

    fn add_relation(
        module: &mut MeaningModule,
        source: DeclarationId,
        target: RelationTarget,
        role: RelationRole,
    ) {
        module.relations.push(SemanticRelation {
            source: RelationSource::Declaration(source),
            target,
            role,
        });
        module.relations.sort();
        module.relations.dedup();
    }

    fn next_random(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *state
    }

    fn random_index(state: &mut u64, upper: usize) -> usize {
        let upper = u64::try_from(upper).expect("index bound");
        usize::try_from(next_random(state) % upper).expect("bounded random index")
    }

    #[test]
    fn summary_is_revision_independent_deterministic_strict_and_rebuildable() {
        let package = package();
        let module = meaning_module(
            b"summary",
            "summary",
            vec!["public"],
            vec![
                pure_function("public", Type::I64, 1),
                pure_function("private", Type::I64, 2),
                test_case("public_test"),
            ],
        );
        let first = build_module_summary(&package, &module).expect("summary");
        let second = build_module_summary(&package, &module).expect("summary rebuild");
        assert_eq!(first, second);
        assert_eq!(
            first
                .dependencies_of_kind(SummaryDependencyKind::Call)
                .count(),
            0
        );
        first
            .verify_against(&package, &module)
            .expect("rebuild equality");
        let bytes = first.encode().expect("encode");
        assert_eq!(
            ModuleSemanticSummary::decode(&bytes).expect("decode"),
            first
        );

        let mut corrupt = bytes.clone();
        corrupt[20] ^= 1;
        assert_eq!(
            ModuleSemanticSummary::decode(&corrupt)
                .expect_err("checksum")
                .code,
            "packed_checksum"
        );
        let mut trailing = bytes;
        trailing.push(0);
        assert_eq!(
            ModuleSemanticSummary::decode(&trailing)
                .expect_err("trailing")
                .code,
            "packed_length_mismatch"
        );

        let mut unordered = first;
        unordered.declarations.reverse();
        let unordered_bytes = packed::encode(
            SUMMARY_MAGIC,
            SUMMARY_ENVELOPE_DOMAIN,
            &unordered,
            MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES,
        )
        .expect("raw invalid envelope");
        assert_eq!(
            ModuleSemanticSummary::decode(&unordered_bytes)
                .expect_err("order")
                .code,
            "semantic_summary_declaration_order"
        );

        let mut hostile = vec![0_u8; PACKED_ENVELOPE_BYTES];
        hostile[..8].copy_from_slice(&SUMMARY_MAGIC);
        hostile[8..10].copy_from_slice(&packed::PACKED_ENVELOPE_VERSION.to_le_bytes());
        hostile[10..18].copy_from_slice(
            &(u64::try_from(MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES).expect("limit") + 1)
                .to_le_bytes(),
        );
        assert_eq!(
            ModuleSemanticSummary::decode(&hostile)
                .expect_err("hostile length")
                .code,
            "packed_payload_limit"
        );
    }

    #[test]
    fn reverse_index_is_revision_bound_and_rebuilds_independently_of_input_order() {
        let package = package();
        let base_revision = revision(2);
        let provider = meaning_module(
            b"provider",
            "provider",
            vec!["api"],
            vec![pure_function("api", Type::I64, 1)],
        );
        let provider_id = provider.declarations[0].id;
        let mut consumer = meaning_module(
            b"consumer",
            "consumer",
            vec!["use_api"],
            vec![pure_function("use_api", Type::I64, 1)],
        );
        let consumer_id = consumer.declarations[0].id;
        add_relation(
            &mut consumer,
            consumer_id,
            declaration_target(&package, &provider, provider_id),
            RelationRole::Call,
        );
        let provider_summary = build_module_summary(&package, &provider).expect("provider summary");
        let consumer_summary = build_module_summary(&package, &consumer).expect("consumer summary");
        let first = build_reverse_dependency_index(
            base_revision,
            &[provider_summary.clone(), consumer_summary.clone()],
        )
        .expect("index");
        let reverse = build_reverse_dependency_index(
            base_revision,
            &[consumer_summary.clone(), provider_summary.clone()],
        )
        .expect("reverse index");
        assert_eq!(first, reverse);
        let bytes = first.encode().expect("index encode");
        assert_eq!(
            ReverseDependencyIndex::decode(&bytes).expect("index decode"),
            first
        );
        let mut corrupt = bytes;
        corrupt[20] ^= 1;
        assert_eq!(
            ReverseDependencyIndex::decode(&corrupt)
                .expect_err("index checksum")
                .code,
            "packed_checksum"
        );
        let mut unordered = first.clone();
        unordered.entries.reverse();
        let unordered_bytes = packed::encode(
            INDEX_MAGIC,
            INDEX_ENVELOPE_DOMAIN,
            &unordered,
            MAXIMUM_REVERSE_INDEX_PAYLOAD_BYTES,
        )
        .expect("raw invalid index envelope");
        assert_eq!(
            ReverseDependencyIndex::decode(&unordered_bytes)
                .expect_err("index entry order")
                .code,
            "semantic_summary_index_entry_order"
        );
        let target = DependencyTarget {
            package: package.clone(),
            module: provider.module_id,
            declaration: Some(provider_id),
        };
        assert_eq!(
            first.dependents(&target, SummaryDependencyKind::Call),
            &[SummaryOwner::declaration(consumer.module_id, consumer_id)]
        );

        let later_revision = build_reverse_dependency_index(
            revision(3),
            &[provider_summary.clone(), consumer_summary.clone()],
        )
        .expect("same summaries at later revision");
        assert_ne!(first.revision, later_revision.revision);
        assert_eq!(first.summaries, later_revision.summaries);
        assert_eq!(first.tests, later_revision.tests);
        assert_eq!(first.entries, later_revision.entries);
        assert_ne!(first.input, later_revision.input);
        assert_ne!(first.digest, later_revision.digest);

        assert_eq!(
            provider_summary,
            build_module_summary(&package, &provider).expect("provider reusable rebuild")
        );
        assert_eq!(
            consumer_summary,
            build_module_summary(&package, &consumer).expect("consumer reusable rebuild")
        );
    }

    #[test]
    fn reverse_index_delta_replaces_adds_removes_and_rejects_owner_mismatches() {
        let package = package();
        let provider = meaning_module(
            b"delta-provider",
            "provider",
            vec!["api"],
            vec![pure_function("api", Type::I64, 1)],
        );
        let provider_id = provider.declarations[0].id;
        let mut consumer = meaning_module(
            b"delta-consumer",
            "consumer",
            vec!["consume"],
            vec![pure_function("consume", Type::I64, 2)],
        );
        let consumer_id = consumer.declarations[0].id;
        add_relation(
            &mut consumer,
            consumer_id,
            declaration_target(&package, &provider, provider_id),
            RelationRole::Call,
        );
        let mut old_tests = meaning_module(
            b"delta-tests",
            "tests",
            Vec::new(),
            vec![test_case("consumer_test")],
        );
        let test_id = old_tests.declarations[0].id;
        add_relation(
            &mut old_tests,
            test_id,
            declaration_target(&package, &consumer, consumer_id),
            RelationRole::TestDependency,
        );

        let provider_summary = build_module_summary(&package, &provider).expect("provider");
        let consumer_summary = build_module_summary(&package, &consumer).expect("consumer");
        let test_summary = build_module_summary(&package, &old_tests).expect("tests");
        let base = build_reverse_dependency_index(
            revision(10),
            &[provider_summary.clone(), consumer_summary, test_summary],
        )
        .expect("base index");

        let replacement_provider = meaning_module(
            b"delta-replacement-provider",
            "replacement_provider",
            vec!["api"],
            vec![pure_function("api", Type::I64, 3)],
        );
        let replacement_provider_id = replacement_provider.declarations[0].id;
        let replacement_provider_summary =
            build_module_summary(&package, &replacement_provider).expect("added provider");
        let mut replacement_consumer = consumer.clone();
        replacement_consumer.relations.clear();
        let Declaration::Function(function) = &mut replacement_consumer.module.declarations[0]
        else {
            panic!("consumer function")
        };
        function.body = Expression::I64(4, function.body.span().clone());
        add_relation(
            &mut replacement_consumer,
            consumer_id,
            declaration_target(&package, &replacement_provider, replacement_provider_id),
            RelationRole::Call,
        );
        let replacement_consumer_summary =
            build_module_summary(&package, &replacement_consumer).expect("replacement consumer");
        let replacements = [
            replacement_provider_summary.clone(),
            replacement_consumer_summary.clone(),
        ];
        let removals = [old_tests.module_id];
        let updated =
            update_reverse_dependency_index(revision(11), &base, &replacements, &removals)
                .expect("delta update");
        let updated_reordered = update_reverse_dependency_index(
            revision(11),
            &base,
            &[
                replacement_consumer_summary.clone(),
                replacement_provider_summary.clone(),
            ],
            &removals,
        )
        .expect("reordered delta update");
        let rebuilt = build_reverse_dependency_index(
            revision(11),
            &[
                replacement_consumer_summary.clone(),
                provider_summary,
                replacement_provider_summary.clone(),
            ],
        )
        .expect("full rebuild");
        assert_eq!(updated, rebuilt);
        assert_eq!(updated_reordered, rebuilt);
        assert_eq!(
            updated.encode().expect("delta bytes"),
            rebuilt.encode().expect("rebuilt bytes")
        );
        let old_target = DependencyTarget {
            package: package.clone(),
            module: provider.module_id,
            declaration: Some(provider_id),
        };
        assert!(
            updated
                .dependents(&old_target, SummaryDependencyKind::Call)
                .is_empty()
        );
        let replacement_target = DependencyTarget {
            package: package.clone(),
            module: replacement_provider.module_id,
            declaration: Some(replacement_provider_id),
        };
        assert_eq!(
            updated.dependents(&replacement_target, SummaryDependencyKind::Call),
            &[SummaryOwner::declaration(consumer.module_id, consumer_id)]
        );
        assert!(
            !updated
                .modules()
                .any(|module| module == old_tests.module_id)
        );
        assert!(
            updated
                .tests
                .iter()
                .all(|owner| owner.module != old_tests.module_id)
        );

        assert_eq!(
            update_reverse_dependency_index(
                revision(12),
                &base,
                &[
                    replacement_consumer_summary.clone(),
                    replacement_consumer_summary.clone(),
                ],
                &[],
            )
            .expect_err("duplicate replacement")
            .code,
            "semantic_summary_delta_duplicate_replacement"
        );
        assert_eq!(
            update_reverse_dependency_index(
                revision(12),
                &base,
                std::slice::from_ref(&replacement_consumer_summary),
                &[consumer.module_id],
            )
            .expect_err("replacement and removal overlap")
            .code,
            "semantic_summary_delta_owner_overlap"
        );
        assert_eq!(
            update_reverse_dependency_index(
                revision(12),
                &base,
                &[],
                &[replacement_provider.module_id],
            )
            .expect_err("unknown removal")
            .code,
            "semantic_summary_delta_missing_removal"
        );
        assert_eq!(
            update_reverse_dependency_index(
                revision(12),
                &base,
                &[],
                &[old_tests.module_id, old_tests.module_id],
            )
            .expect_err("duplicate removal")
            .code,
            "semantic_summary_delta_duplicate_removal"
        );
        let foreign_package =
            PackageId::parse("22222222222222222222222222222222").expect("foreign package");
        let foreign_summary =
            build_module_summary(&foreign_package, &replacement_provider).expect("foreign summary");
        assert_eq!(
            update_reverse_dependency_index(revision(12), &base, &[foreign_summary], &[],)
                .expect_err("foreign package")
                .code,
            "semantic_summary_delta_generation"
        );
        let mut foreign_validator = replacement_provider_summary;
        foreign_validator.validator_contract = SemanticSummaryDigest::from_bytes([0x33; 32]);
        assert_eq!(
            update_reverse_dependency_index(revision(12), &base, &[foreign_validator], &[],)
                .expect_err("foreign validator")
                .code,
            "semantic_summary_validator_contract"
        );
    }

    #[test]
    fn reverse_index_delta_matches_full_build_over_deterministic_random_sequence() {
        let package = package();
        let mut modules = BTreeMap::new();
        for number in 0_u8..6 {
            let name = format!("module_{number}");
            let module = meaning_module(
                &[number, 0x5a],
                &name,
                vec!["value"],
                vec![pure_function("value", Type::I64, i64::from(number))],
            );
            modules.insert(module.module_id, module);
        }
        let summaries = modules
            .values()
            .map(|module| build_module_summary(&package, module))
            .collect::<Result<Vec<_>, _>>()
            .expect("initial summaries");
        let mut index =
            build_reverse_dependency_index(revision(20), &summaries).expect("initial index");
        let mut random = 0x5eed_cafe_f00d_baad_u64;

        for step in 0_u8..72 {
            let mut replacements = Vec::new();
            let mut removals = Vec::new();
            match step % 3 {
                0 => {
                    let selected = random_index(&mut random, modules.len());
                    let module_id = *modules.keys().nth(selected).expect("selected module");
                    let mut changed = modules.get(&module_id).expect("module").clone();
                    let Declaration::Function(function) = &mut changed.module.declarations[0]
                    else {
                        panic!("value function")
                    };
                    function.body = Expression::I64(
                        i64::from_le_bytes(next_random(&mut random).to_le_bytes()),
                        function.body.span().clone(),
                    );
                    changed.relations.clear();
                    if next_random(&mut random) & 1 == 1 {
                        let target_index = random_index(&mut random, modules.len());
                        let target = modules.values().nth(target_index).expect("target module");
                        let target_declaration = target.declarations[0].id;
                        let source = changed.declarations[0].id;
                        let target = declaration_target(&package, target, target_declaration);
                        add_relation(&mut changed, source, target, RelationRole::Call);
                    }
                    replacements.push(
                        build_module_summary(&package, &changed).expect("replacement summary"),
                    );
                    modules.insert(module_id, changed);
                }
                1 => {
                    let seed = [step, 0xa5, 0x17];
                    let name = format!("added_{step}");
                    let mut added = meaning_module(
                        &seed,
                        &name,
                        vec!["value"],
                        vec![pure_function("value", Type::I64, i64::from(step))],
                    );
                    if let Some(target) = modules
                        .values()
                        .nth(random_index(&mut random, modules.len()))
                    {
                        let source = added.declarations[0].id;
                        let target =
                            declaration_target(&package, target, target.declarations[0].id);
                        add_relation(&mut added, source, target, RelationRole::ValueReference);
                    }
                    replacements
                        .push(build_module_summary(&package, &added).expect("addition summary"));
                    modules.insert(added.module_id, added);
                }
                _ => {
                    let selected = random_index(&mut random, modules.len());
                    let module_id = *modules.keys().nth(selected).expect("removed module");
                    modules.remove(&module_id).expect("remove selected module");
                    removals.push(module_id);
                }
            }

            let next_revision = revision(step + 21);
            let updated =
                update_reverse_dependency_index(next_revision, &index, &replacements, &removals)
                    .expect("sequence delta");
            let complete_summaries = modules
                .values()
                .map(|module| build_module_summary(&package, module))
                .collect::<Result<Vec<_>, _>>()
                .expect("complete summaries");
            let rebuilt = build_reverse_dependency_index(next_revision, &complete_summaries)
                .expect("sequence full rebuild");
            assert_eq!(updated, rebuilt, "step {step}");
            assert_eq!(
                updated.encode().expect("updated encode"),
                rebuilt.encode().expect("rebuilt encode"),
                "encoded step {step}"
            );
            index = updated;
        }
    }

    #[test]
    fn private_body_and_public_signature_have_distinct_frontiers() {
        let package = package();
        let base = revision(4);
        let mut provider = meaning_module(
            b"frontier-provider",
            "provider",
            vec!["api"],
            vec![
                pure_function("api", Type::I64, 1),
                pure_function("helper", Type::I64, 2),
                test_case("provider_test"),
            ],
        );
        let api = provider.declarations[0].id;
        let helper = provider.declarations[1].id;
        let provider_test = provider.declarations[2].id;
        let api_target = declaration_target(&package, &provider, api);
        let helper_target = declaration_target(&package, &provider, helper);
        add_relation(
            &mut provider,
            provider_test,
            api_target,
            RelationRole::TestDependency,
        );
        add_relation(
            &mut provider,
            provider_test,
            helper_target,
            RelationRole::TestDependency,
        );

        let mut consumer = meaning_module(
            b"frontier-consumer",
            "consumer",
            vec!["use_api"],
            vec![pure_function("use_api", Type::I64, 1)],
        );
        let use_api = consumer.declarations[0].id;
        add_relation(
            &mut consumer,
            use_api,
            declaration_target(&package, &provider, api),
            RelationRole::Call,
        );

        let mut tests = meaning_module(
            b"frontier-tests",
            "tests",
            Vec::new(),
            vec![test_case("consumer_test")],
        );
        let consumer_test = tests.declarations[0].id;
        add_relation(
            &mut tests,
            consumer_test,
            declaration_target(&package, &consumer, use_api),
            RelationRole::TestDependency,
        );

        let provider_before = build_module_summary(&package, &provider).expect("provider before");
        let consumer_summary = build_module_summary(&package, &consumer).expect("consumer");
        let tests_summary = build_module_summary(&package, &tests).expect("tests");
        let reverse = build_reverse_dependency_index(
            base,
            &[provider_before.clone(), consumer_summary, tests_summary],
        )
        .expect("reverse");

        let mut private_change = provider.clone();
        let Declaration::Function(helper_function) = &mut private_change.module.declarations[1]
        else {
            panic!("helper function")
        };
        helper_function.body = Expression::I64(3, helper_function.body.span().clone());
        let private_after = build_module_summary(&package, &private_change).expect("private after");
        assert_eq!(
            classify_module_change(&provider_before, &private_after).expect("private class"),
            ModuleInvalidationClass::PrivateImplementation
        );
        let private_frontier =
            invalidation_frontier(base, &provider_before, &private_after, &reverse)
                .expect("private");
        assert_eq!(private_frontier.validate_modules, vec![provider.module_id]);
        assert_eq!(private_frontier.retest_modules, vec![provider.module_id]);
        assert_eq!(private_frontier.changed_declarations, vec![helper]);

        let mut public_change = provider;
        let Declaration::Function(api_function) = &mut public_change.module.declarations[0] else {
            panic!("api function")
        };
        api_function.result = Type::Text;
        let public_after = build_module_summary(&package, &public_change).expect("public after");
        assert_eq!(
            classify_module_change(&provider_before, &public_after).expect("public class"),
            ModuleInvalidationClass::PublicSignature
        );
        let public_frontier =
            invalidation_frontier(base, &provider_before, &public_after, &reverse).expect("public");
        let mut expected_validation =
            vec![provider_before.module, consumer.module_id, tests.module_id];
        expected_validation.sort();
        assert_eq!(public_frontier.validate_modules, expected_validation);
        let mut expected_tests = vec![provider_before.module, tests.module_id];
        expected_tests.sort();
        assert_eq!(public_frontier.retest_modules, expected_tests);
        assert_eq!(public_frontier.changed_declarations, vec![api]);
        assert!(public_frontier.traversed_edges >= 2);

        assert_eq!(
            invalidation_frontier(revision(5), &provider_before, &public_after, &reverse,)
                .expect_err("stale revision-bound index")
                .code,
            "semantic_summary_index_stale"
        );
    }
}
