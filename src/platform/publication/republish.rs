//! Public, exact-authority republication across operational format cuts.
//!
//! The input is the existing public, code-complete package transport. Receipt and
//! retry history are deliberately replaced by one current-profile genesis. The
//! complete meaning and all canonical owner identities are preserved.

use super::*;
use crate::platform::kernel::{PackageTransportDigest, SemanticStateDigest};
use crate::platform::package_transport::source::PackageContainer;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug)]
pub struct RepublicationReceipt {
    pub previous: HeadRecord,
    pub current: HeadRecord,
    pub semantic_state: SemanticStateDigest,
    pub source_sha256: String,
    pub previous_head_sha256: String,
    pub current_head_sha256: String,
    pub owners: u64,
    pub types: u64,
    pub retirements: u64,
    pub blobs: u64,
    pub dependencies: u64,
    pub store_work: StoreWork,
}

fn sha256(bytes: &[u8]) -> String {
    crate::platform::semantic_id::encode_hex(&Sha256::digest(bytes))
}

fn count(value: usize) -> Result<u64, Diagnostic> {
    u64::try_from(value).map_err(|_| {
        repository_error(
            DiagnosticClass::Resource,
            "publication_republish_count",
            "complete source inventory is not representable",
        )
    })
}

impl GraphRepository {
    /// Re-admit the exact exported current meaning into current-format history.
    ///
    /// The caller selects the complete old HEAD hash and the exact source
    /// transport. The operation does not decode an old receipt, trust an old
    /// validation certificate, recreate identities from text, or modify stored
    /// canonical objects. It stages and verifies a normal initial publication,
    /// then atomically replaces HEAD under the ordinary repository write lock.
    ///
    /// Previous retry requests retain their old base and deterministically
    /// become stale; this operation starts a new acceptance/idempotency history.
    pub fn republish_current_transport(
        root: &Path,
        expected_head_sha256: &str,
        transport: PackageTransportDigest,
        source: &[u8],
        control: &ExecutionControl,
    ) -> Result<RepublicationReceipt, Diagnostic> {
        republish(root, expected_head_sha256, transport, source, control, None)
    }
}

fn republish(
    root: &Path,
    expected_head_sha256: &str,
    transport: PackageTransportDigest,
    source: &[u8],
    control: &ExecutionControl,
    fault: Option<PublicationPoint>,
) -> Result<RepublicationReceipt, Diagnostic> {
    if expected_head_sha256.len() != 64
        || !expected_head_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(repository_error(
            DiagnosticClass::Source,
            "publication_republish_head_hash",
            "expected HEAD must be one canonical lowercase SHA-256 digest",
        ));
    }
    control.check().map_err(execution_diagnostic)?;
    let metadata = fs::symlink_metadata(root)
        .map_err(|error| io_diagnostic("publication_republish_root", root, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(repository_error(
            DiagnosticClass::Source,
            "publication_republish_root",
            "republication requires an existing non-symlink project directory",
        ));
    }
    let root = root
        .canonicalize()
        .map_err(|error| io_diagnostic("publication_republish_root", root, error))?;
    let root_directory = open_directory(&root)?;
    let lock = open_lock(&root_directory)?;
    FileExt::lock_exclusive(&lock)
        .map_err(|error| io_diagnostic("publication_republish_lock", &root, error))?;
    control.check().map_err(execution_diagnostic)?;

    let old_bytes = read_optional_regular_at(
        &root_directory,
        HEAD_FILE,
        MAXIMUM_HEAD_BYTES,
        "publication_republish_head",
    )?
    .ok_or_else(|| {
        repository_error(
            DiagnosticClass::Source,
            "publication_republish_head",
            "republication requires an existing exact accepted HEAD",
        )
    })?;
    let previous_head_sha256 = sha256(&old_bytes);
    if previous_head_sha256 != expected_head_sha256 {
        return Err(repository_error(
            DiagnosticClass::Source,
            "publication_republish_stale",
            "current HEAD differs from the exact exported republication base",
        ));
    }
    let previous = HeadRecord::decode(&old_bytes)?;
    // This deliberately opens only the packed store. GraphRepository::open would
    // require a receipt decoder from the old operational profile.
    let mut store = StorageBackend::Disk(Box::new(
        PackDirectoryStore::open(&root).map_err(store_diagnostic)?,
    ));
    let mut store_work = StoreWork::default();
    let old_record = read_required(
        &store,
        ObjectDomain::Revision,
        previous.record.bytes(),
        &mut store_work,
    )?;
    let old_record = RevisionRecord::decode(&old_record, previous.record)?;
    if old_record.revision != previous.revision
        || old_record.core.repository_id != previous.repository_id
    {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            "publication_republish_head_binding",
            "HEAD and its revision record do not bind the same source authority",
        ));
    }

    let container = PackageContainer::decode(source, transport)?;
    let admitted = container.admit_with_control(control)?;
    let package = admitted
        .packages
        .get(&container.root.package_revision)
        .ok_or_else(|| {
            repository_error(
                DiagnosticClass::Corrupt,
                "publication_republish_source",
                "complete source transport omits its root package",
            )
        })?;
    if package.revision.revision != old_record.core
        || package.transport.semantic_root != old_record.publication.semantic_root
        || package.snapshot.root.repository_id != previous.repository_id
        || semantic_state_digest_from_root(&package.snapshot.root)?
            != old_record.core.semantic_state
    {
        return Err(repository_error(
            DiagnosticClass::Source,
            "publication_republish_source_binding",
            "source transport does not bind the exact current revision and semantic root",
        ));
    }
    let logical = &package.snapshot;
    let source_sha256 = sha256(source);
    let intent = format!(
        "Current-format republication of {} from HEAD sha256={} and complete source sha256={}; fresh acceptance and retry history, canonical identities preserved.",
        previous.revision, previous_head_sha256, source_sha256
    );
    control.check().map_err(execution_diagnostic)?;
    // The public source container includes exact dependency selections and every
    // local blob. Its normal installation path verifies and seals them before
    // the normal full-authority preparation can reference them.
    let installed = install_package_container(
        Some(&root_directory),
        &mut store,
        transport,
        source,
        None,
        control,
    )?;
    store_work.add(installed.work);
    let prepared =
        prepare_initial_publication(logical, &store, Some(intent)).map_err(collapse_diagnostics)?;
    let publication = &prepared.publication;
    if prepared.snapshot.owners != logical.owners
        || prepared.snapshot.types != logical.types
        || prepared.snapshot.dependencies != logical.dependencies
        || prepared.snapshot.retirements != logical.retirements
        || prepared.snapshot.blobs != logical.blobs
        || prepared.snapshot.dependency_types != logical.dependency_types
        || prepared.snapshot.dependency_interfaces != logical.dependency_interfaces
        || prepared.snapshot.representation_dependencies != logical.representation_dependencies
        || publication.authority.semantic.state != old_record.core.semantic_state
        || prepared.snapshot.root.repository_id != logical.root.repository_id
        || prepared.snapshot.root.package_id != logical.root.package_id
        || prepared.snapshot.root.package_name != logical.root.package_name
    {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            "publication_republish_identity",
            "current-format preparation changed complete supplied meaning or identities",
        ));
    }
    validate_prepared_repository(publication)?;
    validate_history_base(
        None,
        publication.receipt.status,
        &publication.transaction,
        &publication.semantic_diff,
    )?;
    validate_prepared_dependency_sources(&store, publication)?;
    validate_candidate_idempotency_transition(
        &store,
        publication.revision.publication.idempotency_receipts,
        None,
        &publication.objects,
        &mut store_work,
    )?;
    inject(fault, PublicationPoint::BeforeObjectStage)?;
    for (index, (key, bytes)) in publication.objects.iter().enumerate() {
        control.check().map_err(execution_diagnostic)?;
        store
            .stage(*key, bytes, &mut store_work)
            .map_err(store_diagnostic)?;
        if index == 0 {
            inject(fault, PublicationPoint::AfterFirstObjectStage)?;
        }
    }
    store
        .seal_staged(TARGET_PACK_BYTES, &mut store_work)
        .map_err(store_diagnostic)?;
    inject(fault, PublicationPoint::AfterPacksSealed)?;
    verify_prepared_closure(
        &store,
        None,
        publication,
        &mut store_work,
        CanonicalReadAdmission::unbounded(),
    )?;
    control.check().map_err(execution_diagnostic)?;
    replace_head(&root_directory, &publication.head_bytes, fault)?;
    let current = read_current_optional(&root_directory, &store)?.ok_or_else(|| {
        repository_error(
            DiagnosticClass::Infrastructure,
            "publication_republish_visibility",
            "current-format HEAD is absent after atomic publication",
        )
    })?;
    if current.head != publication.head {
        return Err(repository_error(
            DiagnosticClass::Infrastructure,
            "publication_republish_visibility",
            "visible HEAD differs from the verified current-format publication",
        ));
    }
    Ok(RepublicationReceipt {
        previous,
        current: current.head,
        semantic_state: publication.authority.semantic.state,
        source_sha256,
        previous_head_sha256,
        current_head_sha256: sha256(&publication.head_bytes),
        owners: count(logical.owners.len())?,
        types: count(logical.types.len())?,
        retirements: count(logical.retirements.len())?,
        blobs: count(logical.blobs.len())?,
        dependencies: count(logical.dependencies.len())?,
        store_work,
    })
}

#[cfg(test)]
#[path = "republish_tests.rs"]
mod tests;
