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
fn artifact_manifest_admits_only_current_profile() {
    let current = manifest();
    assert_eq!(
        (
            current.contract_version,
            current.graph_contract_version,
            current.compiler_contract_version,
            current.bytecode_contract_version
        ),
        (41, 34, 34, 28)
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
    for dimension in 0..5 {
        let mut foreign = current.clone();
        match dimension {
            0 => foreign.contract_version ^= 1,
            1 => foreign.graph_contract_version ^= 1,
            2 => foreign.compiler_contract_version ^= 1,
            3 => foreign.bytecode_contract_version ^= 1,
            _ => foreign.compilation_manifest_contract_version ^= 1,
        }
        assert_eq!(
            foreign.encode().unwrap_err().code,
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
