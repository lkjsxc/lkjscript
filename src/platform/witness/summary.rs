//! Owner-granular semantic summary and committed witness manifest records.

use super::contract::{
    OWNER_SUMMARY_CONTRACT_VERSION, WITNESS_CONTRACT_VERSION, validator_contract_digest,
};
use super::{
    OwnerSummaryDigest, SemanticDigest, ValidationCertificateDigest, ValidatorContractDigest,
};
use crate::platform::kernel::{
    OwnerKey, OwnerKind, OwnerObjectDigest, PackageId, SemanticRootDigest,
};
use crate::platform::persistent_map::MapRoot;
use crate::platform::semantic_id::RepositoryId;
use bincode::{Decode, Encode};

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct OwnerSummary {
    pub contract_version: u16,
    pub owner: OwnerKey,
    pub kind: OwnerKind,
    pub record: OwnerObjectDigest,
    pub semantic_interface: SemanticDigest,
    pub implementation: SemanticDigest,
    pub type_digest: SemanticDigest,
    pub effect: SemanticDigest,
    pub capability: SemanticDigest,
    pub relations: SemanticDigest,
    pub presentation: SemanticDigest,
    pub test: Option<SemanticDigest>,
    pub validation_dependencies: SemanticDigest,
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct WitnessRoots {
    pub owner_summaries: MapRoot,
    pub namespaces: MapRoot,
    pub ownership: MapRoot,
    pub forward_relations: MapRoot,
    pub reverse_relations: MapRoot,
    pub test_dependencies: MapRoot,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct ValidationWitnessManifest {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub validator_contract: ValidatorContractDigest,
    pub repository_id: RepositoryId,
    pub package_id: PackageId,
    pub semantic_root: SemanticRootDigest,
    pub roots: WitnessRoots,
    pub certificate: ValidationCertificateDigest,
}

#[derive(Encode)]
pub(crate) struct CertificateCore {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub validator_contract: ValidatorContractDigest,
    pub repository_id: RepositoryId,
    pub package_id: PackageId,
    pub semantic_root: SemanticRootDigest,
    pub roots: WitnessRoots,
}

impl ValidationWitnessManifest {
    pub(crate) fn core(&self) -> CertificateCore {
        CertificateCore {
            contract_version: self.contract_version,
            graph_contract_version: self.graph_contract_version,
            validator_contract: self.validator_contract,
            repository_id: self.repository_id,
            package_id: self.package_id,
            semantic_root: self.semantic_root,
            roots: self.roots,
        }
    }

    pub(crate) fn contract_is_current(&self) -> bool {
        self.contract_version == WITNESS_CONTRACT_VERSION
            && self.graph_contract_version
                == crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
            && self.validator_contract == validator_contract_digest()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SummaryBinding {
    pub kind: OwnerKind,
    pub summary: OwnerSummaryDigest,
}

impl SummaryBinding {
    pub fn encode(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(33);
        bytes.push(self.kind.tag());
        bytes.extend_from_slice(&self.summary.bytes());
        bytes
    }
}

pub(crate) const fn current_summary_contract() -> u16 {
    OWNER_SUMMARY_CONTRACT_VERSION
}
