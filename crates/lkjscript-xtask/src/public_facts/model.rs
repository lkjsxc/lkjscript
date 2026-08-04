use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub contract: String,
    pub platform_revision: u64,
    pub shards: Vec<ShardSpec>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShardSpec {
    pub path: String,
    pub first: String,
    pub last: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shard {
    pub schema: String,
    pub contract: String,
    pub facts: Vec<Fact>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    pub id: String,
    pub kind: FactKind,
    pub status: Status,
    pub scope: Vec<String>,
    pub interface: String,
    pub exclusions: Vec<Exclusion>,
    pub authority: Authority,
    pub implementation_anchors: Vec<String>,
    pub evidence: Vec<Evidence>,
    pub projections: Vec<String>,
    pub dependencies: Vec<String>,
    pub invalidated_by: Vec<String>,
    pub platform_revision: u64,
    pub contracts: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FactKind {
    Capability,
    Contract,
    Evidence,
    Policy,
    Selection,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Current,
    AcceptedTarget,
    AcceptedContract,
    AcceptedSelection,
    Experimental,
    Deferred,
    Rejected,
    Superseded,
    Historical,
}

impl Status {
    pub fn name(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::AcceptedTarget => "accepted-target",
            Self::AcceptedContract => "accepted-contract",
            Self::AcceptedSelection => "accepted-selection",
            Self::Experimental => "experimental",
            Self::Deferred => "deferred",
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
            Self::Historical => "historical",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Exclusion {
    pub id: String,
    pub interface: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Authority {
    RepositoryPath { path: String },
    MachineSource { source: MachineSource },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MachineSource {
    CapsuleManifests,
    CargoMetadata,
    CliDescribe,
    CompilerVocabulary,
    ContractRegistry,
    PlatformRevision,
    StructurePolicy,
    UnsafeRegistry,
}

impl MachineSource {
    pub fn name(self) -> &'static str {
        match self {
            Self::CapsuleManifests => "capsule-manifests",
            Self::CargoMetadata => "cargo-metadata",
            Self::CliDescribe => "cli-describe",
            Self::CompilerVocabulary => "compiler-vocabulary",
            Self::ContractRegistry => "contract-registry",
            Self::PlatformRevision => "platform-revision",
            Self::StructurePolicy => "structure-policy",
            Self::UnsafeRegistry => "unsafe-registry",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub path: String,
    pub class: EvidenceClass,
    #[serde(default)]
    pub commit: Option<String>,
    #[serde(default)]
    pub result: Option<EvidenceResult>,
    #[serde(default)]
    pub platform_revision: Option<u64>,
    #[serde(default)]
    pub contracts: Vec<String>,
    #[serde(default)]
    pub not_tested: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceClass {
    ContractRecord,
    ExecutedRecord,
    HistoricalRecord,
    ImplementationTest,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceResult {
    Failed,
    NotRun,
    Passed,
}

#[derive(Clone, Debug)]
pub struct LocatedFact {
    pub fact: Fact,
    pub shard: String,
    pub digest: String,
    pub content_digests: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub struct Registry {
    pub manifest: Manifest,
    pub facts: BTreeMap<String, LocatedFact>,
}
