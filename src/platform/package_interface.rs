//! Derived, implementation-free package interface records for exact Graph 10 dependencies.
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

pub const PACKAGE_INTERFACE_CONTRACT_IDENTITY: &str = "lkjscript-package-interface-owner-15";
pub const PACKAGE_INTERFACE_CONTRACT_VERSION: u16 = 15;
pub const PACKAGE_INTERFACE_MAGIC: [u8; 8] = *b"LKJPIF15";
pub const PACKAGE_INTERFACE_ENVELOPE_DOMAIN: &str =
    "lkjscript.package-interface-owner-envelope.v15";
const PACKAGE_INTERFACE_IDENTITY_MAGIC: [u8; 8] = *b"LKJPIFI1";
const PACKAGE_INTERFACE_IDENTITY_DOMAIN: &str = "lkjscript.package-interface-identity.v1";
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

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct PackageInterfaceOwner {
    pub contract_version: u16,
    pub record: PackageInterfaceRecord,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
struct PackageInterfaceOwner10 {
    contract_version: u16,
    record: crate::platform::kernel::wire14::PackageInterfaceRecord14,
}

/// Frozen generation 14 predates source-tied borrowed results.
#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
struct PackageInterfaceOwner14 {
    contract_version: u16,
    record: crate::platform::kernel::wire26::PackageInterfaceRecord26,
}

#[derive(Clone, Debug, Decode, Encode)]
struct PackageInterfaceOwner11 {
    contract_version: u16,
    record: crate::platform::kernel::interface11::PackageInterfaceRecord11,
}

/// Generation 13 predates parameterized contract application fields.
#[derive(Clone, Debug, Decode, Encode)]
struct PackageInterfaceOwner13 {
    contract_version: u16,
    record: crate::platform::kernel::wire25::PackageInterfaceRecord25,
}

/// Frozen generation 12 admits only the original three constraint tags.
#[derive(Clone, Debug, Decode, Encode)]
struct PackageInterfaceOwner12 {
    contract_version: u16,
    record: PackageInterfaceRecord12,
}

#[derive(Clone, Debug, Decode, Encode)]
enum PackageInterfaceRecord12 {
    Declaration(crate::platform::kernel::wire25::PackageInterfaceDeclaration25),
    TypeParameter(TypeParameterRecord12),
    EffectParameter(crate::platform::kernel::EffectParameterRecord),
    Field(crate::platform::kernel::FieldRecord),
    Case(crate::platform::kernel::CaseRecord),
    Operation(crate::platform::kernel::OperationRecord),
    Parameter(crate::platform::kernel::ParameterRecord),
    Requirement(crate::platform::kernel::RequirementRecord),
    Port(crate::platform::kernel::PackageInterfacePort),
    RequirementParameter(crate::platform::kernel::RequirementParameterRecord),
}

#[derive(Clone, Debug, Decode, Encode)]
struct TypeParameterRecord12 {
    header: crate::platform::kernel::OwnerHeader,
    declaration: DeclarationId,
    name: crate::platform::kernel::Name,
    constraints: TypeParameterConstraints12,
}

#[derive(Clone, Debug, Decode, Encode)]
enum TypeParameterConstraints12 {
    None,
    CaptureSafe,
    Owned,
}

impl From<PackageInterfaceRecord12> for PackageInterfaceRecord {
    fn from(record: PackageInterfaceRecord12) -> Self {
        use PackageInterfaceRecord12 as W;
        match record {
            W::Declaration(v) => Self::Declaration(v.into()),
            W::TypeParameter(v) => {
                Self::TypeParameter(crate::platform::kernel::TypeParameterRecord {
                    header: v.header,
                    declaration: v.declaration,
                    name: v.name,
                    constraints: match v.constraints {
                        TypeParameterConstraints12::None => {
                            crate::platform::kernel::TypeParameterConstraints::None
                        }
                        TypeParameterConstraints12::CaptureSafe => {
                            crate::platform::kernel::TypeParameterConstraints::CaptureSafe
                        }
                        TypeParameterConstraints12::Owned => {
                            crate::platform::kernel::TypeParameterConstraints::Owned
                        }
                    },
                })
            }
            W::EffectParameter(v) => Self::EffectParameter(v),
            W::Field(v) => Self::Field(v),
            W::Case(v) => Self::Case(v),
            W::Operation(v) => Self::Operation(v),
            W::Parameter(v) => Self::Parameter(v),
            W::Requirement(v) => Self::Requirement(v),
            W::Port(v) => Self::Port(v),
            W::RequirementParameter(v) => Self::RequirementParameter(v),
        }
    }
}

impl TryFrom<PackageInterfaceRecord> for PackageInterfaceRecord12 {
    type Error = Diagnostic;
    fn try_from(record: PackageInterfaceRecord) -> Result<Self, Self::Error> {
        use PackageInterfaceRecord as C;
        Ok(match record {
            C::Declaration(v) => Self::Declaration(v.try_into()?),
            C::TypeParameter(v) => Self::TypeParameter(TypeParameterRecord12 {
                header: v.header,
                declaration: v.declaration,
                name: v.name,
                constraints: match v.constraints {
                    crate::platform::kernel::TypeParameterConstraints::None => {
                        TypeParameterConstraints12::None
                    }
                    crate::platform::kernel::TypeParameterConstraints::CaptureSafe => {
                        TypeParameterConstraints12::CaptureSafe
                    }
                    crate::platform::kernel::TypeParameterConstraints::Owned => {
                        TypeParameterConstraints12::Owned
                    }
                    _ => {
                        return Err(interface_corrupt(
                            "transferable constraints require interface generation 13",
                        ));
                    }
                },
            }),
            C::EffectParameter(v) => Self::EffectParameter(v),
            C::Field(v) => Self::Field(v),
            C::Case(v) => Self::Case(v),
            C::Operation(v) => Self::Operation(v),
            C::Parameter(v) => Self::Parameter(v),
            C::Requirement(v) => Self::Requirement(v),
            C::Port(v) => Self::Port(v),
            C::RequirementParameter(v) => Self::RequirementParameter(v),
        })
    }
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
            contract_version: if canonical.header().contract_version == 14 {
                10
            } else if canonical.header().contract_version < 18 {
                11
            } else if canonical.header().contract_version < 22 {
                12
            } else if canonical.header().contract_version < 26 {
                13
            } else if canonical.header().contract_version < 27 {
                14
            } else {
                PACKAGE_INTERFACE_CONTRACT_VERSION
            },
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
        if self.contract_version == 10 {
            let wire = PackageInterfaceOwner10 {
                contract_version: 10,
                record: self.record.clone().try_into()?,
            };
            let bytes = crate::platform::packed::encode(
                *b"LKJPIF10",
                "lkjscript.package-interface-owner-envelope.v10",
                &wire,
                MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
            )?;
            return Ok((PackageInterfaceOwnerDigest::of(&bytes), bytes));
        }
        if self.contract_version == 11 {
            let wire = PackageInterfaceOwner11 {
                contract_version: 11,
                record: self.record.clone().try_into()?,
            };
            let bytes = crate::platform::packed::encode(
                *b"LKJPIF11",
                "lkjscript.package-interface-owner-envelope.v11",
                &wire,
                MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
            )?;
            return Ok((PackageInterfaceOwnerDigest::of(&bytes), bytes));
        }
        if self.contract_version == 12 {
            let wire = PackageInterfaceOwner12 {
                contract_version: 12,
                record: self.record.clone().try_into()?,
            };
            let bytes = crate::platform::packed::encode(
                *b"LKJPIF12",
                "lkjscript.package-interface-owner-envelope.v12",
                &wire,
                MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
            )?;
            return Ok((PackageInterfaceOwnerDigest::of(&bytes), bytes));
        }
        if self.contract_version == 13 {
            let wire = PackageInterfaceOwner13 {
                contract_version: 13,
                record: self.record.clone().try_into()?,
            };
            let bytes = crate::platform::packed::encode(
                *b"LKJPIF13",
                "lkjscript.package-interface-owner-envelope.v13",
                &wire,
                MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
            )?;
            return Ok((PackageInterfaceOwnerDigest::of(&bytes), bytes));
        }
        if self.contract_version == 14 {
            let wire = PackageInterfaceOwner14 {
                contract_version: 14,
                record: self.record.clone().try_into()?,
            };
            let bytes = crate::platform::packed::encode(
                *b"LKJPIF14",
                "lkjscript.package-interface-owner-envelope.v14",
                &wire,
                MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
            )?;
            return Ok((PackageInterfaceOwnerDigest::of(&bytes), bytes));
        }
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
        let value: Self = if bytes.starts_with(b"LKJPIF14") {
            let wire: PackageInterfaceOwner14 = crate::platform::packed::decode(
                bytes,
                *b"LKJPIF14",
                "lkjscript.package-interface-owner-envelope.v14",
                MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
            )?;
            if wire.contract_version != 14 {
                return Err(interface_corrupt(
                    "predecessor interface envelope has a foreign generation",
                ));
            }
            Self {
                contract_version: 14,
                record: wire.record.into(),
            }
        } else if bytes.starts_with(b"LKJPIF13") {
            let wire: PackageInterfaceOwner13 = crate::platform::packed::decode(
                bytes,
                *b"LKJPIF13",
                "lkjscript.package-interface-owner-envelope.v13",
                MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
            )?;
            if wire.contract_version != 13 {
                return Err(interface_corrupt(
                    "predecessor interface envelope has a foreign generation",
                ));
            }
            Self {
                contract_version: 13,
                record: wire.record.into(),
            }
        } else if bytes.starts_with(b"LKJPIF12") {
            let wire: PackageInterfaceOwner12 = crate::platform::packed::decode(
                bytes,
                *b"LKJPIF12",
                "lkjscript.package-interface-owner-envelope.v12",
                MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
            )?;
            if wire.contract_version != 12 {
                return Err(interface_corrupt(
                    "predecessor interface envelope has a foreign generation",
                ));
            }
            Self {
                contract_version: 12,
                record: wire.record.into(),
            }
        } else if bytes.starts_with(b"LKJPIF11") {
            let wire: PackageInterfaceOwner11 = crate::platform::packed::decode(
                bytes,
                *b"LKJPIF11",
                "lkjscript.package-interface-owner-envelope.v11",
                MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
            )?;
            if wire.contract_version != 11 {
                return Err(interface_corrupt(
                    "predecessor interface envelope has a foreign generation",
                ));
            }
            Self {
                contract_version: 11,
                record: wire.record.into(),
            }
        } else if bytes.starts_with(b"LKJPIF10") {
            let wire: PackageInterfaceOwner10 = crate::platform::packed::decode(
                bytes,
                *b"LKJPIF10",
                "lkjscript.package-interface-owner-envelope.v10",
                MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
            )?;
            if wire.contract_version != 10 {
                return Err(interface_corrupt(
                    "predecessor interface envelope has a foreign generation",
                ));
            }
            Self {
                contract_version: wire.contract_version,
                record: wire.record.into(),
            }
        } else {
            crate::platform::packed::decode(
                bytes,
                PACKAGE_INTERFACE_MAGIC,
                PACKAGE_INTERFACE_ENVELOPE_DOMAIN,
                MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
            )?
        };
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
        if self.contract_version != PACKAGE_INTERFACE_CONTRACT_VERSION
            && self.contract_version != 10
            && self.contract_version != 11
            && self.contract_version != 12
            && self.contract_version != 13
            && self.contract_version != 14
        {
            return Err(interface_error(
                DiagnosticClass::Source,
                "package_interface_contract",
                "package-interface owner uses a predecessor or foreign contract",
            ));
        }
        if self.contract_version < 15
            && matches!(&self.record, PackageInterfaceRecord::Declaration(d) if match &d.payload {
                PackageInterfaceDeclarationPayload::Function(f) => f.result_borrow.is_some(),
                PackageInterfaceDeclarationPayload::OwnedContract(c) => c.methods.iter().any(|m| m.result_borrow.is_some()),
                _ => false,
            })
        {
            return Err(interface_corrupt(
                "source-tied borrowed results require interface generation 15",
            ));
        }
        if self.contract_version < 13
            && matches!(&self.record,
            PackageInterfaceRecord::TypeParameter(p) if p.constraints.requires_transfer())
        {
            return Err(interface_corrupt(
                "transferable constraints require interface generation 13",
            ));
        }
        if self.contract_version < 14
            && matches!(&self.record, PackageInterfaceRecord::Declaration(d) if match &d.payload {
                PackageInterfaceDeclarationPayload::OwnedContract(c) => !c.type_parameters.is_empty(),
                PackageInterfaceDeclarationPayload::OwnedImplementation(i) => !i.type_arguments.is_empty(),
                PackageInterfaceDeclarationPayload::Function(f) => f.implementation_parameters.iter().any(|p| !p.type_arguments.is_empty()),
                _ => false,
            })
        {
            return Err(interface_corrupt(
                "owned contract parameters and arguments require interface generation 14",
            ));
        }
        self.record.validate_local()
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
    if !crate::platform::kernel::contract::supported_graph_contract(graph_contract_version) {
        return Err(interface_error(
            DiagnosticClass::Source,
            "package_interface_graph_contract",
            "unsupported interface graph generation",
        ));
    }
    let identity = PackageInterfaceIdentity {
        contract_version: 1,
        graph_contract_version,
        package,
        owners,
    };
    let bytes = crate::platform::packed::encode(
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
            DeclarationPayload::OwnedImplementation(_) => {}
            DeclarationPayload::Record { fields, .. } => self.fields.extend(fields),
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
    if root.entries() > MAXIMUM_PACKAGE_INTERFACE_VALIDATION_WORK as u64 {
        return Err(interface_error(
            DiagnosticClass::Resource,
            "package_interface_owner_work",
            "package-interface owner count exceeds the current explicit validation work budget",
        ));
    }
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
    // Reserve owner/child-relation validation visits before traversing retained interface records.
    // Canonical byte reads are charged separately by the caller's immutable source admission.
    for owner in owners.values() {
        visit(interface_owner_validation_visits(owner))?;
    }
    let (type_objects, type_keys) = validate_type_closure(package, &owners, store, work, visit)?;
    for object in type_objects.values() {
        visit(1)?;
        if let TypeForm::TaskFunction { effect, .. } = &object.form {
            visit(effect.requirements.len() as u64)?;
        }
    }
    validate_owner_closure(package, &owners, &type_objects)?;
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
        PackageInterfaceRecord::Declaration(declaration) => match &declaration.payload {
            PackageInterfaceDeclarationPayload::OwnedContract(c) => {
                c.methods
                    .iter()
                    .map(|m| {
                        let row = m.effect.row();
                        m.parameters.len() + row.requirements.len() + row.parameters.len() + 1
                    })
                    .sum::<usize>()
                    + c.type_parameters.len()
                    + 1
            }
            PackageInterfaceDeclarationPayload::OwnedImplementation(i) => {
                i.methods.len() + i.type_arguments.len() + 1
            }
            PackageInterfaceDeclarationPayload::Record {
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
        },
        PackageInterfaceRecord::Operation(operation) => operation.parameters.len(),
        PackageInterfaceRecord::Requirement(requirement) => 1 + requirement.operations.len(),
        PackageInterfaceRecord::RequirementParameter(parameter) => {
            1 + parameter.constraint.operations.len()
        }
        _ => 0,
    };
    1 + children as u64
}

fn validate_owner_closure(
    package: PackageId,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
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
                    if p.constraints != crate::platform::kernel::TypeParameterConstraints::Owned
                        || !parameter_names.insert(&p.name)
                    {
                        return Err(interface_corrupt(
                            "owned contract parameters require distinct names and the exact Owned constraint",
                        ));
                    }
                    if *parameter != c.self_parameter && p.header.contract_version < 26 {
                        return Err(interface_error(
                            DiagnosticClass::Semantic,
                            "kernel_parameterized_contract_generation",
                            "additional owned contract parameter owners require Graph 26",
                        ));
                    }
                }
                for method in &c.methods {
                    if let Some(position) = method.result_borrow {
                        let Some(source) = method.parameters.get(position as usize) else {
                            return Err(interface_corrupt(
                                "borrowed method result source position is absent",
                            ));
                        };
                        if source.use_mode != crate::platform::kernel::ParameterUse::Borrow
                            || !interface_owned_type(source.ty, types, owners)
                            || !interface_owned_type(method.result, types, owners)
                        {
                            return Err(interface_corrupt(
                                "borrowed method result requires an Owned result and borrowed Owned source",
                            ));
                        }
                    }
                    if declaration.header.contract_version < 26 {
                        for ty in method
                            .parameters
                            .iter()
                            .map(|p| p.ty)
                            .chain([method.result])
                        {
                            let Some(object) = types.get(&ty) else {
                                return Err(interface_corrupt(
                                    "owned method signature type is absent",
                                ));
                            };
                            if matches!(
                                object.form,
                                TypeForm::ByteBuffer
                                    | TypeForm::OwnedI64Cell
                                    | TypeForm::OwnedProduct { .. }
                                    | TypeForm::OwnedChoice { .. }
                                    | TypeForm::OwnedSequence { .. }
                            ) {
                                return Err(interface_error(
                                    DiagnosticClass::Semantic,
                                    "kernel_parameterized_contract_generation",
                                    "structural owned method signatures require Graph 26",
                                ));
                            }
                        }
                    }
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
            PackageInterfaceDeclarationPayload::OwnedImplementation(_) => {}
            PackageInterfaceDeclarationPayload::Record {
                fields,
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
                if let Some(source) = signature.result_borrow {
                    let Some(PackageInterfaceOwner {
                        record: PackageInterfaceRecord::Parameter(parameter),
                        ..
                    }) = owners.get(&OwnerKey::Parameter(source))
                    else {
                        return Err(interface_corrupt(
                            "borrowed function result source parameter is absent",
                        ));
                    };
                    if parameter.use_mode != crate::platform::kernel::ParameterUse::Borrow
                        || !interface_owned_type(parameter.ty, types, owners)
                        || !interface_owned_type(signature.result, types, owners)
                    {
                        return Err(interface_corrupt(
                            "borrowed function result requires an Owned result and borrowed Owned source",
                        ));
                    }
                }
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
    ty: TypeObjectDigest,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>,
) -> bool {
    match types.get(&ty).map(|object| &object.form) {
        Some(
            TypeForm::ByteBuffer
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
        _ => false,
    }
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
    visit: &mut dyn FnMut(u64) -> Result<(), Diagnostic>,
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
        visit(1)?;
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
        if matches!(object.form, TypeForm::OwnedSequence { .. })
            && (owners
                .get(&source)
                .is_some_and(|owner| owner.record.header().contract_version < 25)
                || semantic_declaration(source, owners)
                    .and_then(|declaration| owners.get(&OwnerKey::Declaration(declaration)))
                    .is_some_and(|owner| owner.record.header().contract_version < 25))
        {
            return Err(interface_error(
                DiagnosticClass::Semantic,
                "kernel_sequence_generation",
                "owned sequence interface type closure requires Graph 25 owners",
            ));
        }
        if matches!(object.form, TypeForm::OwnedChoice { .. })
            && owners
                .get(&source)
                .is_some_and(|owner| owner.record.header().contract_version < 20)
        {
            return Err(interface_error(
                DiagnosticClass::Semantic,
                "kernel_choice_generation",
                "owned choice interface type closure requires Graph 20 owners",
            ));
        }
        if matches!(object.form, TypeForm::OwnedProduct { .. })
            && owners
                .get(&source)
                .is_some_and(|owner| owner.record.header().contract_version < 19)
        {
            return Err(interface_error(
                DiagnosticClass::Semantic,
                "kernel_product_generation",
                "owned product interface type closure requires Graph 19 owners",
            ));
        }
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
            if !matches!(value.kind(), OwnerKind::Record | OwnerKind::Variant) {
                return Err(interface_error(
                    DiagnosticClass::Semantic,
                    "package_interface_named_type_kind",
                    "public signature names a declaration that is not a record or variant",
                ));
            }
            if let PackageInterfaceRecord::Declaration(record) = &value.record {
                let parameters = match &record.payload {
                    PackageInterfaceDeclarationPayload::Record {
                        type_parameters, ..
                    }
                    | PackageInterfaceDeclarationPayload::Variant {
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

    #[test]
    fn interface15_preserves_borrowed_result_relationship_and_rejects_erasure() {
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
                    result: ty,
                    result_borrow: Some(parameter),
                    effect: FunctionEffect::Pure,
                }),
            }),
        };
        let (digest, bytes) = original.encode().unwrap();
        assert_eq!(&bytes[..8], b"LKJPIF15");
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
        f.result_borrow = None;
        assert_ne!(erased.encode().unwrap().0, digest);
        let mut predecessor = original;
        predecessor.contract_version = 14;
        assert!(
            predecessor
                .encode()
                .unwrap_err()
                .message
                .contains("generation 15")
        );
    }

    #[test]
    fn interface14_literal_function_layout_remains_exact() {
        use crate::platform::kernel::*;
        let seed = b"literal-interface14-function";
        let declaration = DeclarationId::migrate(seed, 0);
        let owner = OwnerKey::Declaration(declaration);
        let header = OwnerHeader {
            contract_version: 26,
            owner,
            kind: OwnerKind::PureFunction,
        };
        let name = Name::new("view").unwrap();
        let result = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
            .unwrap()
            .0;
        let literal = crate::platform::packed::encode(
            *b"LKJPIF14",
            "lkjscript.package-interface-owner-envelope.v14",
            &(
                14_u16,
                0_u32,
                header,
                &name,
                4_u32,
                Vec::<ImplementationParameter>::new(),
                Vec::<crate::platform::semantic_id::RequirementParameterId>::new(),
                Vec::<crate::platform::semantic_id::EffectParameterId>::new(),
                Vec::<TypeParameterId>::new(),
                Vec::<ParameterId>::new(),
                result,
                FunctionEffect::Pure,
            ),
            MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
        )
        .unwrap();
        let digest = PackageInterfaceOwnerDigest::of(&literal);
        let original = PackageInterfaceOwner::decode(&literal, owner, digest).unwrap();
        assert_eq!(original.encode().unwrap(), (digest, literal));
        let PackageInterfaceRecord::Declaration(d) = original.record else {
            unreachable!()
        };
        let PackageInterfaceDeclarationPayload::Function(f) = d.payload else {
            unreachable!()
        };
        assert_eq!(f.result_borrow, None);
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
                                result: ty,
                                result_borrow: Some(parameter),
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
    fn interface14_preserves_literal_interface13_contract_layout() {
        use crate::platform::kernel::*;
        use crate::platform::semantic_id::MethodId;
        let seed = b"frozen-interface-contract";
        let owner = OwnerKey::Declaration(DeclarationId::migrate(seed, 0));
        let header = OwnerHeader {
            contract_version: 25,
            owner,
            kind: OwnerKind::OwnedContract,
        };
        let name = Name::new("Collection").unwrap();
        let self_parameter = TypeParameterId::migrate(seed, 0);
        let result = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
            .unwrap()
            .0;
        let methods = [OwnedMethod {
            id: MethodId::migrate(seed, 0),
            name: Name::new("length").unwrap(),
            parameters: Vec::new(),
            result,
            result_borrow: None,
            effect: FunctionEffect::Pure,
        }];

        let original_methods = methods
            .iter()
            .map(|method| {
                (
                    &method.id,
                    &method.name,
                    &method.parameters,
                    &method.result,
                    &method.effect,
                )
            })
            .collect::<Vec<_>>();
        // Original interface and payload ordinals, with the original two
        // contract fields, bypass both current and frozen Rust wire records.
        let original = crate::platform::packed::encode(
            *b"LKJPIF13",
            "lkjscript.package-interface-owner-envelope.v13",
            &(
                13_u16,
                0_u32,
                header,
                &name,
                5_u32,
                self_parameter,
                &original_methods,
            ),
            MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
        )
        .unwrap();
        let digest = PackageInterfaceOwnerDigest::of(&original);
        let mut value = PackageInterfaceOwner::decode(&original, owner, digest).unwrap();
        assert_eq!(value.encode().unwrap(), (digest, original));
        let PackageInterfaceRecord::Declaration(d) = &mut value.record else {
            panic!("declaration");
        };
        let PackageInterfaceDeclarationPayload::OwnedContract(c) = &mut d.payload else {
            panic!("contract");
        };
        c.type_parameters.push(TypeParameterId::migrate(seed, 1));
        d.header.contract_version = 26;
        assert!(value.encode().is_err());
        value.contract_version = 14;
        let (digest, bytes) = value.encode().unwrap();
        assert_eq!(&bytes[..8], b"LKJPIF14");
        assert_eq!(
            PackageInterfaceOwner::decode(&bytes, owner, digest).unwrap(),
            value
        );
    }

    #[test]
    fn structural_self_only_contracts_cannot_acquire_predecessor_interface_authority() {
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
                    contract_version: 13,
                    record: PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                        header: OwnerHeader {
                            contract_version: 25,
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
                                result: product_type,
                                result_borrow: None,
                                effect: FunctionEffect::Pure,
                            }],
                        }),
                    }),
                },
            ),
            (
                OwnerKey::TypeParameter(self_parameter),
                PackageInterfaceOwner {
                    contract_version: 13,
                    record: PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
                        header: OwnerHeader {
                            contract_version: 25,
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
        assert_eq!(
            validate_owner_closure(package, &owners, &types)
                .unwrap_err()
                .code,
            "kernel_parameterized_contract_generation"
        );
        let contract_owner = owners.get_mut(&owner).unwrap();
        contract_owner.contract_version = 14;
        let PackageInterfaceRecord::Declaration(d) = &mut contract_owner.record else {
            panic!("declaration");
        };
        d.header.contract_version = 26;
        assert!(validate_owner_closure(package, &owners, &types).is_ok());
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
                contract_version: 13,
                record: PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
                    header: OwnerHeader {
                        contract_version: 25,
                        owner: OwnerKey::TypeParameter(extra),
                        kind: OwnerKind::TypeParameter,
                    },
                    declaration,
                    name: Name::new("Item").unwrap(),
                    constraints: TypeParameterConstraints::Owned,
                }),
            },
        );
        assert_eq!(
            validate_owner_closure(package, &owners, &types)
                .unwrap_err()
                .code,
            "kernel_parameterized_contract_generation"
        );
        let extra_owner = owners.get_mut(&OwnerKey::TypeParameter(extra)).unwrap();
        extra_owner.contract_version = 14;
        let PackageInterfaceRecord::TypeParameter(p) = &mut extra_owner.record else {
            panic!("parameter");
        };
        p.header.contract_version = 26;
        assert!(validate_owner_closure(package, &owners, &types).is_ok());
    }

    #[test]
    fn transferable_interface13_preserves_frozen_interface12_and_rejects_new_tags() {
        use crate::platform::kernel::{
            Name, OwnerHeader, TypeParameterConstraints as C, TypeParameterRecord,
        };
        let owner =
            OwnerKey::TypeParameter(TypeParameterId::migrate(b"interface-transfer-bounds", 0));
        let mut parameter = TypeParameterRecord {
            header: OwnerHeader::new(owner, OwnerKind::TypeParameter),
            declaration: DeclarationId::migrate(b"interface-transfer-bounds", 0),
            name: Name::new("T").unwrap(),
            constraints: C::None,
        };
        for constraints in [C::None, C::CaptureSafe, C::Owned] {
            parameter.header.contract_version = 21;
            parameter.constraints = constraints;
            let value = PackageInterfaceOwner {
                contract_version: 12,
                record: PackageInterfaceRecord::TypeParameter(parameter.clone()),
            };
            let original = crate::platform::packed::encode(
                *b"LKJPIF12",
                "lkjscript.package-interface-owner-envelope.v12",
                &value,
                MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
            )
            .unwrap();
            let (digest, bytes) = value.encode().unwrap();
            assert_eq!(
                bytes,
                original,
                "frozen constraint tag {}",
                constraints.tag()
            );
            assert_eq!(
                PackageInterfaceOwner::decode(&bytes, owner, digest).unwrap(),
                value
            );
        }
        for constraints in [
            C::Transferable,
            C::CaptureSafeTransferable,
            C::OwnedTransferable,
        ] {
            parameter.header.contract_version = 22;
            parameter.constraints = constraints;
            let mut value = PackageInterfaceOwner {
                contract_version: 13,
                record: PackageInterfaceRecord::TypeParameter(parameter.clone()),
            };
            let (digest, bytes) = value.encode().unwrap();
            assert_eq!(&bytes[..8], b"LKJPIF13");
            assert_eq!(
                PackageInterfaceOwner::decode(&bytes, owner, digest).unwrap(),
                value
            );
            value.contract_version = 12;
            assert!(value.encode().is_err());
            let forged = crate::platform::packed::encode(
                *b"LKJPIF12",
                "lkjscript.package-interface-owner-envelope.v12",
                &value,
                MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
            )
            .unwrap();
            assert!(
                PackageInterfaceOwner::decode(
                    &forged,
                    owner,
                    PackageInterfaceOwnerDigest::of(&forged)
                )
                .is_err()
            );
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
        predecessor[..8].copy_from_slice(b"LKJPIF02");
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

    #[test]
    fn owned_sequence_interface_closure_requires_generation25_on_each_signature_owner() {
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
        for key in owners.keys() {
            let mut hostile = owners.clone();
            match &mut hostile.get_mut(key).unwrap().record {
                PackageInterfaceRecord::Declaration(record) => record.header.contract_version = 24,
                PackageInterfaceRecord::Parameter(record) => record.header.contract_version = 24,
                _ => continue,
            }
            assert_eq!(
                validate(&hostile).unwrap_err().code,
                "kernel_sequence_generation"
            );
        }
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

#[cfg(test)]
mod owned_generation_tests {
    use super::*;
    #[test]
    fn owned_generation_preserves_frozen_predecessor_interface11_layouts() {
        let bytes = include_bytes!("../../tests/fixtures/owned-legacy-interface11.lkjp");
        let transport =
            "package_transport_458b038313692227f3a3842e420e85a247ada81ca3fdcbecdf7b715191aa5f31"
                .parse()
                .unwrap();
        let container =
            crate::platform::package_transport::source::PackageContainer::decode(bytes, transport)
                .unwrap();
        let admitted = container.admit().unwrap();
        let independently_reconstructed =
            crate::platform::package_transport::oracle::reconstruct(&container).unwrap();
        assert_eq!(independently_reconstructed.snapshots.len(), 1);
        let package = &admitted.packages[&container.root.package_revision];
        assert_eq!(package.snapshot.root.graph_contract_version, 17);
        let mut kinds = BTreeSet::new();
        for interface in package.interface_owners.values() {
            assert_eq!(interface.contract_version, 11);
            let (digest, encoded) = interface.encode().unwrap();
            assert_eq!(&encoded[..8], b"LKJPIF11");
            assert_eq!(
                PackageInterfaceOwner::decode(&encoded, interface.owner(), digest).unwrap(),
                *interface
            );
            assert!(
                container.objects.values().any(|bytes| *bytes == encoded),
                "exact predecessor bytes survive re-encoding"
            );
            if let PackageInterfaceRecord::Declaration(d) = &interface.record {
                match &d.payload {
                    PackageInterfaceDeclarationPayload::Function(f) => {
                        assert!(f.implementation_parameters.is_empty());
                        kinds.insert("function");
                    }
                    PackageInterfaceDeclarationPayload::Constant { .. } => {
                        assert_eq!(d.name.as_str(), "seed");
                        kinds.insert("constant");
                    }
                    PackageInterfaceDeclarationPayload::Component { ports, .. } => {
                        assert_eq!(ports.len(), 1);
                        kinds.insert("component");
                    }
                    _ => unreachable!(),
                }
            }
        }
        assert_eq!(kinds, BTreeSet::from(["function", "constant", "component"]));
    }
}
