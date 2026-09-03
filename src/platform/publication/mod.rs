//! Private Graph 9 accepted-history binding and publication protocol.

#![allow(
    unused_imports,
    reason = "private publication exports become repository consumers at the Graph 9 cutover"
)]

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

pub(crate) use diff::validate_owner_entry;
pub use diff::{
    DependencyDiffEntry, OwnerChangeClass, OwnerDiffEntry, RetirementDiffEntry, SemanticDiff,
    SemanticDiffBody, SummaryDimensions,
};
pub use digest::{
    ReceiptObjectDigest, RevisionObjectDigest, SemanticDiffDigest, TransactionDigest,
};
pub use idempotency::IdempotencyBinding;
pub(crate) use idempotency::idempotency_key_is_valid;
pub use prepare::{
    PreparedInitialPublication, PreparedPublication, PublicationOptions,
    prepare_change_publication, prepare_initial_publication,
};
pub use read_view::{
    ExportedPackageTransport, MAXIMUM_RELATION_READ_ITEMS, MAXIMUM_TEST_DEPENDENCY_READ_ITEMS,
    PreparedAuthoredPublication, RelationRead, RepositoryReadWork, RepositoryView, RevisionRead,
    RevisionWitnessMapUpdate, TestDependencyRead,
};
pub(crate) use read_view::{
    RepositoryDefinitionAdmission, RepositoryDefinitionReader, RepositoryQueryAdmission,
    RepositoryQueryRangeRead, RepositoryRelationQueryRange,
};
pub use receipt::{
    ChangeCounts, FullOracleStatus, PublicationReceipt, PublicationStatus, ValidationEvidence,
    ValidationProfile, WorkObservation,
};
pub use repository::{
    CreatedRepository, CurrentPublication, GraphRepository, InitialPackageTransport,
    PackageTransportStageReceipt, PublicationOutcome, PublicationPoint, ReconciliationResult,
    ReconciliationStatus, ReconciliationWork,
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
