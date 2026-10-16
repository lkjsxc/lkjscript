//! Structural reachability preserves accepted history without granting current execution.

use super::*;
use crate::platform::compiler::{
    CompilationBinding, CompilationManifest, CompilationManifestDigest, CompilationUnit,
};
use crate::platform::kernel::*;
use crate::platform::package_interface::{PackageInterfaceOwner, decode_package_interface_binding};
use crate::platform::package_transport::source::PackageReadiness;
use crate::platform::publication::{IdempotencyBinding, SemanticDiffBody};
use crate::platform::storage::directory::PackCompactionReceipt;
use crate::platform::witness::{SummaryBinding, decode_owner_summary, decode_witness_manifest};
use std::collections::VecDeque;

#[path = "gc/retain.rs"]
mod retain;
use retain::{MAXIMUM_OBJECTS, Retained};

#[derive(Debug, serde::Serialize)]
pub(crate) struct GarbageCollectionReceipt {
    pub operation: &'static str,
    pub status: &'static str,
    pub revision: String,
    pub objects_before: usize,
    pub objects_retained: usize,
    pub objects_reclaimed: usize,
    pub canonical_bytes_retained: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disk: Option<PackCompactionReceipt>,
}

impl GraphRepository {
    pub(crate) fn collect_garbage(
        &self,
        control: &ExecutionControl,
    ) -> Result<GarbageCollectionReceipt, Diagnostic> {
        checkpoint(control)?;
        if let Some(memory) = &self.memory {
            let mut graph = memory_lock(&memory.graph)?;
            let head = HeadRecord::decode(
                graph
                    .head
                    .as_ref()
                    .ok_or_else(memory_repository::unpublished)?
                    .as_slice(),
            )?;
            let store = StorageBackend::Memory(graph.store.clone());
            let readiness = read_store_readiness(&store)?;
            let compiled = graph
                .compilation_head
                .as_ref()
                .map(|bytes| crate::platform::compiler::gc_manifest_from_bytes(bytes.as_slice()))
                .transpose()?;
            let objects = collect(&store, head, &readiness, compiled, control)?;
            let result = receipt(head, graph.store.objects.len(), &objects, None);
            checkpoint(control)?;
            graph
                .store
                .objects
                .retain(|key| objects.contains_key(key))
                .map_err(store_diagnostic)?;
            return Ok(result);
        }
        let root = open_directory(&self.root)?;
        let lock = open_lock(&root)?;
        FileExt::lock_exclusive(&lock)
            .map_err(|error| io_diagnostic("gc_publication_lock", &self.root, error))?;
        let mut store = PackDirectoryStore::open_for_gc(&self.root).map_err(store_diagnostic)?;
        let current =
            read_current_optional(&root, &store)?.ok_or_else(memory_repository::unpublished)?;
        let head = current.head;
        let readiness = read_package_readiness(&self.root)?;
        let compiled = crate::platform::compiler::gc_manifest(self)?;
        let objects = collect(&store, head, &readiness, compiled, control)?;
        let mut result = receipt(
            head,
            store.catalog_observation().entries as usize,
            &objects,
            None,
        );
        checkpoint(control)?;
        result.disk = Some(store.compact_reachable(objects).map_err(store_diagnostic)?);
        Ok(result)
    }
}

fn receipt(
    head: HeadRecord,
    before: usize,
    objects: &BTreeMap<ObjectKey, Vec<u8>>,
    disk: Option<PackCompactionReceipt>,
) -> GarbageCollectionReceipt {
    GarbageCollectionReceipt {
        operation: "gc",
        status: "compacted",
        revision: head.revision.to_string(),
        objects_before: before,
        objects_retained: objects.len(),
        objects_reclaimed: before.saturating_sub(objects.len()),
        canonical_bytes_retained: objects.values().map(|bytes| bytes.len() as u64).sum(),
        disk,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Pending {
    Revision(HeadRecord),
    Root(SemanticRootDigest),
    Witness(crate::platform::witness::ValidationWitnessDigest),
    Type(TypeObjectDigest),
    Package(PackageTransportBinding),
    PackageRevision(PackageRevisionDigest),
    Compilation(CompilationManifestDigest),
}

impl Pending {
    fn key(self) -> ObjectKey {
        let (domain, bytes) = match self {
            Self::Revision(head) => (ObjectDomain::Revision, head.record.bytes()),
            Self::Root(digest) => (ObjectDomain::SemanticRoot, digest.bytes()),
            Self::Witness(digest) => (ObjectDomain::ValidationWitness, digest.bytes()),
            Self::Type(digest) => (ObjectDomain::Type, digest.bytes()),
            Self::Package(binding) => (ObjectDomain::PackageTransport, binding.transport.bytes()),
            Self::PackageRevision(digest) => (ObjectDomain::PackageRevision, digest.bytes()),
            Self::Compilation(digest) => (ObjectDomain::CompilationManifest, digest.bytes()),
        };
        ObjectKey::from_digest(domain, bytes)
    }
}

#[derive(Default)]
struct Queue {
    seen: BTreeMap<ObjectKey, Pending>,
    pending: VecDeque<Pending>,
}

impl Queue {
    fn push(&mut self, item: Pending) -> Result<(), Diagnostic> {
        let key = item.key();
        if let Some(previous) = self.seen.get(&key) {
            if *previous != item {
                return Err(gc_error(
                    DiagnosticClass::Corrupt,
                    "gc_reference_binding",
                    "one reachable object has conflicting reference bindings",
                ));
            }
            return Ok(());
        }
        if self.seen.len() == MAXIMUM_OBJECTS {
            return Err(gc_error(
                DiagnosticClass::Resource,
                "gc_reference_capacity",
                "reachable reference inventory exceeds compaction capacity",
            ));
        }
        self.seen.insert(key, item);
        self.pending.push_back(item);
        Ok(())
    }

    fn types(
        &mut self,
        types: impl IntoIterator<Item = TypeObjectDigest>,
    ) -> Result<(), Diagnostic> {
        for digest in types {
            self.push(Pending::Type(digest))?;
        }
        Ok(())
    }
}

fn collect<S: ImmutableObjectStore + ?Sized>(
    base: &S,
    head: HeadRecord,
    readiness: &PackageReadiness,
    compiled: Option<CompilationManifestDigest>,
    control: &ExecutionControl,
) -> Result<BTreeMap<ObjectKey, Vec<u8>>, Diagnostic> {
    let store = Retained::new(base, control);
    let mut queue = Queue::default();
    queue.push(Pending::Revision(head))?;
    for (revision, transport) in &readiness.bindings {
        queue.push(Pending::Package(PackageTransportBinding {
            package_revision: *revision,
            transport: *transport,
        }))?;
    }
    if let Some(digest) = compiled {
        queue.push(Pending::Compilation(digest))?;
    }
    while let Some(item) = queue.pending.pop_front() {
        checkpoint(control)?;
        visit(&store, &mut queue, readiness, item)?;
    }
    checkpoint(control)?;
    Ok(store.finish())
}

fn visit<S: ImmutableObjectStore + ?Sized>(
    store: &Retained<'_, S>,
    queue: &mut Queue,
    readiness: &PackageReadiness,
    item: Pending,
) -> Result<(), Diagnostic> {
    match item {
        Pending::Revision(head) => {
            let current = read_publication(store, head)?;
            let binding = &current.revision.publication;
            queue.push(Pending::Root(binding.semantic_root))?;
            queue.push(Pending::Witness(binding.validation.witness))?;
            for parent in &binding.parents {
                queue.push(Pending::Revision(HeadRecord {
                    revision: parent.revision,
                    record: parent.record,
                    ..head
                }))?;
            }
            store.map(binding.idempotency_receipts, |_, value| {
                let previous = IdempotencyBinding::decode(value)?;
                queue.push(Pending::Revision(previous.result))
            })?;
            if let TransactionBody::Change { supplied_types, .. } = current.transaction.body {
                queue.types(supplied_types)?;
            }
            if let SemanticDiffBody::Change { supplied_types, .. } = current.semantic_diff.body {
                queue.types(supplied_types)?;
            }
        }
        Pending::Root(digest) => {
            let root = decode_root(
                &store.required(ObjectDomain::SemanticRoot, digest.bytes())?,
                digest,
            )?;
            store.map(root.owners, |key, value| {
                let owner = EncodedOwnerKey::decode(key)?;
                let binding = decode_owner_binding(value, owner)?;
                let record = decode_owner(
                    &store.required(ObjectDomain::Owner, binding.object.bytes())?,
                    owner,
                    binding.kind,
                    binding.object,
                )?;
                retain_owner_values(store, queue, &record)
            })?;
            store.map(root.dependencies, |key, value| {
                let package = key
                    .try_into()
                    .ok()
                    .and_then(PackageId::from_bytes)
                    .ok_or_else(|| {
                        gc_error(
                            DiagnosticClass::Corrupt,
                            "gc_dependency_key",
                            "invalid reachable package identity",
                        )
                    })?;
                let binding = decode_dependency_binding(value)?;
                let record = decode_dependency(
                    &store.required(ObjectDomain::Dependency, binding.object.bytes())?,
                    &package,
                    binding.object,
                )?;
                queue.push(Pending::PackageRevision(record.package_revision))?;
                select_package(queue, readiness, record.package_revision)
            })?;
            store.map(root.retirements, |key, value| {
                let owner = EncodedOwnerKey::decode(key)?;
                let binding = decode_retirement_binding(value)?;
                decode_retirement(
                    &store.required(ObjectDomain::Retirement, binding.object.bytes())?,
                    owner,
                    binding.object,
                )?;
                Ok(())
            })?;
        }
        Pending::Witness(digest) => {
            let witness = decode_witness_manifest(
                &store.required(ObjectDomain::ValidationWitness, digest.bytes())?,
                digest,
            )?;
            queue.push(Pending::Root(witness.semantic_root))?;
            store.map(witness.roots.owner_summaries, |key, value| {
                let owner = EncodedOwnerKey::decode(key)?;
                let binding = SummaryBinding::decode(value, owner)?;
                let summary = decode_owner_summary(
                    &store.required(ObjectDomain::OwnerSummary, binding.summary.bytes())?,
                    binding.summary,
                )?;
                if summary.owner != owner || summary.kind != binding.kind {
                    return Err(gc_error(
                        DiagnosticClass::Corrupt,
                        "gc_summary_binding",
                        "reachable owner summary has inconsistent identity",
                    ));
                }
                let record = decode_owner(
                    &store.required(ObjectDomain::Owner, summary.record.bytes())?,
                    owner,
                    summary.kind,
                    summary.record,
                )?;
                retain_owner_values(store, queue, &record)
            })?;
            for root in [
                witness.roots.namespaces,
                witness.roots.ownership,
                witness.roots.forward_relations,
                witness.roots.reverse_relations,
                witness.roots.test_dependencies,
            ] {
                store.map(root, |_, _| Ok(()))?;
            }
        }
        Pending::Type(digest) => {
            let value =
                decode_type_object(&store.required(ObjectDomain::Type, digest.bytes())?, digest)?;
            queue.types(value.child_types())?;
        }
        Pending::Package(binding) => {
            let transport = PackageTransport::decode(
                &store.required(ObjectDomain::PackageTransport, binding.transport.bytes())?,
                binding.transport,
            )?;
            if transport.package_revision != binding.package_revision {
                return Err(gc_error(
                    DiagnosticClass::Corrupt,
                    "gc_package_binding",
                    "selected package transport binds a different revision",
                ));
            }
            queue.push(Pending::PackageRevision(binding.package_revision))?;
            queue.push(Pending::Root(transport.semantic_root))?;
            // Imported transports embed witness commitments; their independent admission
            // rebuilds witness maps. Those derived map objects are not container references.
            store.map(transport.interface_owners, |key, value| {
                let owner = EncodedOwnerKey::decode(key)?;
                let digest = decode_package_interface_binding(value)?;
                let projected = PackageInterfaceOwner::decode(
                    &store.required(ObjectDomain::PackageInterface, digest.bytes())?,
                    owner,
                    digest,
                )?;
                queue.types(projected.type_roots())
            })?;
        }
        Pending::PackageRevision(digest) => {
            let revision = PackageRevision::decode(
                &store.required(ObjectDomain::PackageRevision, digest.bytes())?,
                digest,
            )?;
            for dependency in revision.dependencies {
                queue.push(Pending::PackageRevision(dependency.package_revision))?;
                select_package(queue, readiness, dependency.package_revision)?;
            }
        }
        Pending::Compilation(digest) => {
            let manifest = CompilationManifest::decode(
                &store.required(ObjectDomain::CompilationManifest, digest.bytes())?,
                digest,
            )?;
            queue.push(Pending::PackageRevision(manifest.package_revision))?;
            store.map(manifest.units, |key, value| {
                let owner = EncodedOwnerKey::decode(key)?;
                let binding = CompilationBinding::decode(value, owner)?;
                let unit = CompilationUnit::decode(
                    &store.required(ObjectDomain::CompilerUnit, binding.object.bytes())?,
                    binding.object.object_key(),
                )?;
                if unit.source.package != manifest.package_id
                    || unit.source.owner != owner
                    || unit.source.kind != binding.kind
                    || unit.key != binding.key
                {
                    return Err(gc_error(
                        DiagnosticClass::Corrupt,
                        "gc_compilation_binding",
                        "selected compiler unit has inconsistent identity",
                    ));
                }
                // Unit type tables also contain inferred parallel/substitution identities
                // reconstructed by the linker, with no stored object. Canonical type/blob
                // roots come from the complete retained source, not these derived tables.
                Ok(())
            })?;
        }
    }
    Ok(())
}

fn retain_owner_values<S: ImmutableObjectStore + ?Sized>(
    store: &Retained<'_, S>,
    queue: &mut Queue,
    record: &OwnerRecord,
) -> Result<(), Diagnostic> {
    queue.types(record.type_roots())?;
    for (blob, length) in record.blob_roots() {
        if store.required(ObjectDomain::Blob, blob.bytes())?.len() as u64 != length {
            return Err(gc_error(
                DiagnosticClass::Corrupt,
                "gc_blob_length",
                "reachable blob disagrees with its canonical length",
            ));
        }
    }
    Ok(())
}

fn select_package(
    queue: &mut Queue,
    readiness: &PackageReadiness,
    revision: PackageRevisionDigest,
) -> Result<(), Diagnostic> {
    let transport = readiness.bindings.get(&revision).ok_or_else(|| {
        gc_error(
            DiagnosticClass::Corrupt,
            "gc_dependency_selection",
            "reachable dependency has no exact selected source transport",
        )
    })?;
    queue.push(Pending::Package(PackageTransportBinding {
        package_revision: revision,
        transport: *transport,
    }))
}

fn checkpoint(control: &ExecutionControl) -> Result<(), Diagnostic> {
    control.check().map_err(execution_diagnostic)
}

fn gc_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
#[path = "gc/tests.rs"]
mod tests;
