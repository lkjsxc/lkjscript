//! Private Graph 5 accepted-history binding and publication protocol.

#![allow(
    unused_imports,
    reason = "private publication exports become repository consumers at the Graph 5 cutover"
)]

mod authored_protocol;
pub(crate) mod contract;
mod diff;
mod digest;
mod idempotency;
mod prepare;
mod read_view;
mod receipt;
mod repository;
mod revision;
mod transaction;

pub use authored_protocol::{
    AUTHORED_CHANGE_CONTRACT_IDENTITY, AUTHORED_CHANGE_CONTRACT_VERSION,
    AUTHORED_PROTOCOL_SCHEMA_DIGEST_DOMAIN, AUTHORED_PROTOCOL_SCHEMA_ID, AuthoredChangeContract,
    AuthoredChangeRequest, AuthoredChangeResponse, AuthoredChangeResponseStatus,
    MAXIMUM_AUTHORED_JSON_DEPTH, MAXIMUM_AUTHORED_JSON_ITEMS, MAXIMUM_AUTHORED_RESPONSE_BYTES,
    authored_protocol_schema, authored_protocol_schema_bytes, authored_protocol_schema_digest,
};
pub use diff::{
    DependencyDiffEntry, OwnerChangeClass, OwnerDiffEntry, RetirementDiffEntry, SemanticDiff,
    SemanticDiffBody, SummaryDimensions,
};
pub use digest::{
    ReceiptObjectDigest, RevisionObjectDigest, SemanticDiffDigest, TransactionDigest,
};
pub use idempotency::IdempotencyBinding;
pub use prepare::{
    PreparedInitialPublication, PreparedPublication, PublicationOptions,
    prepare_change_publication, prepare_initial_publication,
};
pub use read_view::{
    ExportedPackageObject, MAXIMUM_RELATION_READ_ITEMS, MAXIMUM_TEST_DEPENDENCY_READ_ITEMS,
    PreparedAuthoredPublication, RelationRead, RepositoryReadWork, RepositoryView, RevisionRead,
    RevisionWitnessMapUpdate, TestDependencyRead,
};
pub use receipt::{
    ChangeCounts, FullOracleStatus, PublicationReceipt, PublicationStatus, ValidationEvidence,
    ValidationProfile, WorkObservation,
};
pub use repository::{
    CreatedRepository, CurrentPublication, GraphRepository, PackageObjectStageReceipt,
    PublicationOutcome, PublicationPoint, ReconciliationResult, ReconciliationStatus,
    ReconciliationWork,
};
pub use revision::{
    AcceptedBinding, HeadRecord, ParentRevision, PublicationBinding, RevisionCore, RevisionRecord,
    ValidationEvidenceBinding,
};
pub use transaction::{
    DependencyTransactionEdit, DigestEdit, NormalizedTransaction, OwnerTransactionEdit,
    RetirementTransactionEdit, TransactionBody,
};

#[cfg(test)]
mod repository_tests;
#[cfg(test)]
mod tests;
