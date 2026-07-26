use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
    pub resource_profile: Option<String>,
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
    pub package_sha256: String,
    pub dependencies: Vec<String>,
    pub modules: Vec<LockedModule>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedModule {
    pub id: String,
    pub source_sha256: String,
    pub module_sha256: String,
    pub exports: Vec<String>,
}
