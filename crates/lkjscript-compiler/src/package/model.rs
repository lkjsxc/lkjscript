use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[path = "model/witness.rs"]
mod witness;
pub use witness::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    pub contract: String,
    pub name: String,
    pub source_root: String,
    pub modules: Vec<String>,
    pub public: Vec<String>,
    pub dependencies: Vec<Dependency>,
    pub capabilities: Vec<String>,
    pub targets: Vec<Target>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub name: String,
    pub path: String,
    pub content_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub name: String,
    pub module: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockFile {
    pub schema: String,
    pub contract: String,
    pub root: String,
    pub contracts: BTreeMap<String, String>,
    pub packages: Vec<LockedPackage>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedPackage {
    pub name: String,
    pub origin: String,
    pub manifest_sha256: String,
    pub package_memory_interface_sha256: String,
    pub package_sha256: String,
    pub dependencies: Vec<String>,
    pub modules: Vec<LockedModule>,
    pub targets: Vec<LockedTargetMemory>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedModule {
    pub id: String,
    pub source_sha256: String,
    pub source_closure: Vec<LockedSourceIdentity>,
    pub module_interface_contract: String,
    pub module_interface_sha256: String,
    pub module_memory_interface_sha256: String,
    pub module_sha256: String,
    pub exports: Vec<String>,
    pub memory_interfaces: Vec<PackageMemoryInterface>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedSourceIdentity {
    pub id: String,
    pub source_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageMemoryInterface {
    pub declaration_identity: String,
    pub export_identity: String,
    pub name: String,
    pub declaration_kind: String,
    pub type_parameters: Vec<String>,
    pub trait_parameters: Vec<LockedTraitParameter>,
    pub memory_requirements: Vec<LockedMemoryRequirement>,
    pub parameter_modes: Vec<LockedMemoryParameterMode>,
    pub result_mode: LockedMemoryResultMode,
    pub equality_constraints: Vec<LockedMemoryConstraint>,
    pub process_codec_constraints: Vec<LockedMemoryConstraint>,
    pub module_interface_contract: String,
    pub package_memory_interface_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedTraitParameter {
    pub parameter: String,
    pub trait_identity: String,
    pub trait_name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedMemoryRequirement {
    pub parameter: String,
    pub operations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedMemoryConstraint {
    pub parameter: String,
    pub support: LockedConstraintSupport,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LockedConstraintSupport {
    Unsupported,
    Value,
    List,
    Eligible,
    Ineligible,
    CallerWitnessRequired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LockedMemoryParameterMode {
    Copy,
    BorrowShared,
    BorrowExclusive,
    Consume,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LockedMemoryResultMode {
    NotApplicable,
    Trivial,
    Owned,
    SealedShared,
    External,
}
