mod measure;

pub(crate) use measure::measure;

use crate::semantic::codec::{error, MAX_SCHEMA_NODES, MAX_WORK_UNITS};
use crate::semantic::schema::{
    Charges, ProtocolError, ProtocolErrorCode, ProtocolLimitsRecord, ResourceProfile,
    ResourceProfileIdentityRecord,
};
use lkjscript_core::ResourceCategory;

pub(crate) const MAX_SOURCE_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_SOURCE_UNITS: u64 = 4_096;

#[derive(Clone, Copy)]
pub(crate) struct ProtocolLimits {
    pub source_bytes: u64,
    pub source_units: u64,
    source_nodes: u64,
    pub(crate) work_units: u64,
    pub(crate) request_bytes: u64,
    pub response_bytes: usize,
    hole_count: u64,
    hole_candidates: u64,
    hole_search_work: u64,
    legal_actions: u64,
    transactions: u64,
    transaction_operations: u64,
    transaction_impact_nodes: u64,
    staged_publication_bytes: u64,
    staged_publication_nodes: u64,
}

impl ProtocolLimits {
    pub(crate) fn for_profile(profile: ResourceProfile) -> Self {
        Self::for_core(profile.core())
    }

    pub(crate) fn for_core(profile: lkjscript_core::ResourceProfile) -> Self {
        let ceilings = profile.ceilings();
        let response = ceilings.limit(ResourceCategory::ProtocolResponseBytes);
        Self {
            source_bytes: ceilings
                .limit(ResourceCategory::SourceBytes)
                .min(MAX_SOURCE_BYTES),
            source_units: ceilings
                .limit(ResourceCategory::SourceUnits)
                .min(MAX_SOURCE_UNITS),
            source_nodes: ceilings
                .limit(ResourceCategory::SchemaNodes)
                .min(MAX_SCHEMA_NODES),
            work_units: ceilings
                .limit(ResourceCategory::ValidationWork)
                .min(MAX_WORK_UNITS),
            request_bytes: ceilings.limit(ResourceCategory::ProtocolRequestBytes),
            response_bytes: usize::try_from(response)
                .unwrap_or(usize::MAX)
                .min(crate::semantic::codec::MAX_OUTPUT_BYTES),
            hole_count: ceilings.limit(ResourceCategory::HoleCount),
            hole_candidates: ceilings.limit(ResourceCategory::HoleCandidates),
            hole_search_work: ceilings.limit(ResourceCategory::HoleSearchWork),
            legal_actions: ceilings.limit(ResourceCategory::LegalActions),
            transactions: ceilings.limit(ResourceCategory::Transactions),
            transaction_operations: ceilings.limit(ResourceCategory::TransactionOperations),
            transaction_impact_nodes: ceilings.limit(ResourceCategory::TransactionImpactNodes),
            staged_publication_bytes: ceilings.limit(ResourceCategory::StagedPublicationBytes),
            staged_publication_nodes: ceilings.limit(ResourceCategory::StagedPublicationNodes),
        }
    }

    pub(crate) fn record(self) -> ProtocolLimitsRecord {
        ProtocolLimitsRecord {
            request_bytes: self.request_bytes,
            response_bytes: u64::try_from(self.response_bytes).unwrap_or(u64::MAX),
            source_bytes: self.source_bytes,
            source_units: self.source_units,
            source_nodes: self.source_nodes,
            work_units: self.work_units,
            hole_count: self.hole_count,
            hole_candidates: self.hole_candidates,
            hole_search_work: self.hole_search_work,
            legal_actions: self.legal_actions,
            transactions: self.transactions,
            transaction_operations: self.transaction_operations,
            transaction_impact_nodes: self.transaction_impact_nodes,
            staged_publication_bytes: self.staged_publication_bytes,
            staged_publication_nodes: self.staged_publication_nodes,
        }
    }

    pub(crate) fn check_request(self, bytes: usize) -> Result<(), ProtocolError> {
        let bytes = u64::try_from(bytes).map_err(|_| overflow())?;
        self.check("protocol_request_bytes", bytes, self.request_bytes)
    }

    pub(crate) fn check_charges(self, charges: &Charges) -> Result<(), ProtocolError> {
        self.check("source_bytes", charges.source_bytes, self.source_bytes)?;
        self.check("source_units", charges.source_units, self.source_units)?;
        self.check("schema_nodes", charges.source_nodes, self.source_nodes)?;
        self.check("validation_work", charges.work_units, self.work_units)?;
        self.check("hole_count", charges.hole_count, self.hole_count)?;
        self.check(
            "hole_candidates",
            charges.hole_candidates,
            self.hole_candidates,
        )?;
        self.check(
            "hole_search_work",
            charges.hole_search_work,
            self.hole_search_work,
        )?;
        self.check("legal-actions", charges.legal_actions, self.legal_actions)?;
        self.check("transactions", charges.transactions, self.transactions)?;
        self.check(
            "transaction_operations",
            charges.transaction_operations,
            self.transaction_operations,
        )?;
        self.check(
            "transaction_impact_nodes",
            charges.transaction_impact_nodes,
            self.transaction_impact_nodes,
        )?;
        self.check(
            "staged_publication_bytes",
            charges.staged_publication_bytes,
            self.staged_publication_bytes,
        )?;
        self.check(
            "staged_publication_nodes",
            charges.staged_publication_nodes,
            self.staged_publication_nodes,
        )
    }

    fn check(self, category: &str, observed: u64, limit: u64) -> Result<(), ProtocolError> {
        if observed <= limit {
            return Ok(());
        }
        Err(error(
            ProtocolErrorCode::ResourceLimit,
            format!("protocol {category} charge {observed} exceeds profile limit {limit}"),
        ))
    }
}

fn overflow() -> ProtocolError {
    error(
        ProtocolErrorCode::ResourceLimit,
        "protocol aggregate charge overflow",
    )
}

#[cfg(test)]
pub(crate) fn identity(profile: ResourceProfile) -> ResourceProfileIdentityRecord {
    identity_core(profile.core())
}

pub(crate) fn identity_core(
    profile: lkjscript_core::ResourceProfile,
) -> ResourceProfileIdentityRecord {
    let identity = profile.identity();
    ResourceProfileIdentityRecord {
        schema: identity.schema.to_string(),
        contract: identity.contract.to_hex(),
        name: identity.name.as_str().to_string(),
        resource_categories: identity.resource_categories.to_hex(),
        implementation_maxima_sha256: hex(&identity.implementation_maxima_sha256),
        ceilings_sha256: hex(&identity.ceilings_sha256),
        host_lowered_ceilings_sha256: identity
            .host_lowered_ceilings_sha256
            .map(|value| hex(&value)),
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(*byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(*byte & 0x0f)]));
    }
    output
}
