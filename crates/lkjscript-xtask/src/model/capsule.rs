use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CapsuleKind {
    Workspace,
    Collection,
    Crate,
    Source,
    Tooling,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnsafeStatus {
    Forbidden,
    Confined,
    Crosses,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityKind {
    None,
    Defines,
    Uses,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapsuleProvenanceKind {
    Authored,
    Generated,
    Vendored,
    ImmutableEvidence,
}

impl CapsuleProvenanceKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Authored => "authored",
            Self::Generated => "generated",
            Self::Vendored => "vendored",
            Self::ImmutableEvidence => "immutable-evidence",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryStatus {
    pub status: UnsafeStatus,
    pub boundaries: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityStatus {
    pub status: CapabilityKind,
    pub names: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapsuleProvenance {
    pub class: CapsuleProvenanceKind,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextCard {
    pub goal: String,
    pub interfaces: Vec<String>,
    pub invariants: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Capsule {
    pub schema: String,
    pub version: u32,
    pub id: String,
    pub root: String,
    pub kind: CapsuleKind,
    pub purpose: String,
    pub layer: String,
    pub concepts: Vec<String>,
    pub facade: Vec<String>,
    pub allowed_dependencies: Vec<String>,
    pub forbidden_dependencies: Vec<String>,
    pub tests: Vec<String>,
    pub decisions: Vec<String>,
    pub unsafe_boundary: BoundaryStatus,
    pub capability: CapabilityStatus,
    pub provenance: CapsuleProvenance,
    pub verification: Vec<String>,
    pub context_card: ContextCard,
}
