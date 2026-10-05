//! Hostile, coherently re-encoded cross-package callbacks are independently rejected by source
//! transport and executable admission. No temporary producer or production-rule bypass is used.
use super::*;
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::kernel::{
    EncodedOwnerKey, KernelSnapshot, OwnerBinding, encode_owner_binding, encode_root,
    semantic_state_digest_from_root,
};
use crate::platform::package_transport::PackageTransportBinding;
use crate::platform::package_transport::source::{AdmittedClosure, PackageContainer};

const SUPPLIER: &str = r#"declarations.begin
(units (module create supplier
  (owned-contract create Bounce (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create Other (constraint owned))
    (method method_aa000000000000000000000000000001 bounce
      (parameters) (returns I64) (effect pure)))
  (function create dispatch (visibility public) (effect pure)
    (type-parameter create S (constraint owned))
    (type-parameter create P (constraint owned))
    (implementation-parameter implparam_aa000000000000000000000000000001
      bounce Bounce S (types P))
    (returns I64)
    (body (method-call parameter@dispatch@implparam_aa000000000000000000000000000001
      Bounce method_aa000000000000000000000000000001)))))
declarations.end"#;

struct Fixture {
    _temporary: tempfile::TempDir,
    source: AdmittedClosure,
    artifact: LoadedArtifact,
}
fn fixture(permuted: bool) -> Fixture {
    let temporary = tempfile::tempdir().unwrap();
    let make = |name: &str| {
        GraphRepository::create(
            &temporary.path().join(name),
            &structurally_empty_snapshot(name.as_bytes()),
            None,
        )
        .unwrap()
        .repository
    };
    let supplier = make("composed-supplier");
    owned_closure_tests::author(&supplier, SUPPLIER);
    let exported = supplier.export_package_transport().unwrap();
    let supplier_artifact = owned_closure_tests::build(&supplier, &[]);
    let consumer = make("composed-consumer");
    consumer
        .stage_package_transport(exported.transport_digest, &exported.container)
        .unwrap();
    let arguments = if permuted { "U T" } else { "T U" };
    owned_closure_tests::author(
        &consumer,
        &format!(
            r#"{}declarations.begin
(units (use upstream {} {}) (module create consumer
  (function create step (visibility private) (effect pure)
    (type-parameter create T (constraint owned))
    (type-parameter create U (constraint owned)) (returns I64)
    (body (if (bool true) (i64 0)
      (implementation-call upstream::dispatch (types {arguments})
        (implementations (implementation Scheme (types {arguments})))))))
  (owned-implementation create Scheme (visibility private)
    (type-parameter create T (constraint owned))
    (type-parameter create U (constraint owned))
    (contract upstream::Bounce) (self T) (types U)
    (method method_aa000000000000000000000000000001 step (types T U)))
  (function create main (visibility public) (effect pure) (returns I64) (body (i64 0)))))
declarations.end"#,
            owned_closure_tests::dependency(&exported),
            exported.revision.package,
            exported.revision_digest
        ),
    );
    let source = consumer.export_package_container().unwrap();
    let artifact = owned_closure_tests::build(&consumer, &[supplier_artifact]);
    Fixture {
        _temporary: temporary,
        source,
        artifact,
    }
}

fn snapshot_with_mapping(
    original: &AdmittedClosure,
    grow: bool,
) -> (KernelSnapshot, OwnerRecord, Vec<(ObjectKey, Vec<u8>)>) {
    let mut source = original.packages[&original.container.root.package_revision]
        .snapshot
        .clone();
    let owner = OwnerKey::Declaration(declaration_named(&source, "Scheme"));
    let mut replacement = source.owners[&owner].clone();
    let OwnerRecord::Declaration(record) = &mut replacement else {
        unreachable!()
    };
    let crate::platform::kernel::DeclarationPayload::OwnedImplementation(i) = &mut record.payload
    else {
        unreachable!()
    };
    let mut extra = Vec::new();
    if grow {
        let object = TypeObject::new(TypeForm::OwnedSequence {
            item: i.methods[0].type_arguments[0],
        })
        .unwrap();
        let (digest, bytes) = encode_type_object(&object).unwrap();
        source.types.insert(digest, object);
        i.methods[0].type_arguments[0] = digest;
        extra.push((
            ObjectKey::from_digest(ObjectDomain::Type, digest.bytes()),
            bytes,
        ));
    }
    source.owners.insert(owner, replacement.clone());
    (source, replacement, extra)
}

// Preserve all supplier objects and exact dependency selections. Only the private downstream
// owner, its map/root/revision/transport, and required new structural type are re-encoded.
fn source_bytes(original: &AdmittedClosure, grow: bool) -> (PackageContainer, KernelSnapshot) {
    let (mut source, replacement, extra) = snapshot_with_mapping(original, grow);
    let package = &original.packages[&original.container.root.package_revision];
    let mut objects = original.container.objects.clone();
    let (old_owner, _) = encode_owner(&package.snapshot.owners[&replacement.owner()]).unwrap();
    objects.remove(&ObjectKey::from_digest(
        ObjectDomain::Owner,
        old_owner.bytes(),
    ));
    let (owner_digest, owner_bytes) = encode_owner(&replacement).unwrap();
    objects.insert(
        ObjectKey::from_digest(ObjectDomain::Owner, owner_digest.bytes()),
        owner_bytes,
    );
    objects.extend(extra);
    let mut owners = crate::platform::package_transport::source::entries(
        &original.container.objects,
        source.root.owners,
    )
    .unwrap();
    let encoded = EncodedOwnerKey::new(replacement.owner()).bytes().to_vec();
    owners
        .iter_mut()
        .find(|(key, _)| *key == encoded)
        .unwrap()
        .1 = encode_owner_binding(&OwnerBinding {
        kind: replacement.kind(),
        object: owner_digest,
    });
    let mut pages = MemoryPageStore::default();
    source.root.owners = PersistentMap::from_sorted(&mut pages, owners, &mut MapWork::default())
        .unwrap()
        .root();
    let reader = ObjectPageReader::new(&original.container.objects);
    for other in original.packages.values() {
        for root in [
            other.snapshot.root.dependencies,
            other.snapshot.root.retirements,
            other.transport.interface_owners,
        ] {
            PersistentMap::from_root(root)
                .copy_reachable(&reader, &mut pages, &mut MapWork::default())
                .unwrap();
        }
        if other.snapshot.root.package_id != source.root.package_id {
            PersistentMap::from_root(other.snapshot.root.owners)
                .copy_reachable(&reader, &mut pages, &mut MapWork::default())
                .unwrap();
        }
    }
    objects.retain(|key, _| key.domain != ObjectDomain::MapPage);
    for (digest, bytes) in pages.objects() {
        objects.insert(
            ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes()),
            bytes.to_vec(),
        );
    }
    objects.remove(&ObjectKey::from_digest(
        ObjectDomain::SemanticRoot,
        package.transport.semantic_root.bytes(),
    ));
    let (root_digest, root_bytes) = encode_root(&source.root).unwrap();
    objects.insert(
        ObjectKey::from_digest(ObjectDomain::SemanticRoot, root_digest.bytes()),
        root_bytes,
    );
    objects.remove(&ObjectKey::from_digest(
        ObjectDomain::PackageRevision,
        package.binding.package_revision.bytes(),
    ));
    let mut revision = package.revision.clone();
    revision.revision.semantic_state = semantic_state_digest_from_root(&source.root).unwrap();
    let (revision_digest, revision_bytes) = revision.encode().unwrap();
    objects.insert(
        ObjectKey::from_digest(ObjectDomain::PackageRevision, revision_digest.bytes()),
        revision_bytes,
    );
    objects.remove(&ObjectKey::from_digest(
        ObjectDomain::PackageTransport,
        package.binding.transport.bytes(),
    ));
    let mut transport = package.transport.clone();
    transport.semantic_root = root_digest;
    transport.package_revision = revision_digest;
    let (witness, digest, _) = bind_witness_manifest(
        source.root.repository_id,
        source.root.package_id,
        root_digest,
        transport.witness.roots,
    )
    .unwrap();
    transport.witness = witness;
    transport.validation_witness = digest;
    let (transport_digest, transport_bytes) = transport.encode().unwrap();
    objects.insert(
        ObjectKey::from_digest(ObjectDomain::PackageTransport, transport_digest.bytes()),
        transport_bytes,
    );
    let root = PackageTransportBinding {
        package_revision: revision_digest,
        transport: transport_digest,
    };
    let mut selections = original.container.selections.clone();
    *selections
        .iter_mut()
        .find(|binding| **binding == original.container.root)
        .unwrap() = root;
    selections.sort_by_key(|binding| binding.package_revision);
    (
        PackageContainer {
            root,
            selections,
            objects,
        },
        source,
    )
}

fn replace_map(
    loaded: &mut LoadedArtifact,
    root: MapRoot,
    entries: Vec<(Vec<u8>, Vec<u8>)>,
) -> MapRoot {
    let mut old_pages = MemoryPageStore::default();
    PersistentMap::from_root(root)
        .copy_reachable(
            &ObjectPageReader::new(loaded),
            &mut old_pages,
            &mut MapWork::default(),
        )
        .unwrap();
    for (digest, _) in old_pages.objects() {
        loaded.objects.remove(&ObjectKey::from_digest(
            ObjectDomain::MapPage,
            digest.bytes(),
        ));
    }
    replace_artifact_map(&mut loaded.objects, entries)
}

fn artifact_bytes(original: &LoadedArtifact, source: &KernelSnapshot) -> Vec<u8> {
    let mut loaded = original.clone();
    let package_index = loaded
        .manifest
        .packages
        .iter()
        .position(|p| p.package == source.root.package_id)
        .unwrap();
    let scheme = OwnerKey::Declaration(declaration_named(source, "Scheme"));
    let replacement = &source.owners[&scheme];
    let (digest, bytes) = encode_owner(replacement).unwrap();
    let binding = loaded.manifest.packages[package_index]
        .runtime_owners
        .iter_mut()
        .find(|b| b.owner == scheme)
        .unwrap();
    let old_owner = ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes());
    binding.object = digest;
    loaded.objects.remove(&old_owner).unwrap();
    loaded.objects.insert(
        ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
        bytes,
    );
    let reference_root = loaded.manifest.packages[package_index].reference_owners;
    let mut references = artifact_map_entries(&loaded, reference_root);
    let encoded = EncodedOwnerKey::new(scheme).bytes().to_vec();
    if let Some((_, value)) = references.iter_mut().find(|(key, _)| *key == encoded) {
        *value = encode_owner_binding(&OwnerBinding {
            kind: replacement.kind(),
            object: digest,
        });
        loaded.manifest.packages[package_index].reference_owners =
            replace_map(&mut loaded, reference_root, references);
    }
    for (digest, object) in &source.types {
        let (_, bytes) = encode_type_object(object).unwrap();
        // Only actual roots retained by the units are needed; the fixture adds just one type.
        if matches!(object.form, TypeForm::OwnedSequence { .. }) {
            loaded.objects.insert(
                ObjectKey::from_digest(ObjectDomain::Type, digest.bytes()),
                bytes,
            );
        }
    }
    owned_closure_tests::rebind(&mut loaded, source);
    let old_compilation = loaded.manifest.packages[package_index].compilation;
    let mut compilation = CompilationManifest::decode(
        &loaded.objects[&old_compilation.object_key()],
        old_compilation,
    )
    .unwrap();
    let mut units = artifact_map_entries(&loaded, compilation.units);
    let facts = crate::platform::witness::rebuild_canonical_facts(source).unwrap();
    for (owner, value) in &mut units {
        let owner = EncodedOwnerKey::decode(owner).unwrap();
        let mut binding = CompilationBinding::decode(value, owner).unwrap();
        let old_unit = binding.object.object_key();
        let mut unit = CompilationUnit::decode(&loaded.objects[&old_unit], old_unit).unwrap();
        if owner == scheme {
            let OwnerRecord::Declaration(record) = replacement else {
                unreachable!()
            };
            let crate::platform::kernel::DeclarationPayload::OwnedImplementation(i) =
                &record.payload
            else {
                unreachable!()
            };
            unit.payload = CompilationPayload::OwnedImplementation(i.clone());
            for ty in &i.methods[0].type_arguments {
                if !unit.tables.types.contains(ty) {
                    unit.tables.types.push(*ty);
                }
            }
        }
        let summary = &facts.summaries[&owner];
        unit.source = CompilationSource {
            package: source.root.package_id,
            owner,
            kind: summary.kind,
            semantic_interface: summary.semantic_interface,
            implementation: summary.implementation,
            type_digest: summary.type_digest,
            effect: summary.effect,
            capability: summary.capability,
            test: summary.test,
            validation_dependencies: summary.validation_dependencies,
        };
        unit.key = CompilationUnitKey::derive(&unit.source, unit.optimization).unwrap();
        let (key, bytes) = unit.encode().unwrap();
        CompilationUnit::decode(&bytes, key).unwrap();
        loaded.objects.remove(&old_unit).unwrap();
        loaded.objects.insert(key, bytes);
        binding.object = CompilerUnitObjectDigest::from_bytes(key.digest.bytes());
        binding.key = unit.key;
        binding.kind = unit.source.kind;
        *value = binding.encode(owner).unwrap();
    }
    compilation.units = replace_map(&mut loaded, compilation.units, units);
    let (digest, bytes) = compilation.encode().unwrap();
    loaded
        .objects
        .remove(&old_compilation.object_key())
        .unwrap();
    loaded.objects.insert(digest.object_key(), bytes);
    loaded.manifest.packages[package_index].compilation = digest;
    let (closure, count, bytes) = super::super::artifact::closure_facts(&loaded.objects).unwrap();
    loaded.manifest.closure = closure;
    loaded.manifest.object_count = count;
    loaded.manifest.object_bytes = bytes;
    nominal_session_tests::hostile_bundle(&loaded.manifest, &loaded.objects)
}

#[test]
fn neutral_cross_package_plain_and_permuted_controls_admit_at_both_boundaries() {
    for permuted in [false, true] {
        let fixture = fixture(permuted);
        let (container, source) = source_bytes(&fixture.source, false);
        let decoded =
            PackageContainer::decode(&container.encode().unwrap(), container.root.transport)
                .unwrap();
        decoded.admit().unwrap();
        crate::platform::package_transport::oracle::reconstruct(&decoded).unwrap();
        load_artifact(&artifact_bytes(&fixture.artifact, &source)).unwrap();
    }
}

#[test]
fn forged_cross_package_callback_growth_rejects_transport_oracle_and_artifact_before_preparation() {
    let fixture = fixture(false);
    let (container, source) = source_bytes(&fixture.source, true);
    // The changed mapping is independently valid in its own package; only supplier callbacks
    // compose it into recursive growth. A private mapping leaves the public interface unchanged.
    crate::platform::kernel::validate_full(&source).unwrap();
    let decoded =
        PackageContainer::decode(&container.encode().unwrap(), container.root.transport).unwrap();
    let error = decoded.admit().unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic, "{error:?}");
    assert_eq!(error.code, "kernel_callable_expansion", "{error:?}");
    let error = crate::platform::package_transport::oracle::reconstruct(&decoded).unwrap_err();
    assert_eq!(error.code, "kernel_callable_expansion", "{error:?}");
    let target = GraphRepository::create(
        &fixture._temporary.path().join("rejected-stage"),
        &structurally_empty_snapshot(b"composed-rejected-stage"),
        None,
    )
    .unwrap()
    .repository;
    target
        .stage_package_transport(
            fixture.source.container.root.transport,
            &fixture.source.container.encode().unwrap(),
        )
        .unwrap();
    let ready = target.root().join("PACKAGE-TRANSPORTS/CURRENT");
    let before_ready = std::fs::read(&ready).unwrap();
    let before_head = target.current().unwrap().head;
    let pack_inventory = || {
        std::fs::read_dir(target.root().join("packs"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<BTreeSet<_>>()
    };
    let before_packs = pack_inventory();
    let error = target
        .stage_package_transport(decoded.root.transport, &decoded.encode().unwrap())
        .unwrap_err();
    assert_eq!(error.code, "kernel_callable_expansion", "{error:?}");
    assert_eq!(target.current().unwrap().head, before_head);
    assert_eq!(std::fs::read(ready).unwrap(), before_ready);
    assert_eq!(pack_inventory(), before_packs);
    let error = load_artifact(&artifact_bytes(&fixture.artifact, &source)).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic, "{error:?}");
    assert_eq!(error.code, "kernel_callable_expansion", "{error:?}");
}
