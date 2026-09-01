//! Exact Graph 6 semantic-root and validation-certificate staging over generic immutable storage.

use super::{
    CanonicalBaseRead, CanonicalDelta, CanonicalReadWork, PreparedChangeAnalysis, WitnessBaseRead,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DependencyBinding, KernelSnapshot, OwnerBinding, OwnerKey, OwnerRecord, RetirementBinding,
    SemanticRoot, SemanticRootDigest, SemanticStateDigest, dependency_map_key, encode_dependency,
    encode_dependency_binding, encode_owner, encode_owner_binding, encode_retirement,
    encode_retirement_binding, encode_root, encode_type_object, owner_map_key, retirement_map_key,
    semantic_state_digest, semantic_state_digest_from_root,
};
use crate::platform::package_transport::validate_package_revision_closure;
use crate::platform::persistent_map::{
    BatchOutcome, MapAdmission, MapEdit, MapError, MapErrorClass, MapWork, PageStore, PersistentMap,
};
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, ObjectStage, StageOutcome, StoreError,
    StoreErrorClass, StoreWork,
};
use crate::platform::storage::page_store::ObjectPageStore;
use crate::platform::witness::{
    FullWitness, ValidationWitnessDigest, ValidationWitnessManifest, bind_witness_manifest,
    rebuild_full_witness,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CanonicalMapEditCounts {
    pub inserted: u64,
    pub replaced: u64,
    pub removed: u64,
    pub unchanged: u64,
}

#[derive(Clone, Debug)]
pub struct SemanticAuthority {
    pub root: SemanticRoot,
    pub digest: SemanticRootDigest,
    pub state: SemanticStateDigest,
    pub bytes: Vec<u8>,
    pub map_work: MapWork,
    pub map_edits: CanonicalMapEditCounts,
    pub canonical_read_work: CanonicalReadWork,
}

#[derive(Clone, Debug)]
pub struct WitnessAuthority {
    pub manifest: ValidationWitnessManifest,
    pub digest: ValidationWitnessDigest,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct PreparedAuthority {
    pub semantic: SemanticAuthority,
    pub witness: WitnessAuthority,
    pub store_work: StoreWork,
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedOwnerMapEdits {
    map_edits: Vec<MapEdit>,
    pub(crate) before_records: BTreeMap<OwnerKey, OwnerRecord>,
    pub(crate) read_work: CanonicalReadWork,
}

#[derive(Clone, Debug)]
pub struct FullStagedAuthority {
    pub binding: PreparedAuthority,
    pub snapshot: KernelSnapshot,
    pub witness: FullWitness,
}

/// Stages one already validated normalized delta without changing accepted storage visibility.
/// The supplied store is normally an [`crate::platform::storage::object::ObjectStage`] over the
/// exact accepted pack set. Exact base owner records are prepared first so their read work can be
/// deducted before the stage receives the remaining object and map admissions.
pub(crate) fn stage_prepared_authority<
    W: WitnessBaseRead + ?Sized,
    S: ImmutableObjectStore + ?Sized,
>(
    base_root: &SemanticRoot,
    base_witness: &W,
    prepared: &PreparedChangeAnalysis,
    owner_map_edits: &PreparedOwnerMapEdits,
    store: &mut ObjectStage<'_, S>,
    map_admission: MapAdmission,
) -> Result<PreparedAuthority, Vec<Diagnostic>> {
    let base_manifest = base_witness.witness_manifest();
    let (base_digest, _) = encode_root(base_root).map_err(|error| vec![error])?;
    if base_manifest.semantic_root != base_digest
        || base_manifest.repository_id != base_root.repository_id
        || base_manifest.package_id != base_root.package_id
    {
        return Err(vec![authority_error(
            DiagnosticClass::Corrupt,
            "change_authority_base_witness",
            "base validation witness does not bind the exact semantic authority",
        )]);
    }

    let mut store_work = StoreWork::default();
    let semantic = stage_semantic_delta(
        base_root,
        &prepared.canonical,
        owner_map_edits,
        store,
        &mut store_work,
        map_admission,
    )
    .map_err(|error| vec![error])?;
    stage_incremental_witness_objects(prepared, store, &mut store_work)
        .map_err(|error| vec![error])?;
    let (manifest, digest, bytes) = bind_witness_manifest(
        semantic.root.repository_id,
        semantic.root.package_id,
        semantic.digest,
        prepared.witness.roots,
    )
    .map_err(|error| vec![error])?;
    stage_typed(
        store,
        ObjectDomain::ValidationWitness,
        digest.bytes(),
        &bytes,
        &mut store_work,
    )
    .map_err(|error| vec![error])?;
    Ok(PreparedAuthority {
        semantic,
        witness: WitnessAuthority {
            manifest,
            digest,
            bytes,
        },
        store_work,
    })
}

/// Full bootstrap and reconstruction oracle. It validates logical records independently, builds
/// canonical maps from sorted entries, then rebuilds and stages the complete witness. Ordinary
/// edits use [`stage_prepared_authority`] instead.
pub fn stage_full_authority<S: ImmutableObjectStore + ?Sized>(
    logical: &KernelSnapshot,
    store: &mut S,
) -> Result<FullStagedAuthority, Vec<Diagnostic>> {
    crate::platform::kernel::validate_full(logical)?;
    let mut store_work = StoreWork::default();
    let semantic =
        stage_full_semantic(logical, store, &mut store_work).map_err(|error| vec![error])?;
    let mut snapshot = logical.clone();
    snapshot.root = semantic.root.clone();
    let witness = rebuild_full_witness(&snapshot)?;
    stage_full_witness_objects(&witness, store, &mut store_work).map_err(|error| vec![error])?;
    Ok(FullStagedAuthority {
        binding: PreparedAuthority {
            semantic,
            witness: WitnessAuthority {
                manifest: witness.manifest.clone(),
                digest: witness.manifest_digest,
                bytes: witness.manifest_bytes.clone(),
            },
            store_work,
        },
        snapshot,
        witness,
    })
}

fn stage_semantic_delta<S: ImmutableObjectStore + ?Sized>(
    base_root: &SemanticRoot,
    delta: &CanonicalDelta,
    owner_map_edits: &PreparedOwnerMapEdits,
    store: &mut S,
    store_work: &mut StoreWork,
    map_admission: MapAdmission,
) -> Result<SemanticAuthority, Diagnostic> {
    stage_delta_objects(delta, store, store_work)?;
    let mut map_work = MapWork::with_admission(map_admission);
    let mut counts = CanonicalMapEditCounts::default();
    let root = base_root;
    let dependency_map_edits = dependency_edits(delta);
    let retirement_map_edits = retirement_edits(delta);
    let (owners, dependencies, retirements, page_store_work) = {
        let mut page_store = ObjectPageStore::new(&mut *store);
        let owners = apply_map(
            root.owners,
            &owner_map_edits.map_edits,
            &mut page_store,
            &mut map_work,
            &mut counts,
        )?;
        let dependencies = apply_map(
            root.dependencies,
            &dependency_map_edits,
            &mut page_store,
            &mut map_work,
            &mut counts,
        )?;
        let retirements = apply_map(
            root.retirements,
            &retirement_map_edits,
            &mut page_store,
            &mut map_work,
            &mut counts,
        )?;
        (owners, dependencies, retirements, page_store.work())
    };
    merge_store_work(store_work, page_store_work);

    let root = SemanticRoot {
        graph_contract_version: root.graph_contract_version,
        repository_id: root.repository_id,
        package_id: root.package_id,
        package_name: root.package_name.clone(),
        owners,
        dependencies,
        retirements,
    };
    let (digest, bytes) = encode_root(&root)?;
    let state = semantic_state_digest_from_root(&root)?;
    stage_typed(
        store,
        ObjectDomain::SemanticRoot,
        digest.bytes(),
        &bytes,
        store_work,
    )?;
    Ok(SemanticAuthority {
        root,
        digest,
        state,
        bytes,
        map_work,
        map_edits: counts,
        canonical_read_work: owner_map_edits.read_work,
    })
}

fn stage_full_semantic<S: ImmutableObjectStore + ?Sized>(
    logical: &KernelSnapshot,
    store: &mut S,
    store_work: &mut StoreWork,
) -> Result<SemanticAuthority, Diagnostic> {
    let mut owner_entries = Vec::with_capacity(logical.owners.len());
    for (owner, record) in &logical.owners {
        let (digest, bytes) = encode_owner(record)?;
        stage_typed(
            store,
            ObjectDomain::Owner,
            digest.bytes(),
            &bytes,
            store_work,
        )?;
        owner_entries.push((
            owner_map_key(*owner).to_vec(),
            encode_owner_binding(&OwnerBinding {
                kind: record.kind(),
                object: digest,
            }),
        ));
    }
    for (digest, object) in &logical.types {
        let (actual, bytes) = encode_type_object(object)?;
        if actual != *digest {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "change_authority_type_digest",
                "logical type map is bound under a foreign digest",
            ));
        }
        stage_typed(
            store,
            ObjectDomain::Type,
            digest.bytes(),
            &bytes,
            store_work,
        )?;
    }
    for digest in logical.blobs.keys() {
        let key = ObjectKey::from_digest(ObjectDomain::Blob, digest.bytes());
        if !store.contains(key, store_work).map_err(store_diagnostic)? {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "change_authority_blob_missing",
                format!("reachable blob object {digest} is absent from immutable storage"),
            ));
        }
    }

    let mut dependency_entries = Vec::with_capacity(logical.dependencies.len());
    for (package, record) in &logical.dependencies {
        validate_package_revision_closure(
            store,
            record.package_revision,
            Some(record),
            store_work,
        )?;
        let (digest, bytes) = encode_dependency(record)?;
        stage_typed(
            store,
            ObjectDomain::Dependency,
            digest.bytes(),
            &bytes,
            store_work,
        )?;
        dependency_entries.push((
            dependency_map_key(*package).to_vec(),
            encode_dependency_binding(&DependencyBinding { object: digest }),
        ));
    }
    let mut retirement_entries = Vec::with_capacity(logical.retirements.len());
    for (owner, record) in &logical.retirements {
        let (digest, bytes) = encode_retirement(record)?;
        stage_typed(
            store,
            ObjectDomain::Retirement,
            digest.bytes(),
            &bytes,
            store_work,
        )?;
        retirement_entries.push((
            retirement_map_key(*owner).to_vec(),
            encode_retirement_binding(&RetirementBinding { object: digest }),
        ));
    }

    let mut map_work = MapWork::default();
    let (owners, dependencies, retirements, page_store_work) = {
        let mut page_store = ObjectPageStore::new(&mut *store);
        let owners = PersistentMap::from_sorted(&mut page_store, owner_entries, &mut map_work)
            .map_err(map_diagnostic)?
            .root();
        let dependencies =
            PersistentMap::from_sorted(&mut page_store, dependency_entries, &mut map_work)
                .map_err(map_diagnostic)?
                .root();
        let retirements =
            PersistentMap::from_sorted(&mut page_store, retirement_entries, &mut map_work)
                .map_err(map_diagnostic)?
                .root();
        (owners, dependencies, retirements, page_store.work())
    };
    merge_store_work(store_work, page_store_work);

    let root = SemanticRoot {
        graph_contract_version: logical.root.graph_contract_version,
        repository_id: logical.root.repository_id,
        package_id: logical.root.package_id,
        package_name: logical.root.package_name.clone(),
        owners,
        dependencies,
        retirements,
    };
    let (digest, bytes) = encode_root(&root)?;
    let state = semantic_state_digest_from_root(&root)?;
    let oracle = semantic_state_digest(logical)?;
    if state != oracle {
        return Err(authority_error(
            DiagnosticClass::Corrupt,
            "change_authority_semantic_state",
            "staged sparse semantic-state commitment disagrees with the independent full reconstruction",
        ));
    }
    stage_typed(
        store,
        ObjectDomain::SemanticRoot,
        digest.bytes(),
        &bytes,
        store_work,
    )?;
    Ok(SemanticAuthority {
        root,
        digest,
        state,
        bytes,
        map_work,
        map_edits: CanonicalMapEditCounts {
            inserted: owners
                .entries()
                .saturating_add(dependencies.entries())
                .saturating_add(retirements.entries()),
            ..CanonicalMapEditCounts::default()
        },
        canonical_read_work: CanonicalReadWork::default(),
    })
}

fn stage_delta_objects<S: ImmutableObjectStore + ?Sized>(
    delta: &CanonicalDelta,
    store: &mut S,
    work: &mut StoreWork,
) -> Result<(), Diagnostic> {
    for edit in delta.owners.values() {
        if let Some((expected, record)) = &edit.after {
            let (digest, bytes) = encode_owner(record)?;
            require_digest("owner", digest.bytes(), expected.bytes())?;
            stage_typed(store, ObjectDomain::Owner, digest.bytes(), &bytes, work)?;
        }
    }
    for (expected, object) in &delta.type_additions {
        let (digest, bytes) = encode_type_object(object)?;
        require_digest("type", digest.bytes(), expected.bytes())?;
        stage_typed(store, ObjectDomain::Type, digest.bytes(), &bytes, work)?;
    }
    for edit in delta.dependencies.values() {
        if let Some((expected, record)) = &edit.after {
            validate_package_revision_closure(store, record.package_revision, Some(record), work)?;
            let (digest, bytes) = encode_dependency(record)?;
            require_digest("dependency", digest.bytes(), expected.bytes())?;
            stage_typed(
                store,
                ObjectDomain::Dependency,
                digest.bytes(),
                &bytes,
                work,
            )?;
        }
    }
    for edit in delta.retirements.values() {
        if let Some((expected, record)) = &edit.after {
            let (digest, bytes) = encode_retirement(record)?;
            require_digest("retirement", digest.bytes(), expected.bytes())?;
            stage_typed(
                store,
                ObjectDomain::Retirement,
                digest.bytes(),
                &bytes,
                work,
            )?;
        }
    }
    Ok(())
}

pub(crate) fn prepare_owner_map_edits<B: CanonicalBaseRead + ?Sized>(
    base: &B,
    delta: &CanonicalDelta,
) -> Result<PreparedOwnerMapEdits, Diagnostic> {
    let mut map_edits = Vec::with_capacity(delta.owners.len());
    let mut before_records = BTreeMap::new();
    let mut read_work = CanonicalReadWork::default();
    for (owner, edit) in &delta.owners {
        let before = match edit.before {
            Some(object) => {
                let read = base.read_owner(*owner)?;
                read_work.add(read.work);
                let record = read.value.ok_or_else(|| {
                    authority_error(
                        DiagnosticClass::Corrupt,
                        "change_authority_owner_before",
                        "owner delta names a missing exact base record",
                    )
                })?;
                let (actual, _) = encode_owner(&record)?;
                if actual != object {
                    return Err(authority_error(
                        DiagnosticClass::Corrupt,
                        "change_authority_owner_before_digest",
                        "owner delta before digest disagrees with exact base authority",
                    ));
                }
                let kind = record.kind();
                before_records.insert(*owner, record);
                Some(encode_owner_binding(&OwnerBinding { kind, object }))
            }
            None => None,
        };
        let after = edit.after.as_ref().map(|(object, record)| {
            encode_owner_binding(&OwnerBinding {
                kind: record.kind(),
                object: *object,
            })
        });
        map_edits.push(MapEdit {
            key: owner_map_key(*owner).to_vec(),
            before,
            after,
        });
    }
    Ok(PreparedOwnerMapEdits {
        map_edits,
        before_records,
        read_work,
    })
}

fn dependency_edits(delta: &CanonicalDelta) -> Vec<MapEdit> {
    delta
        .dependencies
        .iter()
        .map(|(package, edit)| MapEdit {
            key: dependency_map_key(*package).to_vec(),
            before: edit
                .before
                .map(|object| encode_dependency_binding(&DependencyBinding { object })),
            after: edit.after.as_ref().map(|(object, _)| {
                encode_dependency_binding(&DependencyBinding { object: *object })
            }),
        })
        .collect()
}

fn retirement_edits(delta: &CanonicalDelta) -> Vec<MapEdit> {
    delta
        .retirements
        .iter()
        .map(|(owner, edit)| MapEdit {
            key: retirement_map_key(*owner).to_vec(),
            before: edit
                .before
                .map(|object| encode_retirement_binding(&RetirementBinding { object })),
            after: edit.after.as_ref().map(|(object, _)| {
                encode_retirement_binding(&RetirementBinding { object: *object })
            }),
        })
        .collect()
}

fn apply_map<S: PageStore + ?Sized>(
    root: crate::platform::persistent_map::MapRoot,
    edits: &[MapEdit],
    store: &mut S,
    work: &mut MapWork,
    counts: &mut CanonicalMapEditCounts,
) -> Result<crate::platform::persistent_map::MapRoot, Diagnostic> {
    let (map, outcome) = PersistentMap::from_root(root)
        .apply_sorted_edits(store, edits, work)
        .map_err(map_diagnostic)?;
    add_map_counts(counts, outcome);
    Ok(map.root())
}

fn add_map_counts(counts: &mut CanonicalMapEditCounts, outcome: BatchOutcome) {
    counts.inserted = counts.inserted.saturating_add(outcome.inserted);
    counts.replaced = counts.replaced.saturating_add(outcome.replaced);
    counts.removed = counts.removed.saturating_add(outcome.removed);
    counts.unchanged = counts.unchanged.saturating_add(outcome.unchanged);
}

fn stage_incremental_witness_objects<S: ImmutableObjectStore + ?Sized>(
    prepared: &PreparedChangeAnalysis,
    store: &mut S,
    work: &mut StoreWork,
) -> Result<(), Diagnostic> {
    for (digest, bytes) in &prepared.summaries.final_delta.new_objects {
        stage_typed(
            store,
            ObjectDomain::OwnerSummary,
            digest.bytes(),
            bytes,
            work,
        )?;
    }
    for (digest, bytes) in prepared.witness.new_pages.objects() {
        stage_typed(store, ObjectDomain::MapPage, digest.bytes(), bytes, work)?;
    }
    Ok(())
}

fn stage_full_witness_objects<S: ImmutableObjectStore + ?Sized>(
    witness: &FullWitness,
    store: &mut S,
    work: &mut StoreWork,
) -> Result<(), Diagnostic> {
    for (digest, bytes) in &witness.summary_objects {
        stage_typed(
            store,
            ObjectDomain::OwnerSummary,
            digest.bytes(),
            bytes,
            work,
        )?;
    }
    for (digest, bytes) in witness.pages.objects() {
        stage_typed(store, ObjectDomain::MapPage, digest.bytes(), bytes, work)?;
    }
    stage_typed(
        store,
        ObjectDomain::ValidationWitness,
        witness.manifest_digest.bytes(),
        &witness.manifest_bytes,
        work,
    )?;
    Ok(())
}

fn stage_typed<S: ImmutableObjectStore + ?Sized>(
    store: &mut S,
    domain: ObjectDomain,
    digest: [u8; 32],
    bytes: &[u8],
    work: &mut StoreWork,
) -> Result<StageOutcome, Diagnostic> {
    store
        .stage(ObjectKey::from_digest(domain, digest), bytes, work)
        .map_err(store_diagnostic)
}

fn require_digest(label: &str, actual: [u8; 32], expected: [u8; 32]) -> Result<(), Diagnostic> {
    if actual != expected {
        return Err(authority_error(
            DiagnosticClass::Corrupt,
            "change_authority_digest",
            format!("normalized {label} digest disagrees with canonical bytes"),
        ));
    }
    Ok(())
}

fn merge_store_work(total: &mut StoreWork, update: StoreWork) {
    total.catalog_lookups = total.catalog_lookups.saturating_add(update.catalog_lookups);
    total.packs_opened = total.packs_opened.saturating_add(update.packs_opened);
    total.objects_read = total.objects_read.saturating_add(update.objects_read);
    total.objects_staged = total.objects_staged.saturating_add(update.objects_staged);
    total.objects_reused = total.objects_reused.saturating_add(update.objects_reused);
    total.bytes_read = total.bytes_read.saturating_add(update.bytes_read);
    total.bytes_staged = total.bytes_staged.saturating_add(update.bytes_staged);
    total.pages_staged = total.pages_staged.saturating_add(update.pages_staged);
    total.packs_sealed = total.packs_sealed.saturating_add(update.packs_sealed);
}

fn map_diagnostic(error: MapError) -> Diagnostic {
    let class = match error.class {
        MapErrorClass::Input => DiagnosticClass::Semantic,
        MapErrorClass::Resource => DiagnosticClass::Resource,
        MapErrorClass::Corrupt => DiagnosticClass::Corrupt,
        MapErrorClass::Store => DiagnosticClass::Infrastructure,
    };
    let code = match error.code {
        "persistent_map_admission_pages_read" => "change_budget_canonical_map_pages",
        "persistent_map_admission_bytes_read" => "change_budget_canonical_bytes",
        "persistent_map_admission_entries_visited" => "change_budget_canonical_map_entries",
        "persistent_map_admission_pages_encoded" => "change_budget_canonical_map_pages_encoded",
        "persistent_map_admission_bytes_encoded" => "change_budget_canonical_map_bytes_encoded",
        "object_read_catalog_lookups_exhausted" => "change_budget_canonical_catalog_lookups",
        "object_read_objects_exhausted" => "change_budget_canonical_objects",
        "object_read_bytes_exhausted" => "change_budget_canonical_bytes",
        code => code,
    };
    Diagnostic::new(class, code, error.message)
}

fn store_diagnostic(error: StoreError) -> Diagnostic {
    let class = match error.class {
        StoreErrorClass::Input => DiagnosticClass::Source,
        StoreErrorClass::Resource => DiagnosticClass::Resource,
        StoreErrorClass::Corrupt => DiagnosticClass::Corrupt,
        StoreErrorClass::Io => DiagnosticClass::Infrastructure,
    };
    let code = match error.code {
        "object_read_catalog_lookups_exhausted" => "change_budget_canonical_catalog_lookups",
        "object_read_objects_exhausted" => "change_budget_canonical_objects",
        "object_read_bytes_exhausted" => "change_budget_canonical_bytes",
        code => code,
    };
    Diagnostic::new(class, code, error.message)
}

fn authority_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
