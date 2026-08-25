//! Deterministic linker from revision-bound normalized compiler units to a standalone artifact.

use super::artifact::{
    ARTIFACT_CONTRACT_VERSION, ArtifactManifest, ArtifactPackage, ArtifactRuntimeOwner,
    EncodedArtifact, LoadedArtifact, MAXIMUM_ARTIFACT_REFERENCE_OWNERS, artifact_error,
    closure_facts, encode_artifact, runtime_owner_expectations,
};
use super::cache::load_exact_current_compilation;
use super::manifest::{COMPILATION_MANIFEST_CONTRACT_VERSION, CompilationBinding};
use super::unit::{
    BYTECODE_CONTRACT_VERSION, COMPILER_UNIT_CONTRACT_VERSION, CompilationPayload, CompilationUnit,
    CompiledText,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DeclarationPayload, EncodedOwnerKey, ExpressionOperation, LocalValueReference, OwnerBinding,
    OwnerKey, OwnerKind, OwnerRecord, PackageId, PortImplementation, TypeObjectDigest,
    decode_type_object, encode_owner, encode_owner_binding,
};
use crate::platform::persistent_map::{
    MapError, MapErrorClass, MapWork, MemoryPageStore, PersistentMap,
};
use crate::platform::publication::{GraphRepository, RepositoryReadWork, RepositoryView};
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StoreError, StoreErrorClass, StoreWork,
};
use crate::platform::storage::pack::PackMetadata;
use crate::platform::storage::page_store::ObjectPageReader;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ArtifactLinkWork {
    pub repository: RepositoryReadWork,
    pub compilation_map: MapWork,
    pub store: StoreWork,
    pub dependency_artifacts: u64,
    pub packages: u64,
    pub compiler_units: u64,
    pub type_objects: u64,
    pub blob_objects: u64,
    pub runtime_owners: u64,
    pub reference_owners: u64,
    pub reference_map: MapWork,
    pub closure_objects: u64,
    pub closure_bytes: u64,
    pub output_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct ArtifactLinkReceipt {
    pub artifact: EncodedArtifact,
    pub work: ArtifactLinkWork,
}

/// Links the exact current package and already validated dependency artifacts.
///
/// Dependency bundles are flattened by immutable object identity. The strict artifact loader then
/// proves that the final package set is exactly the root package-object dependency closure, so an
/// unrelated or stale supplied dependency artifact cannot enter the result.
pub fn link_artifact(
    repository: &GraphRepository,
    compilation: super::manifest::CompilationManifestDigest,
    dependencies: &[LoadedArtifact],
) -> Result<ArtifactLinkReceipt, Diagnostic> {
    let view = repository.view_current()?;
    let cached = load_exact_current_compilation(repository, compilation)?;
    let mut work = ArtifactLinkWork::default();
    let mut objects = BTreeMap::new();
    let mut packages = BTreeMap::new();

    for dependency in dependencies {
        work.dependency_artifacts = work.dependency_artifacts.saturating_add(1);
        for package in &dependency.manifest.packages {
            insert_package(&mut packages, package.clone())?;
        }
        for (key, bytes) in &dependency.objects {
            insert_object(&mut objects, *key, bytes.clone())?;
        }
    }

    let exported = repository.export_package_object()?;
    add_repository_work(&mut work.repository, exported.read_work);
    add_map_work(&mut work.compilation_map, exported.interface_map_work);
    work.store.add(exported.closure_work);
    for pack in &exported.packs {
        import_pack(pack, &mut objects)?;
    }
    if exported.object.package != view.package()
        || exported.object.semantic_revision != view.revision()
        || exported.object.semantic_root != view.current().accepted.semantic_root
        || exported.object.witness.certificate != view.current().witness.certificate
    {
        return Err(link_error(
            DiagnosticClass::Corrupt,
            "artifact_link_package_export",
            "exported package object does not bind the exact linker revision",
        ));
    }
    for dependency in &exported.object.dependencies {
        let supplied = packages.get(&dependency.package).ok_or_else(|| {
            link_error(
                DiagnosticClass::Semantic,
                "artifact_link_dependency_missing",
                "linking requires an exact compiled artifact for every semantic dependency",
            )
        })?;
        if supplied.package_object != dependency.package_object {
            return Err(link_error(
                DiagnosticClass::Semantic,
                "artifact_link_dependency_binding",
                "dependency artifact does not bind the exact accepted package object",
            ));
        }
    }

    let store = repository.object_store()?;
    let compilation_key = compilation.object_key();
    let compilation_bytes = store
        .read(
            compilation_key,
            ObjectDomain::CompilationManifest.maximum_bytes(),
            &mut work.store,
        )
        .map_err(store_diagnostic)?
        .ok_or_else(|| {
            link_error(
                DiagnosticClass::Corrupt,
                "artifact_link_compilation_missing",
                "current compilation manifest object is missing",
            )
        })?;
    insert_object(&mut objects, compilation_key, compilation_bytes)?;

    let reader = ObjectPageReader::new(&store);
    let units_map = PersistentMap::from_root(cached.manifest.units);
    let mut pages = MemoryPageStore::default();
    units_map
        .copy_reachable(&reader, &mut pages, &mut work.compilation_map)
        .map_err(map_diagnostic)?;
    for (digest, bytes) in pages.objects() {
        insert_object(
            &mut objects,
            ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes()),
            bytes.to_vec(),
        )?;
    }
    let mut bindings = Vec::new();
    let mut captured = None;
    let result = units_map.for_each(&reader, &mut work.compilation_map, |key, value| {
        let operation = (|| {
            let owner = EncodedOwnerKey::decode(key)?;
            let binding = CompilationBinding::decode(value, owner)?;
            bindings.push((owner, binding));
            Ok::<(), Diagnostic>(())
        })();
        match operation {
            Ok(()) => Ok(()),
            Err(diagnostic) => {
                captured = Some(diagnostic);
                Err(MapError {
                    class: MapErrorClass::Corrupt,
                    code: "artifact_link_unit_iteration_stop",
                    message: "artifact linker stopped after an exact compiler-unit diagnostic"
                        .to_owned(),
                })
            }
        }
    });
    if let Some(diagnostic) = captured {
        return Err(diagnostic);
    }
    result.map_err(map_diagnostic)?;
    work.store.add(reader.work());

    let mut types = BTreeSet::new();
    let mut blobs = BTreeMap::new();
    let mut local_units = BTreeMap::new();
    for (owner, binding) in bindings {
        let unit_bytes = store
            .read(
                binding.object.object_key(),
                ObjectDomain::CompilerUnit.maximum_bytes(),
                &mut work.store,
            )
            .map_err(store_diagnostic)?
            .ok_or_else(|| {
                link_error(
                    DiagnosticClass::Corrupt,
                    "artifact_link_unit_missing",
                    "compilation manifest references a missing compiler unit",
                )
            })?;
        let unit = CompilationUnit::decode(&unit_bytes, binding.object.object_key())?;
        if unit.key != binding.key
            || unit.source.package != view.package()
            || unit.source.owner != owner
            || unit.source.kind != binding.kind
        {
            return Err(link_error(
                DiagnosticClass::Corrupt,
                "artifact_link_unit_binding",
                "compiler unit disagrees with the exact compilation-manifest binding",
            ));
        }
        types.extend(unit.tables.types.iter().copied());
        for text in &unit.tables.texts {
            if let CompiledText::Blob { digest, bytes } = text
                && let Some(previous) = blobs.insert(*digest, *bytes)
                && previous != *bytes
            {
                return Err(link_error(
                    DiagnosticClass::Corrupt,
                    "artifact_link_blob_binding",
                    "compiler units bind one blob digest to conflicting lengths",
                ));
            }
        }
        insert_object(&mut objects, binding.object.object_key(), unit_bytes)?;
        if local_units.insert((view.package(), owner), unit).is_some() {
            return Err(link_error(
                DiagnosticClass::Corrupt,
                "artifact_link_unit_duplicate",
                "compilation manifest repeats one exact compiler-unit owner",
            ));
        }
        work.compiler_units = work.compiler_units.saturating_add(1);
    }

    let mut runtime_owners = Vec::new();
    let mut runtime_records = BTreeMap::new();
    for ((package, owner), expectation) in runtime_owner_expectations(&local_units)? {
        if package != view.package() {
            return Err(link_error(
                DiagnosticClass::Corrupt,
                "artifact_link_runtime_owner_package",
                "local compiler units produced runtime metadata for another package",
            ));
        }
        let read = view.owner(owner)?;
        add_repository_work(&mut work.repository, read.work);
        let record = read.value.ok_or_else(|| {
            link_error(
                DiagnosticClass::Corrupt,
                "artifact_link_runtime_owner_missing",
                "compiler units require a missing canonical runtime owner record",
            )
        })?;
        let (digest, bytes) = encode_owner(&record)?;
        insert_object(
            &mut objects,
            ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
            bytes,
        )?;
        runtime_owners.push(ArtifactRuntimeOwner {
            owner,
            kind: expectation.kind(),
            object: digest,
        });
        if runtime_records.insert(owner, record).is_some() {
            return Err(link_error(
                DiagnosticClass::Corrupt,
                "artifact_link_runtime_owner_duplicate",
                "compiler units produced duplicate exact runtime-owner metadata",
            ));
        }
        work.runtime_owners = work.runtime_owners.saturating_add(1);
    }

    let reference_owners = build_reference_owner_map(
        &view,
        &local_units,
        &runtime_records,
        &mut objects,
        &mut work,
    )?;

    let mut resolved_types = BTreeSet::new();
    while let Some(digest) = types.pop_first() {
        if !resolved_types.insert(digest) {
            continue;
        }
        let key = ObjectKey::from_digest(ObjectDomain::Type, digest.bytes());
        let bytes = store
            .read(key, ObjectDomain::Type.maximum_bytes(), &mut work.store)
            .map_err(store_diagnostic)?
            .ok_or_else(|| {
                link_error(
                    DiagnosticClass::Corrupt,
                    "artifact_link_type_missing",
                    "compiler unit references a missing canonical type object",
                )
            })?;
        let object = decode_type_object(&bytes, digest)?;
        types.extend(object.child_types());
        insert_object(&mut objects, key, bytes)?;
        work.type_objects = work.type_objects.saturating_add(1);
    }
    for (digest, expected_length) in blobs {
        let key = ObjectKey::from_digest(ObjectDomain::Blob, digest.bytes());
        let bytes = store
            .read(key, ObjectDomain::Blob.maximum_bytes(), &mut work.store)
            .map_err(store_diagnostic)?
            .ok_or_else(|| {
                link_error(
                    DiagnosticClass::Corrupt,
                    "artifact_link_blob_missing",
                    "compiler unit references a missing canonical blob object",
                )
            })?;
        if bytes.len() as u64 != expected_length {
            return Err(link_error(
                DiagnosticClass::Corrupt,
                "artifact_link_blob_length",
                "canonical blob length disagrees with the compiler-unit binding",
            ));
        }
        insert_object(&mut objects, key, bytes)?;
        work.blob_objects = work.blob_objects.saturating_add(1);
    }

    insert_package(
        &mut packages,
        ArtifactPackage {
            repository_id: view.current().head.repository_id,
            package: view.package(),
            package_object: exported.digest,
            compilation,
            runtime_owners,
            reference_owners,
        },
    )?;
    let packages = packages.into_values().collect::<Vec<_>>();
    let (closure, object_count, object_bytes) = closure_facts(&objects)?;
    let manifest = ArtifactManifest {
        contract_version: ARTIFACT_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        compiler_contract_version: COMPILER_UNIT_CONTRACT_VERSION,
        bytecode_contract_version: BYTECODE_CONTRACT_VERSION,
        compilation_manifest_contract_version: COMPILATION_MANIFEST_CONTRACT_VERSION,
        root_package: view.package(),
        packages,
        closure,
        object_count,
        object_bytes,
    };
    let artifact = encode_artifact(manifest, &objects)?;
    work.packages = artifact.manifest.packages.len() as u64;
    work.closure_objects = object_count;
    work.closure_bytes = object_bytes;
    work.output_bytes = artifact.bytes.len() as u64;
    Ok(ArtifactLinkReceipt { artifact, work })
}

fn build_reference_owner_map(
    view: &RepositoryView,
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    runtime_owners: &BTreeMap<OwnerKey, OwnerRecord>,
    objects: &mut BTreeMap<ObjectKey, Vec<u8>>,
    work: &mut ArtifactLinkWork,
) -> Result<crate::platform::persistent_map::MapRoot, Diagnostic> {
    let package = view.package();
    let mut pending = BTreeSet::new();
    for ((unit_package, owner), unit) in units {
        if *unit_package == package && reference_compilation_payload(&unit.payload) {
            pending.insert(*owner);
        }
    }
    for record in runtime_owners.values() {
        if let OwnerRecord::Port(port) = record
            && let PortImplementation::Expression(expression) = port.implementation
        {
            pending.insert(OwnerKey::Expression(expression));
        }
    }

    let mut bindings = BTreeMap::<OwnerKey, OwnerBinding>::new();
    while let Some(owner) = pending.pop_first() {
        if bindings.contains_key(&owner) {
            continue;
        }
        if bindings.len() as u64 == MAXIMUM_ARTIFACT_REFERENCE_OWNERS {
            return Err(link_error(
                DiagnosticClass::Resource,
                "artifact_link_reference_owner_count",
                "reference-execution closure exceeds its per-package owner bound",
            ));
        }
        let read = view.owner(owner)?;
        add_repository_work(&mut work.repository, read.work);
        let record = read.value.ok_or_else(|| {
            link_error(
                DiagnosticClass::Corrupt,
                "artifact_link_reference_owner_missing",
                "reference-execution closure requires a missing exact canonical owner",
            )
        })?;
        match &record {
            OwnerRecord::Declaration(declaration) => {
                let unit = units.get(&(package, owner)).ok_or_else(|| {
                    link_error(
                        DiagnosticClass::Corrupt,
                        "artifact_link_reference_declaration_unit",
                        "reference declaration has no exact local compiler unit",
                    )
                })?;
                if !reference_payload_matches(&unit.payload, &declaration.payload) {
                    return Err(link_error(
                        DiagnosticClass::Corrupt,
                        "artifact_link_reference_declaration_payload",
                        "reference declaration kind disagrees with its exact compiler unit",
                    ));
                }
                pending.extend(
                    record
                        .expression_roots()
                        .into_iter()
                        .map(OwnerKey::Expression),
                );
            }
            OwnerRecord::Expression(expression) => {
                pending.extend(
                    expression
                        .children()
                        .into_iter()
                        .map(|child| OwnerKey::Expression(child.expression)),
                );
                pending.extend(
                    reference_expression_bindings(&expression.operation)
                        .into_iter()
                        .map(OwnerKey::Binding),
                );
                for declaration in reference_expression_declarations(&expression.operation) {
                    if declaration.package == package {
                        require_local_reference_callable(units, declaration)?;
                    }
                }
            }
            OwnerRecord::Binding(_) => {
                pending.extend(
                    record
                        .expression_roots()
                        .into_iter()
                        .map(OwnerKey::Expression),
                );
            }
            _ => {
                return Err(link_error(
                    DiagnosticClass::Corrupt,
                    "artifact_link_reference_owner_kind",
                    "reference-execution closure reached a non-executable canonical owner",
                ));
            }
        }
        let (digest, bytes) = encode_owner(&record)?;
        insert_object(
            objects,
            ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
            bytes,
        )?;
        bindings.insert(
            owner,
            OwnerBinding {
                kind: record.kind(),
                object: digest,
            },
        );
    }

    let mut entries = bindings
        .iter()
        .map(|(owner, binding)| {
            (
                EncodedOwnerKey::new(*owner).bytes().to_vec(),
                encode_owner_binding(binding),
            )
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let mut pages = MemoryPageStore::default();
    let mut map_work = MapWork::default();
    let map =
        PersistentMap::from_sorted(&mut pages, entries, &mut map_work).map_err(map_diagnostic)?;
    for (digest, bytes) in pages.objects() {
        insert_object(
            objects,
            ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes()),
            bytes.to_vec(),
        )?;
    }
    add_map_work(&mut work.reference_map, map_work);
    work.reference_owners = bindings.len() as u64;
    Ok(map.root())
}

fn reference_compilation_payload(payload: &CompilationPayload) -> bool {
    matches!(
        payload,
        CompilationPayload::External { .. }
            | CompilationPayload::Function { .. }
            | CompilationPayload::Constant { .. }
            | CompilationPayload::Test { .. }
    )
}

fn reference_callable_payload(payload: &CompilationPayload) -> bool {
    matches!(
        payload,
        CompilationPayload::External { .. }
            | CompilationPayload::Function { .. }
            | CompilationPayload::Constant { .. }
    )
}

fn reference_payload_matches(
    compiled: &CompilationPayload,
    canonical: &DeclarationPayload,
) -> bool {
    matches!(
        (compiled, canonical),
        (
            CompilationPayload::External { .. },
            DeclarationPayload::External(_)
        ) | (
            CompilationPayload::Function { .. },
            DeclarationPayload::Function(_)
        ) | (
            CompilationPayload::Constant { .. },
            DeclarationPayload::Constant { .. }
        ) | (
            CompilationPayload::Test { .. },
            DeclarationPayload::Test { .. }
        )
    )
}

fn require_local_reference_callable(
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    reference: crate::platform::kernel::DeclarationReference,
) -> Result<(), Diagnostic> {
    let key = (
        reference.package,
        OwnerKey::Declaration(reference.declaration),
    );
    if units
        .get(&key)
        .is_none_or(|unit| !reference_callable_payload(&unit.payload))
    {
        return Err(link_error(
            DiagnosticClass::Corrupt,
            "artifact_link_reference_callable_missing",
            "canonical reference execution names no exact callable local compiler unit",
        ));
    }
    Ok(())
}

fn reference_expression_declarations(
    operation: &ExpressionOperation,
) -> Vec<crate::platform::kernel::DeclarationReference> {
    match operation {
        ExpressionOperation::Constant { declaration } => vec![*declaration],
        ExpressionOperation::Call { function, .. }
        | ExpressionOperation::FunctionValue { function, .. } => vec![*function],
        _ => Vec::new(),
    }
}

fn reference_expression_bindings(
    operation: &ExpressionOperation,
) -> Vec<crate::platform::semantic_id::BindingId> {
    let mut bindings = Vec::new();
    match operation {
        ExpressionOperation::Local {
            value:
                LocalValueReference::LexicalBinding(binding)
                | LocalValueReference::MatchPayload(binding)
                | LocalValueReference::TransactionBinding(binding),
        } => bindings.push(*binding),
        ExpressionOperation::Let {
            bindings: declared, ..
        } => bindings.extend(declared.iter().copied()),
        ExpressionOperation::Match { arms, .. } => {
            bindings.extend(arms.iter().filter_map(|arm| arm.payload_binding));
        }
        ExpressionOperation::Transaction { binding, .. } => bindings.push(*binding),
        _ => {}
    }
    bindings
}

fn import_pack(bytes: &[u8], objects: &mut BTreeMap<ObjectKey, Vec<u8>>) -> Result<(), Diagnostic> {
    let metadata = PackMetadata::decode(bytes, true).map_err(store_diagnostic)?;
    for entry in &metadata.entries {
        let value = metadata
            .read(bytes, entry.key, entry.key.domain.maximum_bytes())
            .map_err(store_diagnostic)?
            .ok_or_else(|| {
                link_error(
                    DiagnosticClass::Corrupt,
                    "artifact_link_pack_entry_missing",
                    "verified package transport lost one indexed object",
                )
            })?;
        insert_object(objects, entry.key, value)?;
    }
    Ok(())
}

fn insert_package(
    packages: &mut BTreeMap<PackageId, ArtifactPackage>,
    package: ArtifactPackage,
) -> Result<(), Diagnostic> {
    match packages.entry(package.package) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(package);
            Ok(())
        }
        std::collections::btree_map::Entry::Occupied(entry) if entry.get() == &package => Ok(()),
        std::collections::btree_map::Entry::Occupied(_) => Err(link_error(
            DiagnosticClass::Corrupt,
            "artifact_link_package_collision",
            "one package identity is bound to different artifact manifests",
        )),
    }
}

fn insert_object(
    objects: &mut BTreeMap<ObjectKey, Vec<u8>>,
    key: ObjectKey,
    bytes: Vec<u8>,
) -> Result<(), Diagnostic> {
    key.verify(&bytes).map_err(store_diagnostic)?;
    match objects.entry(key) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(bytes);
            Ok(())
        }
        std::collections::btree_map::Entry::Occupied(entry) if entry.get() == &bytes => Ok(()),
        std::collections::btree_map::Entry::Occupied(_) => Err(link_error(
            DiagnosticClass::Corrupt,
            "artifact_link_object_collision",
            "one immutable artifact object identity is bound to different bytes",
        )),
    }
}

fn add_repository_work(total: &mut RepositoryReadWork, other: RepositoryReadWork) {
    add_map_work(&mut total.map, other.map);
    total.store.add(other.store);
    total.canonical_records_decoded = total
        .canonical_records_decoded
        .saturating_add(other.canonical_records_decoded);
    total.witness_records_decoded = total
        .witness_records_decoded
        .saturating_add(other.witness_records_decoded);
    total.items_returned = total.items_returned.saturating_add(other.items_returned);
}

fn add_map_work(total: &mut MapWork, other: MapWork) {
    total.pages_read = total.pages_read.saturating_add(other.pages_read);
    total.pages_decoded = total.pages_decoded.saturating_add(other.pages_decoded);
    total.pages_encoded = total.pages_encoded.saturating_add(other.pages_encoded);
    total.pages_written = total.pages_written.saturating_add(other.pages_written);
    total.pages_reused = total.pages_reused.saturating_add(other.pages_reused);
    total.bytes_read = total.bytes_read.saturating_add(other.bytes_read);
    total.bytes_encoded = total.bytes_encoded.saturating_add(other.bytes_encoded);
    total.bytes_written = total.bytes_written.saturating_add(other.bytes_written);
    total.key_comparisons = total.key_comparisons.saturating_add(other.key_comparisons);
    total.entries_visited = total.entries_visited.saturating_add(other.entries_visited);
    total.differences_emitted = total
        .differences_emitted
        .saturating_add(other.differences_emitted);
    total.subtrees_skipped = total
        .subtrees_skipped
        .saturating_add(other.subtrees_skipped);
    total.entries_skipped = total.entries_skipped.saturating_add(other.entries_skipped);
}

fn map_diagnostic(error: MapError) -> Diagnostic {
    link_error(
        match error.class {
            MapErrorClass::Input => DiagnosticClass::Source,
            MapErrorClass::Resource => DiagnosticClass::Resource,
            MapErrorClass::Corrupt => DiagnosticClass::Corrupt,
            MapErrorClass::Store => DiagnosticClass::Infrastructure,
        },
        error.code,
        error.message,
    )
}

fn store_diagnostic(error: StoreError) -> Diagnostic {
    link_error(
        match error.class {
            StoreErrorClass::Input => DiagnosticClass::Source,
            StoreErrorClass::Resource => DiagnosticClass::Resource,
            StoreErrorClass::Corrupt => DiagnosticClass::Corrupt,
            StoreErrorClass::Io => DiagnosticClass::Infrastructure,
        },
        error.code,
        error.message,
    )
}

fn link_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    artifact_error(class, code, message)
}
