//! Derived, implementation-free package interface records for exact current dependencies.
//!
//! These records are not accepted program authority. They are deterministic projections of one
//! validated package revision. Exact dependency bindings select one storage-independent package
//! revision; its separately replaceable transport commits to a persistent map of these records.

use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DeclarationPayload, DeclarationVisibility, EncodedOwnerKey, ExternalDeclaration,
    FunctionDeclaration, FunctionEffect, OwnerKey, OwnerKind, OwnerRecord, PackageId,
    PackageInterfaceDeclarationPayload, PackageInterfaceDigest, PackageInterfaceRecord,
    ParameterParent, TypeForm, TypeObject, TypeObjectDigest, decode_type_object, encode_owner,
};
use crate::platform::persistent_map::{
    MapAdmission, MapContentRoot, MapError, MapErrorClass, MapRoot, MapWork, MemoryPageStore,
    PageDigest, PageStore, PageWrite, PersistentMap,
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
use crate::platform::witness::OwnerSummary;
use bincode::{Decode, Encode};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub(crate) mod read_views;

pub const PACKAGE_INTERFACE_CONTRACT_IDENTITY: &str = "lkjscript-package-interface-owner";
pub const PACKAGE_INTERFACE_CONTRACT_VERSION: u16 = crate::platform::canonical_schema::SCHEMA_EPOCH;
pub const PACKAGE_INTERFACE_MAGIC: [u8; 8] = *b"LKJPIF22";
pub const PACKAGE_INTERFACE_ENVELOPE_DOMAIN: &str =
    "lkjscript.package-interface-owner-envelope.v22";
const PACKAGE_INTERFACE_IDENTITY_MAGIC: [u8; 8] = *b"LKJPIFI1";
const PACKAGE_INTERFACE_IDENTITY_DOMAIN: &str = "lkjscript.package-interface-identity.v1";
pub const MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES: usize = 1024 * 1024;
const MAXIMUM_REPRESENTATION_METADATA_BYTES: usize = 64 * 1_048_576;

struct RepresentationBudget {
    bytes: usize,
    maximum: usize,
}

impl Default for RepresentationBudget {
    fn default() -> Self {
        Self {
            bytes: 0,
            maximum: MAXIMUM_REPRESENTATION_METADATA_BYTES,
        }
    }
}

impl RepresentationBudget {
    fn reserve<T>(&mut self, count: usize) -> Result<(), Diagnostic> {
        self.bytes = count
            .checked_mul(std::mem::size_of::<T>() + 4 * std::mem::size_of::<usize>())
            .and_then(|bytes| self.bytes.checked_add(bytes))
            .filter(|bytes| *bytes <= self.maximum)
            .ok_or_else(|| {
                interface_error(
                    DiagnosticClass::Resource,
                    "package_interface_representation_storage",
                    "owned representation metadata exceeds its storage admission",
                )
            })?;
        Ok(())
    }

    fn append_children(
        &mut self,
        object: &TypeObject,
        pending: &mut Vec<TypeObjectDigest>,
    ) -> Result<(), Diagnostic> {
        // Reserve both the temporary child inventory and its queued copy before allocation.
        self.reserve::<TypeObjectDigest>(object.child_type_count())?;
        self.reserve::<TypeObjectDigest>(object.child_type_count())?;
        pending.extend(object.child_types());
        Ok(())
    }

    fn append_types(
        &mut self,
        values: Vec<TypeObjectDigest>,
        pending: &mut Vec<TypeObjectDigest>,
    ) -> Result<(), Diagnostic> {
        self.reserve::<TypeObjectDigest>(values.len())?;
        pending.extend(values);
        Ok(())
    }
}

#[cfg(test)]
#[path = "package_interface_generalized_sequence_tests.rs"]
mod generalized_sequence_tests;

#[cfg(test)]
#[path = "package_interface_owned_scheme_tests.rs"]
mod owned_scheme_tests;

#[cfg(test)]
#[path = "package_interface_owned_record_tests.rs"]
mod owned_record_tests;

#[cfg(test)]
#[path = "package_interface_mixed_storage_tests.rs"]
mod mixed_storage_tests;

#[cfg(test)]
#[path = "package_interface_associated_authority_tests.rs"]
mod associated_authority_tests;

#[cfg(test)]
#[path = "package_interface_read_view_tests.rs"]
mod read_view_tests;

#[cfg(test)]
#[path = "package_interface_work_admission_tests.rs"]
mod work_admission_tests;

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

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageInterfaceOwner {
    pub contract_version: u16,
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
        let bytes = crate::platform::packed::encode_canonical(
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
        let value: Self = crate::platform::packed::decode_canonical(
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
        Ok(value)
    }

    fn validate_local(&self) -> Result<(), Diagnostic> {
        self.validate_contract()?;
        self.record.validate_local()
    }

    fn validate_contract(&self) -> Result<(), Diagnostic> {
        if self.contract_version != PACKAGE_INTERFACE_CONTRACT_VERSION {
            return Err(interface_error(
                DiagnosticClass::Source,
                "package_interface_contract",
                "package-interface owner uses a foreign contract",
            ));
        }
        if self.record.header().contract_version
            != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
        {
            return Err(interface_corrupt(
                "package-interface owner uses a foreign graph contract",
            ));
        }
        Ok(())
    }
}

#[derive(Encode)]
struct PackageInterfaceIdentity {
    contract_version: u16,
    graph_contract_version: u16,
    package: PackageId,
    owners: MapContentRoot,
}

pub fn package_interface_digest(
    package: PackageId,
    owners: MapContentRoot,
) -> Result<PackageInterfaceDigest, Diagnostic> {
    package_interface_digest_for_graph(
        package,
        owners,
        crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
    )
}

pub fn package_interface_digest_for_graph(
    package: PackageId,
    owners: MapContentRoot,
    graph_contract_version: u16,
) -> Result<PackageInterfaceDigest, Diagnostic> {
    if graph_contract_version != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION {
        return Err(interface_error(
            DiagnosticClass::Source,
            "package_interface_graph_contract",
            "unsupported interface graph generation",
        ));
    }
    let identity = PackageInterfaceIdentity {
        contract_version: crate::platform::canonical_schema::SCHEMA_EPOCH,
        graph_contract_version,
        package,
        owners,
    };
    let bytes = crate::platform::packed::encode_canonical(
        PACKAGE_INTERFACE_IDENTITY_MAGIC,
        PACKAGE_INTERFACE_IDENTITY_DOMAIN,
        &identity,
        1024,
    )?;
    Ok(PackageInterfaceDigest::of(&bytes))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageInterfaceSelection {
    package: PackageId,
    declarations: BTreeSet<DeclarationId>,
    type_parameters: BTreeSet<TypeParameterId>,
    effect_parameters: BTreeSet<crate::platform::semantic_id::EffectParameterId>,
    requirement_parameters: BTreeSet<crate::platform::semantic_id::RequirementParameterId>,
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
            effect_parameters: BTreeSet::new(),
            requirement_parameters: BTreeSet::new(),
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
            OwnerKey::EffectParameter(id) => self.effect_parameters.contains(&id),
            OwnerKey::RequirementParameter(id) => self.requirement_parameters.contains(&id),
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
            | OwnerKey::HttpRoute(_)
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
        self.type_parameters
            .extend(record.payload.type_parameters().iter().copied());
        match &record.payload {
            DeclarationPayload::OwnedContract(c) => {
                self.type_parameters.insert(c.self_parameter);
                for method in &c.methods {
                    self.requirements.extend(
                        method
                            .effect
                            .row()
                            .requirements
                            .iter()
                            .filter_map(|requirement| requirement.concrete())
                            .filter(|requirement| requirement.package == self.package)
                            .map(|requirement| requirement.requirement),
                    );
                }
            }
            DeclarationPayload::OwnedImplementation(i) => {
                self.type_parameters
                    .extend(i.type_parameters.iter().copied());
            }
            DeclarationPayload::ReadView(_) => {}
            DeclarationPayload::Record { fields, .. }
            | DeclarationPayload::OwnedRecord { fields, .. } => self.fields.extend(fields),
            DeclarationPayload::Variant { cases, .. } => self.cases.extend(cases),
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
                implementation_parameters: _,
                effect_parameters,
                requirement_parameters,
                type_parameters,
                parameters,
                effect,
                ..
            }) => {
                self.effect_parameters.extend(effect_parameters);
                self.requirement_parameters.extend(requirement_parameters);
                self.type_parameters.extend(type_parameters);
                self.parameters.extend(parameters);
                if let FunctionEffect::Task {
                    effect_parameters: _,
                    requirements,
                } = effect
                {
                    self.requirements.extend(
                        requirements
                            .iter()
                            .filter_map(|requirement| requirement.concrete())
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

    /// Extend only the representation closure rooted in exported owned records.
    /// Ordinary public signatures still require public nominal references.
    pub(crate) fn observe_owned_representations(
        &mut self,
        records: &BTreeMap<OwnerKey, OwnerRecord>,
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
        dependency_types: &BTreeMap<TypeObjectDigest, TypeObject>,
    ) -> Result<(), Diagnostic> {
        self.observe_owned_representations_admitted(
            records,
            types,
            dependency_types,
            &mut RepresentationBudget::default(),
        )
    }

    fn observe_owned_representations_admitted(
        &mut self,
        records: &BTreeMap<OwnerKey, OwnerRecord>,
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
        dependency_types: &BTreeMap<TypeObjectDigest, TypeObject>,
        budget: &mut RepresentationBudget,
    ) -> Result<(), Diagnostic> {
        self.observe_owned_representations_indexed_admitted(
            |key| Ok(records.get(&key).cloned()),
            |digest| {
                Ok(types
                    .get(&digest)
                    .or_else(|| dependency_types.get(&digest))
                    .cloned())
            },
            budget,
        )
    }

    /// Read only the exact owner/type closure needed by exported owned layouts.
    /// Indexed producers use the same selection and metadata admission as source reconstruction.
    pub(crate) fn observe_owned_representations_indexed(
        &mut self,
        read_owner: impl FnMut(OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic>,
        read_type: impl FnMut(TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic>,
    ) -> Result<(), Diagnostic> {
        self.observe_owned_representations_indexed_admitted(
            read_owner,
            read_type,
            &mut RepresentationBudget::default(),
        )
    }

    fn observe_owned_representations_indexed_admitted(
        &mut self,
        mut read_owner: impl FnMut(OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic>,
        mut read_type: impl FnMut(TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic>,
        budget: &mut RepresentationBudget,
    ) -> Result<(), Diagnostic> {
        let mut pending = Vec::new();
        for declaration in &self.declarations {
            budget.reserve::<OwnerRecord>(1)?;
            if let Some(owner @ OwnerRecord::Declaration(_)) =
                read_owner(OwnerKey::Declaration(*declaration))?
            {
                for ty in owner.type_roots() {
                    budget.reserve::<TypeObjectDigest>(1)?;
                    pending.push(ty);
                }
                if let OwnerRecord::Declaration(record) = &owner
                    && let DeclarationPayload::OwnedRecord { fields, .. } = &record.payload
                {
                    for field in fields {
                        budget.reserve::<OwnerRecord>(1)?;
                        match read_owner(OwnerKey::Field(*field))? {
                            Some(OwnerRecord::Field(field)) => {
                                budget.reserve::<TypeObjectDigest>(1)?;
                                pending.push(field.ty);
                            }
                            _ => {
                                return Err(interface_corrupt(
                                    "owned representation field is absent",
                                ));
                            }
                        }
                    }
                }
            }
        }
        let mut visited = BTreeSet::new();
        let mut observed_declarations = BTreeSet::new();
        while let Some(digest) = pending.pop() {
            if visited.contains(&digest) {
                continue;
            }
            budget.reserve::<TypeObjectDigest>(1)?;
            visited.insert(digest);
            budget.reserve::<TypeObject>(1)?;
            let object = read_type(digest)?
                .ok_or_else(|| interface_corrupt("owned representation type is absent"))?;
            budget.append_children(&object, &mut pending)?;
            let reference = match object.form {
                TypeForm::Named { declaration }
                | TypeForm::Applied { declaration, .. }
                | TypeForm::ReadViewApplied { declaration, .. }
                    if declaration.package == self.package =>
                {
                    declaration
                }
                _ => continue,
            };
            if observed_declarations.contains(&reference.declaration) {
                continue;
            }
            budget.reserve::<DeclarationId>(1)?;
            observed_declarations.insert(reference.declaration);
            budget.reserve::<OwnerRecord>(1)?;
            let Some(OwnerRecord::Declaration(record)) =
                read_owner(OwnerKey::Declaration(reference.declaration))?
            else {
                return Err(interface_corrupt(
                    "owned representation nominal declaration is absent",
                ));
            };
            if !self.declarations.contains(&reference.declaration) {
                budget.reserve::<DeclarationId>(1)?;
                self.declarations.insert(reference.declaration);
            }
            for parameter in record.payload.type_parameters().iter() {
                if !self.type_parameters.contains(parameter) {
                    budget.reserve::<TypeParameterId>(1)?;
                    self.type_parameters.insert(*parameter);
                }
            }
            match &record.payload {
                DeclarationPayload::ReadView(view) => {
                    budget.reserve::<TypeObjectDigest>(view.fields.len())?;
                    pending.extend(view.fields.iter().map(|field| field.ty));
                }
                DeclarationPayload::OwnedRecord { fields, .. }
                | DeclarationPayload::Record { fields, .. } => {
                    for field in fields {
                        if !self.fields.contains(field) {
                            budget.reserve::<FieldId>(1)?;
                            self.fields.insert(*field);
                        }
                        budget.reserve::<OwnerRecord>(1)?;
                        match read_owner(OwnerKey::Field(*field))? {
                            Some(OwnerRecord::Field(field)) => {
                                budget.reserve::<TypeObjectDigest>(1)?;
                                pending.push(field.ty);
                            }
                            _ => {
                                return Err(interface_corrupt(
                                    "owned representation field is absent",
                                ));
                            }
                        }
                    }
                }
                DeclarationPayload::Variant { cases, .. } => {
                    for case in cases {
                        if !self.cases.contains(case) {
                            budget.reserve::<CaseId>(1)?;
                            self.cases.insert(*case);
                        }
                        budget.reserve::<OwnerRecord>(1)?;
                        match read_owner(OwnerKey::Case(*case))? {
                            Some(OwnerRecord::Case(case)) => {
                                budget.reserve::<TypeObjectDigest>(usize::from(
                                    case.payload.is_some(),
                                ))?;
                                pending.extend(case.payload);
                            }
                            _ => {
                                return Err(interface_corrupt(
                                    "owned representation case is absent",
                                ));
                            }
                        }
                    }
                }
                _ => {
                    return Err(interface_corrupt(
                        "owned representation names a non-layout declaration",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Concrete requirements in callable signatures are public descriptive meaning even when
    /// the enclosing function is pure. Selecting their records does not expose their component.
    pub fn observe_type(&mut self, form: &TypeForm) {
        if let TypeForm::TaskFunction { effect, .. } = form {
            self.requirements.extend(
                effect
                    .requirements
                    .iter()
                    .filter_map(|reference| reference.concrete())
                    .filter(|reference| reference.package == self.package)
                    .map(|reference| reference.requirement),
            );
        }
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
            .chain(
                self.requirement_parameters
                    .iter()
                    .copied()
                    .map(OwnerKey::RequirementParameter),
            )
            .chain(self.fields.iter().copied().map(OwnerKey::Field))
            .chain(self.cases.iter().copied().map(OwnerKey::Case))
            .chain(self.operations.iter().copied().map(OwnerKey::Operation))
            .chain(self.parameters.iter().copied().map(OwnerKey::Parameter))
            .chain(self.requirements.iter().copied().map(OwnerKey::Requirement))
            .chain(self.ports.iter().copied().map(OwnerKey::Port))
            .chain(
                self.effect_parameters
                    .iter()
                    .copied()
                    .map(OwnerKey::EffectParameter),
            )
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
    build_package_interface_with_physical_target(owners, types, None)
}

#[cfg(test)]
pub(crate) fn build_package_interface_with_leaf_target(
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    types: &BTreeMap<TypeObjectDigest, Vec<u8>>,
    target_leaf_bytes: usize,
) -> Result<PackageInterfaceBuild, Diagnostic> {
    build_package_interface_with_physical_target(owners, types, Some(target_leaf_bytes))
}

fn build_package_interface_with_physical_target(
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    types: &BTreeMap<TypeObjectDigest, Vec<u8>>,
    target_leaf_bytes: Option<usize>,
) -> Result<PackageInterfaceBuild, Diagnostic> {
    let mut page_store = MemoryPageStore::default();
    let mut map_work = MapWork::default();
    let mut entries = Vec::with_capacity(owners.len());
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
        entries.push((
            EncodedOwnerKey::new(*owner).bytes().to_vec(),
            encode_package_interface_binding(digest),
        ));
        owner_bytes.push((digest, bytes));
    }
    let map = if let Some(target_leaf_bytes) = target_leaf_bytes {
        PersistentMap::from_sorted_with_leaf_target(
            &mut page_store,
            entries,
            target_leaf_bytes,
            &mut map_work,
        )
        .map_err(map_diagnostic)?
    } else {
        PersistentMap::from_sorted(&mut page_store, entries, &mut map_work)
            .map_err(map_diagnostic)?
    };

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
/// This structural result alone grants no readiness. Installed imports first admit every
/// immutable canonical body and independently reproduce this projection in `source::admit`;
/// artifact loaders verify projection coherence and their complete retained canonical meaning.
/// Field eligibility and all instantiated generic constraints belong to those semantic passes.
pub fn validate_package_interface<S: ImmutableObjectStore + ?Sized>(
    package: PackageId,
    root: MapRoot,
    store: &S,
    work: &mut StoreWork,
) -> Result<PackageInterfaceValidation, Diagnostic> {
    validate_package_interface_admitted(package, root, store, work, &mut MapAdmission::unbounded())
}

pub(crate) fn validate_package_interface_admitted<S: ImmutableObjectStore + ?Sized>(
    package: PackageId,
    root: MapRoot,
    store: &S,
    work: &mut StoreWork,
    admission: &mut MapAdmission,
) -> Result<PackageInterfaceValidation, Diagnostic> {
    validate_package_interface_metered(package, root, store, work, admission, &mut |_| Ok(()))
}

pub(crate) fn validate_package_interface_metered<S: ImmutableObjectStore + ?Sized>(
    package: PackageId,
    root: MapRoot,
    store: &S,
    work: &mut StoreWork,
    admission: &mut MapAdmission,
    visit: &mut dyn FnMut(u64) -> Result<(), Diagnostic>,
) -> Result<PackageInterfaceValidation, Diagnostic> {
    let map = PersistentMap::from_root(root);
    let object_reader = ObjectPageReader::new(store);
    let reader = ReachablePageReader::new(&object_reader);
    let mut map_work = MapWork::with_admission(*admission);
    let mut bindings = Vec::with_capacity(
        usize::try_from(root.entries())
            .unwrap_or(usize::MAX)
            .min(64),
    );
    let read_result = map.for_each(&reader, &mut map_work, |key, value| {
        bindings.push((key.to_vec(), value.to_vec()));
        Ok(())
    });
    *admission = map_work.remaining_admission();
    read_result.map_err(map_diagnostic)?;
    work.add(object_reader.work());

    let mut reachable_objects = reader
        .into_pages()
        .into_iter()
        .map(|digest| ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes()))
        .collect::<BTreeSet<_>>();
    let mut owners = BTreeMap::new();
    for (key_bytes, binding_bytes) in bindings {
        let owner = EncodedOwnerKey::decode(&key_bytes)?;
        let digest = decode_package_interface_binding(&binding_bytes)?;
        let key = ObjectKey::from_digest(ObjectDomain::PackageInterface, digest.bytes());
        let bytes = store
            .read(key, Some(MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES), work)
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
    // Reserve owner/child-relation validation visits before traversing retained interface records.
    // Canonical byte reads are charged separately by the caller's immutable source admission.
    for owner in owners.values() {
        visit(interface_owner_validation_visits(owner))?;
    }
    let mut associated_admission = AssociatedInterfaceAdmission::new(visit);
    let (type_objects, type_keys) =
        validate_type_closure(package, &owners, store, work, &mut associated_admission)?;
    for object in type_objects.values() {
        (associated_admission.visit)(1)?;
        if let TypeForm::TaskFunction { effect, .. } = &object.form {
            (associated_admission.visit)(effect.requirements.len() as u64)?;
        }
    }
    validate_owner_closure_metered(package, &owners, &type_objects, &mut associated_admission)?;
    reachable_objects.extend(type_keys);
    Ok(PackageInterfaceValidation {
        owners,
        type_objects,
        reachable_objects,
        map_work,
    })
}

pub(crate) fn interface_owner_validation_visits(owner: &PackageInterfaceOwner) -> u64 {
    let children = match &owner.record {
        PackageInterfaceRecord::Declaration(declaration)
        | PackageInterfaceRecord::RepresentationDeclaration(declaration) => {
            match &declaration.payload {
                PackageInterfaceDeclarationPayload::OwnedContract(c) => {
                    c.associated_types.len()
                        + c.methods
                            .iter()
                            .map(|m| {
                                let row = m.effect.row();
                                m.parameters.len()
                                    + row.requirements.len()
                                    + row.parameters.len()
                                    + 1
                            })
                            .sum::<usize>()
                        + c.type_parameters.len()
                        + 1
                }
                PackageInterfaceDeclarationPayload::OwnedImplementation(i) => {
                    i.associated_types.len()
                        + i.methods.len()
                        + i.type_arguments.len()
                        + i.type_parameters.len()
                        + i.implementation_parameters.len()
                        + i.implementation_parameters
                            .iter()
                            .map(|p| 1 + p.type_arguments.len())
                            .sum::<usize>()
                        + i.methods
                            .iter()
                            .map(|m| {
                                m.type_arguments.len()
                                    + m.implementations
                                        .iter()
                                        .flat_map(|operand| operand.walk())
                                        .map(|operand| {
                                            match operand {
                                    crate::platform::kernel::ImplementationOperand::Concrete {
                                        type_arguments,
                                        ..
                                    } => 1 + type_arguments.len(),
                                    crate::platform::kernel::ImplementationOperand::Parameter {
                                        ..
                                    } => 1,
                                }
                                        })
                                        .sum::<usize>()
                            })
                            .sum::<usize>()
                        + 1
                }
                PackageInterfaceDeclarationPayload::ReadView(view) => {
                    view.type_parameters.len() + view.fields.len() + 1
                }
                PackageInterfaceDeclarationPayload::Record {
                    fields,
                    type_parameters,
                }
                | PackageInterfaceDeclarationPayload::OwnedRecord {
                    fields,
                    type_parameters,
                } => fields.len().saturating_add(type_parameters.len()),
                PackageInterfaceDeclarationPayload::Variant {
                    cases,
                    type_parameters,
                } => cases.len().saturating_add(type_parameters.len()),
                PackageInterfaceDeclarationPayload::Interface { operations } => operations.len(),
                PackageInterfaceDeclarationPayload::Function(function) => {
                    function.parameters.len()
                        + function.implementation_parameters.len()
                        + function
                            .implementation_parameters
                            .iter()
                            .map(|p| p.type_arguments.len())
                            .sum::<usize>()
                        + function.type_parameters.len()
                        + function.requirement_parameters.len()
                        + function.effect_parameters.len()
                        + function.read_scope_parameters.len()
                        + function.read_scope_sources.len()
                        + match &function.effect {
                            FunctionEffect::Pure => 0,
                            FunctionEffect::Task {
                                effect_parameters: _,
                                requirements,
                            } => requirements.len(),
                        }
                }
                PackageInterfaceDeclarationPayload::External(function) => {
                    function.parameters.len() + function.type_parameters.len()
                }
                PackageInterfaceDeclarationPayload::Component {
                    requirements,
                    ports,
                } => requirements.len() + ports.len(),
                PackageInterfaceDeclarationPayload::Constant { .. } => 0,
            }
        }
        PackageInterfaceRecord::Operation(operation) => operation.parameters.len(),
        PackageInterfaceRecord::Requirement(requirement) => 1 + requirement.operations.len(),
        PackageInterfaceRecord::RequirementParameter(parameter) => {
            1 + parameter.constraint.operations.len()
        }
        _ => 0,
    };
    1 + children as u64
}

#[cfg(test)]
fn validate_owner_closure(
    package: PackageId,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
) -> Result<(), Diagnostic> {
    validate_owner_closure_metered(
        package,
        owners,
        types,
        &mut AssociatedInterfaceAdmission::new(&mut |_| Ok(())),
    )
}

fn validate_owner_closure_metered(
    package: PackageId,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    associated_admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<(), Diagnostic> {
    for value in owners.values() {
        associated_admission.step()?;
        value.validate_contract()?;
    }
    let mut expected = owners
        .iter()
        .filter_map(|(owner, value)| {
            matches!(value.record, PackageInterfaceRecord::Declaration(_)).then_some(*owner)
        })
        .collect::<BTreeSet<_>>();
    expected.extend(representation_declarations(
        package,
        owners,
        types,
        &mut *associated_admission.visit,
    )?);
    for (owner, value) in owners {
        let declaration = match &value.record {
            PackageInterfaceRecord::Declaration(declaration)
            | PackageInterfaceRecord::RepresentationDeclaration(declaration) => declaration,
            _ => continue,
        };
        let OwnerKey::Declaration(declaration_id) = owner else {
            return Err(interface_corrupt(
                "declaration has a foreign owner identity",
            ));
        };
        match &declaration.payload {
            PackageInterfaceDeclarationPayload::ReadView(view) => {
                for parameter in &view.type_parameters {
                    require_child(
                        owners,
                        &mut expected,
                        OwnerKey::TypeParameter(*parameter),
                        OwnerKind::TypeParameter,
                        Some(*declaration_id),
                    )?;
                }
            }
            PackageInterfaceDeclarationPayload::OwnedContract(c) => {
                let mut parameter_names = BTreeSet::new();
                for parameter in std::iter::once(&c.self_parameter).chain(c.type_parameters.iter())
                {
                    require_child(
                        owners,
                        &mut expected,
                        OwnerKey::TypeParameter(*parameter),
                        OwnerKind::TypeParameter,
                        Some(*declaration_id),
                    )?;
                    let Some(PackageInterfaceOwner {
                        record: PackageInterfaceRecord::TypeParameter(p),
                        ..
                    }) = owners.get(&OwnerKey::TypeParameter(*parameter))
                    else {
                        return Err(interface_corrupt("owned contract parameter is absent"));
                    };
                    let self_parameter = *parameter == c.self_parameter;
                    let constraints_valid = if self_parameter {
                        p.constraints == crate::platform::kernel::TypeParameterConstraints::Owned
                    } else {
                        matches!(p.constraints,
                            crate::platform::kernel::TypeParameterConstraints::Owned
                                | crate::platform::kernel::TypeParameterConstraints::Transferable
                                | crate::platform::kernel::TypeParameterConstraints::CaptureSafeTransferable)
                    };
                    if !constraints_valid || !parameter_names.insert(&p.name) {
                        return Err(interface_corrupt(
                            "owned contract parameters require distinct names, exact Owned Self, and Owned or ordinary Transferable additional bounds",
                        ));
                    }
                }
                for method in &c.methods {
                    validate_interface_method_borrow_result(
                        package,
                        *owner,
                        method,
                        types,
                        owners,
                        associated_admission,
                    )?;
                    let row = method.effect.row();
                    row.validate()?;
                    if !row.is_closed() {
                        return Err(interface_corrupt("owned method effect row is not closed"));
                    }
                    for requirement in row.requirements {
                        if requirement.package() == package {
                            require_child(
                                owners,
                                &mut expected,
                                requirement.owner(),
                                OwnerKind::Requirement,
                                None,
                            )?;
                        }
                    }
                }
            }
            PackageInterfaceDeclarationPayload::OwnedImplementation(i) => {
                for parameter in &i.type_parameters {
                    require_child(
                        owners,
                        &mut expected,
                        OwnerKey::TypeParameter(*parameter),
                        OwnerKind::TypeParameter,
                        Some(*declaration_id),
                    )?;
                    let Some(parameter_owner) = owners.get(&OwnerKey::TypeParameter(*parameter))
                    else {
                        return Err(interface_corrupt(
                            "owned implementation type parameter is absent",
                        ));
                    };
                    let PackageInterfaceRecord::TypeParameter(p) = &parameter_owner.record else {
                        return Err(interface_corrupt(
                            "owned implementation type parameter has a foreign kind",
                        ));
                    };
                    if !(p.constraints.has_owned() || matches!(p.constraints,
                        crate::platform::kernel::TypeParameterConstraints::Transferable
                            | crate::platform::kernel::TypeParameterConstraints::CaptureSafeTransferable)) {
                        return Err(interface_corrupt(
                            "owned implementation type parameter requires Owned or ordinary Transferable bounds",
                        ));
                    }
                }
                validate_interface_implementation_parameters(
                    *declaration_id,
                    &i.type_parameters,
                    &i.implementation_parameters,
                    types,
                    owners,
                )?;
                let scope = crate::platform::kernel::DeclarationReference {
                    package,
                    declaration: *declaration_id,
                };
                let parameters = i
                    .implementation_parameters
                    .iter()
                    .map(|p| p.id)
                    .collect::<BTreeSet<_>>();
                for method in &i.methods {
                    for operand in &method.implementations {
                        operand.validate_local()?;
                        for operand in operand.walk() {
                            if let crate::platform::kernel::ImplementationOperand::Parameter {
                                scope: actual,
                                parameter,
                            } = operand
                                && (*actual != scope || !parameters.contains(parameter))
                            {
                                return Err(interface_corrupt(
                                    "mapped witness parameter is outside its exact implementing declaration",
                                ));
                            }
                        }
                    }
                }
            }
            PackageInterfaceDeclarationPayload::Record {
                fields,
                type_parameters,
            }
            | PackageInterfaceDeclarationPayload::OwnedRecord {
                fields,
                type_parameters,
            } => {
                let owned = matches!(
                    declaration.payload,
                    PackageInterfaceDeclarationPayload::OwnedRecord { .. }
                );
                let mut parameter_names = BTreeSet::new();
                for parameter in type_parameters {
                    require_child(
                        owners,
                        &mut expected,
                        OwnerKey::TypeParameter(*parameter),
                        OwnerKind::TypeParameter,
                        Some(*declaration_id),
                    )?;
                    if owned {
                        let Some(PackageInterfaceOwner {
                            record: PackageInterfaceRecord::TypeParameter(p),
                            ..
                        }) = owners.get(&OwnerKey::TypeParameter(*parameter))
                        else {
                            return Err(interface_corrupt("owned record parameter is absent"));
                        };
                        if !(p.constraints.has_owned() || p.constraints.requires_transfer())
                            || !parameter_names.insert(&p.name)
                        {
                            return Err(interface_error(
                                DiagnosticClass::Semantic,
                                "package_interface_owned_record_parameter",
                                "owned record parameters require distinct names and Owned or ordinary Transferable bounds",
                            ));
                        }
                    }
                }
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
            PackageInterfaceDeclarationPayload::Variant {
                cases,
                type_parameters,
            } => {
                for parameter in type_parameters {
                    require_child(
                        owners,
                        &mut expected,
                        OwnerKey::TypeParameter(*parameter),
                        OwnerKind::TypeParameter,
                        Some(*declaration_id),
                    )?;
                }
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
                validate_interface_implementation_parameters(
                    *declaration_id,
                    &signature.type_parameters,
                    &signature.implementation_parameters,
                    types,
                    owners,
                )?;
                validate_interface_function_borrow_result(
                    package,
                    *declaration_id,
                    signature,
                    types,
                    owners,
                    associated_admission,
                )?;
                for parameter in &signature.requirement_parameters {
                    require_child(
                        owners,
                        &mut expected,
                        OwnerKey::RequirementParameter(*parameter),
                        OwnerKind::RequirementParameter,
                        Some(*declaration_id),
                    )?;
                }
                for parameter in &signature.effect_parameters {
                    require_child(
                        owners,
                        &mut expected,
                        OwnerKey::EffectParameter(*parameter),
                        OwnerKind::EffectParameter,
                        Some(*declaration_id),
                    )?;
                }
                require_signature_children(
                    owners,
                    &mut expected,
                    *declaration_id,
                    &signature.type_parameters,
                    &signature.parameters,
                )?;
                if let FunctionEffect::Task {
                    effect_parameters,
                    requirements,
                } = &signature.effect
                {
                    for parameter in effect_parameters {
                        if parameter.package != package
                            || !signature.effect_parameters.contains(&parameter.parameter)
                        {
                            return Err(interface_error(
                                DiagnosticClass::Semantic,
                                "package_interface_effect_scope",
                                "function row uses a foreign effect parameter",
                            ));
                        }
                    }
                    for requirement in requirements {
                        if let crate::platform::kernel::RequirementOperand::Parameter(parameter) =
                            requirement
                            && (parameter.package != package
                                || !signature
                                    .requirement_parameters
                                    .contains(&parameter.parameter))
                        {
                            return Err(interface_error(
                                DiagnosticClass::Semantic,
                                "package_interface_requirement_scope",
                                "function row uses a foreign requirement parameter",
                            ));
                        }
                        if requirement.package() == package {
                            require_child(
                                owners,
                                &mut expected,
                                requirement.owner(),
                                if requirement.concrete().is_some() {
                                    OwnerKind::Requirement
                                } else {
                                    OwnerKind::RequirementParameter
                                },
                                requirement.concrete().is_none().then_some(*declaration_id),
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
        // Admit exact signature children and legacy source-tied results before the complete
        // read-view pass. A missing child is never replaced by a partial scope/type proof.
        read_views::validate_declaration_metered(
            package,
            *owner,
            &declaration.payload,
            owners,
            types,
            &mut *associated_admission.visit,
        )?;
    }
    // A public signature may describe a concrete callback requirement without publishing the
    // component or function that owns its declaration. The type closure independently proves
    // this descriptive edge; it does not establish a deployment grant.
    for object in types.values() {
        if let TypeForm::TaskFunction { effect, .. } = &object.form {
            for requirement in &effect.requirements {
                if requirement.package() == package {
                    require_child(
                        owners,
                        &mut expected,
                        requirement.owner(),
                        if requirement.concrete().is_some() {
                            OwnerKind::Requirement
                        } else {
                            OwnerKind::RequirementParameter
                        },
                        None,
                    )?;
                }
            }
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

fn validate_interface_implementation_parameters(
    declaration: DeclarationId,
    type_parameters: &[TypeParameterId],
    parameters: &[crate::platform::kernel::ImplementationParameter],
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
) -> Result<(), Diagnostic> {
    let type_parameters = type_parameters.iter().copied().collect::<BTreeSet<_>>();
    for parameter in parameters {
        let Some(TypeObject {
            form: TypeForm::TypeParameter {
                parameter: self_parameter,
            },
            ..
        }) = types.get(&parameter.self_type)
        else {
            return Err(interface_corrupt(
                "implementation prerequisite Self must be an exact Owned type parameter",
            ));
        };
        if !type_parameters.contains(self_parameter)
            || !matches!(owners.get(&OwnerKey::TypeParameter(*self_parameter)),
                Some(PackageInterfaceOwner { record: PackageInterfaceRecord::TypeParameter(p), .. })
                    if p.declaration == declaration && p.constraints.has_owned())
        {
            return Err(interface_corrupt(
                "implementation prerequisite Self is outside its exact Owned declaration scope",
            ));
        }
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

fn interface_owned_type(
    package: PackageId,
    ty: TypeObjectDigest,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
) -> bool {
    match types.get(&ty).map(|object| &object.form) {
        Some(
            TypeForm::RegionCap {
                mode:
                    crate::platform::kernel::RegionMode::Unique
                    | crate::platform::kernel::RegionMode::Frozen
                    | crate::platform::kernel::RegionMode::Pinned,
                ..
            }
            | TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. }
            | TypeForm::OwnedSequence { .. },
        ) => true,
        Some(TypeForm::TypeParameter { parameter }) => matches!(
            owners.get(&OwnerKey::TypeParameter(*parameter)),
            Some(PackageInterfaceOwner { record: PackageInterfaceRecord::TypeParameter(p), .. })
                if p.constraints.has_owned()
        ),
        Some(TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. })
            if declaration.package == package =>
        {
            matches!(
                owners.get(&OwnerKey::Declaration(declaration.declaration)).map(|owner| &owner.record),
                Some(PackageInterfaceRecord::Declaration(record) | PackageInterfaceRecord::RepresentationDeclaration(record))
                    if matches!(record.payload, PackageInterfaceDeclarationPayload::OwnedRecord { .. })
            )
        }
        _ => false,
    }
}

/// Isolated Borrow requirements retain foreign obligations without classifying them as Owned.
/// The complete exact interface closure rechecks their retained owner/type occurrences before
/// readiness. This result grants no transfer, sharing, capture or executable source authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InterfaceBorrowRequirementProof {
    Owned,
    ReadView,
    DeferredReadView(crate::platform::kernel::DeclarationReference),
    DeferredNominal(crate::platform::kernel::DeclarationReference),
    DeferredAssociated(crate::platform::kernel::DeclarationReference),
}

impl InterfaceBorrowRequirementProof {
    fn retain(self, package: PackageId) -> Result<Self, Diagnostic> {
        match self {
            Self::DeferredNominal(reference)
            | Self::DeferredAssociated(reference)
            | Self::DeferredReadView(reference)
                if reference.package == package =>
            {
                Err(associated_interface_error(
                    "an isolated Borrow requirement cannot defer local declaration authority",
                ))
            }
            _ => Ok(self),
        }
    }
}

fn interface_borrow_requirement_at(
    package: PackageId,
    source: OwnerKey,
    ty: TypeObjectDigest,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<Option<InterfaceBorrowRequirementProof>, Diagnostic> {
    admission.checkpoint()?;
    let Some(object) = types.get(&ty) else {
        return Ok(None);
    };
    if object.form.is_associated() || object.form.is_read_view() {
        return admission
            .validate_form(package, source, &object.form, owners, types, &mut |_| {
                Ok(None)
            })
            .map(|proof| {
                Some(match proof {
                    AssociatedInterfaceProof::Owned => InterfaceBorrowRequirementProof::Owned,
                    AssociatedInterfaceProof::ReadView => InterfaceBorrowRequirementProof::ReadView,
                    AssociatedInterfaceProof::DeferredReadView(reference) => {
                        InterfaceBorrowRequirementProof::DeferredReadView(reference)
                    }
                    AssociatedInterfaceProof::Deferred(contract) => {
                        InterfaceBorrowRequirementProof::DeferredAssociated(contract)
                    }
                })
            });
    }
    if let TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } = object.form
        && declaration.package != package
    {
        return Ok(Some(InterfaceBorrowRequirementProof::DeferredNominal(
            declaration,
        )));
    }
    Ok(interface_owned_type(package, ty, types, owners)
        .then_some(InterfaceBorrowRequirementProof::Owned))
}

fn validate_interface_method_borrow_result(
    package: PackageId,
    owner: OwnerKey,
    method: &crate::platform::kernel::OwnedMethod,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<Option<[InterfaceBorrowRequirementProof; 2]>, Diagnostic> {
    admission.checkpoint()?;
    let position = match method.result.relation {
        crate::platform::kernel::ResultRelation::Value => return Ok(None),
        crate::platform::kernel::ResultRelation::ReadFrom { source }
        | crate::platform::kernel::ResultRelation::ViewFrom { source, .. } => source,
    };
    let source = method
        .parameters
        .get(position as usize)
        .ok_or_else(|| interface_corrupt("borrowed method result source position is absent"))?;
    let invalid = || {
        interface_corrupt(
            "borrowed method result requires an Owned result and borrowed Owned source",
        )
    };
    if !matches!(method.effect, FunctionEffect::Pure)
        || source.use_mode != crate::platform::kernel::ParameterUse::Borrow
    {
        return Err(invalid());
    }
    let source =
        interface_borrow_requirement_at(package, owner, source.ty, types, owners, admission)?
            .ok_or_else(invalid)?;
    let result = interface_borrow_requirement_at(
        package,
        owner,
        method.result.ty,
        types,
        owners,
        admission,
    )?
    .ok_or_else(invalid)?;
    validate_result_category(&method.result.relation, result)?;
    Ok(Some([source.retain(package)?, result.retain(package)?]))
}

fn validate_interface_function_borrow_result(
    package: PackageId,
    declaration: DeclarationId,
    signature: &crate::platform::kernel::PackageFunctionSignature,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<Option<[InterfaceBorrowRequirementProof; 2]>, Diagnostic> {
    admission.checkpoint()?;
    let source = match signature.result.relation {
        crate::platform::kernel::ResultRelation::Value => return Ok(None),
        crate::platform::kernel::ResultRelation::ReadFrom { source }
        | crate::platform::kernel::ResultRelation::ViewFrom { source, .. } => source,
    };
    let Some(parameter_owner) = owners.get(&OwnerKey::Parameter(source)) else {
        return Err(interface_corrupt(
            "borrowed function result source parameter is absent",
        ));
    };
    let PackageInterfaceRecord::Parameter(parameter) = &parameter_owner.record else {
        return Err(interface_corrupt(
            "borrowed function result source parameter has a foreign record kind",
        ));
    };
    let invalid = || {
        interface_corrupt(
            "borrowed function result requires an Owned result and borrowed Owned source",
        )
    };
    let mut occurrences = 0;
    for id in &signature.parameters {
        // The existing owner/child visit charge covers this inventory. Keep its numeric allowance
        // unchanged for feature-free interfaces while exposing cancellation on the actual scan.
        admission.checkpoint()?;
        occurrences += usize::from(*id == source);
    }
    if !matches!(signature.effect, FunctionEffect::Pure)
        || parameter_owner.owner() != OwnerKey::Parameter(source)
        || parameter_owner.kind() != OwnerKind::Parameter
        || occurrences != 1
        || parameter.parent != ParameterParent::Function(declaration)
        || parameter.use_mode != crate::platform::kernel::ParameterUse::Borrow
    {
        return Err(invalid());
    }
    let source_proof = interface_borrow_requirement_at(
        package,
        OwnerKey::Parameter(source),
        parameter.ty,
        types,
        owners,
        admission,
    )?
    .ok_or_else(invalid)?;
    let result_proof = interface_borrow_requirement_at(
        package,
        OwnerKey::Declaration(declaration),
        signature.result.ty,
        types,
        owners,
        admission,
    )?
    .ok_or_else(invalid)?;
    validate_result_category(&signature.result.relation, result_proof)?;
    Ok(Some([
        source_proof.retain(package)?,
        result_proof.retain(package)?,
    ]))
}

fn validate_result_category<S>(
    relation: &crate::platform::kernel::ResultRelation<S>,
    proof: InterfaceBorrowRequirementProof,
) -> Result<(), Diagnostic> {
    if matches!(
        (relation, proof),
        (
            crate::platform::kernel::ResultRelation::ReadFrom { .. },
            InterfaceBorrowRequirementProof::ReadView
                | InterfaceBorrowRequirementProof::DeferredReadView(_)
        ) | (
            crate::platform::kernel::ResultRelation::ViewFrom { .. },
            InterfaceBorrowRequirementProof::Owned
        )
    ) {
        return Err(interface_corrupt(
            "result relation differs from its independently admitted value category",
        ));
    }
    Ok(())
}

/// Descriptive interface authority is deliberately separate from executable source admission.
/// A foreign obligation is not an Owned proof until the exact dependency closure discharges it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AssociatedInterfaceProof {
    Owned,
    ReadView,
    Deferred(crate::platform::kernel::DeclarationReference),
    DeferredReadView(crate::platform::kernel::DeclarationReference),
}

pub(crate) struct AssociatedInterfaceAdmission<'a> {
    visit: &'a mut dyn FnMut(u64) -> Result<(), Diagnostic>,
    storage: RepresentationBudget,
}

impl<'v> AssociatedInterfaceAdmission<'v> {
    pub(crate) fn new(visit: &'v mut dyn FnMut(u64) -> Result<(), Diagnostic>) -> Self {
        Self {
            visit,
            storage: RepresentationBudget::default(),
        }
    }

    pub(crate) fn step(&mut self) -> Result<(), Diagnostic> {
        self.charge(1)
    }

    pub(crate) fn checkpoint(&mut self) -> Result<(), Diagnostic> {
        (self.visit)(0)
    }

    pub(crate) fn charge(&mut self, count: usize) -> Result<(), Diagnostic> {
        // The caller owns the exact optional work allowance and cancellation. A second
        // cumulative counter would silently restrict ordinary unmetered interface admission.
        (self.visit)(u64::try_from(count).map_err(|_| associated_interface_storage())?)
    }

    pub(crate) fn current(
        &mut self,
        owner: &PackageInterfaceOwner,
        key: OwnerKey,
    ) -> Result<(), Diagnostic> {
        self.step()?;
        if owner.owner() != key
            || owner.contract_version != PACKAGE_INTERFACE_CONTRACT_VERSION
            || owner.record.header().contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
        {
            return Err(associated_interface_error(
                "associated occurrence requires an exact current interface owner",
            ));
        }
        Ok(())
    }

    fn declaration<'a>(
        &mut self,
        value: &'a PackageInterfaceOwner,
        reference: crate::platform::kernel::DeclarationReference,
    ) -> Result<&'a crate::platform::kernel::PackageInterfaceDeclaration, Diagnostic> {
        self.current(value, OwnerKey::Declaration(reference.declaration))?;
        match &value.record {
            PackageInterfaceRecord::Declaration(declaration) => Ok(declaration),
            _ => Err(associated_interface_error(
                "representation metadata grants no associated declaration authority",
            )),
        }
    }

    fn scope<'a>(
        &mut self,
        package: PackageId,
        source: OwnerKey,
        owners: &'a BTreeMap<OwnerKey, PackageInterfaceOwner>,
    ) -> Result<
        (
            &'a crate::platform::kernel::PackageInterfaceDeclaration,
            crate::platform::kernel::DeclarationReference,
        ),
        Diagnostic,
    > {
        let value = owners
            .get(&source)
            .ok_or_else(|| associated_interface_error("associated consuming owner is absent"))?;
        self.current(value, source)?;
        let declaration = match &value.record {
            PackageInterfaceRecord::Declaration(_) => match source {
                OwnerKey::Declaration(declaration) => declaration,
                _ => {
                    return Err(associated_interface_error(
                        "associated declaration has a foreign identity kind",
                    ));
                }
            },
            PackageInterfaceRecord::Parameter(parameter) => {
                if value.kind() != OwnerKind::Parameter {
                    return Err(associated_interface_error(
                        "associated parameter has a foreign owner kind",
                    ));
                }
                let (OwnerKey::Parameter(id), ParameterParent::Function(declaration)) =
                    (source, parameter.parent)
                else {
                    return Err(associated_interface_error(
                        "associated parameter has no exact function parent",
                    ));
                };
                let parent = owners
                    .get(&OwnerKey::Declaration(declaration))
                    .ok_or_else(|| {
                        associated_interface_error("associated parameter parent is absent")
                    })?;
                let parent = self.declaration(
                    parent,
                    crate::platform::kernel::DeclarationReference {
                        package,
                        declaration,
                    },
                )?;
                let PackageInterfaceDeclarationPayload::Function(function) = &parent.payload else {
                    return Err(associated_interface_error(
                        "associated parameter parent is not a function signature",
                    ));
                };
                let mut occurrences = 0;
                for parameter in &function.parameters {
                    self.step()?;
                    occurrences += usize::from(*parameter == id);
                }
                if occurrences != 1 {
                    return Err(associated_interface_error(
                        "associated parameter is not an exact listed signature child",
                    ));
                }
                declaration
            }
            _ => {
                return Err(associated_interface_error(
                    "associated forms are forbidden in this interface owner context",
                ));
            }
        };
        let reference = crate::platform::kernel::DeclarationReference {
            package,
            declaration,
        };
        let value = owners
            .get(&OwnerKey::Declaration(declaration))
            .ok_or_else(|| associated_interface_error("associated scope is absent"))?;
        Ok((self.declaration(value, reference)?, reference))
    }

    /// Checks one actual occurrence without inventing a body for a public function signature.
    pub(crate) fn validate_form<'a>(
        &mut self,
        package: PackageId,
        source: OwnerKey,
        form: &TypeForm,
        owners: &'a BTreeMap<OwnerKey, PackageInterfaceOwner>,
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
        resolve: &mut impl FnMut(
            crate::platform::kernel::DeclarationReference,
        ) -> Result<Option<&'a PackageInterfaceOwner>, Diagnostic>,
    ) -> Result<AssociatedInterfaceProof, Diagnostic> {
        if let TypeForm::RegionCap {
            region,
            mode: crate::platform::kernel::RegionMode::Shared { scope },
        } = form
        {
            read_views::validate_region_read_resolved_metered(
                package,
                source,
                *region,
                scope,
                owners,
                types,
                resolve,
                &mut *self.visit,
            )?;
            return Ok(AssociatedInterfaceProof::ReadView);
        }
        if let TypeForm::ReadViewApplied {
            declaration,
            arguments,
            read_scope,
        } = form
        {
            return read_views::validate_view_application_resolved_metered(
                package,
                source,
                *declaration,
                arguments,
                read_scope,
                owners,
                types,
                resolve,
                &mut *self.visit,
            )
            .map(|deferred| {
                if deferred {
                    AssociatedInterfaceProof::DeferredReadView(*declaration)
                } else {
                    AssociatedInterfaceProof::ReadView
                }
            });
        }
        let (scope_record, actual_scope) = self.scope(package, source, owners)?;
        let read_scope = match form {
            TypeForm::AssociatedReadViewMember { read_scope, .. }
            | TypeForm::AssociatedReadViewProjection { read_scope, .. } => Some(read_scope),
            _ => None,
        };
        if let Some(read_scope) = read_scope {
            read_views::validate_scope_argument(package, source, read_scope, owners)?;
        }
        let (contract, member, lexical) = match form {
            TypeForm::AssociatedContractMember { contract, member }
            | TypeForm::AssociatedReadViewMember {
                contract, member, ..
            } => {
                if *contract != actual_scope
                    || scope_record.header.kind != OwnerKind::OwnedContract
                    || !matches!(
                        scope_record.payload,
                        PackageInterfaceDeclarationPayload::OwnedContract(_)
                    )
                {
                    return Err(associated_interface_error(
                        "contract member escaped its exact contract signature",
                    ));
                }
                (*contract, *member, None)
            }
            TypeForm::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            }
            | TypeForm::AssociatedReadViewProjection {
                scope,
                parameter,
                contract,
                member,
                ..
            } => {
                if *scope != actual_scope {
                    return Err(associated_interface_error(
                        "projection escaped its exact lexical scope",
                    ));
                }
                let (parameters, ordinary) = match &scope_record.payload {
                    PackageInterfaceDeclarationPayload::Function(function)
                        if scope_record.header.kind
                            == match function.effect {
                                FunctionEffect::Pure => OwnerKind::PureFunction,
                                FunctionEffect::Task { .. } => OwnerKind::TaskFunction,
                            } =>
                    {
                        (
                            &function.implementation_parameters,
                            &function.type_parameters,
                        )
                    }
                    PackageInterfaceDeclarationPayload::OwnedImplementation(implementation)
                        if scope_record.header.kind == OwnerKind::OwnedImplementation =>
                    {
                        (
                            &implementation.implementation_parameters,
                            &implementation.type_parameters,
                        )
                    }
                    _ => {
                        return Err(associated_interface_error(
                            "projection scope is not an exact function or implementation",
                        ));
                    }
                };
                let mut selected = None;
                for (index, candidate) in parameters.iter().enumerate() {
                    self.step()?;
                    for previous in &parameters[..index] {
                        self.step()?;
                        if previous.id == candidate.id || previous.name == candidate.name {
                            return Err(associated_interface_error(
                                "lexical witness identities and names must be distinct",
                            ));
                        }
                    }
                    if candidate.id == *parameter {
                        selected = Some(candidate);
                    }
                }
                let selected = selected.ok_or_else(|| {
                    associated_interface_error("projection has no exact lexical witness formal")
                })?;
                if selected.contract != *contract {
                    return Err(associated_interface_error(
                        "projection contract differs from its exact witness formal",
                    ));
                }
                self.step()?;
                let Some(TypeObject {
                    form: TypeForm::TypeParameter { parameter },
                    ..
                }) = types.get(&selected.self_type)
                else {
                    return Err(associated_interface_error(
                        "lexical witness Self is not an exact Owned type parameter",
                    ));
                };
                let parameter_owner = owners
                    .get(&OwnerKey::TypeParameter(*parameter))
                    .ok_or_else(|| {
                        associated_interface_error("lexical witness Self parameter is absent")
                    })?;
                self.current(parameter_owner, OwnerKey::TypeParameter(*parameter))?;
                let PackageInterfaceRecord::TypeParameter(record) = &parameter_owner.record else {
                    return Err(associated_interface_error(
                        "lexical witness Self has the wrong owner record kind",
                    ));
                };
                let mut occurrences = 0;
                for id in ordinary {
                    self.step()?;
                    occurrences += usize::from(*id == *parameter);
                }
                if record.header.kind != OwnerKind::TypeParameter
                    || record.declaration != actual_scope.declaration
                    || !record.constraints.has_owned()
                    || occurrences != 1
                {
                    return Err(associated_interface_error(
                        "lexical witness Self is outside the exact Owned parameter inventory",
                    ));
                }
                (*contract, *member, Some(selected))
            }
            _ => {
                return Err(associated_interface_error(
                    "associated authority proof requires an associated type form",
                ));
            }
        };
        self.step()?;
        let value = if contract.package == package {
            Some(
                owners
                    .get(&OwnerKey::Declaration(contract.declaration))
                    .ok_or_else(|| {
                        associated_interface_error("local associated contract is absent")
                    })?,
            )
        } else {
            resolve(contract)?
        };
        let Some(value) = value else {
            return Ok(if read_scope.is_some() {
                AssociatedInterfaceProof::DeferredReadView(contract)
            } else {
                AssociatedInterfaceProof::Deferred(contract)
            });
        };
        let declaration = self.declaration(value, contract)?;
        let PackageInterfaceDeclarationPayload::OwnedContract(contract_record) =
            &declaration.payload
        else {
            return Err(associated_interface_error(
                "associated member does not name a public OwnedContract",
            ));
        };
        if declaration.header.kind != OwnerKind::OwnedContract {
            return Err(associated_interface_error(
                "associated contract has a foreign owner kind",
            ));
        }
        if lexical.is_some_and(|parameter| {
            parameter.type_arguments.len() != contract_record.type_parameters.len()
        }) {
            return Err(associated_interface_error(
                "lexical witness contract argument inventory has the wrong arity",
            ));
        }
        let mut found = false;
        for (index, entry) in contract_record.associated_types.iter().enumerate() {
            self.step()?;
            for previous in &contract_record.associated_types[..index] {
                self.step()?;
                if previous.id == entry.id || previous.name == entry.name {
                    return Err(associated_interface_error(
                        "contract associated member identities and names must be distinct",
                    ));
                }
            }
            found |= entry.id == member
                && matches!(
                    (&entry.bound, read_scope),
                    (crate::platform::kernel::AssociatedTypeBound::Owned, None)
                        | (
                            crate::platform::kernel::AssociatedTypeBound::ReadViewFamily { .. },
                            Some(_),
                        )
                );
        }
        if !found {
            return Err(associated_interface_error(
                "associated member kind is absent from its exact contract inventory",
            ));
        }
        Ok(if read_scope.is_some() {
            AssociatedInterfaceProof::ReadView
        } else {
            AssociatedInterfaceProof::Owned
        })
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the actual source occurrence, complete descriptor maps and authored resolver remain explicit"
    )]
    pub(crate) fn validate_occurrences<'a>(
        &mut self,
        package: PackageId,
        source: OwnerKey,
        roots: &[TypeObjectDigest],
        projection_free: bool,
        owners: &'a BTreeMap<OwnerKey, PackageInterfaceOwner>,
        types: &BTreeMap<TypeObjectDigest, TypeObject>,
        resolve: &mut impl FnMut(
            crate::platform::kernel::DeclarationReference,
        ) -> Result<Option<&'a PackageInterfaceOwner>, Diagnostic>,
    ) -> Result<bool, Diagnostic> {
        self.charge(roots.len())?;
        self.storage
            .reserve::<(TypeObjectDigest, usize)>(roots.len())?;
        let mut pending = Vec::new();
        pending
            .try_reserve_exact(roots.len())
            .map_err(|_| associated_interface_storage())?;
        pending.extend(roots.iter().map(|ty| (*ty, 0_usize)));
        let mut deferred = false;
        while let Some((ty, depth)) = pending.pop() {
            self.step()?;
            if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                return Err(interface_error(
                    DiagnosticClass::Resource,
                    "package_interface_associated_depth",
                    "associated interface traversal exceeds its depth admission",
                ));
            }
            let object = types.get(&ty).ok_or_else(|| {
                associated_interface_error("associated interface type closure is incomplete")
            })?;
            if object.form.is_associated() || object.form.is_read_view() {
                if projection_free {
                    if matches!(object.form, TypeForm::ReadViewApplied { .. }) {
                        // A family binding is nominal and projection-free, but all of its type
                        // and scope actuals still require ordinary independent admission below.
                    } else {
                        return Err(associated_interface_error(
                            "stored associated binding contains a projection or contract member",
                        ));
                    }
                }
                deferred |= matches!(
                    self.validate_form(package, source, &object.form, owners, types, resolve)?,
                    AssociatedInterfaceProof::Deferred(_)
                        | AssociatedInterfaceProof::DeferredReadView(_)
                );
            }
            let count = object.child_type_count();
            self.charge(count)?;
            self.storage.reserve::<TypeObjectDigest>(count)?;
            self.storage.reserve::<(TypeObjectDigest, usize)>(count)?;
            pending
                .try_reserve(count)
                .map_err(|_| associated_interface_storage())?;
            pending.extend(
                object
                    .child_types()
                    .into_iter()
                    .map(|child| (child, depth + 1)),
            );
        }
        Ok(deferred)
    }

    pub(crate) fn reserve_roots(
        &mut self,
        owner: &PackageInterfaceOwner,
    ) -> Result<(), Diagnostic> {
        let count = usize::try_from(interface_owner_validation_visits(owner))
            .map_err(|_| associated_interface_storage())?;
        self.charge(count)?;
        self.storage.reserve::<TypeObjectDigest>(count)
    }
}

fn associated_interface_error(message: &str) -> Diagnostic {
    interface_error(
        DiagnosticClass::Semantic,
        "package_interface_associated_authority",
        message,
    )
}

fn associated_interface_storage() -> Diagnostic {
    interface_error(
        DiagnosticClass::Resource,
        "package_interface_associated_storage",
        "associated interface proof cannot retain its admitted traversal state",
    )
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
            PackageInterfaceRecord::EffectParameter(record) => record.declaration,
            PackageInterfaceRecord::RequirementParameter(record) => record.declaration,
            PackageInterfaceRecord::Field(record) => record.declaration,
            PackageInterfaceRecord::Case(record) => record.declaration,
            PackageInterfaceRecord::Operation(record) => record.declaration,
            PackageInterfaceRecord::Requirement(record) => record.declaration,
            PackageInterfaceRecord::Port(record) => record.declaration,
            PackageInterfaceRecord::Declaration(_)
            | PackageInterfaceRecord::RepresentationDeclaration(_)
            | PackageInterfaceRecord::Parameter(_) => {
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
    associated_admission: &mut AssociatedInterfaceAdmission<'_>,
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
        // Type traversal and associated proof share the caller's exact optional allowance.
        (associated_admission.visit)(1)?;
        if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
            return Err(interface_error(
                DiagnosticClass::Resource,
                "package_interface_type_work",
                "package-interface type closure exceeds its type-depth bound",
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
    // Lexical witness Self/type actuals may be read before their type objects in the pending
    // traversal. Prove associated authority only once the complete immutable map is available,
    // within the existing per-owner sequence eligibility pass. No second all-pair scan is added.
    // Check every actual owner/digest pair: success under another consumer grants no authority.
    for (source, digest) in visited {
        associated_admission.checkpoint()?;
        let object = objects.get(&digest).ok_or_else(|| {
            associated_interface_error("completed interface type closure lost a visited object")
        })?;
        if object.form.is_associated() || object.form.is_read_view() {
            match associated_admission.validate_form(
                package,
                source,
                &object.form,
                owners,
                &objects,
                &mut |_| Ok(None),
            )? {
                AssociatedInterfaceProof::Owned
                | AssociatedInterfaceProof::ReadView
                | AssociatedInterfaceProof::Deferred(_)
                | AssociatedInterfaceProof::DeferredReadView(_) => {}
            }
            // Foreign Deferred retains the exact occurrence in owners/types. It grants no
            // readiness or property; the complete selected transport closure must discharge it.
        }
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
        TypeForm::RegionBrand { .. } => {
            return Err(interface_error(
                DiagnosticClass::Semantic,
                "package_interface_region_scope",
                "a public signature cannot export a lexical region brand",
            ));
        }
        TypeForm::Region { .. }
        | TypeForm::RelPtr { .. }
        | TypeForm::RegionSlice { .. }
        | TypeForm::RegionCap { .. } => {}
        TypeForm::AssociatedContractMember { .. }
        | TypeForm::AssociatedWitnessProjection { .. }
        | TypeForm::AssociatedReadViewMember { .. }
        | TypeForm::AssociatedReadViewProjection { .. }
        | TypeForm::ReadViewApplied { .. } => {
            // validate_type_closure proves every associated occurrence after loading its complete
            // local map. This early reference pass proves ordinary scope only.
        }
        TypeForm::TaskFunction { effect, .. } => {
            effect.validate()?;
            for parameter in &effect.parameters {
                if parameter.package != package
                    || !matches!(owners.get(&OwnerKey::EffectParameter(parameter.parameter)).map(|v| &v.record),
                    Some(PackageInterfaceRecord::EffectParameter(record)) if Some(record.declaration) == semantic_declaration(source, owners))
                {
                    return Err(interface_error(
                        DiagnosticClass::Semantic,
                        "package_interface_effect_scope",
                        "task callable row uses an effect parameter outside its exact public declaration",
                    ));
                }
            }
            for requirement in &effect.requirements {
                if let crate::platform::kernel::RequirementOperand::Parameter(reference) =
                    requirement
                {
                    if reference.package != package
                        || !matches!(owners.get(&requirement.owner()).map(|v| &v.record), Some(PackageInterfaceRecord::RequirementParameter(record)) if Some(record.declaration) == semantic_declaration(source, owners))
                    {
                        return Err(interface_error(
                            DiagnosticClass::Semantic,
                            "package_interface_requirement_scope",
                            "requirement parameter is outside its exact public function",
                        ));
                    }
                    continue;
                }
                if requirement.package() == package
                    && !matches!(
                        owners.get(&requirement.owner()).map(|v| &v.record),
                        Some(PackageInterfaceRecord::Requirement(_))
                    )
                {
                    return Err(interface_error(
                        DiagnosticClass::Semantic,
                        "package_interface_effect_requirement",
                        "task callable row requirement is absent from its exact interface",
                    ));
                }
            }
        }
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
        TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. }
            if declaration.package == package =>
        {
            let key = OwnerKey::Declaration(declaration.declaration);
            let Some(value) = owners.get(&key) else {
                return Err(interface_error(
                    DiagnosticClass::Semantic,
                    "package_interface_named_type_missing",
                    "public signature names a local declaration absent from the package interface",
                ));
            };
            if matches!(
                value.record,
                PackageInterfaceRecord::RepresentationDeclaration(_)
            ) && !interface_representation_source(source, owners)
            {
                return Err(interface_error(
                    DiagnosticClass::Semantic,
                    "package_interface_private_signature",
                    "public signatures cannot name private representation declarations",
                ));
            }
            if !matches!(
                value.kind(),
                OwnerKind::Record | OwnerKind::Variant | OwnerKind::OwnedRecord
            ) {
                return Err(interface_error(
                    DiagnosticClass::Semantic,
                    "package_interface_named_type_kind",
                    "public signature names a declaration that is not an admitted nominal type",
                ));
            }
            if let PackageInterfaceRecord::Declaration(record)
            | PackageInterfaceRecord::RepresentationDeclaration(record) = &value.record
            {
                let parameters = match &record.payload {
                    PackageInterfaceDeclarationPayload::Record {
                        type_parameters, ..
                    }
                    | PackageInterfaceDeclarationPayload::Variant {
                        type_parameters, ..
                    }
                    | PackageInterfaceDeclarationPayload::OwnedRecord {
                        type_parameters, ..
                    } => type_parameters,
                    _ => return Err(interface_corrupt("nominal interface has a foreign kind")),
                };
                let count = match form {
                    TypeForm::Applied { arguments, .. } => arguments.len(),
                    _ => 0,
                };
                if parameters.len() != count {
                    return Err(interface_error(
                        DiagnosticClass::Semantic,
                        "package_interface_nominal_arity",
                        "interface type has wrong nominal application arity",
                    ));
                }
            }
        }
        TypeForm::CapabilityResource { interface } if interface.package == package => {
            let key = OwnerKey::Declaration(interface.declaration);
            let Some(value) = owners.get(&key) else {
                return Err(interface_error(
                    DiagnosticClass::Semantic,
                    "package_interface_resource_interface_missing",
                    "capability resource names a local interface absent from the package interface",
                ));
            };
            if value.kind() != OwnerKind::Interface {
                return Err(interface_error(
                    DiagnosticClass::Semantic,
                    "package_interface_resource_interface_kind",
                    "capability resource reference does not name an interface",
                ));
            }
        }
        TypeForm::Named { .. }
        | TypeForm::Applied { .. }
        | TypeForm::CapabilityResource { .. }
        | TypeForm::Unit
        | TypeForm::Bool
        | TypeForm::I64
        | TypeForm::F64
        | TypeForm::ByteBuffer
        | TypeForm::OwnedI64Cell
        | TypeForm::OwnedProduct { .. }
        | TypeForm::OwnedChoice { .. }
        | TypeForm::OwnedSequence { .. }
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
        OwnerKey::RequirementParameter(parameter) => {
            match &owners
                .get(&OwnerKey::RequirementParameter(parameter))?
                .record
            {
                PackageInterfaceRecord::RequirementParameter(record) => Some(record.declaration),
                _ => None,
            }
        }
        OwnerKey::EffectParameter(parameter) => {
            match &owners.get(&OwnerKey::EffectParameter(parameter))?.record {
                PackageInterfaceRecord::EffectParameter(record) => Some(record.declaration),
                _ => None,
            }
        }
        OwnerKey::TypeParameter(parameter) => {
            match &owners.get(&OwnerKey::TypeParameter(parameter))?.record {
                PackageInterfaceRecord::TypeParameter(record) => Some(record.declaration),
                PackageInterfaceRecord::EffectParameter(record) => Some(record.declaration),
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
        | OwnerKey::HttpRoute(_)
        | OwnerKey::Documentation(_)
        | OwnerKey::Annotation(_) => None,
    }
}

fn interface_representation_source(
    source: OwnerKey,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
) -> bool {
    if let OwnerKey::Declaration(_) = source {
        return matches!(owners.get(&source).map(|owner| &owner.record),
            Some(PackageInterfaceRecord::Declaration(record) | PackageInterfaceRecord::RepresentationDeclaration(record))
                if matches!(record.payload, PackageInterfaceDeclarationPayload::ReadView(_)));
    }
    if !matches!(source, OwnerKey::Field(_) | OwnerKey::Case(_)) {
        return false;
    }
    semantic_declaration(source, owners)
        .and_then(|declaration| owners.get(&OwnerKey::Declaration(declaration)))
        .is_some_and(|owner| match &owner.record {
            PackageInterfaceRecord::RepresentationDeclaration(_) => true,
            PackageInterfaceRecord::Declaration(record) => matches!(
                record.payload,
                PackageInterfaceDeclarationPayload::OwnedRecord { .. }
            ),
            _ => false,
        })
}

fn interface_nominal_layout_types(
    declaration: DeclarationId,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    budget: &mut RepresentationBudget,
) -> Result<Vec<TypeObjectDigest>, Diagnostic> {
    let record = match owners
        .get(&OwnerKey::Declaration(declaration))
        .map(|owner| &owner.record)
    {
        Some(
            PackageInterfaceRecord::Declaration(record)
            | PackageInterfaceRecord::RepresentationDeclaration(record),
        ) => record,
        _ => {
            return Err(interface_corrupt(
                "representation nominal declaration is absent",
            ));
        }
    };
    match &record.payload {
        PackageInterfaceDeclarationPayload::ReadView(view) => {
            budget.reserve::<TypeObjectDigest>(view.fields.len())?;
            Ok(view.fields.iter().map(|field| field.ty).collect())
        }
        PackageInterfaceDeclarationPayload::Record { fields, .. }
        | PackageInterfaceDeclarationPayload::OwnedRecord { fields, .. } => {
            budget.reserve::<TypeObjectDigest>(fields.len())?;
            fields
                .iter()
                .map(|field| {
                    match owners
                        .get(&OwnerKey::Field(*field))
                        .map(|owner| &owner.record)
                    {
                        Some(PackageInterfaceRecord::Field(record))
                            if record.declaration == declaration =>
                        {
                            Ok(record.ty)
                        }
                        _ => Err(interface_corrupt(
                            "representation field is absent or belongs to another declaration",
                        )),
                    }
                })
                .collect()
        }
        PackageInterfaceDeclarationPayload::Variant { cases, .. } => {
            budget.reserve::<TypeObjectDigest>(cases.len())?;
            let mut roots = Vec::new();
            for case in cases {
                match owners
                    .get(&OwnerKey::Case(*case))
                    .map(|owner| &owner.record)
                {
                    Some(PackageInterfaceRecord::Case(record))
                        if record.declaration == declaration =>
                    {
                        roots.extend(record.payload)
                    }
                    _ => {
                        return Err(interface_corrupt(
                            "representation case is absent or belongs to another declaration",
                        ));
                    }
                }
            }
            Ok(roots)
        }
        _ => Err(interface_corrupt(
            "representation names a non-layout declaration",
        )),
    }
}

/// Establish private evidence reachability independently of interface presence, and reject
/// recursive owned layout edges even when an ordinary container separates the declarations.
fn representation_declarations(
    package: PackageId,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    visit: &mut dyn FnMut(u64) -> Result<(), Diagnostic>,
) -> Result<BTreeSet<OwnerKey>, Diagnostic> {
    representation_declarations_admitted(
        package,
        owners,
        types,
        visit,
        &mut RepresentationBudget::default(),
    )
}

fn representation_declarations_admitted(
    package: PackageId,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    visit: &mut dyn FnMut(u64) -> Result<(), Diagnostic>,
    budget: &mut RepresentationBudget,
) -> Result<BTreeSet<OwnerKey>, Diagnostic> {
    let mut charge = || visit(1);
    let mut owned = BTreeSet::new();
    for (key, owner) in owners {
        match &owner.record {
            PackageInterfaceRecord::Declaration(record)
            | PackageInterfaceRecord::RepresentationDeclaration(record)
                if matches!(
                    record.payload,
                    PackageInterfaceDeclarationPayload::OwnedRecord { .. }
                        | PackageInterfaceDeclarationPayload::ReadView(_)
                ) =>
            {
                budget.reserve::<OwnerKey>(1)?;
                owned.insert(*key);
            }
            _ => {}
        }
    }
    let mut reachable = BTreeSet::new();
    let mut pending = Vec::new();
    for owner in owners.values() {
        if let PackageInterfaceRecord::Declaration(record) = &owner.record {
            budget.append_types(record.type_roots(), &mut pending)?;
        }
    }
    for key in &owned {
        if let OwnerKey::Declaration(id) = key
            && matches!(
                owners.get(key).map(|owner| &owner.record),
                Some(PackageInterfaceRecord::Declaration(_))
            )
        {
            let roots = interface_nominal_layout_types(*id, owners, budget)?;
            budget.append_types(roots, &mut pending)?;
        }
    }
    let mut visited = BTreeSet::new();
    while let Some(digest) = pending.pop() {
        charge()?;
        if visited.contains(&digest) {
            continue;
        }
        budget.reserve::<TypeObjectDigest>(1)?;
        visited.insert(digest);
        let object = types
            .get(&digest)
            .ok_or_else(|| interface_corrupt("owned representation type is absent"))?;
        budget.append_children(object, &mut pending)?;
        if let TypeForm::Named { declaration }
        | TypeForm::Applied { declaration, .. }
        | TypeForm::ReadViewApplied { declaration, .. } = object.form
            && declaration.package == package
        {
            let key = OwnerKey::Declaration(declaration.declaration);
            if !reachable.contains(&key) {
                budget.reserve::<OwnerKey>(1)?;
                reachable.insert(key);
                let roots =
                    interface_nominal_layout_types(declaration.declaration, owners, budget)?;
                budget.append_types(roots, &mut pending)?;
            }
        }
    }
    let mut edges = BTreeMap::<OwnerKey, BTreeSet<OwnerKey>>::new();
    for root in &owned {
        let OwnerKey::Declaration(id) = root else {
            continue;
        };
        let mut pending = interface_nominal_layout_types(*id, owners, budget)?;
        let mut seen_types = BTreeSet::new();
        let mut seen_declarations = BTreeSet::new();
        while let Some(digest) = pending.pop() {
            charge()?;
            if seen_types.contains(&digest) {
                continue;
            }
            budget.reserve::<TypeObjectDigest>(1)?;
            seen_types.insert(digest);
            let object = types
                .get(&digest)
                .ok_or_else(|| interface_corrupt("owned representation type is absent"))?;
            budget.append_children(object, &mut pending)?;
            if let TypeForm::Named { declaration }
            | TypeForm::Applied { declaration, .. }
            | TypeForm::ReadViewApplied { declaration, .. } = object.form
                && declaration.package == package
            {
                let key = OwnerKey::Declaration(declaration.declaration);
                if owned.contains(&key) {
                    if !edges.contains_key(root) {
                        budget.reserve::<(OwnerKey, BTreeSet<OwnerKey>)>(1)?;
                        edges.insert(*root, BTreeSet::new());
                    }
                    let targets = edges.get_mut(root).ok_or_else(|| {
                        interface_corrupt("admitted owned layout adjacency is absent")
                    })?;
                    if !targets.contains(&key) {
                        budget.reserve::<OwnerKey>(1)?;
                        targets.insert(key);
                    }
                } else if !seen_declarations.contains(&key) {
                    budget.reserve::<OwnerKey>(1)?;
                    seen_declarations.insert(key);
                    let roots =
                        interface_nominal_layout_types(declaration.declaration, owners, budget)?;
                    budget.append_types(roots, &mut pending)?;
                }
            }
        }
    }
    // Memoize completed declarations. Shared DAG tails are proved once, without copying
    // or enumerating every root-to-leaf path. Exit frames derive the longest owned depth.
    let mut completed = BTreeMap::<OwnerKey, usize>::new();
    let mut active = BTreeSet::new();
    let mut frames = Vec::new();
    for root in &owned {
        if completed.contains_key(root) {
            continue;
        }
        budget.reserve::<(OwnerKey, bool)>(1)?;
        frames.push((*root, false));
        while let Some((current, exiting)) = frames.pop() {
            charge()?;
            if exiting {
                let mut depth = 0usize;
                for target in edges.get(&current).into_iter().flatten() {
                    charge()?;
                    let child_depth = completed
                        .get(target)
                        .ok_or_else(|| interface_corrupt("owned layout child proof is absent"))?;
                    depth = depth.max(child_depth.saturating_add(1));
                }
                if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                    return Err(interface_error(
                        DiagnosticClass::Resource,
                        "package_interface_type_work",
                        "owned layout traversal exceeds its type-depth bound",
                    ));
                }
                active.remove(&current);
                budget.reserve::<(OwnerKey, usize)>(1)?;
                completed.insert(current, depth);
                continue;
            }
            if completed.contains_key(&current) {
                continue;
            }
            if active.contains(&current) {
                return Err(interface_error(
                    DiagnosticClass::Semantic,
                    "package_interface_owned_record_cycle",
                    "recursive nominal owned record layouts are unsupported",
                ));
            }
            budget.reserve::<OwnerKey>(1)?;
            active.insert(current);
            budget.reserve::<(OwnerKey, bool)>(1)?;
            frames.push((current, true));
            for target in edges.get(&current).into_iter().flatten() {
                charge()?;
                budget.reserve::<(OwnerKey, bool)>(1)?;
                frames.push((*target, false));
            }
        }
    }
    Ok(reachable)
}

struct ReachablePageReader<'a, P: ?Sized> {
    source: &'a P,
    pages: RefCell<BTreeSet<PageDigest>>,
}

impl<'a, P: PageStore + ?Sized> ReachablePageReader<'a, P> {
    fn new(source: &'a P) -> Self {
        Self {
            source,
            pages: RefCell::new(BTreeSet::new()),
        }
    }

    fn into_pages(self) -> BTreeSet<PageDigest> {
        self.pages.into_inner()
    }
}

impl<P: PageStore + ?Sized> PageStore for ReachablePageReader<'_, P> {
    fn read_page(
        &self,
        digest: PageDigest,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, MapError> {
        let bytes = self.source.read_page(digest, maximum_bytes)?;
        if bytes.is_some() {
            self.pages.borrow_mut().insert(digest);
        }
        Ok(bytes)
    }

    fn write_page(&mut self, _digest: PageDigest, _bytes: &[u8]) -> Result<PageWrite, MapError> {
        Err(MapError {
            class: MapErrorClass::Store,
            code: "package_interface_reader_write",
            message: "package-interface validation page source is read-only".to_owned(),
        })
    }
}

struct EmptyObjectStore;

static EMPTY_OBJECT_STORE: EmptyObjectStore = EmptyObjectStore;

impl ImmutableObjectStore for EmptyObjectStore {
    fn read(
        &self,
        _key: ObjectKey,
        _maximum_bytes: Option<usize>,
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

    #[test]
    fn interface_preserves_borrowed_result_relationship_and_rejects_erasure() {
        use crate::platform::kernel::*;
        let seed = b"borrowed-interface-result";
        let declaration = DeclarationId::migrate(seed, 0);
        let parameter = ParameterId::migrate(seed, 0);
        let ty = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
            .unwrap()
            .0;
        let owner = OwnerKey::Declaration(declaration);
        let original = PackageInterfaceOwner {
            contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
            record: PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                header: OwnerHeader::new(owner, OwnerKind::PureFunction),
                name: Name::new("view").unwrap(),
                payload: PackageInterfaceDeclarationPayload::Function(PackageFunctionSignature {
                    implementation_parameters: Vec::new(),
                    requirement_parameters: Vec::new(),
                    effect_parameters: Vec::new(),
                    type_parameters: Vec::new(),
                    parameters: vec![parameter],
                    result: crate::platform::kernel::CallableResult {
                        ty,
                        relation: crate::platform::kernel::ResultRelation::ReadFrom {
                            source: parameter,
                        },
                    },
                    read_scope_parameters: Vec::new(),
                    read_scope_sources: Vec::new(),
                    effect: FunctionEffect::Pure,
                }),
            }),
        };
        let (digest, bytes) = original.encode().unwrap();
        assert_eq!(&bytes[..8], b"LKJPIF22");
        assert_eq!(
            PackageInterfaceOwner::decode(&bytes, owner, digest).unwrap(),
            original
        );
        let mut erased = original.clone();
        let PackageInterfaceRecord::Declaration(d) = &mut erased.record else {
            unreachable!()
        };
        let PackageInterfaceDeclarationPayload::Function(f) = &mut d.payload else {
            unreachable!()
        };
        f.result.relation = crate::platform::kernel::ResultRelation::Value;
        assert_ne!(erased.encode().unwrap().0, digest);
    }

    #[test]
    fn borrowed_result_interface_requires_borrowed_owned_source() {
        use crate::platform::kernel::*;
        let seed = b"borrowed-interface-source-admission";
        let package = PackageId::migrate(seed, 0);
        let declaration = DeclarationId::migrate(seed, 0);
        let parameter = ParameterId::migrate(seed, 0);
        let object = TypeObject::new(TypeForm::ByteBuffer).unwrap();
        let ty = encode_type_object(&object).unwrap().0;
        let mut owners = BTreeMap::from([
            (
                OwnerKey::Declaration(declaration),
                PackageInterfaceOwner {
                    contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                    record: PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                        header: OwnerHeader::new(
                            OwnerKey::Declaration(declaration),
                            OwnerKind::PureFunction,
                        ),
                        name: Name::new("view").unwrap(),
                        payload: PackageInterfaceDeclarationPayload::Function(
                            PackageFunctionSignature {
                                implementation_parameters: Vec::new(),
                                requirement_parameters: Vec::new(),
                                effect_parameters: Vec::new(),
                                type_parameters: Vec::new(),
                                parameters: vec![parameter],
                                result: crate::platform::kernel::CallableResult {
                                    ty,
                                    relation: crate::platform::kernel::ResultRelation::ReadFrom {
                                        source: parameter,
                                    },
                                },
                                read_scope_parameters: Vec::new(),
                                read_scope_sources: Vec::new(),
                                effect: FunctionEffect::Pure,
                            },
                        ),
                    }),
                },
            ),
            (
                OwnerKey::Parameter(parameter),
                PackageInterfaceOwner {
                    contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                    record: PackageInterfaceRecord::Parameter(ParameterRecord {
                        header: OwnerHeader::new(
                            OwnerKey::Parameter(parameter),
                            OwnerKind::Parameter,
                        ),
                        parent: ParameterParent::Function(declaration),
                        name: Name::new("storage").unwrap(),
                        ty,
                        use_mode: ParameterUse::Borrow,
                        resource_requirement: None,
                    }),
                },
            ),
        ]);
        let types = BTreeMap::from([(ty, object)]);
        validate_owner_closure(package, &owners, &types).unwrap();
        let PackageInterfaceRecord::Parameter(p) = &mut owners
            .get_mut(&OwnerKey::Parameter(parameter))
            .unwrap()
            .record
        else {
            unreachable!()
        };
        p.use_mode = ParameterUse::Consume;
        assert!(
            validate_owner_closure(package, &owners, &types)
                .unwrap_err()
                .message
                .contains("borrowed Owned source")
        );
    }

    #[test]
    fn structural_contract_interface_requires_exact_parameter_children() {
        use crate::platform::kernel::*;
        use crate::platform::semantic_id::MethodId;
        let seed = b"structural-contract-interface-generation";
        let package = "pkg_10000000000000000000000000000001".parse().unwrap();
        let declaration = DeclarationId::migrate(seed, 0);
        let owner = OwnerKey::Declaration(declaration);
        let self_parameter = TypeParameterId::migrate(seed, 0);
        let self_object = TypeObject::new(TypeForm::TypeParameter {
            parameter: self_parameter,
        })
        .unwrap();
        let self_type = encode_type_object(&self_object).unwrap().0;
        let product_object = TypeObject::new(TypeForm::OwnedProduct {
            fields: vec![StructuralTypeField {
                name: Name::new("rest").unwrap(),
                ty: self_type,
            }],
        })
        .unwrap();
        let product_type = encode_type_object(&product_object).unwrap().0;
        let mut owners = BTreeMap::from([
            (
                owner,
                PackageInterfaceOwner {
                    contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                    record: PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                        header: OwnerHeader {
                            contract_version:
                                crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                            owner,
                            kind: OwnerKind::OwnedContract,
                        },
                        name: Name::new("Worklist").unwrap(),
                        payload: PackageInterfaceDeclarationPayload::OwnedContract(OwnedContract {
                            self_parameter,
                            type_parameters: Vec::new(),
                            methods: vec![OwnedMethod {
                                id: MethodId::migrate(seed, 0),
                                name: Name::new("pop").unwrap(),
                                parameters: vec![OwnedMethodParameter {
                                    ty: self_type,
                                    use_mode: ParameterUse::Consume,
                                }],
                                result: crate::platform::kernel::CallableResult {
                                    ty: product_type,
                                    relation: crate::platform::kernel::ResultRelation::Value,
                                },
                                read_scope_parameters: Vec::new(),
                                read_scope_sources: Vec::new(),
                                effect: FunctionEffect::Pure,
                            }],
                            associated_types: Vec::new(),
                        }),
                    }),
                },
            ),
            (
                OwnerKey::TypeParameter(self_parameter),
                PackageInterfaceOwner {
                    contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                    record: PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
                        header: OwnerHeader {
                            contract_version:
                                crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                            owner: OwnerKey::TypeParameter(self_parameter),
                            kind: OwnerKind::TypeParameter,
                        },
                        declaration,
                        name: Name::new("Self").unwrap(),
                        constraints: TypeParameterConstraints::Owned,
                    }),
                },
            ),
        ]);
        let types = BTreeMap::from([(self_type, self_object), (product_type, product_object)]);
        let (digest, bytes) = owners[&owner].encode().unwrap();
        owners.insert(
            owner,
            PackageInterfaceOwner::decode(&bytes, owner, digest).unwrap(),
        );
        validate_owner_closure(package, &owners, &types).unwrap();
        let extra = TypeParameterId::migrate(seed, 1);
        let PackageInterfaceRecord::Declaration(d) = &mut owners.get_mut(&owner).unwrap().record
        else {
            panic!("declaration");
        };
        let PackageInterfaceDeclarationPayload::OwnedContract(c) = &mut d.payload else {
            panic!("contract");
        };
        c.type_parameters.push(extra);
        assert_eq!(
            validate_owner_closure(package, &owners, &types)
                .unwrap_err()
                .code,
            "package_interface_child_missing"
        );
        owners.insert(
            OwnerKey::TypeParameter(extra),
            PackageInterfaceOwner {
                contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                record: PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
                    header: OwnerHeader::new(
                        OwnerKey::TypeParameter(extra),
                        OwnerKind::TypeParameter,
                    ),
                    declaration,
                    name: Name::new("Item").unwrap(),
                    constraints: TypeParameterConstraints::Owned,
                }),
            },
        );
        assert!(validate_owner_closure(package, &owners, &types).is_ok());
    }

    #[test]
    fn interface_flat_schema_preserves_all_parameter_constraints() {
        use crate::platform::kernel::{
            Name, OwnerHeader, TypeParameterConstraints as C, TypeParameterRecord,
        };
        let seed = b"interface-parameter-constraints";
        let owner = OwnerKey::TypeParameter(TypeParameterId::migrate(seed, 0));
        for constraints in [
            C::None,
            C::CaptureSafe,
            C::Owned,
            C::Transferable,
            C::CaptureSafeTransferable,
            C::OwnedTransferable,
            C::OwnedShareable,
            C::OwnedTransferableShareable,
        ] {
            let value = PackageInterfaceOwner {
                contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                record: PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
                    header: OwnerHeader::new(owner, OwnerKind::TypeParameter),
                    declaration: DeclarationId::migrate(seed, 0),
                    name: Name::new("T").unwrap(),
                    constraints,
                }),
            };
            let (digest, bytes) = value.encode().unwrap();
            assert_eq!(&bytes[..8], &PACKAGE_INTERFACE_MAGIC);
            assert_eq!(
                PackageInterfaceOwner::decode(&bytes, owner, digest).unwrap(),
                value
            );
            assert_eq!(value.encode().unwrap(), (digest, bytes));
        }
    }

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
                | OwnerKey::HttpRoute(_)
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
    fn interface_decode_rejects_wrong_owner_and_foreign_kind() {
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
        let mut foreign_envelope = bytes;
        foreign_envelope[..8].copy_from_slice(b"FOREIGN!");
        let foreign_digest = PackageInterfaceOwnerDigest::of(&foreign_envelope);
        assert!(PackageInterfaceOwner::decode(&foreign_envelope, owner, foreign_digest).is_err());
    }

    #[test]
    fn interface_current_envelope_independently_admits_contract_and_graph_fields() {
        let (_, _, owners) = built_fixture();
        let original = owners
            .values()
            .find(|value| matches!(value.record, PackageInterfaceRecord::Declaration(_)))
            .unwrap();
        for graph_field in [false, true] {
            let mut value = original.clone();
            if graph_field {
                let PackageInterfaceRecord::Declaration(record) = &mut value.record else {
                    unreachable!()
                };
                record.header.contract_version = u16::MAX;
            } else {
                value.contract_version = 0;
            }
            assert!(value.encode().is_err());
            let bytes = crate::platform::packed::encode_canonical(
                PACKAGE_INTERFACE_MAGIC,
                PACKAGE_INTERFACE_ENVELOPE_DOMAIN,
                &value,
                MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
            )
            .unwrap();
            assert!(
                PackageInterfaceOwner::decode(
                    &bytes,
                    value.owner(),
                    PackageInterfaceOwnerDigest::of(&bytes)
                )
                .is_err()
            );
        }
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

    #[test]
    fn owned_sequence_interface_closure_admits_complete_signature() {
        let snapshot =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
                r#"declarations.begin
(units (module create sequence-interface
  (function create relay (visibility public) (effect pure)
    (parameter create p (type (owned-sequence ByteBuffer)) (use consume))
    (returns Unit) (body (unit)))))
declarations.end"#,
            )
            .unwrap();
        let witness = rebuild_full_witness(&snapshot).unwrap();
        let selection =
            PackageInterfaceSelection::from_records(snapshot.root.package_id, &snapshot.owners)
                .unwrap();
        let owners = snapshot
            .owners
            .iter()
            .filter_map(|(key, record)| {
                PackageInterfaceOwner::project(record, &witness.summaries[key], &selection)
                    .unwrap()
                    .map(|owner| (*key, owner))
            })
            .collect::<BTreeMap<_, _>>();
        let types = snapshot
            .types
            .iter()
            .map(|(digest, object)| (*digest, encode_type_object(object).unwrap().1))
            .collect();
        let validate = |owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>| {
            let build = build_package_interface(owners, &types).unwrap();
            let mut store = MemoryPackedStore::default();
            let mut work = StoreWork::default();
            for (key, bytes) in &build.objects {
                store.stage(*key, bytes, &mut work).unwrap();
            }
            validate_package_interface(snapshot.root.package_id, build.root, &store, &mut work)
        };
        assert!(validate(&owners).is_ok());
    }

    #[test]
    fn interface_identity_uses_logical_content_not_physical_page_identity() {
        let (package, build, _) = built_fixture();
        let repacked = MapRoot::from_parts(
            PageDigest::from_bytes([0xa7; 32]),
            build.root.entries(),
            build.root.content(),
        );
        assert_ne!(build.root.page(), repacked.page());
        assert_eq!(build.root.content_root(), repacked.content_root());
        assert_eq!(
            package_interface_digest(package, build.root.content_root()).unwrap(),
            package_interface_digest(package, repacked.content_root()).unwrap()
        );
    }
}
