//! Content-bound, rebuildable semantic summaries for local invalidation.
//!
//! Module summaries are exactly reusable across revisions while their canonical module input and
//! validator contract remain unchanged. Persistent reverse facts are owned by `semantic_fact`.
//! Neither derived form authorizes accepted meaning, and callers rebuild from canonical modules
//! after any contract or integrity mismatch.

use super::contract::registry::{
    DECLARATION_EFFECT_DIGEST_DOMAIN, DECLARATION_IMPLEMENTATION_DIGEST_DOMAIN,
    DECLARATION_SIGNATURE_DIGEST_DOMAIN, MODULE_IMPLEMENTATION_DIGEST_DOMAIN,
    PUBLIC_SIGNATURE_DIGEST_DOMAIN, SUMMARY_DEPENDENCY_DIGEST_DOMAIN as DEPENDENCY_DIGEST_DOMAIN,
    SUMMARY_ENVELOPE_DOMAIN, SUMMARY_INPUT_DIGEST_DOMAIN, SUMMARY_MAGIC,
    SUMMARY_RECORD_DIGEST_DOMAIN, VALIDATOR_DIGEST_DOMAIN,
};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::language::Declaration;
use super::meaning::{
    DeclarationIdentity, DeclarationKind, MeaningModule, RelationRole, RelationSource,
    RelationTarget,
};
use super::package::PackageId;
use super::packed;
use super::semantic_digest::ModuleObjectDigest;
use super::semantic_id::{DeclarationId, ModuleId};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const SEMANTIC_SUMMARY_CONTRACT_VERSION: u16 = 3;
pub const SEMANTIC_SUMMARY_CONTRACT_IDENTITY: &str = "lkjscript-semantic-summary-3";
pub const SEMANTIC_VALIDATOR_CONTRACT_IDENTITY: &str = "lkjscript-semantic-validator-3";

// This is a hostile-decoder and single-object implementation bound, not a semantic project limit.
const MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES: usize = 16 * 1_048_576;
pub const MAXIMUM_MODULE_SUMMARY_ENCODED_BYTES: usize =
    MAXIMUM_MODULE_SUMMARY_PAYLOAD_BYTES + PACKED_ENVELOPE_BYTES;
const MAXIMUM_SUMMARY_DECLARATIONS: usize = 100_000;
const MAXIMUM_SUMMARY_DEPENDENCIES: usize = 2_000_000;
const PACKED_ENVELOPE_BYTES: usize = 8 + 2 + 8 + 32;

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

    pub(crate) fn target(&self, package: &PackageId) -> DependencyTarget {
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
        .ok_or_else(summary_size_exhausted)?;
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

fn summary_size_exhausted() -> Diagnostic {
    summary_error(
        DiagnosticClass::Resource,
        "semantic_summary_size",
        "semantic summary size exceeds its checked single-object budget",
    )
}

fn summary_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::super::language::{Effect, Expression, Function, Module, TestCase, Type};
    use super::super::meaning::RequestIdentityAllocator;
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
}
