//! Hostile package authority is independent of the bundle's global symbol inventory.
use super::*;
use crate::platform::kernel::{KernelSnapshot, semantic_state_digest};
use crate::platform::package_transport::PackageRevision;

fn author(repository: &GraphRepository, text: &str) {
    let input = format!(
        "request base={}\n{text}",
        repository.view_current().unwrap().revision()
    );
    let decoded =
        crate::platform::control::decode_compact_change("owned-closure", input.as_bytes()).unwrap();
    let prepared = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    repository.publish(&prepared.publication).unwrap();
}
fn dependency(exported: &ExportedPackageTransport) -> String {
    format!(
        "add.dependency package={} semantic-revision={} package-revision={}\n",
        exported.revision.package,
        exported.revision.revision.revision_id().unwrap(),
        exported.revision_digest
    )
}
fn build(repository: &GraphRepository, dependencies: &[LoadedArtifact]) -> LoadedArtifact {
    let compiled = build_clean(repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let linked = link_artifact(repository, compiled.manifest_digest, dependencies).unwrap();
    load_artifact(&linked.artifact.bytes).unwrap()
}
fn rebind(
    loaded: &mut LoadedArtifact,
    snapshot: &KernelSnapshot,
) -> crate::platform::kernel::DependencyRecord {
    let package = loaded
        .manifest
        .packages
        .iter_mut()
        .find(|p| p.package == snapshot.root.package_id)
        .unwrap();
    let old = ObjectKey::from_digest(
        ObjectDomain::PackageRevision,
        package.package_revision.bytes(),
    );
    let mut revision = PackageRevision::decode(
        &loaded.objects.remove(&old).unwrap(),
        package.package_revision,
    )
    .unwrap();
    revision.dependencies = snapshot.dependencies.values().cloned().collect();
    revision.revision.semantic_state = semantic_state_digest(snapshot).unwrap();
    let (digest, bytes) = revision.encode().unwrap();
    loaded.objects.insert(
        ObjectKey::from_digest(ObjectDomain::PackageRevision, digest.bytes()),
        bytes,
    );
    package.package_revision = digest;
    package.semantic_state = revision.revision.semantic_state;
    package.semantic_revision = revision.revision.revision_id().unwrap();
    let mut compilation = CompilationManifest::decode(
        &loaded
            .objects
            .remove(&package.compilation.object_key())
            .unwrap(),
        package.compilation,
    )
    .unwrap();
    compilation.package_revision = digest;
    compilation.revision = package.semantic_revision;
    compilation.semantic_state = package.semantic_state;
    let (digest, bytes) = compilation.encode().unwrap();
    package.compilation = digest;
    loaded.objects.insert(digest.object_key(), bytes);
    crate::platform::kernel::DependencyRecord {
        graph_contract_version: revision.graph_contract_version,
        package: package.package,
        semantic_revision: package.semantic_revision,
        package_revision: package.package_revision,
    }
}

#[test]
fn owned_closure_artifact_rejects_rehashed_missing_source_dependency() {
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
    let b = make("owned-closure-b");
    author(
        &b,
        r#"declarations.begin
(units (module create b
  (owned-contract create Marker (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_32000000000000000000000000000001 inspect (parameters) (returns I64)))
  (function create seven (visibility public) (returns I64) (effect pure) (body (i64 7)))
  (owned-implementation create CellMarker (visibility public) (contract Marker) (self OwnedI64Cell)
    (method method_32000000000000000000000000000001 seven))))
declarations.end"#,
    );
    let b_export = b.export_package_transport().unwrap();
    let b_loaded = build(&b, &[]);
    let a = make("owned-closure-a");
    a.stage_package_transport(b_export.transport_digest, &b_export.container)
        .unwrap();
    author(
        &a,
        &format!(
            "{}declarations.begin\n(units (use b {} {}) (module create a (function create probe (visibility public) (returns I64) (effect pure) (body (method-call concrete@b::CellMarker b::Marker method_32000000000000000000000000000001)))))\ndeclarations.end",
            dependency(&b_export),
            b_export.revision.package,
            b_export.revision_digest
        ),
    );
    let a_export = a.export_package_transport().unwrap();
    let a_loaded = build(&a, &[b_loaded.clone()]);
    let r = make("owned-closure-root");
    r.stage_package_transport(a_export.transport_digest, &a_export.container)
        .unwrap();
    r.stage_package_transport(b_export.transport_digest, &b_export.container)
        .unwrap();
    author(
        &r,
        &format!(
            "{}{}declarations.begin\n(units (use a {} {}) (module create root (function create entry (visibility public) (returns I64) (effect pure) (body (call a::probe)))))\ndeclarations.end",
            dependency(&a_export),
            dependency(&b_export),
            a_export.revision.package,
            a_export.revision_digest
        ),
    );
    let valid = build(&r, &[a_loaded, b_loaded]);
    assert_eq!(valid.manifest.packages.len(), 3);
    let mut forged = valid.clone();
    let mut a_source = a
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    assert_eq!(a_source.dependencies.len(), 1);
    a_source.dependencies.clear();
    let replacement = rebind(&mut forged, &a_source);
    let mut root_source = r
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    root_source
        .dependencies
        .insert(replacement.package, replacement);
    rebind(&mut forged, &root_source);
    let (closure, count, bytes) = super::super::artifact::closure_facts(&forged.objects).unwrap();
    forged.manifest.closure = closure;
    forged.manifest.object_count = count;
    forged.manifest.object_bytes = bytes;
    let bytes = nominal_session_tests::hostile_bundle(&forged.manifest, &forged.objects);
    let failure = load_artifact(&bytes).expect_err("bundled B does not authorize A to reference B");
    assert_eq!(failure.code, "artifact_nominal_meaning", "{failure:?}");
    assert!(
        failure.message.contains("kernel_owned_contract"),
        "{failure:?}"
    );
}
