//! Exact Graph 5 package descriptors staged before dependency publication.
//!
//! A package object binds one accepted semantic revision and its committed validation witness.
//! Direct dependency bindings are retained so staging can prove an exact, closed package graph
//! without consulting ambient paths, mutable tags, or a network. Executable units and private
//! implementation objects belong to the later artifact contract, not this descriptor.

use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DependencyRecord, PackageId, PackageObjectDigest, SemanticRootDigest,
};
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StoreError, StoreErrorClass, StoreWork,
};
use crate::platform::witness::{
    ValidationWitnessDigest, ValidationWitnessManifest, encode_witness_manifest,
};
use bincode::{Decode, Encode};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const PACKAGE_OBJECT_CONTRACT_IDENTITY: &str = "lkjscript-package-object-5";
pub const PACKAGE_OBJECT_CONTRACT_VERSION: u16 = 5;
pub const PACKAGE_OBJECT_MAGIC: [u8; 8] = *b"LKJPKG05";
pub const PACKAGE_OBJECT_ENVELOPE_DOMAIN: &str = "lkjscript.package-object-envelope.v5";
pub const MAXIMUM_PACKAGE_OBJECT_BYTES: usize = 4 * 1_048_576;
pub const MAXIMUM_PACKAGE_OBJECT_DEPENDENCIES: usize = 10_000;
pub const MAXIMUM_PACKAGE_OBJECT_CLOSURE: usize = 10_000;

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageObject {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub repository_id: RepositoryId,
    pub package: PackageId,
    pub semantic_revision: RevisionId,
    pub semantic_root: SemanticRootDigest,
    pub validation_witness: ValidationWitnessDigest,
    pub witness: ValidationWitnessManifest,
    pub dependencies: Vec<DependencyRecord>,
}

impl PackageObject {
    pub fn encode(&self) -> Result<(PackageObjectDigest, Vec<u8>), Diagnostic> {
        self.validate()?;
        let bytes = crate::platform::packed::encode(
            PACKAGE_OBJECT_MAGIC,
            PACKAGE_OBJECT_ENVELOPE_DOMAIN,
            self,
            MAXIMUM_PACKAGE_OBJECT_BYTES,
        )?;
        Ok((PackageObjectDigest::of(&bytes), bytes))
    }

    pub fn decode(bytes: &[u8], expected: PackageObjectDigest) -> Result<Self, Diagnostic> {
        if PackageObjectDigest::of(bytes) != expected {
            return Err(package_error(
                DiagnosticClass::Corrupt,
                "package_object_digest",
                "package-object bytes disagree with their exact digest",
            ));
        }
        let value: Self = crate::platform::packed::decode(
            bytes,
            PACKAGE_OBJECT_MAGIC,
            PACKAGE_OBJECT_ENVELOPE_DOMAIN,
            MAXIMUM_PACKAGE_OBJECT_BYTES,
        )?;
        value.validate()?;
        let (digest, canonical) = value.encode()?;
        if digest != expected || canonical != bytes {
            return Err(package_error(
                DiagnosticClass::Corrupt,
                "package_object_canonical",
                "package object is not canonically encoded",
            ));
        }
        Ok(value)
    }

    pub fn matches_dependency(&self, dependency: &DependencyRecord) -> Result<(), Diagnostic> {
        if self.package != dependency.package
            || self.semantic_revision != dependency.semantic_revision
        {
            return Err(package_error(
                DiagnosticClass::Semantic,
                "package_object_dependency_binding",
                "dependency package and semantic revision disagree with the staged package object",
            ));
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != PACKAGE_OBJECT_CONTRACT_VERSION
            || self.graph_contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
        {
            return Err(package_error(
                DiagnosticClass::Source,
                "package_object_contract",
                "package object uses a predecessor or foreign contract",
            ));
        }
        if self.witness.repository_id != self.repository_id
            || self.witness.package_id != self.package
            || self.witness.semantic_root != self.semantic_root
        {
            return Err(package_error(
                DiagnosticClass::Corrupt,
                "package_object_witness_binding",
                "package identity, semantic root, and validation witness do not form one exact binding",
            ));
        }
        let (witness_digest, _) = encode_witness_manifest(&self.witness)?;
        if witness_digest != self.validation_witness {
            return Err(package_error(
                DiagnosticClass::Corrupt,
                "package_object_witness_digest",
                "package-object witness bytes disagree with the committed witness digest",
            ));
        }
        if self.dependencies.len() > MAXIMUM_PACKAGE_OBJECT_DEPENDENCIES {
            return Err(package_error(
                DiagnosticClass::Resource,
                "package_object_dependency_count",
                format!(
                    "package object contains more than {MAXIMUM_PACKAGE_OBJECT_DEPENDENCIES} direct dependencies"
                ),
            ));
        }
        for dependency in &self.dependencies {
            dependency.validate_local()?;
            if dependency.package == self.package {
                return Err(package_error(
                    DiagnosticClass::Semantic,
                    "package_object_self_dependency",
                    "package object cannot depend on its own package identity",
                ));
            }
        }
        if self
            .dependencies
            .windows(2)
            .any(|pair| pair[0].package >= pair[1].package)
        {
            return Err(package_error(
                DiagnosticClass::Corrupt,
                "package_object_dependency_order",
                "package-object dependencies must be strictly ordered by package identity",
            ));
        }
        Ok(())
    }
}

/// Reads and verifies the complete package-object closure rooted at `root`.
///
/// The traversal is exact and bounded. Repeated package identities must retain one revision and
/// one package-object digest, and the complete dependency graph must be acyclic.
pub fn validate_package_object_closure<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    root: PackageObjectDigest,
    expected: Option<&DependencyRecord>,
    work: &mut StoreWork,
) -> Result<PackageObject, Diagnostic> {
    let mut pending = VecDeque::from([root]);
    let mut objects = BTreeMap::<PackageObjectDigest, PackageObject>::new();
    let mut packages = BTreeMap::<PackageId, (RevisionId, PackageObjectDigest)>::new();
    while let Some(digest) = pending.pop_front() {
        if objects.contains_key(&digest) {
            continue;
        }
        if objects.len() == MAXIMUM_PACKAGE_OBJECT_CLOSURE {
            return Err(package_error(
                DiagnosticClass::Resource,
                "package_object_closure_count",
                format!("package-object closure exceeds {MAXIMUM_PACKAGE_OBJECT_CLOSURE} objects"),
            ));
        }
        let key = ObjectKey::from_digest(ObjectDomain::PackageObject, digest.bytes());
        let bytes = store
            .read(key, MAXIMUM_PACKAGE_OBJECT_BYTES, work)
            .map_err(store_diagnostic)?
            .ok_or_else(|| {
                package_error(
                    DiagnosticClass::Semantic,
                    "package_object_missing",
                    format!("required exact package object {digest} is not staged"),
                )
            })?;
        let object = PackageObject::decode(&bytes, digest)?;
        if digest == root
            && let Some(expected) = expected
        {
            object.matches_dependency(expected)?;
        }
        match packages.insert(object.package, (object.semantic_revision, digest)) {
            Some(previous) if previous != (object.semantic_revision, digest) => {
                return Err(package_error(
                    DiagnosticClass::Semantic,
                    "package_object_closure_package_conflict",
                    "one package identity is bound to different exact revisions or package objects",
                ));
            }
            _ => {}
        }
        for dependency in &object.dependencies {
            if let Some(previous) = packages.get(&dependency.package)
                && *previous != (dependency.semantic_revision, dependency.package_object)
            {
                return Err(package_error(
                    DiagnosticClass::Semantic,
                    "package_object_closure_binding_conflict",
                    "package closure contains conflicting exact bindings for one package",
                ));
            }
            pending.push_back(dependency.package_object);
        }
        objects.insert(digest, object);
    }

    for object in objects.values() {
        for dependency in &object.dependencies {
            let child = objects.get(&dependency.package_object).ok_or_else(|| {
                package_error(
                    DiagnosticClass::Corrupt,
                    "package_object_closure_incomplete",
                    "validated package closure lost one required child object",
                )
            })?;
            child.matches_dependency(dependency)?;
        }
    }
    reject_dependency_cycle(&objects)?;
    objects.remove(&root).ok_or_else(|| {
        package_error(
            DiagnosticClass::Corrupt,
            "package_object_closure_root",
            "validated package closure lost its root object",
        )
    })
}

fn reject_dependency_cycle(
    objects: &BTreeMap<PackageObjectDigest, PackageObject>,
) -> Result<(), Diagnostic> {
    let mut indegree = objects
        .keys()
        .copied()
        .map(|digest| (digest, 0_usize))
        .collect::<BTreeMap<_, _>>();
    for object in objects.values() {
        for dependency in &object.dependencies {
            let degree = indegree
                .get_mut(&dependency.package_object)
                .ok_or_else(|| {
                    package_error(
                        DiagnosticClass::Corrupt,
                        "package_object_closure_edge",
                        "package dependency points outside the validated closure",
                    )
                })?;
            *degree = degree.checked_add(1).ok_or_else(|| {
                package_error(
                    DiagnosticClass::Resource,
                    "package_object_closure_indegree",
                    "package dependency indegree overflowed",
                )
            })?;
        }
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(digest, degree)| (*degree == 0).then_some(*digest))
        .collect::<BTreeSet<_>>();
    let mut visited = 0_usize;
    while let Some(digest) = ready.pop_first() {
        visited = visited.saturating_add(1);
        let object = &objects[&digest];
        for dependency in &object.dependencies {
            let degree = indegree
                .get_mut(&dependency.package_object)
                .ok_or_else(|| {
                    package_error(
                        DiagnosticClass::Corrupt,
                        "package_object_closure_topology",
                        "package dependency disappeared during cycle validation",
                    )
                })?;
            *degree -= 1;
            if *degree == 0 {
                ready.insert(dependency.package_object);
            }
        }
    }
    if visited != objects.len() {
        return Err(package_error(
            DiagnosticClass::Semantic,
            "package_object_dependency_cycle",
            "package-object dependency closure contains a cycle",
        ));
    }
    Ok(())
}

fn store_diagnostic(error: StoreError) -> Diagnostic {
    let class = match error.class {
        StoreErrorClass::Input => DiagnosticClass::Source,
        StoreErrorClass::Resource => DiagnosticClass::Resource,
        StoreErrorClass::Corrupt => DiagnosticClass::Corrupt,
        StoreErrorClass::Io => DiagnosticClass::Infrastructure,
    };
    package_error(class, error.code, error.message)
}

fn package_error(
    class: DiagnosticClass,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
    use crate::platform::kernel::{KernelSnapshot, Name, SemanticRoot};
    use crate::platform::persistent_map::{MapRoot, PageDigest};
    use crate::platform::semantic_id::RepositoryId;
    use crate::platform::storage::memory::MemoryPackedStore;
    use crate::platform::storage::object::{ImmutableObjectStore, StageOutcome};
    use crate::platform::witness::rebuild_full_witness;

    fn object(
        seed: u8,
        dependencies: Vec<DependencyRecord>,
    ) -> (PackageObjectDigest, Vec<u8>, PackageObject) {
        let package = PackageId::migrate(b"package-object-test", u64::from(seed));
        let empty = MapRoot::from_parts(PageDigest::from_bytes([seed; 32]), 0);
        let snapshot = KernelSnapshot {
            root: SemanticRoot {
                graph_contract_version: GRAPH_CONTRACT_VERSION,
                repository_id: RepositoryId::migrate(b"package-object-test", u64::from(seed)),
                package_id: package,
                package_name: Name::new(format!("package_{seed}")).expect("fixture package name"),
                owners: empty,
                dependencies: empty,
                retirements: empty,
            },
            owners: BTreeMap::new(),
            types: BTreeMap::new(),
            blobs: BTreeMap::new(),
            dependencies: BTreeMap::new(),
            retirements: BTreeMap::new(),
        };
        let witness = rebuild_full_witness(&snapshot).expect("fixture witness");
        let value = PackageObject {
            contract_version: PACKAGE_OBJECT_CONTRACT_VERSION,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: snapshot.root.repository_id,
            package,
            semantic_revision: RevisionId::from_digest([seed; 32]),
            semantic_root: witness.manifest.semantic_root,
            validation_witness: witness.manifest_digest,
            witness: witness.manifest,
            dependencies,
        };
        let (digest, bytes) = value.encode().expect("package object encoding");
        (digest, bytes, value)
    }

    #[test]
    fn package_object_round_trips_and_rejects_foreign_bytes() {
        let (digest, bytes, value) = object(1, Vec::new());
        assert_eq!(PackageObject::decode(&bytes, digest).unwrap(), value);
        assert_eq!(
            PackageObject::decode(&bytes, PackageObjectDigest::from_bytes([9; 32]))
                .unwrap_err()
                .code,
            "package_object_digest"
        );
        let mut predecessor = bytes;
        predecessor[..8].copy_from_slice(b"LKJPKG03");
        assert!(
            PackageObject::decode(&predecessor, PackageObjectDigest::of(&predecessor)).is_err()
        );
    }

    #[test]
    fn closure_requires_exact_staged_children() {
        let (child_digest, child_bytes, child) = object(2, Vec::new());
        let dependency = DependencyRecord {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            package: child.package,
            semantic_revision: child.semantic_revision,
            package_object: child_digest,
        };
        let (root_digest, root_bytes, root) = object(1, vec![dependency.clone()]);
        let mut store = MemoryPackedStore::default();
        let mut work = StoreWork::default();
        assert_eq!(
            store
                .stage(
                    ObjectKey::from_digest(ObjectDomain::PackageObject, root_digest.bytes()),
                    &root_bytes,
                    &mut work,
                )
                .unwrap(),
            StageOutcome::Inserted
        );
        assert_eq!(
            validate_package_object_closure(&store, root_digest, None, &mut work)
                .unwrap_err()
                .code,
            "package_object_missing"
        );
        store
            .stage(
                ObjectKey::from_digest(ObjectDomain::PackageObject, child_digest.bytes()),
                &child_bytes,
                &mut work,
            )
            .unwrap();
        assert_eq!(
            validate_package_object_closure(&store, root_digest, None, &mut work).unwrap(),
            root
        );

        let wrong = DependencyRecord {
            semantic_revision: RevisionId::from_digest([7; 32]),
            ..dependency
        };
        assert_eq!(
            validate_package_object_closure(&store, child_digest, Some(&wrong), &mut work)
                .unwrap_err()
                .code,
            "package_object_dependency_binding"
        );
    }
}
