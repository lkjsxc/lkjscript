use super::*;
use crate::platform::persistent_map::MemoryPageStore;

fn manifest() -> ArtifactManifest {
    let package = PackageId::migrate(b"artifact-contract", 0);
    let empty = PersistentMap::empty(&mut MemoryPageStore::default(), &mut MapWork::default())
        .unwrap()
        .root();
    ArtifactManifest {
        contract_version: ARTIFACT_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        compiler_contract_version: COMPILER_UNIT_CONTRACT_VERSION,
        bytecode_contract_version: BYTECODE_CONTRACT_VERSION,
        compilation_manifest_contract_version: COMPILATION_MANIFEST_CONTRACT_VERSION,
        root_package: package,
        packages: vec![ArtifactPackage {
            repository_id: RepositoryId::migrate(b"artifact-contract", 0),
            package,
            package_revision: PackageRevisionDigest::from_bytes([1; 32]),
            semantic_revision: RevisionId::from_digest([2; 32]),
            semantic_state: SemanticStateDigest::from_bytes([3; 32]),
            interface: PackageInterfaceDigest::from_bytes([4; 32]),
            interface_owners: empty,
            compilation: CompilationManifestDigest::from_bytes([5; 32]),
            runtime_owners: vec![],
            reference_owners: empty,
        }],
        closure: ArtifactClosureDigest::from_bytes([6; 32]),
        object_count: 1,
        object_bytes: 1,
    }
}

#[test]
fn artifact_manifest_generations_admit_current_and_preserve_historical_boundaries() {
    let current = manifest();
    assert_eq!(
        (
            current.contract_version,
            current.graph_contract_version,
            current.compiler_contract_version,
            current.bytecode_contract_version
        ),
        (40, 33, 33, 27)
    );
    let (digest, bytes) = current.encode().unwrap();
    assert_eq!(ArtifactManifest::decode(&bytes, digest).unwrap(), current);
    let mut owned = current.clone();
    owned.packages[0].runtime_owners.push(ArtifactRuntimeOwner {
        owner: OwnerKey::Declaration(DeclarationId::migrate(b"artifact-owned-contract", 0)),
        kind: OwnerKind::OwnedRecord,
        object: OwnerObjectDigest::from_bytes([7; 32]),
    });
    let (digest, bytes) = owned.encode().unwrap();
    assert_eq!(ArtifactManifest::decode(&bytes, digest).unwrap(), owned);
    for (artifact, graph, compiler, bytecode) in [
        (18, 14, 10, 6),
        (19, 15, 11, 7),
        (20, 16, 12, 8),
        (25, 20, 18, 14),
        (26, 21, 19, 15),
        (27, 21, 20, 16),
        (28, 21, 21, 17),
        (29, 22, 22, 17),
    ] {
        let mut supported = current.clone();
        supported.contract_version = artifact;
        supported.graph_contract_version = graph;
        supported.compiler_contract_version = compiler;
        supported.bytecode_contract_version = bytecode;
        let (digest, bytes) = supported.encode().unwrap();
        assert_eq!(ArtifactManifest::decode(&bytes, digest).unwrap(), supported);
    }
    for (artifact, graph, compiler, bytecode) in [
        (40, 32, 33, 27),
        (40, 33, 32, 27),
        (40, 33, 33, 26),
        (39, 32, 32, 27),
        (29, 33, 33, 27),
        (18, 15, 11, 7),
        (19, 16, 12, 8),
        (20, 17, 13, 9),
        (25, 21, 19, 15),
        (26, 21, 20, 16),
        (27, 21, 21, 17),
        (28, 22, 22, 17),
        (29, 23, 23, 18),
    ] {
        let mut mixed = current.clone();
        mixed.contract_version = artifact;
        mixed.graph_contract_version = graph;
        mixed.compiler_contract_version = compiler;
        mixed.bytecode_contract_version = bytecode;
        assert_eq!(
            mixed.encode().unwrap_err().code,
            "artifact_manifest_contract"
        );
    }
}

pub(super) fn branched_revisions() -> (
    BTreeMap<PackageRevisionDigest, PackageRevision>,
    [PackageRevision; 4],
) {
    fn revision(
        ordinal: u64,
        mut dependencies: Vec<crate::platform::kernel::DependencyRecord>,
    ) -> PackageRevision {
        dependencies.sort_by_key(|dependency| dependency.package);
        PackageRevision {
            contract_version: crate::platform::package_transport::PACKAGE_REVISION_CONTRACT_VERSION,
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            package: PackageId::migrate(b"representation-branch", ordinal),
            revision: crate::platform::publication::RevisionCore {
                contract_version: crate::platform::publication::contract::REVISION_CONTRACT_VERSION,
                graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                repository_id: RepositoryId::migrate(b"representation-branch", ordinal),
                parents: vec![],
                semantic_state: SemanticStateDigest::from_bytes([ordinal as u8; 32]),
            },
            interface: PackageInterfaceDigest::from_bytes([1; 32]),
            dependencies,
        }
    }
    let leaf = revision(1, vec![]);
    let independent = revision(2, vec![]);
    let supplier = revision(3, vec![dependency(&leaf)]);
    let root = revision(4, vec![dependency(&independent), dependency(&supplier)]);
    let revisions = [root, independent, supplier, leaf];
    let map = revisions
        .iter()
        .map(|revision| (revision.encode().unwrap().0, revision.clone()))
        .collect();
    (map, revisions)
}

pub(super) fn dependency(revision: &PackageRevision) -> crate::platform::kernel::DependencyRecord {
    crate::platform::kernel::DependencyRecord {
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        package: revision.package,
        semantic_revision: revision.revision.revision_id().unwrap(),
        package_revision: revision.encode().unwrap().0,
    }
}

#[test]
fn representation_scope_follows_exact_selected_branch_and_admits_work_before_growth() {
    let (revisions, [root, independent, supplier, leaf]) = branched_revisions();
    let mut work = 0;
    assert_eq!(
        artifact_representation_scope(&root.dependencies, &revisions, &mut work, usize::MAX)
            .unwrap(),
        BTreeSet::from([independent.package, supplier.package, leaf.package])
    );
    let admitted = work;
    let mut exact_work = 0;
    artifact_representation_scope(&root.dependencies, &revisions, &mut exact_work, admitted)
        .unwrap();
    assert_eq!(exact_work, admitted);
    let error = artifact_representation_scope(&root.dependencies, &revisions, &mut 0, admitted - 1)
        .unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(error.code, "artifact_representation_work");
    assert!(
        artifact_representation_scope(&independent.dependencies, &revisions, &mut 0, admitted)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        artifact_representation_scope(&supplier.dependencies, &revisions, &mut 0, admitted)
            .unwrap(),
        BTreeSet::from([leaf.package])
    );
    let mut forged = dependency(&independent);
    forged.package_revision = dependency(&leaf).package_revision;
    let error = artifact_representation_scope(&[forged], &revisions, &mut 0, admitted).unwrap_err();
    assert_eq!(error.code, "package_revision_dependency_binding");
    let mut missing = revisions;
    missing.remove(&dependency(&leaf).package_revision);
    let error = artifact_representation_scope(&supplier.dependencies, &missing, &mut 0, admitted)
        .unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Corrupt);
    assert_eq!(error.code, "artifact_representation_binding");
}
