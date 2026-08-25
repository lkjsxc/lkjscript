//! Small fixed Graph 5 semantic root plus exact dependency and retirement records.

use super::contract::GRAPH_CONTRACT_VERSION;
use super::digest::{
    ChangeDigest, DependencyObjectDigest, PackageRevisionDigest, RetirementObjectDigest,
};
use super::id::{EncodedOwnerKey, OwnerKey, OwnerKind, PackageId};
use super::name::Name;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::persistent_map::MapRoot;
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticRoot {
    pub graph_contract_version: u16,
    pub repository_id: RepositoryId,
    pub package_id: PackageId,
    pub package_name: Name,
    pub owners: MapRoot,
    pub dependencies: MapRoot,
    pub retirements: MapRoot,
}

impl SemanticRoot {
    pub(crate) fn validate_local(&self) -> Result<(), Diagnostic> {
        if self.graph_contract_version != GRAPH_CONTRACT_VERSION {
            return Err(root_error(
                "kernel_root_contract",
                format!(
                    "root contract {} is not Graph Contract {GRAPH_CONTRACT_VERSION}",
                    self.graph_contract_version
                ),
            ));
        }
        if self.package_name.as_str().is_empty() {
            return Err(root_error(
                "kernel_root_package_name",
                "package name is empty",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyBinding {
    pub object: DependencyObjectDigest,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyRecord {
    pub graph_contract_version: u16,
    pub package: PackageId,
    pub semantic_revision: RevisionId,
    pub package_revision: PackageRevisionDigest,
}

impl DependencyRecord {
    pub(crate) fn validate_local(&self) -> Result<(), Diagnostic> {
        if self.graph_contract_version != GRAPH_CONTRACT_VERSION {
            return Err(root_error(
                "kernel_dependency_contract",
                "dependency record uses a foreign graph contract",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetirementBinding {
    pub object: RetirementObjectDigest,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetirementRecord {
    pub graph_contract_version: u16,
    pub owner: OwnerKey,
    pub last_kind: OwnerKind,
    pub last_name: Option<Name>,
    pub last_parent: Option<OwnerKey>,
    pub last_live_revision: RevisionId,
    pub deletion_change: ChangeDigest,
}

impl RetirementRecord {
    pub(crate) fn validate_local(&self) -> Result<(), Diagnostic> {
        if self.graph_contract_version != GRAPH_CONTRACT_VERSION {
            return Err(root_error(
                "kernel_retirement_contract",
                "retirement record uses a foreign graph contract",
            ));
        }
        Ok(())
    }
}

pub const fn owner_map_key(owner: OwnerKey) -> [u8; 17] {
    EncodedOwnerKey::new(owner).bytes()
}

pub const fn dependency_map_key(package: PackageId) -> [u8; 16] {
    package.bytes()
}

pub const fn retirement_map_key(owner: OwnerKey) -> [u8; 17] {
    owner_map_key(owner)
}

fn root_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}
