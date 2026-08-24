//! Derived, implementation-free package interface records for exact Graph 5 dependencies.
//!
//! These records are not accepted program authority. They are deterministic projections of one
//! validated package revision and carry the committed summary dimensions needed to detect drift
//! between canonical owners and the exported interface. Exact dependency bindings select one
//! package object; the package object will in turn commit to a persistent map of these records.

use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DeclarationPayload, DeclarationVisibility, EncodedOwnerKey, ExternalDeclaration,
    FunctionDeclaration, FunctionEffect, OwnerKey, OwnerKind, OwnerRecord, PackageId,
    PackageInterfaceDeclarationPayload, PackageInterfaceRecord, ParameterParent, TypeForm,
    TypeObject, TypeObjectDigest, decode_type_object, encode_owner,
};
use crate::platform::persistent_map::{
    MapEdit, MapError, MapErrorClass, MapRoot, MapWork, MemoryPageStore, PageDigest, PageStore,
    PageWrite, PersistentMap,
};
use crate::platform::semantic_id::{
    CaseId, DeclarationId, FieldId, OperationId, ParameterId, PortId, RequirementId,
    TypeParameterId,
};
use crate::platform::storage::contract::PACKAGE_INTERFACE_OWNER_DIGEST_DOMAIN;
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, ObjectStage, StageOutcome, StoreError,
    StoreErrorClass, StoreWork,
};
use crate::platform::storage::page_store::{ObjectPageReader, ObjectPageStore};
use crate::platform::witness::{OwnerSummary, SemanticDigest};
use bincode::{Decode, Encode};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const PACKAGE_INTERFACE_CONTRACT_IDENTITY: &str = "lkjscript-package-interface-owner-2";
pub const PACKAGE_INTERFACE_CONTRACT_VERSION: u16 = 2;
pub const PACKAGE_INTERFACE_MAGIC: [u8; 8] = *b"LKJPIF02";
pub const PACKAGE_INTERFACE_ENVELOPE_DOMAIN: &str = "lkjscript.package-interface-owner-envelope.v2";
pub const MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES: usize = 1024 * 1024;
pub const MAXIMUM_PACKAGE_INTERFACE_VALIDATION_WORK: usize =
    crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PackageInterfaceOwnerDigest([u8; 32]);

impl PackageInterfaceOwnerDigest {
    pub fn of(bytes: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new_derive_key(PACKAGE_INTERFACE_OWNER_DIGEST_DOMAIN);
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
        Self(*hasher.finalize().as_bytes())
    }

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for PackageInterfaceOwnerDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("package_interface_owner_")?;
        formatter.write_str(&crate::platform::semantic_id::encode_hex(&self.0))
    }
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageInterfaceSummary {
    pub semantic_interface: SemanticDigest,
    pub type_digest: SemanticDigest,
    pub effect: SemanticDigest,
    pub capability: SemanticDigest,
    pub presentation: SemanticDigest,
}

impl From<&OwnerSummary> for PackageInterfaceSummary {
    fn from(summary: &OwnerSummary) -> Self {
        Self {
            semantic_interface: summary.semantic_interface,
            type_digest: summary.type_digest,
            effect: summary.effect,
            capability: summary.capability,
            presentation: summary.presentation,
        }
    }
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageInterfaceOwner {
    pub contract_version: u16,
    pub summary: PackageInterfaceSummary,
    pub record: PackageInterfaceRecord,
}

impl PackageInterfaceOwner {
    pub fn project(
        canonical: &OwnerRecord,
        summary: &OwnerSummary,
        selection: &PackageInterfaceSelection,
    ) -> Result<Option<Self>, Diagnostic> {
        if summary.owner != canonical.owner() || summary.kind != canonical.kind() {
            return Err(interface_error(
                DiagnosticClass::Corrupt,
                "package_interface_summary_owner",
                "accepted owner summary disagrees with the canonical owner identity or kind",
            ));
        }
        let (record_digest, _) = encode_owner(canonical)?;
        if summary.record != record_digest {
            return Err(interface_error(
                DiagnosticClass::Corrupt,
                "package_interface_summary_record",
                "accepted owner summary is not bound to the projected canonical owner bytes",
            ));
        }
        if !selection.contains(canonical.owner()) {
            return Ok(None);
        }
        let Some(record) = PackageInterfaceRecord::project_public(canonical)? else {
            return Ok(None);
        };
        let value = Self {
            contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
            summary: summary.into(),
            record,
        };
        value.validate_local()?;
        Ok(Some(value))
    }

    pub fn owner(&self) -> OwnerKey {
        self.record.header().owner
    }

    pub fn kind(&self) -> OwnerKind {
        self.record.header().kind
    }

    pub fn type_roots(&self) -> Vec<TypeObjectDigest> {
        self.record.type_roots()
    }

    pub fn encode(&self) -> Result<(PackageInterfaceOwnerDigest, Vec<u8>), Diagnostic> {
        self.validate_local()?;
        let bytes = crate::platform::packed::encode(
            PACKAGE_INTERFACE_MAGIC,
            PACKAGE_INTERFACE_ENVELOPE_DOMAIN,
            self,
            MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
        )?;
        Ok((PackageInterfaceOwnerDigest::of(&bytes), bytes))
    }

    pub fn decode(
        bytes: &[u8],
        expected_owner: OwnerKey,
        expected_digest: PackageInterfaceOwnerDigest,
    ) -> Result<Self, Diagnostic> {
        if PackageInterfaceOwnerDigest::of(bytes) != expected_digest {
            return Err(interface_error(
                DiagnosticClass::Corrupt,
                "package_interface_digest",
                "package-interface owner bytes disagree with their exact digest",
            ));
        }
        let value: Self = crate::platform::packed::decode(
            bytes,
            PACKAGE_INTERFACE_MAGIC,
            PACKAGE_INTERFACE_ENVELOPE_DOMAIN,
            MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
        )?;
        value.validate_local()?;
        if value.owner() != expected_owner {
            return Err(interface_error(
                DiagnosticClass::Corrupt,
                "package_interface_owner_key",
                "package-interface map key disagrees with the decoded owner identity",
            ));
        }
        let (digest, canonical) = value.encode()?;
        if digest != expected_digest || canonical != bytes {
            return Err(interface_error(
                DiagnosticClass::Corrupt,
                "package_interface_canonical",
                "package-interface owner is not canonically encoded",
            ));
        }
        Ok(value)
    }

    fn validate_local(&self) -> Result<(), Diagnostic> {
        if self.contract_version != PACKAGE_INTERFACE_CONTRACT_VERSION {
            return Err(interface_error(
                DiagnosticClass::Source,
                "package_interface_contract",
                "package-interface owner uses a predecessor or foreign contract",
            ));
        }
        self.record.validate_local()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageInterfaceSelection {
    package: PackageId,
    declarations: BTreeSet<DeclarationId>,
    type_parameters: BTreeSet<TypeParameterId>,
    fields: BTreeSet<FieldId>,
    cases: BTreeSet<CaseId>,
    operations: BTreeSet<OperationId>,
    parameters: BTreeSet<ParameterId>,
    requirements: BTreeSet<RequirementId>,
    ports: BTreeSet<PortId>,
}

impl PackageInterfaceSelection {
    pub fn new(package: PackageId) -> Self {
        Self {
            package,
            declarations: BTreeSet::new(),
            type_parameters: BTreeSet::new(),
            fields: BTreeSet::new(),
            cases: BTreeSet::new(),
            operations: BTreeSet::new(),
            parameters: BTreeSet::new(),
            requirements: BTreeSet::new(),
            ports: BTreeSet::new(),
        }
    }

    fn contains(&self, owner: OwnerKey) -> bool {
        match owner {
            OwnerKey::Declaration(id) => self.declarations.contains(&id),
            OwnerKey::TypeParameter(id) => self.type_parameters.contains(&id),
            OwnerKey::Field(id) => self.fields.contains(&id),
            OwnerKey::Case(id) => self.cases.contains(&id),
            OwnerKey::Operation(id) => self.operations.contains(&id),
            OwnerKey::Parameter(id) => self.parameters.contains(&id),
            OwnerKey::Requirement(id) => self.requirements.contains(&id),
            OwnerKey::Port(id) => self.ports.contains(&id),
            OwnerKey::Module(_)
            | OwnerKey::Binding(_)
            | OwnerKey::Expression(_)
            | OwnerKey::Target(_)
            | OwnerKey::Documentation(_)
            | OwnerKey::Annotation(_) => false,
        }
    }

    pub fn from_records(
        package: PackageId,
        records: &BTreeMap<OwnerKey, OwnerRecord>,
    ) -> Result<Self, Diagnostic> {
        let mut selection = Self::new(package);
        for record in records.values() {
            selection.observe_declaration(record)?;
        }
        for record in records.values() {
            selection.observe_operation(record)?;
        }
        Ok(selection)
    }

    pub fn observe_declaration(&mut self, owner: &OwnerRecord) -> Result<(), Diagnostic> {
        let OwnerRecord::Declaration(record) = owner else {
            return Ok(());
        };
        if record.visibility != DeclarationVisibility::Public {
            return Ok(());
        }
        if matches!(record.payload, DeclarationPayload::Test { .. }) {
            return Err(interface_error(
                DiagnosticClass::Semantic,
                "package_interface_public_test",
                "tests are executable package-local evidence and cannot have public visibility",
            ));
        }
        let declaration = declaration_id(record.header.owner)?;
        self.declarations.insert(declaration);
        match &record.payload {
            DeclarationPayload::Record { fields } => self.fields.extend(fields),
            DeclarationPayload::Variant { cases } => self.cases.extend(cases),
            DeclarationPayload::Interface { operations } => self.operations.extend(operations),
            DeclarationPayload::External(ExternalDeclaration {
                type_parameters,
                parameters,
                ..
            }) => {
                self.type_parameters.extend(type_parameters);
                self.parameters.extend(parameters);
            }
            DeclarationPayload::Function(FunctionDeclaration {
                type_parameters,
                parameters,
                effect,
                ..
            }) => {
                self.type_parameters.extend(type_parameters);
                self.parameters.extend(parameters);
                if let FunctionEffect::Task { requirements } = effect {
                    self.requirements.extend(
                        requirements
                            .iter()
                            .filter(|requirement| requirement.package == self.package)
                            .map(|requirement| requirement.requirement),
                    );
                }
            }
            DeclarationPayload::Component {
                requirements,
                ports,
            } => {
                self.requirements.extend(requirements);
                self.ports.extend(ports);
            }
            DeclarationPayload::Constant { .. } | DeclarationPayload::Test { .. } => {}
        }
        Ok(())
    }

    pub fn observe_operation(&mut self, owner: &OwnerRecord) -> Result<(), Diagnostic> {
        let OwnerRecord::Operation(record) = owner else {
            return Ok(());
        };
        let operation = operation_id(record.header.owner)?;
        if self.operations.contains(&operation) {
            self.parameters.extend(&record.parameters);
        }
        Ok(())
    }

    pub fn owners(&self) -> impl Iterator<Item = OwnerKey> + '_ {
        self.declarations
            .iter()
            .copied()
            .map(OwnerKey::Declaration)
            .chain(
                self.type_parameters
                    .iter()
                    .copied()
                    .map(OwnerKey::TypeParameter),
            )
            .chain(self.fields.iter().copied().map(OwnerKey::Field))
            .chain(self.cases.iter().copied().map(OwnerKey::Case))
            .chain(self.operations.iter().copied().map(OwnerKey::Operation))
            .chain(self.parameters.iter().copied().map(OwnerKey::Parameter))
            .chain(self.requirements.iter().copied().map(OwnerKey::Requirement))
            .chain(self.ports.iter().copied().map(OwnerKey::Port))
    }
}

#[derive(Clone, Debug)]
pub struct PackageInterfaceBuild {
    pub root: MapRoot,
    pub objects: BTreeMap<ObjectKey, Vec<u8>>,
    pub owner_count: u64,
    pub type_count: u64,
    pub map_work: MapWork,
    pub store_work: StoreWork,
}

/// Builds one detached immutable closure. Only pages reachable from the final interface root are
/// retained; the initial empty page and any superseded construction pages are omitted.
pub fn build_package_interface(
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    types: &BTreeMap<TypeObjectDigest, Vec<u8>>,
) -> Result<PackageInterfaceBuild, Diagnostic> {
    let mut page_store = MemoryPageStore::default();
    let mut map_work = MapWork::default();
    let empty = PersistentMap::empty(&mut page_store, &mut map_work).map_err(map_diagnostic)?;
    let mut edits = Vec::with_capacity(owners.len());
    let mut owner_bytes = Vec::with_capacity(owners.len());
    for (owner, value) in owners {
        if value.owner() != *owner {
            return Err(interface_error(
                DiagnosticClass::Corrupt,
                "package_interface_build_owner",
                "package-interface build key disagrees with its owner record",
            ));
        }
        let (digest, bytes) = value.encode()?;
        edits.push(MapEdit {
            key: EncodedOwnerKey::new(*owner).bytes().to_vec(),
            before: None,
            after: Some(encode_package_interface_binding(digest)),
        });
        owner_bytes.push((digest, bytes));
    }
    let (map, _) = empty
        .apply_sorted_edits(&mut page_store, &edits, &mut map_work)
        .map_err(map_diagnostic)?;

    let mut detached = ObjectStage::new(&EMPTY_OBJECT_STORE);
    let page_store_work;
    {
        let mut destination = ObjectPageStore::new(&mut detached);
        map.copy_reachable(&page_store, &mut destination, &mut map_work)
            .map_err(map_diagnostic)?;
        page_store_work = destination.work();
    }
    let mut store_work = StoreWork::default();
    store_work.add(page_store_work);
    for (digest, bytes) in owner_bytes {
        detached
            .stage(
                ObjectKey::from_digest(ObjectDomain::PackageInterface, digest.bytes()),
                &bytes,
                &mut store_work,
            )
            .map_err(store_diagnostic)?;
    }
    for (digest, bytes) in types {
        let decoded = decode_type_object(bytes, *digest)?;
        if decoded
            .child_types()
            .iter()
            .any(|child| !types.contains_key(child))
        {
            return Err(interface_error(
                DiagnosticClass::Semantic,
                "package_interface_build_type_child",
                "package-interface type closure omits a referenced structural child",
            ));
        }
        detached
            .stage(
                ObjectKey::from_digest(ObjectDomain::Type, digest.bytes()),
                bytes,
                &mut store_work,
            )
            .map_err(store_diagnostic)?;
    }
    Ok(PackageInterfaceBuild {
        root: map.root(),
        objects: detached.into_objects(),
        owner_count: owners.len() as u64,
        type_count: types.len() as u64,
        map_work,
        store_work,
    })
}

#[derive(Clone, Debug)]
pub struct PackageInterfaceValidation {
    pub owners: BTreeMap<OwnerKey, PackageInterfaceOwner>,
    pub type_objects: BTreeMap<TypeObjectDigest, TypeObject>,
    pub reachable_objects: BTreeSet<ObjectKey>,
    pub map_work: MapWork,
}

/// Independently checks the complete implementation-free interface reachable from one package
/// object. This never reads canonical owner objects or treats witness summaries as a second
/// writer; it validates the exact derived closure that the dependency binding selected.
pub fn validate_package_interface<S: ImmutableObjectStore + ?Sized>(
    package: PackageId,
    root: MapRoot,
    store: &S,
    work: &mut StoreWork,
) -> Result<PackageInterfaceValidation, Diagnostic> {
    if root.entries() > MAXIMUM_PACKAGE_INTERFACE_VALIDATION_WORK as u64 {
        return Err(interface_error(
            DiagnosticClass::Resource,
            "package_interface_owner_work",
            "package-interface owner count exceeds the current explicit validation work budget",
        ));
    }
    let map = PersistentMap::from_root(root);
    let reader = ObjectPageReader::new(store);
    let mut map_work = MapWork::default();
    let mut page_recorder = ReachablePageRecorder::default();
    map.copy_reachable(&reader, &mut page_recorder, &mut map_work)
        .map_err(map_diagnostic)?;
    let mut bindings = Vec::with_capacity(root.entries() as usize);
    map.for_each(&reader, &mut map_work, |key, value| {
        bindings.push((key.to_vec(), value.to_vec()));
        Ok(())
    })
    .map_err(map_diagnostic)?;
    work.add(reader.work());

    let mut reachable_objects = page_recorder
        .pages
        .iter()
        .copied()
        .map(|digest| ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes()))
        .collect::<BTreeSet<_>>();
    let mut owners = BTreeMap::new();
    for (key_bytes, binding_bytes) in bindings {
        let owner = EncodedOwnerKey::decode(&key_bytes)?;
        let digest = decode_package_interface_binding(&binding_bytes)?;
        let key = ObjectKey::from_digest(ObjectDomain::PackageInterface, digest.bytes());
        let bytes = store
            .read(key, MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES, work)
            .map_err(store_diagnostic)?
            .ok_or_else(|| {
                interface_error(
                    DiagnosticClass::Semantic,
                    "package_interface_owner_missing",
                    format!("package interface omits exact owner object {digest}"),
                )
            })?;
        let value = PackageInterfaceOwner::decode(&bytes, owner, digest)?;
        reachable_objects.insert(key);
        if owners.insert(owner, value).is_some() {
            return Err(interface_error(
                DiagnosticClass::Corrupt,
                "package_interface_owner_duplicate",
                "package-interface map decodes one owner identity more than once",
            ));
        }
    }
    validate_owner_closure(package, &owners)?;
    let (type_objects, type_keys) = validate_type_closure(package, &owners, store, work)?;
    reachable_objects.extend(type_keys);
    Ok(PackageInterfaceValidation {
        owners,
        type_objects,
        reachable_objects,
        map_work,
    })
}

fn validate_owner_closure(
    package: PackageId,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
) -> Result<(), Diagnostic> {
    let mut expected = owners
        .iter()
        .filter_map(|(owner, value)| {
            matches!(value.record, PackageInterfaceRecord::Declaration(_)).then_some(*owner)
        })
        .collect::<BTreeSet<_>>();
    for (owner, value) in owners {
        let PackageInterfaceRecord::Declaration(declaration) = &value.record else {
            continue;
        };
        let OwnerKey::Declaration(declaration_id) = owner else {
            return Err(interface_corrupt(
                "declaration has a foreign owner identity",
            ));
        };
        match &declaration.payload {
            PackageInterfaceDeclarationPayload::Record { fields } => {
                for field in fields {
                    require_child(
                        owners,
                        &mut expected,
                        OwnerKey::Field(*field),
                        OwnerKind::Field,
                        Some(*declaration_id),
                    )?;
                }
            }
            PackageInterfaceDeclarationPayload::Variant { cases } => {
                for case in cases {
                    require_child(
                        owners,
                        &mut expected,
                        OwnerKey::Case(*case),
                        OwnerKind::Case,
                        Some(*declaration_id),
                    )?;
                }
            }
            PackageInterfaceDeclarationPayload::Interface { operations } => {
                for operation in operations {
                    let operation_key = OwnerKey::Operation(*operation);
                    require_child(
                        owners,
                        &mut expected,
                        operation_key,
                        OwnerKind::Operation,
                        Some(*declaration_id),
                    )?;
                    let Some(PackageInterfaceOwner {
                        record: PackageInterfaceRecord::Operation(record),
                        ..
                    }) = owners.get(&operation_key)
                    else {
                        return Err(interface_corrupt(
                            "validated operation child disappeared or changed record variant",
                        ));
                    };
                    for parameter in &record.parameters {
                        require_parameter(
                            owners,
                            &mut expected,
                            *parameter,
                            ParameterParent::Operation(*operation),
                        )?;
                    }
                }
            }
            PackageInterfaceDeclarationPayload::External(signature) => {
                require_signature_children(
                    owners,
                    &mut expected,
                    *declaration_id,
                    &signature.type_parameters,
                    &signature.parameters,
                )?;
            }
            PackageInterfaceDeclarationPayload::Function(signature) => {
                require_signature_children(
                    owners,
                    &mut expected,
                    *declaration_id,
                    &signature.type_parameters,
                    &signature.parameters,
                )?;
                if let FunctionEffect::Task { requirements } = &signature.effect {
                    for requirement in requirements {
                        if requirement.package == package {
                            require_child(
                                owners,
                                &mut expected,
                                OwnerKey::Requirement(requirement.requirement),
                                OwnerKind::Requirement,
                                None,
                            )?;
                        }
                    }
                }
            }
            PackageInterfaceDeclarationPayload::Component {
                requirements,
                ports,
            } => {
                for requirement in requirements {
                    require_child(
                        owners,
                        &mut expected,
                        OwnerKey::Requirement(*requirement),
                        OwnerKind::Requirement,
                        Some(*declaration_id),
                    )?;
                }
                for port in ports {
                    require_child(
                        owners,
                        &mut expected,
                        OwnerKey::Port(*port),
                        OwnerKind::Port,
                        Some(*declaration_id),
                    )?;
                }
            }
            PackageInterfaceDeclarationPayload::Constant { .. } => {}
        }
    }
    let actual = owners.keys().copied().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(interface_error(
            DiagnosticClass::Corrupt,
            "package_interface_unreachable_owner",
            "package-interface map contains an owner outside its public declaration closure",
        ));
    }
    Ok(())
}

fn require_signature_children(
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    expected: &mut BTreeSet<OwnerKey>,
    declaration: DeclarationId,
    type_parameters: &[TypeParameterId],
    parameters: &[ParameterId],
) -> Result<(), Diagnostic> {
    for parameter in type_parameters {
        require_child(
            owners,
            expected,
            OwnerKey::TypeParameter(*parameter),
            OwnerKind::TypeParameter,
            Some(declaration),
        )?;
    }
    for parameter in parameters {
        require_parameter(
            owners,
            expected,
            *parameter,
            ParameterParent::Function(declaration),
        )?;
    }
    Ok(())
}

fn require_parameter(
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    expected: &mut BTreeSet<OwnerKey>,
    parameter: ParameterId,
    parent: ParameterParent,
) -> Result<(), Diagnostic> {
    let key = OwnerKey::Parameter(parameter);
    require_child(owners, expected, key, OwnerKind::Parameter, None)?;
    let Some(PackageInterfaceOwner {
        record: PackageInterfaceRecord::Parameter(record),
        ..
    }) = owners.get(&key)
    else {
        return Err(interface_corrupt(
            "validated parameter child disappeared or changed record variant",
        ));
    };
    if record.parent != parent {
        return Err(interface_error(
            DiagnosticClass::Semantic,
            "package_interface_parameter_parent",
            "package-interface parameter disagrees with its signature parent",
        ));
    }
    Ok(())
}

fn require_child(
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    expected: &mut BTreeSet<OwnerKey>,
    child: OwnerKey,
    kind: OwnerKind,
    declaration: Option<DeclarationId>,
) -> Result<(), Diagnostic> {
    let Some(value) = owners.get(&child) else {
        return Err(interface_error(
            DiagnosticClass::Semantic,
            "package_interface_child_missing",
            format!("package interface omits required child {child:?}"),
        ));
    };
    if value.kind() != kind {
        return Err(interface_error(
            DiagnosticClass::Semantic,
            "package_interface_child_kind",
            "package-interface child has a foreign owner kind",
        ));
    }
    if let Some(declaration) = declaration {
        let actual = match &value.record {
            PackageInterfaceRecord::TypeParameter(record) => record.declaration,
            PackageInterfaceRecord::Field(record) => record.declaration,
            PackageInterfaceRecord::Case(record) => record.declaration,
            PackageInterfaceRecord::Operation(record) => record.declaration,
            PackageInterfaceRecord::Requirement(record) => record.declaration,
            PackageInterfaceRecord::Port(record) => record.declaration,
            PackageInterfaceRecord::Declaration(_) | PackageInterfaceRecord::Parameter(_) => {
                return Err(interface_corrupt("child kind has no declaration parent"));
            }
        };
        if actual != declaration {
            return Err(interface_error(
                DiagnosticClass::Semantic,
                "package_interface_child_parent",
                "package-interface child disagrees with its declaration parent",
            ));
        }
    }
    expected.insert(child);
    Ok(())
}

fn validate_type_closure<S: ImmutableObjectStore + ?Sized>(
    package: PackageId,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    store: &S,
    work: &mut StoreWork,
) -> Result<(BTreeMap<TypeObjectDigest, TypeObject>, BTreeSet<ObjectKey>), Diagnostic> {
    let mut objects = BTreeMap::new();
    let mut keys = BTreeSet::new();
    let mut pending = owners
        .iter()
        .flat_map(|(owner, record)| {
            record
                .type_roots()
                .into_iter()
                .map(|digest| (*owner, digest, 0_usize))
        })
        .collect::<Vec<_>>();
    let mut visited = BTreeSet::new();
    while let Some((source, digest, depth)) = pending.pop() {
        if !visited.insert((source, digest)) {
            continue;
        }
        if visited.len() > MAXIMUM_PACKAGE_INTERFACE_VALIDATION_WORK
            || depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
        {
            return Err(interface_error(
                DiagnosticClass::Resource,
                "package_interface_type_work",
                "package-interface type closure exceeds its explicit validation work budget",
            ));
        }
        let key = ObjectKey::from_digest(ObjectDomain::Type, digest.bytes());
        let bytes = store
            .read(key, ObjectDomain::Type.maximum_bytes(), work)
            .map_err(store_diagnostic)?
            .ok_or_else(|| {
                interface_error(
                    DiagnosticClass::Semantic,
                    "package_interface_type_missing",
                    format!("package interface omits exact type object {digest}"),
                )
            })?;
        let object = decode_type_object(&bytes, digest)?;
        validate_interface_type_reference(package, source, &object.form, owners)?;
        for child in object.child_types() {
            pending.push((source, child, depth.saturating_add(1)));
        }
        keys.insert(key);
        objects.entry(digest).or_insert(object);
    }
    Ok((objects, keys))
}

fn validate_interface_type_reference(
    package: PackageId,
    source: OwnerKey,
    form: &TypeForm,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
) -> Result<(), Diagnostic> {
    match form {
        TypeForm::TypeParameter { parameter } => {
            let key = OwnerKey::TypeParameter(*parameter);
            let Some(value) = owners.get(&key) else {
                return Err(interface_error(
                    DiagnosticClass::Semantic,
                    "package_interface_type_parameter_missing",
                    "package interface omits a type parameter used by a public signature",
                ));
            };
            let PackageInterfaceRecord::TypeParameter(parameter) = &value.record else {
                return Err(interface_corrupt(
                    "type-parameter identity has a foreign kind",
                ));
            };
            if semantic_declaration(source, owners) != Some(parameter.declaration) {
                return Err(interface_error(
                    DiagnosticClass::Semantic,
                    "package_interface_type_parameter_scope",
                    "public signature uses a type parameter outside its declaration",
                ));
            }
        }
        TypeForm::Named { declaration } if declaration.package == package => {
            let key = OwnerKey::Declaration(declaration.declaration);
            let Some(value) = owners.get(&key) else {
                return Err(interface_error(
                    DiagnosticClass::Semantic,
                    "package_interface_named_type_missing",
                    "public signature names a local declaration absent from the package interface",
                ));
            };
            if !matches!(value.kind(), OwnerKind::Record | OwnerKind::Variant) {
                return Err(interface_error(
                    DiagnosticClass::Semantic,
                    "package_interface_named_type_kind",
                    "public signature names a declaration that is not a record or variant",
                ));
            }
        }
        TypeForm::Named { .. }
        | TypeForm::Unit
        | TypeForm::Bool
        | TypeForm::I64
        | TypeForm::Bytes
        | TypeForm::Text
        | TypeForm::StaticText
        | TypeForm::Secret
        | TypeForm::StructuralRecord { .. }
        | TypeForm::List { .. }
        | TypeForm::Map { .. }
        | TypeForm::Option { .. }
        | TypeForm::Result { .. }
        | TypeForm::Stream { .. }
        | TypeForm::Function { .. } => {}
    }
    Ok(())
}

fn semantic_declaration(
    owner: OwnerKey,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
) -> Option<DeclarationId> {
    match owner {
        OwnerKey::Declaration(declaration) => Some(declaration),
        OwnerKey::TypeParameter(parameter) => {
            match &owners.get(&OwnerKey::TypeParameter(parameter))?.record {
                PackageInterfaceRecord::TypeParameter(record) => Some(record.declaration),
                _ => None,
            }
        }
        OwnerKey::Field(field) => match &owners.get(&OwnerKey::Field(field))?.record {
            PackageInterfaceRecord::Field(record) => Some(record.declaration),
            _ => None,
        },
        OwnerKey::Case(case) => match &owners.get(&OwnerKey::Case(case))?.record {
            PackageInterfaceRecord::Case(record) => Some(record.declaration),
            _ => None,
        },
        OwnerKey::Operation(operation) => {
            match &owners.get(&OwnerKey::Operation(operation))?.record {
                PackageInterfaceRecord::Operation(record) => Some(record.declaration),
                _ => None,
            }
        }
        OwnerKey::Parameter(parameter) => {
            match &owners.get(&OwnerKey::Parameter(parameter))?.record {
                PackageInterfaceRecord::Parameter(record) => match record.parent {
                    ParameterParent::Function(declaration) => Some(declaration),
                    ParameterParent::Operation(operation) => {
                        semantic_declaration(OwnerKey::Operation(operation), owners)
                    }
                },
                _ => None,
            }
        }
        OwnerKey::Requirement(requirement) => {
            match &owners.get(&OwnerKey::Requirement(requirement))?.record {
                PackageInterfaceRecord::Requirement(record) => Some(record.declaration),
                _ => None,
            }
        }
        OwnerKey::Port(port) => match &owners.get(&OwnerKey::Port(port))?.record {
            PackageInterfaceRecord::Port(record) => Some(record.declaration),
            _ => None,
        },
        OwnerKey::Module(_)
        | OwnerKey::Binding(_)
        | OwnerKey::Expression(_)
        | OwnerKey::Target(_)
        | OwnerKey::Documentation(_)
        | OwnerKey::Annotation(_) => None,
    }
}

#[derive(Default)]
struct ReachablePageRecorder {
    pages: BTreeSet<PageDigest>,
}

impl PageStore for ReachablePageRecorder {
    fn read_page(&self, _digest: PageDigest) -> Result<Option<Vec<u8>>, MapError> {
        Ok(None)
    }

    fn write_page(&mut self, digest: PageDigest, bytes: &[u8]) -> Result<PageWrite, MapError> {
        if PageDigest::of(bytes) != digest {
            return Err(MapError {
                class: MapErrorClass::Corrupt,
                code: "package_interface_page_digest",
                message: "reachable interface page changed digest during validation".to_owned(),
            });
        }
        Ok(if self.pages.insert(digest) {
            PageWrite::Inserted
        } else {
            PageWrite::Reused
        })
    }
}

struct EmptyObjectStore;

static EMPTY_OBJECT_STORE: EmptyObjectStore = EmptyObjectStore;

impl ImmutableObjectStore for EmptyObjectStore {
    fn read(
        &self,
        _key: ObjectKey,
        _maximum_bytes: usize,
        _work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        Ok(None)
    }

    fn stage(
        &mut self,
        _key: ObjectKey,
        _bytes: &[u8],
        _work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        Err(StoreError::new(
            StoreErrorClass::Input,
            "package_interface_empty_store_write",
            "detached package-interface base is read-only",
        ))
    }
}

fn interface_corrupt(message: impl Into<String>) -> Diagnostic {
    interface_error(
        DiagnosticClass::Corrupt,
        "package_interface_closure",
        message,
    )
}

fn map_diagnostic(error: MapError) -> Diagnostic {
    let class = match error.class {
        MapErrorClass::Input => DiagnosticClass::Source,
        MapErrorClass::Resource => DiagnosticClass::Resource,
        MapErrorClass::Corrupt => DiagnosticClass::Corrupt,
        MapErrorClass::Store => DiagnosticClass::Infrastructure,
    };
    interface_error(class, error.code, error.message)
}

fn store_diagnostic(error: StoreError) -> Diagnostic {
    let class = match error.class {
        StoreErrorClass::Input => DiagnosticClass::Source,
        StoreErrorClass::Resource => DiagnosticClass::Resource,
        StoreErrorClass::Corrupt => DiagnosticClass::Corrupt,
        StoreErrorClass::Io => DiagnosticClass::Infrastructure,
    };
    interface_error(class, error.code, error.message)
}

pub fn encode_package_interface_binding(digest: PackageInterfaceOwnerDigest) -> Vec<u8> {
    digest.bytes().to_vec()
}

pub fn decode_package_interface_binding(
    bytes: &[u8],
) -> Result<PackageInterfaceOwnerDigest, Diagnostic> {
    let digest = bytes.try_into().map_err(|_| {
        interface_error(
            DiagnosticClass::Corrupt,
            "package_interface_binding_length",
            "package-interface binding has a noncanonical digest length",
        )
    })?;
    Ok(PackageInterfaceOwnerDigest::from_bytes(digest))
}

macro_rules! owner_id {
    ($name:ident, $variant:ident, $ty:ty) => {
        fn $name(owner: OwnerKey) -> Result<$ty, Diagnostic> {
            let OwnerKey::$variant(id) = owner else {
                return Err(interface_error(
                    DiagnosticClass::Corrupt,
                    "package_interface_owner_domain",
                    "canonical owner header uses a foreign stable-identity domain",
                ));
            };
            Ok(id)
        }
    };
}

owner_id!(declaration_id, Declaration, DeclarationId);
owner_id!(type_parameter_id, TypeParameter, TypeParameterId);
owner_id!(field_id, Field, FieldId);
owner_id!(case_id, Case, CaseId);
owner_id!(operation_id, Operation, OperationId);
owner_id!(parameter_id, Parameter, ParameterId);
owner_id!(requirement_id, Requirement, RequirementId);
owner_id!(port_id, Port, PortId);

fn interface_error(
    class: DiagnosticClass,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::kernel::encode_type_object;
    use crate::platform::storage::memory::MemoryPackedStore;
    use crate::platform::witness::rebuild_full_witness;

    fn built_fixture() -> (
        PackageId,
        PackageInterfaceBuild,
        BTreeMap<OwnerKey, PackageInterfaceOwner>,
    ) {
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let witness = rebuild_full_witness(&snapshot).expect("valid witness fixture");
        let selection =
            PackageInterfaceSelection::from_records(snapshot.root.package_id, &snapshot.owners)
                .expect("public selection");
        let mut owners = BTreeMap::new();
        let mut pending = Vec::new();
        for (owner, record) in &snapshot.owners {
            if let Some(interface) =
                PackageInterfaceOwner::project(record, &witness.summaries[owner], &selection)
                    .expect("valid projection")
            {
                pending.extend(interface.type_roots());
                owners.insert(*owner, interface);
            }
        }
        let mut types = BTreeMap::new();
        while let Some(digest) = pending.pop() {
            if types.contains_key(&digest) {
                continue;
            }
            let object = snapshot.types.get(&digest).expect("reachable fixture type");
            pending.extend(object.child_types());
            let (encoded, bytes) = encode_type_object(object).expect("canonical fixture type");
            assert_eq!(encoded, digest);
            types.insert(digest, bytes);
        }
        let build = build_package_interface(&owners, &types).expect("interface closure build");
        (snapshot.root.package_id, build, owners)
    }

    #[test]
    fn interface_projection_is_public_exact_and_implementation_free() {
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let witness = rebuild_full_witness(&snapshot).expect("valid witness fixture");
        let selection =
            PackageInterfaceSelection::from_records(snapshot.root.package_id, &snapshot.owners)
                .expect("public selection");
        let mut projected = BTreeMap::new();
        for (owner, record) in &snapshot.owners {
            if let Some(interface) = PackageInterfaceOwner::project(
                record,
                witness.summaries.get(owner).expect("owner summary"),
                &selection,
            )
            .expect("valid package-interface projection")
            {
                let (digest, bytes) = interface.encode().expect("interface encoding");
                assert_eq!(
                    PackageInterfaceOwner::decode(&bytes, *owner, digest).unwrap(),
                    interface
                );
                projected.insert(*owner, interface);
            }
        }

        assert!(
            projected
                .values()
                .any(|value| matches!(value.record, PackageInterfaceRecord::Declaration(_)))
        );
        assert!(
            projected
                .values()
                .any(|value| matches!(value.record, PackageInterfaceRecord::Field(_)))
        );
        assert!(
            projected
                .values()
                .any(|value| matches!(value.record, PackageInterfaceRecord::Operation(_)))
        );
        assert!(projected.keys().all(|owner| !matches!(
            owner,
            OwnerKey::Module(_)
                | OwnerKey::Expression(_)
                | OwnerKey::Binding(_)
                | OwnerKey::Target(_)
                | OwnerKey::Documentation(_)
                | OwnerKey::Annotation(_)
        )));
        assert!(projected.values().all(|value| {
            !matches!(value.record, PackageInterfaceRecord::Port(_)) || value.encode().is_ok()
        }));
    }

    #[test]
    fn body_identity_does_not_enter_function_interface_bytes() {
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let witness = rebuild_full_witness(&snapshot).expect("valid witness fixture");
        let selection =
            PackageInterfaceSelection::from_records(snapshot.root.package_id, &snapshot.owners)
                .expect("public selection");
        let (owner, canonical) = snapshot
            .owners
            .iter()
            .find(|(_, record)| {
                matches!(
                    record,
                    OwnerRecord::Declaration(record)
                        if record.visibility == DeclarationVisibility::Public
                            && matches!(record.payload, DeclarationPayload::Function(_))
                )
            })
            .expect("public function fixture");
        let original =
            PackageInterfaceOwner::project(canonical, &witness.summaries[owner], &selection)
                .unwrap()
                .unwrap();
        let mut replacement = canonical.clone();
        let OwnerRecord::Declaration(record) = &mut replacement else {
            unreachable!()
        };
        let DeclarationPayload::Function(function) = &mut record.payload else {
            unreachable!()
        };
        function.body = crate::platform::semantic_id::ExpressionId::migrate(
            b"package-interface-body-replacement",
            1,
        );
        let replacement = PackageInterfaceRecord::project_public(&replacement)
            .unwrap()
            .expect("function remains selected");
        assert_eq!(original.record, replacement);
    }

    #[test]
    fn interface_decode_rejects_wrong_owner_and_predecessor_magic() {
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let witness = rebuild_full_witness(&snapshot).expect("valid witness fixture");
        let selection =
            PackageInterfaceSelection::from_records(snapshot.root.package_id, &snapshot.owners)
                .expect("public selection");
        let (owner, value) = snapshot
            .owners
            .iter()
            .find_map(|(owner, record)| {
                PackageInterfaceOwner::project(record, &witness.summaries[owner], &selection)
                    .transpose()
                    .map(|result| result.map(|value| (*owner, value)))
            })
            .expect("projection result")
            .expect("one exported owner");
        let (digest, bytes) = value.encode().unwrap();
        let foreign = selection
            .owners()
            .find(|candidate| *candidate != owner)
            .expect("second interface owner");
        assert_eq!(
            PackageInterfaceOwner::decode(&bytes, foreign, digest)
                .unwrap_err()
                .code,
            "package_interface_owner_key"
        );
        let mut predecessor = bytes;
        predecessor[..8].copy_from_slice(b"LKJPIF01");
        let predecessor_digest = PackageInterfaceOwnerDigest::of(&predecessor);
        assert!(PackageInterfaceOwner::decode(&predecessor, owner, predecessor_digest).is_err());
    }

    #[test]
    fn detached_interface_map_contains_exactly_its_reachable_owner_and_type_closure() {
        let (package, build, owners) = built_fixture();
        let mut store = MemoryPackedStore::default();
        let mut work = StoreWork::default();
        for (key, bytes) in &build.objects {
            store.stage(*key, bytes, &mut work).unwrap();
        }
        let validation =
            validate_package_interface(package, build.root, &store, &mut work).unwrap();
        assert_eq!(validation.owners, owners);
        assert_eq!(validation.reachable_objects.len(), build.objects.len());
        assert_eq!(
            validation.reachable_objects,
            build.objects.keys().copied().collect()
        );
        assert_eq!(build.owner_count, validation.owners.len() as u64);
        assert_eq!(build.type_count, validation.type_objects.len() as u64);
        assert!(
            build
                .objects
                .keys()
                .all(|key| !matches!(key.domain, ObjectDomain::Owner | ObjectDomain::Blob))
        );
    }

    #[test]
    fn interface_validation_rejects_a_missing_exact_owner_object() {
        let (package, build, _) = built_fixture();
        let missing = build
            .objects
            .keys()
            .find(|key| key.domain == ObjectDomain::PackageInterface)
            .copied()
            .expect("interface owner object");
        let mut store = MemoryPackedStore::default();
        let mut work = StoreWork::default();
        for (key, bytes) in &build.objects {
            if *key != missing {
                store.stage(*key, bytes, &mut work).unwrap();
            }
        }
        assert_eq!(
            validate_package_interface(package, build.root, &store, &mut work)
                .unwrap_err()
                .code,
            "package_interface_owner_missing"
        );
    }
}
