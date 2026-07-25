use lkjscript_core::ResourceCategory;

use crate::semantic::codec::{error, MAX_SCHEMA_NODES, MAX_WORK_UNITS};
use crate::semantic::schema::{
    Charges, OperationRequest, ProtocolError, ProtocolErrorCode, ProtocolLimitsRecord,
    ResourceProfile, ResourceProfileIdentityRecord,
};

pub(crate) const MAX_SOURCE_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_SOURCE_UNITS: u64 = 4_096;

#[derive(Clone, Copy)]
pub(crate) struct ProtocolLimits {
    pub source_bytes: u64,
    pub source_units: u64,
    source_nodes: u64,
    work_units: u64,
    request_bytes: u64,
    pub response_bytes: usize,
}

impl ProtocolLimits {
    pub(crate) fn for_profile(profile: ResourceProfile) -> Self {
        let ceilings = profile.core().ceilings();
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
        self.check("validation_work", charges.work_units, self.work_units)
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

pub(crate) fn identity(profile: ResourceProfile) -> ResourceProfileIdentityRecord {
    let identity = profile.core().identity();
    ResourceProfileIdentityRecord {
        schema: identity.schema.to_string(),
        version: identity.version,
        implementation_maxima_version: identity.implementation_maxima_version,
        ceilings_sha256: hex(&identity.ceilings_sha256),
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

pub(crate) fn measure(
    tree: &crate::source::ValidatedSourceTree,
    bytes: usize,
    operation: &OperationRequest,
) -> Result<Charges, ProtocolError> {
    let source_bytes = tree
        .files()
        .iter()
        .try_fold(0u64, |total, file| total.checked_add(file.exact_source_len));
    let source_bytes = source_bytes.ok_or_else(overflow)?;
    let operations = match operation {
        OperationRequest::ApplyTransaction { operations, .. } => {
            u64::try_from(operations.len()).map_err(|_| overflow())?
        }
        _ => 0,
    };
    let request_bytes = u64::try_from(bytes).map_err(|_| overflow())?;
    let source_units = u64::try_from(tree.files().len()).map_err(|_| overflow())?;
    let source_nodes = u64::try_from(tree.nodes().len()).map_err(|_| overflow())?;
    let traversal_multiplier = match operation {
        OperationRequest::ApplyTransaction { .. } => {
            operations.checked_add(4).ok_or_else(overflow)?
        }
        OperationRequest::Diagnostics {
            analysis: crate::semantic::schema::AnalysisLevel::Hir,
            ..
        } => 3,
        _ => 1,
    };
    let traversal_work = source_nodes
        .checked_mul(traversal_multiplier)
        .ok_or_else(overflow)?;
    let operation_work = operations.checked_mul(16).ok_or_else(overflow)?;
    let work_units = traversal_work
        .checked_add(operation_work)
        .ok_or_else(overflow)?;
    Ok(Charges {
        request_bytes,
        source_bytes,
        source_units,
        source_nodes,
        operations,
        work_units,
        output_bytes: 0,
    })
}

fn overflow() -> ProtocolError {
    error(
        ProtocolErrorCode::ResourceLimit,
        "protocol aggregate charge overflow",
    )
}
