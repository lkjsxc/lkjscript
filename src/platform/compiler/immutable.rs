//! Ordinary compilation and linking over admitted immutable canonical package snapshots.

use super::cache::CachedCompilation;
use super::manifest::{
    COMPILATION_MANIFEST_CONTRACT_VERSION, CompilationBinding, CompilationManifest,
    CompilerUnitObjectDigest, MAXIMUM_COMPILATION_UNITS, compilation_map_key,
};
use super::unit::{BYTECODE_CONTRACT_VERSION, COMPILER_UNIT_CONTRACT_VERSION, OptimizationPolicy};
use super::{ArtifactLinkReceipt, LoadedArtifact, compile_unit};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::package_interface::build_package_interface;
use crate::platform::package_transport::source::ImmutablePackage;
use crate::platform::persistent_map::{MapWork, MemoryPageStore, PersistentMap};
use crate::platform::publication::{BuiltPackageRevision, RepositoryReadWork};
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, ObjectStage, StoreWork,
};
use std::collections::BTreeMap;

pub(crate) fn compile_immutable(
    package: &ImmutablePackage,
    source: &BTreeMap<ObjectKey, Vec<u8>>,
    dependencies: &[LoadedArtifact],
) -> Result<ArtifactLinkReceipt, Diagnostic> {
    let optimization = OptimizationPolicy::DeterministicBaseline;
    let mut store = ObjectStage::new(source);
    let mut work = StoreWork::default();
    let mut entries = Vec::new();
    for (owner, record) in &package.snapshot.owners {
        if !record.kind().has_compilation_unit() {
            continue;
        }
        if entries.len() as u64 >= MAXIMUM_COMPILATION_UNITS {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "compilation_immutable_units",
                "immutable package exceeds compiler unit admission",
            ));
        }
        let unit = compile_unit(&package.snapshot, &package.witness, *owner, optimization)?;
        let binding = CompilationBinding {
            kind: unit.unit.source.kind,
            key: unit.key,
            object: CompilerUnitObjectDigest::from_bytes(unit.object.digest.bytes()),
        };
        store
            .stage(unit.object, &unit.bytes, &mut work)
            .map_err(store_error)?;
        entries.push((
            compilation_map_key(*owner).to_vec(),
            binding.encode(*owner)?,
        ));
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let mut pages = MemoryPageStore::default();
    let units = PersistentMap::from_sorted(&mut pages, entries, &mut MapWork::default())
        .map_err(|error| Diagnostic::new(DiagnosticClass::Corrupt, error.code, error.message))?
        .root();
    for (digest, bytes) in pages.objects() {
        store
            .stage(
                ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes()),
                bytes,
                &mut work,
            )
            .map_err(store_error)?;
    }
    let manifest = CompilationManifest {
        contract_version: COMPILATION_MANIFEST_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        compiler_contract_version: COMPILER_UNIT_CONTRACT_VERSION,
        bytecode_contract_version: BYTECODE_CONTRACT_VERSION,
        repository_id: package.snapshot.root.repository_id,
        package_id: package.revision.package,
        revision: package.revision.revision.revision_id()?,
        package_revision: package.binding.package_revision,
        semantic_state: package.revision.revision.semantic_state,
        package_interface: package.revision.interface,
        optimization,
        units,
    };
    let (digest, bytes) = manifest.encode()?;
    store
        .stage(digest.object_key(), &bytes, &mut work)
        .map_err(store_error)?;
    let (revision_digest, revision_bytes) = package.revision.encode()?;
    let exported = BuiltPackageRevision {
        revision: package.revision.clone(),
        revision_digest,
        revision_bytes,
        interface: build_package_interface(&package.interface_owners, &package.interface_types)?,
        interface_owners: package.interface_owners.clone(),
        interface_types: package.interface_types.clone(),
        read_work: RepositoryReadWork::default(),
    };
    super::link::link_prepared(
        &package.snapshot,
        &store,
        &CachedCompilation { manifest, digest },
        &exported,
        dependencies,
    )
}

fn store_error(error: crate::platform::storage::object::StoreError) -> Diagnostic {
    Diagnostic::new(
        match error.class {
            crate::platform::storage::object::StoreErrorClass::Resource => {
                DiagnosticClass::Resource
            }
            crate::platform::storage::object::StoreErrorClass::Io => {
                DiagnosticClass::Infrastructure
            }
            _ => DiagnosticClass::Corrupt,
        },
        error.code,
        error.message,
    )
}
