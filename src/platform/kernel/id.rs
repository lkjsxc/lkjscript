//! Closed Graph 8 semantic owner identity domain.

use super::contract::GRAPH_CONTRACT_VERSION;
use crate::platform::semantic_id::{
    AnnotationId, BindingId, CaseId, DeclarationId, DocumentationId, ExpressionId, FieldId,
    ModuleId, OperationId, ParameterId, PortId, RequirementId, TargetId, TypeParameterId,
};
use crate::platform::{
    diagnostic::Diagnostic, diagnostic::DiagnosticClass, semantic_id::encode_hex,
};
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum OwnerKind {
    Module,
    Record,
    Variant,
    Interface,
    External,
    PureFunction,
    TaskFunction,
    Constant,
    Component,
    Test,
    TypeParameter,
    Field,
    Case,
    Operation,
    Parameter,
    Binding,
    Expression,
    Requirement,
    Port,
    Target,
    Documentation,
    Annotation,
}

impl OwnerKind {
    pub const ALL: [Self; 22] = [
        Self::Module,
        Self::Record,
        Self::Variant,
        Self::Interface,
        Self::External,
        Self::PureFunction,
        Self::TaskFunction,
        Self::Constant,
        Self::Component,
        Self::Test,
        Self::TypeParameter,
        Self::Field,
        Self::Case,
        Self::Operation,
        Self::Parameter,
        Self::Binding,
        Self::Expression,
        Self::Requirement,
        Self::Port,
        Self::Target,
        Self::Documentation,
        Self::Annotation,
    ];

    /// Coarse durable owner kinds that survive the scoped-identity cutover and may therefore be
    /// selected by the current public exact-owner command.
    pub const PUBLIC_EXACT: [Self; 11] = [
        Self::Module,
        Self::Record,
        Self::Variant,
        Self::Interface,
        Self::External,
        Self::PureFunction,
        Self::TaskFunction,
        Self::Constant,
        Self::Component,
        Self::Test,
        Self::Target,
    ];

    pub const fn tag(self) -> u8 {
        match self {
            Self::Module => 1,
            Self::Record => 2,
            Self::Variant => 3,
            Self::Interface => 4,
            Self::External => 5,
            Self::PureFunction => 6,
            Self::TaskFunction => 7,
            Self::Constant => 8,
            Self::Component => 9,
            Self::Test => 10,
            Self::TypeParameter => 11,
            Self::Field => 12,
            Self::Case => 13,
            Self::Operation => 14,
            Self::Parameter => 15,
            Self::Binding => 16,
            Self::Expression => 17,
            Self::Requirement => 18,
            Self::Port => 19,
            Self::Target => 20,
            Self::Documentation => 21,
            Self::Annotation => 22,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Record => "record",
            Self::Variant => "variant",
            Self::Interface => "interface",
            Self::External => "external",
            Self::PureFunction => "pure_function",
            Self::TaskFunction => "task_function",
            Self::Constant => "constant",
            Self::Component => "component",
            Self::Test => "test",
            Self::TypeParameter => "type_parameter",
            Self::Field => "field",
            Self::Case => "case",
            Self::Operation => "operation",
            Self::Parameter => "parameter",
            Self::Binding => "binding",
            Self::Expression => "expression",
            Self::Requirement => "requirement",
            Self::Port => "port",
            Self::Target => "target",
            Self::Documentation => "documentation",
            Self::Annotation => "annotation",
        }
    }

    pub fn parse(value: &str) -> Result<Self, Diagnostic> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.name() == value)
            .ok_or_else(|| {
                id_error(
                    DiagnosticClass::Source,
                    "kernel_owner_kind",
                    format!("unknown semantic owner kind '{value}'"),
                )
            })
    }

    pub const fn accepts_owner(self, owner: OwnerKey) -> bool {
        matches!(
            (self, owner),
            (Self::Module, OwnerKey::Module(_))
                | (Self::Record, OwnerKey::Declaration(_))
                | (Self::Variant, OwnerKey::Declaration(_))
                | (Self::Interface, OwnerKey::Declaration(_))
                | (Self::External, OwnerKey::Declaration(_))
                | (Self::PureFunction, OwnerKey::Declaration(_))
                | (Self::TaskFunction, OwnerKey::Declaration(_))
                | (Self::Constant, OwnerKey::Declaration(_))
                | (Self::Component, OwnerKey::Declaration(_))
                | (Self::Test, OwnerKey::Declaration(_))
                | (Self::TypeParameter, OwnerKey::TypeParameter(_))
                | (Self::Field, OwnerKey::Field(_))
                | (Self::Case, OwnerKey::Case(_))
                | (Self::Operation, OwnerKey::Operation(_))
                | (Self::Parameter, OwnerKey::Parameter(_))
                | (Self::Binding, OwnerKey::Binding(_))
                | (Self::Expression, OwnerKey::Expression(_))
                | (Self::Requirement, OwnerKey::Requirement(_))
                | (Self::Port, OwnerKey::Port(_))
                | (Self::Target, OwnerKey::Target(_))
                | (Self::Documentation, OwnerKey::Documentation(_))
                | (Self::Annotation, OwnerKey::Annotation(_))
        )
    }

    pub const fn has_compilation_unit(self) -> bool {
        matches!(
            self,
            Self::Record
                | Self::Variant
                | Self::Interface
                | Self::External
                | Self::PureFunction
                | Self::TaskFunction
                | Self::Constant
                | Self::Component
                | Self::Test
                | Self::Target
        )
    }
}

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(
    tag = "domain",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OwnerKey {
    Module(ModuleId),
    Declaration(DeclarationId),
    TypeParameter(TypeParameterId),
    Field(FieldId),
    Case(CaseId),
    Operation(OperationId),
    Parameter(ParameterId),
    Binding(BindingId),
    Expression(ExpressionId),
    Requirement(RequirementId),
    Port(PortId),
    Target(TargetId),
    Documentation(DocumentationId),
    Annotation(AnnotationId),
}

impl OwnerKey {
    pub const fn identity_kind(self) -> IdentityKind {
        match self {
            Self::Module(_) => IdentityKind::Module,
            Self::Declaration(_) => IdentityKind::Declaration,
            Self::TypeParameter(_) => IdentityKind::TypeParameter,
            Self::Field(_) => IdentityKind::Field,
            Self::Case(_) => IdentityKind::Case,
            Self::Operation(_) => IdentityKind::Operation,
            Self::Parameter(_) => IdentityKind::Parameter,
            Self::Binding(_) => IdentityKind::Binding,
            Self::Expression(_) => IdentityKind::Expression,
            Self::Requirement(_) => IdentityKind::Requirement,
            Self::Port(_) => IdentityKind::Port,
            Self::Target(_) => IdentityKind::Target,
            Self::Documentation(_) => IdentityKind::Documentation,
            Self::Annotation(_) => IdentityKind::Annotation,
        }
    }

    pub const fn bytes(self) -> [u8; 16] {
        match self {
            Self::Module(id) => id.bytes(),
            Self::Declaration(id) => id.bytes(),
            Self::TypeParameter(id) => id.bytes(),
            Self::Field(id) => id.bytes(),
            Self::Case(id) => id.bytes(),
            Self::Operation(id) => id.bytes(),
            Self::Parameter(id) => id.bytes(),
            Self::Binding(id) => id.bytes(),
            Self::Expression(id) => id.bytes(),
            Self::Requirement(id) => id.bytes(),
            Self::Port(id) => id.bytes(),
            Self::Target(id) => id.bytes(),
            Self::Documentation(id) => id.bytes(),
            Self::Annotation(id) => id.bytes(),
        }
    }
}

impl fmt::Display for OwnerKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Module(value) => value.fmt(formatter),
            Self::Declaration(value) => value.fmt(formatter),
            Self::TypeParameter(value) => value.fmt(formatter),
            Self::Field(value) => value.fmt(formatter),
            Self::Case(value) => value.fmt(formatter),
            Self::Operation(value) => value.fmt(formatter),
            Self::Parameter(value) => value.fmt(formatter),
            Self::Binding(value) => value.fmt(formatter),
            Self::Expression(value) => value.fmt(formatter),
            Self::Requirement(value) => value.fmt(formatter),
            Self::Port(value) => value.fmt(formatter),
            Self::Target(value) => value.fmt(formatter),
            Self::Documentation(value) => value.fmt(formatter),
            Self::Annotation(value) => value.fmt(formatter),
        }
    }
}

impl FromStr for OwnerKey {
    type Err = Diagnostic;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.starts_with(ModuleId::PREFIX) {
            value.parse().map(Self::Module)
        } else if value.starts_with(DeclarationId::PREFIX) {
            value.parse().map(Self::Declaration)
        } else if value.starts_with(TypeParameterId::PREFIX) {
            value.parse().map(Self::TypeParameter)
        } else if value.starts_with(FieldId::PREFIX) {
            value.parse().map(Self::Field)
        } else if value.starts_with(CaseId::PREFIX) {
            value.parse().map(Self::Case)
        } else if value.starts_with(OperationId::PREFIX) {
            value.parse().map(Self::Operation)
        } else if value.starts_with(ParameterId::PREFIX) {
            value.parse().map(Self::Parameter)
        } else if value.starts_with(BindingId::PREFIX) {
            value.parse().map(Self::Binding)
        } else if value.starts_with(ExpressionId::PREFIX) {
            value.parse().map(Self::Expression)
        } else if value.starts_with(RequirementId::PREFIX) {
            value.parse().map(Self::Requirement)
        } else if value.starts_with(PortId::PREFIX) {
            value.parse().map(Self::Port)
        } else if value.starts_with(TargetId::PREFIX) {
            value.parse().map(Self::Target)
        } else if value.starts_with(DocumentationId::PREFIX) {
            value.parse().map(Self::Documentation)
        } else if value.starts_with(AnnotationId::PREFIX) {
            value.parse().map(Self::Annotation)
        } else {
            Err(id_error(
                DiagnosticClass::Source,
                "kernel_owner_identity_domain",
                "owner identity has an unknown typed identity prefix",
            ))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IdentityKind {
    Module,
    Declaration,
    TypeParameter,
    Field,
    Case,
    Operation,
    Parameter,
    Binding,
    Expression,
    Requirement,
    Port,
    Target,
    Documentation,
    Annotation,
}

impl IdentityKind {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Module => 1,
            Self::Declaration => 2,
            Self::TypeParameter => 3,
            Self::Field => 4,
            Self::Case => 5,
            Self::Operation => 6,
            Self::Parameter => 7,
            Self::Binding => 8,
            Self::Expression => 9,
            Self::Requirement => 10,
            Self::Port => 11,
            Self::Target => 12,
            Self::Documentation => 13,
            Self::Annotation => 14,
        }
    }
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerHeader {
    pub contract_version: u16,
    pub owner: OwnerKey,
    pub kind: OwnerKind,
}

impl OwnerHeader {
    pub const fn new(owner: OwnerKey, kind: OwnerKind) -> Self {
        Self {
            contract_version: GRAPH_CONTRACT_VERSION,
            owner,
            kind,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EncodedOwnerKey([u8; 17]);

impl EncodedOwnerKey {
    pub const fn new(owner: OwnerKey) -> Self {
        let source = owner.bytes();
        let mut bytes = [0_u8; 17];
        bytes[0] = owner.identity_kind().tag();
        let mut index = 0;
        while index < source.len() {
            bytes[index + 1] = source[index];
            index += 1;
        }
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 17] {
        self.0
    }

    /// Strictly decodes the canonical tagged owner-key representation used by Graph 8 maps and
    /// witness entries.
    pub fn decode(bytes: &[u8]) -> Result<OwnerKey, Diagnostic> {
        let encoded: [u8; 17] = bytes.try_into().map_err(|_| {
            id_error(
                DiagnosticClass::Corrupt,
                "kernel_owner_key_length",
                "encoded owner key must contain one domain tag and 16 identity bytes",
            )
        })?;
        let mut identity = [0_u8; 16];
        identity.copy_from_slice(&encoded[1..]);
        let owner = match encoded[0] {
            1 => ModuleId::from_bytes(identity).map(OwnerKey::Module),
            2 => DeclarationId::from_bytes(identity).map(OwnerKey::Declaration),
            3 => TypeParameterId::from_bytes(identity).map(OwnerKey::TypeParameter),
            4 => FieldId::from_bytes(identity).map(OwnerKey::Field),
            5 => CaseId::from_bytes(identity).map(OwnerKey::Case),
            6 => OperationId::from_bytes(identity).map(OwnerKey::Operation),
            7 => ParameterId::from_bytes(identity).map(OwnerKey::Parameter),
            8 => BindingId::from_bytes(identity).map(OwnerKey::Binding),
            9 => ExpressionId::from_bytes(identity).map(OwnerKey::Expression),
            10 => RequirementId::from_bytes(identity).map(OwnerKey::Requirement),
            11 => PortId::from_bytes(identity).map(OwnerKey::Port),
            12 => TargetId::from_bytes(identity).map(OwnerKey::Target),
            13 => DocumentationId::from_bytes(identity).map(OwnerKey::Documentation),
            14 => AnnotationId::from_bytes(identity).map(OwnerKey::Annotation),
            tag => {
                return Err(id_error(
                    DiagnosticClass::Corrupt,
                    "kernel_owner_key_domain",
                    format!("encoded owner key contains unknown identity-domain tag {tag}"),
                ));
            }
        };
        owner.ok_or_else(|| {
            id_error(
                DiagnosticClass::Corrupt,
                "kernel_owner_key_zero",
                "encoded owner key contains the reserved all-zero identity",
            )
        })
    }
}

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(deny_unknown_fields)]
pub struct ExactOwnerKey {
    pub package: PackageId,
    pub owner: OwnerKey,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PackageId([u8; 16]);

impl PackageId {
    pub const PREFIX: &'static str = "pkg_";
    const BINARY_TAG: u8 = 18;

    pub fn generate() -> Result<Self, Diagnostic> {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| {
            id_error(
                DiagnosticClass::Infrastructure,
                "kernel_package_identity_entropy",
                "operating-system entropy is unavailable",
            )
        })?;
        if bytes == [0; 16] {
            bytes[15] = 1;
        }
        Ok(Self(bytes))
    }

    pub fn migrate(seed: &[u8], ordinal: u64) -> Self {
        let mut hasher =
            blake3::Hasher::new_derive_key(super::contract::PACKAGE_ID_MIGRATION_DOMAIN);
        hasher.update(&(seed.len() as u64).to_be_bytes());
        hasher.update(seed);
        hasher.update(&ordinal.to_be_bytes());
        let mut bytes = [0_u8; 16];
        bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
        if bytes == [0; 16] {
            bytes[15] = 1;
        }
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Option<Self> {
        if bytes == [0; 16] {
            None
        } else {
            Some(Self(bytes))
        }
    }

    pub const fn bytes(self) -> [u8; 16] {
        self.0
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(Self::PREFIX)?;
        formatter.write_str(&encode_hex(&self.0))
    }
}

impl FromStr for PackageId {
    type Err = Diagnostic;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let encoded = value.strip_prefix(Self::PREFIX).ok_or_else(|| {
            id_error(
                DiagnosticClass::Source,
                "kernel_package_identity_domain",
                format!("package identity must start with '{}'", Self::PREFIX),
            )
        })?;
        if encoded.len() != 32 {
            return Err(id_error(
                DiagnosticClass::Source,
                "kernel_package_identity_length",
                "package identity must contain 32 lowercase hexadecimal characters",
            ));
        }
        let mut bytes = [0_u8; 16];
        for (index, pair) in encoded.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let high = decode_hex(pair[0]).ok_or_else(|| invalid_package_hex(encoded))?;
            let low = decode_hex(pair[1]).ok_or_else(|| invalid_package_hex(encoded))?;
            bytes[index] = (high << 4) | low;
        }
        Self::from_bytes(bytes).ok_or_else(|| {
            id_error(
                DiagnosticClass::Source,
                "kernel_package_identity_zero",
                "all-zero package identity is reserved",
            )
        })
    }
}

impl Serialize for PackageId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PackageId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

impl Encode for PackageId {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        Self::BINARY_TAG.encode(encoder)?;
        self.0.encode(encoder)
    }
}

impl<Context> Decode<Context> for PackageId {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let tag = u8::decode(decoder)?;
        if tag != Self::BINARY_TAG {
            return Err(DecodeError::OtherString(format!(
                "foreign package identity domain tag {tag}; expected {}",
                Self::BINARY_TAG
            )));
        }
        let bytes = <[u8; 16]>::decode(decoder)?;
        Self::from_bytes(bytes).ok_or_else(|| {
            DecodeError::OtherString("all-zero package identity is reserved".to_owned())
        })
    }
}

bincode::impl_borrow_decode!(PackageId);

fn decode_hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn invalid_package_hex(value: &str) -> Diagnostic {
    id_error(
        DiagnosticClass::Source,
        "kernel_package_identity_hex",
        format!("package identity '{value}' is not canonical lowercase hexadecimal"),
    )
}

fn id_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
