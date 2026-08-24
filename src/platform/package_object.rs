//! Exact Graph 5 package descriptors staged before dependency publication.
//!
//! A package object binds one accepted semantic revision and its committed validation witness.
//! Direct dependency bindings are retained so staging can prove an exact, closed package graph
//! without consulting ambient paths, mutable tags, or a network. Its persistent interface-owner
//! map exposes only validated public signatures and members; executable units and private
//! implementation objects belong to the later artifact contract.

use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DependencyRecord, OwnerKey, OwnerKind, PackageId, PackageInterfaceDeclarationPayload,
    PackageInterfaceRecord, PackageObjectDigest, SemanticRootDigest, TypeForm,
};
use crate::platform::package_interface::{PackageInterfaceValidation, validate_package_interface};
use crate::platform::persistent_map::MapRoot;
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StoreError, StoreErrorClass, StoreWork,
};
use crate::platform::witness::{
    ValidationWitnessDigest, ValidationWitnessManifest, encode_witness_manifest,
};
use bincode::{Decode, Encode};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const PACKAGE_OBJECT_CONTRACT_IDENTITY: &str = "lkjscript-package-object-6";
pub const PACKAGE_OBJECT_CONTRACT_VERSION: u16 = 6;
pub const PACKAGE_OBJECT_MAGIC: [u8; 8] = *b"LKJPKG06";
pub const PACKAGE_OBJECT_ENVELOPE_DOMAIN: &str = "lkjscript.package-object-envelope.v6";
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
    pub interface_owners: MapRoot,
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
    Ok(validate_package_object_closure_with_interface(store, root, expected, work)?.root)
}

pub(crate) struct PackageObjectClosureValidation {
    pub root: PackageObject,
    pub root_interface: PackageInterfaceValidation,
}

pub(crate) fn validate_package_object_closure_with_interface<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    root: PackageObjectDigest,
    expected: Option<&DependencyRecord>,
    work: &mut StoreWork,
) -> Result<PackageObjectClosureValidation, Diagnostic> {
    let mut pending = VecDeque::from([root]);
    let mut objects = BTreeMap::<PackageObjectDigest, PackageObject>::new();
    let mut interfaces = BTreeMap::<PackageObjectDigest, PackageInterfaceValidation>::new();
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
        let interface =
            validate_package_interface(object.package, object.interface_owners, store, work)?;
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
        interfaces.insert(digest, interface);
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
    validate_interface_dependencies(&objects, &interfaces)?;
    let root_object = objects.remove(&root).ok_or_else(|| {
        package_error(
            DiagnosticClass::Corrupt,
            "package_object_closure_root",
            "validated package closure lost its root object",
        )
    })?;
    let root_interface = interfaces.remove(&root).ok_or_else(|| {
        package_error(
            DiagnosticClass::Corrupt,
            "package_object_closure_root_interface",
            "validated package closure lost its root package interface",
        )
    })?;
    Ok(PackageObjectClosureValidation {
        root: root_object,
        root_interface,
    })
}

fn validate_interface_dependencies(
    objects: &BTreeMap<PackageObjectDigest, PackageObject>,
    interfaces: &BTreeMap<PackageObjectDigest, PackageInterfaceValidation>,
) -> Result<(), Diagnostic> {
    let closure = PackageInterfaceClosure {
        packages: objects
            .iter()
            .map(|(digest, object)| (object.package, *digest))
            .collect(),
        interfaces,
    };
    for (digest, object) in objects {
        let interface = interfaces.get(digest).ok_or_else(|| {
            package_error(
                DiagnosticClass::Corrupt,
                "package_object_interface_validation_missing",
                "validated package closure lost one package-interface result",
            )
        })?;
        for ty in interface.type_objects.values() {
            if let TypeForm::Named { declaration } = ty.form {
                closure.require_owner(
                    object,
                    declaration.package,
                    OwnerKey::Declaration(declaration.declaration),
                    &[OwnerKind::Record, OwnerKind::Variant],
                    "named type",
                )?;
            }
        }
        for owner in interface.owners.values() {
            let PackageInterfaceRecord::Requirement(requirement) = &owner.record else {
                continue;
            };
            let interface_owner = closure.require_owner(
                object,
                requirement.interface.package,
                OwnerKey::Declaration(requirement.interface.declaration),
                &[OwnerKind::Interface],
                "requirement interface",
            )?;
            if !matches!(
                interface_owner.record,
                PackageInterfaceRecord::Declaration(ref declaration)
                    if matches!(
                        declaration.payload,
                        PackageInterfaceDeclarationPayload::Interface { .. }
                    )
            ) {
                return Err(package_error(
                    DiagnosticClass::Semantic,
                    "package_object_interface_requirement_kind",
                    "requirement interface does not name an interface declaration payload",
                ));
            }
            for operation in &requirement.operations {
                if operation.package != requirement.interface.package {
                    return Err(package_error(
                        DiagnosticClass::Semantic,
                        "package_object_interface_operation_package",
                        "requirement interface and operation belong to different packages",
                    ));
                }
                let operation_owner = closure.require_owner(
                    object,
                    operation.package,
                    OwnerKey::Operation(operation.operation),
                    &[OwnerKind::Operation],
                    "requirement operation",
                )?;
                let PackageInterfaceRecord::Operation(operation_record) = &operation_owner.record
                else {
                    return Err(package_error(
                        DiagnosticClass::Corrupt,
                        "package_object_interface_operation_variant",
                        "validated operation kind disagrees with its package-interface record variant",
                    ));
                };
                if operation_record.declaration != requirement.interface.declaration {
                    return Err(package_error(
                        DiagnosticClass::Semantic,
                        "package_object_interface_operation_owner",
                        "requirement operation does not belong to its exact interface declaration",
                    ));
                }
            }
        }
    }
    Ok(())
}

struct PackageInterfaceClosure<'a> {
    packages: BTreeMap<PackageId, PackageObjectDigest>,
    interfaces: &'a BTreeMap<PackageObjectDigest, PackageInterfaceValidation>,
}

impl<'a> PackageInterfaceClosure<'a> {
    fn require_owner(
        &self,
        source: &PackageObject,
        package: PackageId,
        owner: OwnerKey,
        kinds: &[OwnerKind],
        label: &str,
    ) -> Result<&'a crate::platform::package_interface::PackageInterfaceOwner, Diagnostic> {
        let digest = if package == source.package {
            self.packages.get(&package).copied().ok_or_else(|| {
                package_error(
                    DiagnosticClass::Corrupt,
                    "package_object_interface_source_lost",
                    "validated package interface lost its source package object",
                )
            })?
        } else {
            let dependency = source
                .dependencies
                .binary_search_by_key(&package, |dependency| dependency.package)
                .ok()
                .map(|index| &source.dependencies[index])
                .ok_or_else(|| {
                    package_error(
                        DiagnosticClass::Semantic,
                        "package_object_interface_dependency_missing",
                        format!(
                            "{label} names package {package} outside the exact direct dependency set"
                        ),
                    )
                })?;
            let digest = self.packages.get(&package).ok_or_else(|| {
                package_error(
                    DiagnosticClass::Corrupt,
                    "package_object_interface_dependency_lost",
                    "validated package-interface dependency disappeared from the package closure",
                )
            })?;
            if *digest != dependency.package_object {
                return Err(package_error(
                    DiagnosticClass::Corrupt,
                    "package_object_interface_dependency_binding",
                    "package-interface dependency resolves to a different exact package object",
                ));
            }
            *digest
        };
        let target = self.interfaces.get(&digest).ok_or_else(|| {
            package_error(
                DiagnosticClass::Corrupt,
                "package_object_interface_dependency_validation",
                "validated package lost its package-interface result",
            )
        })?;
        let value = target.owners.get(&owner).ok_or_else(|| {
            package_error(
                DiagnosticClass::Semantic,
                "package_object_interface_owner_missing",
                format!(
                    "{label} names owner {owner:?} absent from exact package interface {package}"
                ),
            )
        })?;
        if !kinds.contains(&value.kind()) {
            return Err(package_error(
                DiagnosticClass::Semantic,
                "package_object_interface_owner_kind",
                format!(
                    "{label} names package-interface owner kind {:?}",
                    value.kind()
                ),
            ));
        }
        Ok(value)
    }
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
    use crate::platform::kernel::{
        DeclarationReference, FunctionEffect, KernelSnapshot, Name, OwnerHeader,
        PackageFunctionSignature, PackageInterfaceDeclaration, SemanticRoot, TypeObject,
        encode_type_object,
    };
    use crate::platform::package_interface::{
        PACKAGE_INTERFACE_CONTRACT_VERSION, PackageInterfaceOwner, PackageInterfaceSummary,
        build_package_interface,
    };
    use crate::platform::persistent_map::{MapRoot, PageDigest};
    use crate::platform::semantic_id::{DeclarationId, RepositoryId};
    use crate::platform::storage::memory::MemoryPackedStore;
    use crate::platform::storage::object::{ImmutableObjectStore, StageOutcome};
    use crate::platform::witness::{SemanticDigest, rebuild_full_witness};

    fn object(
        seed: u8,
        dependencies: Vec<DependencyRecord>,
    ) -> (
        PackageObjectDigest,
        Vec<u8>,
        PackageObject,
        BTreeMap<ObjectKey, Vec<u8>>,
    ) {
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
            dependency_interfaces: BTreeMap::new(),
            dependency_types: BTreeMap::new(),
            blobs: BTreeMap::new(),
            dependencies: BTreeMap::new(),
            retirements: BTreeMap::new(),
        };
        let witness = rebuild_full_witness(&snapshot).expect("fixture witness");
        let interface = build_package_interface(&BTreeMap::new(), &BTreeMap::new())
            .expect("empty package interface");
        let value = PackageObject {
            contract_version: PACKAGE_OBJECT_CONTRACT_VERSION,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: snapshot.root.repository_id,
            package,
            semantic_revision: RevisionId::from_digest([seed; 32]),
            semantic_root: witness.manifest.semantic_root,
            validation_witness: witness.manifest_digest,
            witness: witness.manifest,
            interface_owners: interface.root,
            dependencies,
        };
        let (digest, bytes) = value.encode().expect("package object encoding");
        (digest, bytes, value, interface.objects)
    }

    #[test]
    fn package_object_round_trips_and_rejects_foreign_bytes() {
        let (digest, bytes, value, _) = object(1, Vec::new());
        assert_eq!(PackageObject::decode(&bytes, digest).unwrap(), value);
        assert_eq!(
            PackageObject::decode(&bytes, PackageObjectDigest::from_bytes([9; 32]))
                .unwrap_err()
                .code,
            "package_object_digest"
        );
        let mut predecessor = bytes;
        predecessor[..8].copy_from_slice(b"LKJPKG05");
        assert!(
            PackageObject::decode(&predecessor, PackageObjectDigest::of(&predecessor)).is_err()
        );
    }

    #[test]
    fn closure_requires_exact_staged_children() {
        let (child_digest, child_bytes, child, child_interface) = object(2, Vec::new());
        let dependency = DependencyRecord {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            package: child.package,
            semantic_revision: child.semantic_revision,
            package_object: child_digest,
        };
        let (root_digest, root_bytes, root, root_interface) = object(1, vec![dependency.clone()]);
        let mut store = MemoryPackedStore::default();
        let mut work = StoreWork::default();
        for (key, bytes) in root_interface {
            store.stage(key, &bytes, &mut work).unwrap();
        }
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
        for (key, bytes) in child_interface {
            store.stage(key, &bytes, &mut work).unwrap();
        }
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

    #[test]
    fn closure_rejects_public_types_outside_the_exact_dependency_set() {
        let (_, _, mut root, _) = object(3, Vec::new());
        let foreign_package = PackageId::migrate(b"package-object-unbound-interface", 0);
        let foreign_declaration = DeclarationId::migrate(b"package-object-unbound-interface", 0);
        let named_type = TypeObject::new(TypeForm::Named {
            declaration: DeclarationReference {
                package: foreign_package,
                declaration: foreign_declaration,
            },
        })
        .unwrap();
        let (named_digest, named_bytes) = encode_type_object(&named_type).unwrap();
        let function = DeclarationId::migrate(b"package-object-interface-function", 0);
        let semantic = SemanticDigest::of("lkjscript.package-object.test.summary.v1", b"summary");
        let interface_owner = PackageInterfaceOwner {
            contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
            summary: PackageInterfaceSummary {
                semantic_interface: semantic,
                type_digest: semantic,
                effect: semantic,
                capability: semantic,
                presentation: semantic,
            },
            record: PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                header: OwnerHeader::new(OwnerKey::Declaration(function), OwnerKind::PureFunction),
                name: Name::new("foreign_type").unwrap(),
                payload: PackageInterfaceDeclarationPayload::Function(PackageFunctionSignature {
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    result: named_digest,
                    effect: FunctionEffect::Pure,
                }),
            }),
        };
        let interface = build_package_interface(
            &BTreeMap::from([(OwnerKey::Declaration(function), interface_owner)]),
            &BTreeMap::from([(named_digest, named_bytes)]),
        )
        .unwrap();
        root.interface_owners = interface.root;
        let (root_digest, root_bytes) = root.encode().unwrap();
        let mut store = MemoryPackedStore::default();
        let mut work = StoreWork::default();
        for (key, bytes) in interface.objects {
            store.stage(key, &bytes, &mut work).unwrap();
        }
        store
            .stage(
                ObjectKey::from_digest(ObjectDomain::PackageObject, root_digest.bytes()),
                &root_bytes,
                &mut work,
            )
            .unwrap();
        assert_eq!(
            validate_package_object_closure(&store, root_digest, None, &mut work)
                .unwrap_err()
                .code,
            "package_object_interface_dependency_missing"
        );
    }
}
