//! Scoped semantic identities and canonical keys for the package-wide scoped-record map.
//!
//! These types are deliberately private while the normalized authority is cut over. Local
//! tokens carry continuity only inside an exact declaration, operation, or executable body.
//! Physical persistent-map pages never participate in token or section identity.

use super::{ChangeDigest, PackageId};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{DeclarationId, encode_hex};
use bincode::de::Decoder;
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{Decode, Encode};
use std::fmt;
use std::str::FromStr;

const LOCAL_TOKEN_BYTES: usize = 16;
const LOCAL_TOKEN_HEX_BYTES: usize = LOCAL_TOKEN_BYTES * 2;
const SCOPED_RECORD_KEY_VERSION: u8 = 1;
const DECLARATION_SCOPE_TAG: u8 = 1;
const OPERATION_SCOPE_TAG: u8 = 2;
const BODY_SCOPE_TAG: u8 = 3;
const EXACT_SCOPE_TAG: u8 = 4;
const EXACT_KEY_TAG: u8 = 1;
const BODY_COMMITMENT_TAG: u8 = 1;
const ALLOCATION_DIGEST_DOMAIN: &str = "lkjscript.scoped-token-allocation.v1";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct LocalTokenBytes([u8; LOCAL_TOKEN_BYTES]);

impl LocalTokenBytes {
    fn from_bytes(bytes: [u8; LOCAL_TOKEN_BYTES]) -> Option<Self> {
        (bytes != [0; LOCAL_TOKEN_BYTES]).then_some(Self(bytes))
    }

    const fn bytes(self) -> [u8; LOCAL_TOKEN_BYTES] {
        self.0
    }

    fn allocate(
        parent_scope: ExactScopedRecordScope,
        normalized_request_digest: ChangeDigest,
        domain: ScopedSectionDomain,
        ordinal: u64,
        collision_counter: u32,
    ) -> Result<Self, Diagnostic> {
        validate_scope_domain(parent_scope.scope, domain)?;
        let parent_scope_bytes = parent_scope.allocation_bytes();
        let mut hasher = blake3::Hasher::new_derive_key(ALLOCATION_DIGEST_DOMAIN);
        hasher.update(&(parent_scope_bytes.len() as u64).to_be_bytes());
        hasher.update(&parent_scope_bytes);
        hasher.update(&normalized_request_digest.bytes());
        hasher.update(&[domain.tag()]);
        hasher.update(&ordinal.to_be_bytes());
        hasher.update(&collision_counter.to_be_bytes());
        let mut bytes = [0_u8; LOCAL_TOKEN_BYTES];
        bytes.copy_from_slice(&hasher.finalize().as_bytes()[..LOCAL_TOKEN_BYTES]);
        if bytes == [0; LOCAL_TOKEN_BYTES] {
            bytes[LOCAL_TOKEN_BYTES - 1] = 1;
        }
        Ok(Self(bytes))
    }

    fn parse(value: &str) -> Result<Self, Diagnostic> {
        if value.len() != LOCAL_TOKEN_HEX_BYTES {
            return Err(scoped_error(
                DiagnosticClass::Source,
                "scoped_token_length",
                format!(
                    "scoped token must contain {LOCAL_TOKEN_HEX_BYTES} lowercase hexadecimal characters"
                ),
            ));
        }
        let mut bytes = [0_u8; LOCAL_TOKEN_BYTES];
        for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let high = decode_hex(pair[0]).ok_or_else(|| invalid_token_hex(value))?;
            let low = decode_hex(pair[1]).ok_or_else(|| invalid_token_hex(value))?;
            bytes[index] = (high << 4) | low;
        }
        Self::from_bytes(bytes).ok_or_else(|| {
            scoped_error(
                DiagnosticClass::Source,
                "scoped_token_zero",
                "all-zero scoped token is reserved",
            )
        })
    }
}

macro_rules! scoped_token {
    ($name:ident, $prefix:literal, $domain:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub(crate) struct $name(LocalTokenBytes);

        impl $name {
            pub(crate) const PREFIX: &'static str = $prefix;
            pub(crate) const DOMAIN: ScopedSectionDomain = ScopedSectionDomain::$domain;

            pub(crate) fn allocate(
                parent_scope: ExactScopedRecordScope,
                normalized_request_digest: ChangeDigest,
                ordinal: u64,
                collision_counter: u32,
            ) -> Result<Self, Diagnostic> {
                LocalTokenBytes::allocate(
                    parent_scope,
                    normalized_request_digest,
                    Self::DOMAIN,
                    ordinal,
                    collision_counter,
                )
                .map(Self)
            }

            pub(crate) fn from_bytes(bytes: [u8; LOCAL_TOKEN_BYTES]) -> Option<Self> {
                LocalTokenBytes::from_bytes(bytes).map(Self)
            }

            pub(crate) const fn bytes(self) -> [u8; LOCAL_TOKEN_BYTES] {
                self.0.bytes()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(Self::PREFIX)?;
                formatter.write_str(&encode_hex(&self.bytes()))
            }
        }

        impl FromStr for $name {
            type Err = Diagnostic;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let encoded = value.strip_prefix(Self::PREFIX).ok_or_else(|| {
                    scoped_error(
                        DiagnosticClass::Source,
                        "scoped_token_domain",
                        format!(
                            "scoped token belongs to a foreign domain; expected prefix '{}'",
                            Self::PREFIX
                        ),
                    )
                })?;
                LocalTokenBytes::parse(encoded).map(Self)
            }
        }

        impl Encode for $name {
            fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
                Self::DOMAIN.tag().encode(encoder)?;
                self.bytes().encode(encoder)
            }
        }

        impl<Context> Decode<Context> for $name {
            fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
                let observed = u8::decode(decoder)?;
                if observed != Self::DOMAIN.tag() {
                    return Err(DecodeError::OtherString(format!(
                        "foreign scoped token domain tag {observed}; expected {}",
                        Self::DOMAIN.tag()
                    )));
                }
                let bytes = <[u8; LOCAL_TOKEN_BYTES]>::decode(decoder)?;
                Self::from_bytes(bytes).ok_or_else(|| {
                    DecodeError::OtherString("all-zero scoped token is reserved".to_owned())
                })
            }
        }

        bincode::impl_borrow_decode!($name);

        impl ScopedTokenType for $name {
            const DOMAIN: ScopedSectionDomain = ScopedSectionDomain::$domain;
        }
    };
}

scoped_token!(TypeParameterToken, "stp_", TypeParameter);
scoped_token!(FieldToken, "sfld_", Field);
scoped_token!(CaseToken, "scase_", Case);
scoped_token!(InterfaceOperationToken, "siop_", InterfaceOperation);
scoped_token!(FunctionParameterToken, "sfparam_", FunctionParameter);
scoped_token!(OperationParameterToken, "soparam_", OperationParameter);
scoped_token!(RequirementToken, "sreq_", Requirement);
scoped_token!(PortToken, "sport_", Port);
scoped_token!(ExpressionToken, "sexpr_", Expression);
scoped_token!(BindingToken, "sbind_", Binding);

pub(crate) trait ScopedTokenType: Copy + Eq {
    const DOMAIN: ScopedSectionDomain;
}

pub(crate) trait OrderedScopedToken: ScopedTokenType {}

impl OrderedScopedToken for TypeParameterToken {}
impl OrderedScopedToken for FieldToken {}
impl OrderedScopedToken for CaseToken {}
impl OrderedScopedToken for InterfaceOperationToken {}
impl OrderedScopedToken for FunctionParameterToken {}
impl OrderedScopedToken for OperationParameterToken {}
impl OrderedScopedToken for RequirementToken {}
impl OrderedScopedToken for PortToken {}

/// Closed identity section domains in the package-wide scoped-record map.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum ScopedSectionDomain {
    TypeParameter,
    Field,
    Case,
    InterfaceOperation,
    FunctionParameter,
    OperationParameter,
    Requirement,
    Port,
    Expression,
    Binding,
}

impl ScopedSectionDomain {
    pub(crate) const ALL: [Self; 10] = [
        Self::TypeParameter,
        Self::Field,
        Self::Case,
        Self::InterfaceOperation,
        Self::FunctionParameter,
        Self::OperationParameter,
        Self::Requirement,
        Self::Port,
        Self::Expression,
        Self::Binding,
    ];

    pub(crate) const fn tag(self) -> u8 {
        match self {
            Self::TypeParameter => 33,
            Self::Field => 34,
            Self::Case => 35,
            Self::InterfaceOperation => 36,
            Self::FunctionParameter => 37,
            Self::OperationParameter => 38,
            Self::Requirement => 39,
            Self::Port => 40,
            Self::Expression => 41,
            Self::Binding => 42,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, DecodeError> {
        match tag {
            33 => Ok(Self::TypeParameter),
            34 => Ok(Self::Field),
            35 => Ok(Self::Case),
            36 => Ok(Self::InterfaceOperation),
            37 => Ok(Self::FunctionParameter),
            38 => Ok(Self::OperationParameter),
            39 => Ok(Self::Requirement),
            40 => Ok(Self::Port),
            41 => Ok(Self::Expression),
            42 => Ok(Self::Binding),
            _ => Err(DecodeError::OtherString(format!(
                "unknown scoped section domain tag {tag}"
            ))),
        }
    }
}

impl Encode for ScopedSectionDomain {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.tag().encode(encoder)
    }
}

impl<Context> Decode<Context> for ScopedSectionDomain {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        Self::from_tag(u8::decode(decoder)?)
    }
}

bincode::impl_borrow_decode!(ScopedSectionDomain);

/// One closed local-token union. Its binary form is exactly a domain tag and 128-bit payload.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum ScopedToken {
    TypeParameter(TypeParameterToken),
    Field(FieldToken),
    Case(CaseToken),
    InterfaceOperation(InterfaceOperationToken),
    FunctionParameter(FunctionParameterToken),
    OperationParameter(OperationParameterToken),
    Requirement(RequirementToken),
    Port(PortToken),
    Expression(ExpressionToken),
    Binding(BindingToken),
}

impl ScopedToken {
    pub(crate) const fn domain(self) -> ScopedSectionDomain {
        match self {
            Self::TypeParameter(_) => ScopedSectionDomain::TypeParameter,
            Self::Field(_) => ScopedSectionDomain::Field,
            Self::Case(_) => ScopedSectionDomain::Case,
            Self::InterfaceOperation(_) => ScopedSectionDomain::InterfaceOperation,
            Self::FunctionParameter(_) => ScopedSectionDomain::FunctionParameter,
            Self::OperationParameter(_) => ScopedSectionDomain::OperationParameter,
            Self::Requirement(_) => ScopedSectionDomain::Requirement,
            Self::Port(_) => ScopedSectionDomain::Port,
            Self::Expression(_) => ScopedSectionDomain::Expression,
            Self::Binding(_) => ScopedSectionDomain::Binding,
        }
    }

    pub(crate) const fn bytes(self) -> [u8; LOCAL_TOKEN_BYTES] {
        match self {
            Self::TypeParameter(token) => token.bytes(),
            Self::Field(token) => token.bytes(),
            Self::Case(token) => token.bytes(),
            Self::InterfaceOperation(token) => token.bytes(),
            Self::FunctionParameter(token) => token.bytes(),
            Self::OperationParameter(token) => token.bytes(),
            Self::Requirement(token) => token.bytes(),
            Self::Port(token) => token.bytes(),
            Self::Expression(token) => token.bytes(),
            Self::Binding(token) => token.bytes(),
        }
    }

    fn from_domain_bytes(
        domain: ScopedSectionDomain,
        bytes: [u8; LOCAL_TOKEN_BYTES],
    ) -> Result<Self, DecodeError> {
        macro_rules! token {
            ($type:ident, $variant:ident) => {
                $type::from_bytes(bytes).map(Self::$variant).ok_or_else(|| {
                    DecodeError::OtherString("all-zero scoped token is reserved".to_owned())
                })
            };
        }
        match domain {
            ScopedSectionDomain::TypeParameter => token!(TypeParameterToken, TypeParameter),
            ScopedSectionDomain::Field => token!(FieldToken, Field),
            ScopedSectionDomain::Case => token!(CaseToken, Case),
            ScopedSectionDomain::InterfaceOperation => {
                token!(InterfaceOperationToken, InterfaceOperation)
            }
            ScopedSectionDomain::FunctionParameter => {
                token!(FunctionParameterToken, FunctionParameter)
            }
            ScopedSectionDomain::OperationParameter => {
                token!(OperationParameterToken, OperationParameter)
            }
            ScopedSectionDomain::Requirement => token!(RequirementToken, Requirement),
            ScopedSectionDomain::Port => token!(PortToken, Port),
            ScopedSectionDomain::Expression => token!(ExpressionToken, Expression),
            ScopedSectionDomain::Binding => token!(BindingToken, Binding),
        }
    }
}

impl fmt::Display for ScopedToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TypeParameter(token) => token.fmt(formatter),
            Self::Field(token) => token.fmt(formatter),
            Self::Case(token) => token.fmt(formatter),
            Self::InterfaceOperation(token) => token.fmt(formatter),
            Self::FunctionParameter(token) => token.fmt(formatter),
            Self::OperationParameter(token) => token.fmt(formatter),
            Self::Requirement(token) => token.fmt(formatter),
            Self::Port(token) => token.fmt(formatter),
            Self::Expression(token) => token.fmt(formatter),
            Self::Binding(token) => token.fmt(formatter),
        }
    }
}

impl FromStr for ScopedToken {
    type Err = Diagnostic;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.starts_with(TypeParameterToken::PREFIX) {
            value.parse().map(Self::TypeParameter)
        } else if value.starts_with(FieldToken::PREFIX) {
            value.parse().map(Self::Field)
        } else if value.starts_with(CaseToken::PREFIX) {
            value.parse().map(Self::Case)
        } else if value.starts_with(InterfaceOperationToken::PREFIX) {
            value.parse().map(Self::InterfaceOperation)
        } else if value.starts_with(FunctionParameterToken::PREFIX) {
            value.parse().map(Self::FunctionParameter)
        } else if value.starts_with(OperationParameterToken::PREFIX) {
            value.parse().map(Self::OperationParameter)
        } else if value.starts_with(RequirementToken::PREFIX) {
            value.parse().map(Self::Requirement)
        } else if value.starts_with(PortToken::PREFIX) {
            value.parse().map(Self::Port)
        } else if value.starts_with(ExpressionToken::PREFIX) {
            value.parse().map(Self::Expression)
        } else if value.starts_with(BindingToken::PREFIX) {
            value.parse().map(Self::Binding)
        } else {
            Err(scoped_error(
                DiagnosticClass::Source,
                "scoped_token_domain",
                "scoped token has an unknown domain prefix",
            ))
        }
    }
}

impl Encode for ScopedToken {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.domain().tag().encode(encoder)?;
        self.bytes().encode(encoder)
    }
}

impl<Context> Decode<Context> for ScopedToken {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let domain = ScopedSectionDomain::from_tag(u8::decode(decoder)?)?;
        let bytes = <[u8; LOCAL_TOKEN_BYTES]>::decode(decoder)?;
        Self::from_domain_bytes(domain, bytes)
    }
}

bincode::impl_borrow_decode!(ScopedToken);

macro_rules! scoped_token_conversion {
    ($type:ident, $variant:ident) => {
        impl From<$type> for ScopedToken {
            fn from(token: $type) -> Self {
                Self::$variant(token)
            }
        }
    };
}

scoped_token_conversion!(TypeParameterToken, TypeParameter);
scoped_token_conversion!(FieldToken, Field);
scoped_token_conversion!(CaseToken, Case);
scoped_token_conversion!(InterfaceOperationToken, InterfaceOperation);
scoped_token_conversion!(FunctionParameterToken, FunctionParameter);
scoped_token_conversion!(OperationParameterToken, OperationParameter);
scoped_token_conversion!(RequirementToken, Requirement);
scoped_token_conversion!(PortToken, Port);
scoped_token_conversion!(ExpressionToken, Expression);
scoped_token_conversion!(BindingToken, Binding);

/// One package-local executable expression/binding ownership scope.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct BodyScope {
    pub(crate) declaration: DeclarationId,
    pub(crate) role: BodyRole,
}

impl BodyScope {
    pub(crate) const fn new(declaration: DeclarationId, role: BodyRole) -> Self {
        Self { declaration, role }
    }
}

/// Closed executable body roles. Port expressions bind the exact component-local port token.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum BodyRole {
    Function,
    ConstantValue,
    TestActual,
    TestExpected,
    PortImplementation(PortToken),
}

impl Encode for BodyRole {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self {
            Self::Function => 1_u8.encode(encoder),
            Self::ConstantValue => 2_u8.encode(encoder),
            Self::TestActual => 3_u8.encode(encoder),
            Self::TestExpected => 4_u8.encode(encoder),
            Self::PortImplementation(port) => {
                5_u8.encode(encoder)?;
                port.bytes().encode(encoder)
            }
        }
    }
}

impl<Context> Decode<Context> for BodyRole {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        match u8::decode(decoder)? {
            1 => Ok(Self::Function),
            2 => Ok(Self::ConstantValue),
            3 => Ok(Self::TestActual),
            4 => Ok(Self::TestExpected),
            5 => PortToken::from_bytes(<[u8; LOCAL_TOKEN_BYTES]>::decode(decoder)?)
                .map(Self::PortImplementation)
                .ok_or_else(|| {
                    DecodeError::OtherString(
                        "port implementation contains the reserved zero token".to_owned(),
                    )
                }),
            tag => Err(DecodeError::OtherString(format!(
                "unknown scoped body-role tag {tag}"
            ))),
        }
    }
}

bincode::impl_borrow_decode!(BodyRole);

impl Encode for BodyScope {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        BODY_SCOPE_TAG.encode(encoder)?;
        self.declaration.encode(encoder)?;
        self.role.encode(encoder)
    }
}

impl<Context> Decode<Context> for BodyScope {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let tag = u8::decode(decoder)?;
        if tag != BODY_SCOPE_TAG {
            return Err(DecodeError::OtherString(format!(
                "foreign scoped body tag {tag}; expected {BODY_SCOPE_TAG}"
            )));
        }
        Ok(Self::new(
            DeclarationId::decode(decoder)?,
            BodyRole::decode(decoder)?,
        ))
    }
}

bincode::impl_borrow_decode!(BodyScope);

/// Package-local parent scope for one record in the package's scoped-record map.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum ScopedRecordScope {
    Declaration {
        declaration: DeclarationId,
    },
    InterfaceOperation {
        declaration: DeclarationId,
        operation: InterfaceOperationToken,
    },
    Body(BodyScope),
}

impl ScopedRecordScope {
    pub(crate) const fn declaration(declaration: DeclarationId) -> Self {
        Self::Declaration { declaration }
    }

    pub(crate) const fn interface_operation(
        declaration: DeclarationId,
        operation: InterfaceOperationToken,
    ) -> Self {
        Self::InterfaceOperation {
            declaration,
            operation,
        }
    }

    pub(crate) const fn body(scope: BodyScope) -> Self {
        Self::Body(scope)
    }
}

impl Encode for ScopedRecordScope {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        match self {
            Self::Declaration { declaration } => {
                DECLARATION_SCOPE_TAG.encode(encoder)?;
                declaration.encode(encoder)
            }
            Self::InterfaceOperation {
                declaration,
                operation,
            } => {
                OPERATION_SCOPE_TAG.encode(encoder)?;
                declaration.encode(encoder)?;
                operation.encode(encoder)
            }
            Self::Body(scope) => scope.encode(encoder),
        }
    }
}

impl<Context> Decode<Context> for ScopedRecordScope {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        match u8::decode(decoder)? {
            DECLARATION_SCOPE_TAG => Ok(Self::declaration(DeclarationId::decode(decoder)?)),
            OPERATION_SCOPE_TAG => Ok(Self::interface_operation(
                DeclarationId::decode(decoder)?,
                InterfaceOperationToken::decode(decoder)?,
            )),
            BODY_SCOPE_TAG => Ok(Self::body(BodyScope::new(
                DeclarationId::decode(decoder)?,
                BodyRole::decode(decoder)?,
            ))),
            tag => Err(DecodeError::OtherString(format!(
                "unknown scoped record-scope tag {tag}"
            ))),
        }
    }
}

bincode::impl_borrow_decode!(ScopedRecordScope);

/// Package qualification used by public references and deterministic token allocation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ExactScopedRecordScope {
    pub(crate) package: PackageId,
    pub(crate) scope: ScopedRecordScope,
}

impl ExactScopedRecordScope {
    pub(crate) const fn new(package: PackageId, scope: ScopedRecordScope) -> Self {
        Self { package, scope }
    }

    fn allocation_bytes(self) -> Vec<u8> {
        encode_exact_scope_prefix(self)
    }
}

impl Encode for ExactScopedRecordScope {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        EXACT_SCOPE_TAG.encode(encoder)?;
        self.package.encode(encoder)?;
        self.scope.encode(encoder)
    }
}

impl<Context> Decode<Context> for ExactScopedRecordScope {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let tag = u8::decode(decoder)?;
        if tag != EXACT_SCOPE_TAG {
            return Err(DecodeError::OtherString(format!(
                "foreign exact scoped-record scope tag {tag}; expected {EXACT_SCOPE_TAG}"
            )));
        }
        Ok(Self::new(
            PackageId::decode(decoder)?,
            ScopedRecordScope::decode(decoder)?,
        ))
    }
}

bincode::impl_borrow_decode!(ExactScopedRecordScope);

/// Canonical package-local key in the single package scoped-record map.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ScopedRecordKey {
    pub(crate) scope: ScopedRecordScope,
    pub(crate) token: ScopedToken,
}

impl ScopedRecordKey {
    pub(crate) fn new(
        scope: ScopedRecordScope,
        token: impl Into<ScopedToken>,
    ) -> Result<Self, Diagnostic> {
        let token = token.into();
        validate_scope_domain(scope, token.domain())?;
        Ok(Self { scope, token })
    }

    pub(crate) fn encode(self) -> Vec<u8> {
        let mut bytes = encode_scope_prefix(self.scope);
        bytes.push(self.token.domain().tag());
        bytes.extend_from_slice(&self.token.bytes());
        bytes
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let mut cursor = KeyCursor::new(bytes);
        decode_key_version(&mut cursor)?;
        let key = decode_local_key(&mut cursor)?;
        cursor.finish()?;
        Ok(key)
    }
}

/// Exact package-qualified scoped record reference.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ExactScopedRecordKey {
    pub(crate) package: PackageId,
    pub(crate) key: ScopedRecordKey,
}

impl ExactScopedRecordKey {
    pub(crate) fn new(
        scope: ExactScopedRecordScope,
        token: impl Into<ScopedToken>,
    ) -> Result<Self, Diagnostic> {
        Ok(Self {
            package: scope.package,
            key: ScopedRecordKey::new(scope.scope, token)?,
        })
    }

    pub(crate) const fn scope(self) -> ExactScopedRecordScope {
        ExactScopedRecordScope::new(self.package, self.key.scope)
    }

    pub(crate) fn encode(self) -> Vec<u8> {
        let local = self.key.encode();
        let mut bytes = Vec::with_capacity(local.len() + LOCAL_TOKEN_BYTES);
        bytes.push(SCOPED_RECORD_KEY_VERSION);
        bytes.extend_from_slice(&self.package.bytes());
        bytes.extend(local.into_iter().skip(1));
        bytes
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let mut cursor = KeyCursor::new(bytes);
        decode_key_version(&mut cursor)?;
        let package =
            PackageId::from_bytes(cursor.take_array("package identity")?).ok_or_else(|| {
                key_error("exact scoped record key contains the reserved zero package identity")
            })?;
        let key = decode_local_key(&mut cursor)?;
        cursor.finish()?;
        Ok(Self { package, key })
    }
}

impl Encode for ExactScopedRecordKey {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        EXACT_KEY_TAG.encode(encoder)?;
        self.package.encode(encoder)?;
        self.key.scope.encode(encoder)?;
        self.key.token.encode(encoder)
    }
}

impl<Context> Decode<Context> for ExactScopedRecordKey {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let tag = u8::decode(decoder)?;
        if tag != EXACT_KEY_TAG {
            return Err(DecodeError::OtherString(format!(
                "foreign exact scoped-record key tag {tag}; expected {EXACT_KEY_TAG}"
            )));
        }
        let package = PackageId::decode(decoder)?;
        let scope = ScopedRecordScope::decode(decoder)?;
        let token = ScopedToken::decode(decoder)?;
        Self::new(ExactScopedRecordScope::new(package, scope), token)
            .map_err(|error| DecodeError::OtherString(error.to_string()))
    }
}

bincode::impl_borrow_decode!(ExactScopedRecordKey);

/// Exact lexical bounds for one parent/domain section in the package-local map.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ScopedRecordKeyBounds {
    pub(crate) start_inclusive: Vec<u8>,
    pub(crate) end_inclusive: Vec<u8>,
}

impl ScopedRecordKeyBounds {
    pub(crate) fn contains(&self, key: &[u8]) -> bool {
        self.start_inclusive.as_slice() <= key && key <= self.end_inclusive.as_slice()
    }
}

/// Parent/domain prefix for one bounded package-local section scan.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ScopedSectionPrefix {
    pub(crate) scope: ScopedRecordScope,
    pub(crate) domain: ScopedSectionDomain,
}

impl ScopedSectionPrefix {
    pub(crate) fn new(
        scope: ScopedRecordScope,
        domain: ScopedSectionDomain,
    ) -> Result<Self, Diagnostic> {
        validate_scope_domain(scope, domain)?;
        Ok(Self { scope, domain })
    }

    pub(crate) fn encode(self) -> Vec<u8> {
        let mut bytes = encode_scope_prefix(self.scope);
        bytes.push(self.domain.tag());
        bytes
    }

    pub(crate) fn bounds(self) -> ScopedRecordKeyBounds {
        let prefix = self.encode();
        let mut start_inclusive = Vec::with_capacity(prefix.len() + LOCAL_TOKEN_BYTES);
        start_inclusive.extend_from_slice(&prefix);
        start_inclusive.extend_from_slice(&[0; LOCAL_TOKEN_BYTES]);
        let mut end_inclusive = Vec::with_capacity(prefix.len() + LOCAL_TOKEN_BYTES);
        end_inclusive.extend_from_slice(&prefix);
        end_inclusive.extend_from_slice(&[u8::MAX; LOCAL_TOKEN_BYTES]);
        ScopedRecordKeyBounds {
            start_inclusive,
            end_inclusive,
        }
    }
}

/// Typed head/tail metadata for an ordered declaration or operation section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct OrderedSection<T> {
    pub(crate) count: u64,
    pub(crate) head: Option<T>,
    pub(crate) tail: Option<T>,
}

impl<T: OrderedScopedToken> OrderedSection<T> {
    pub(crate) fn new(count: u64, head: Option<T>, tail: Option<T>) -> Result<Self, Diagnostic> {
        validate_ordered_section(count, head, tail)?;
        Ok(Self { count, head, tail })
    }

    pub(crate) fn validate_for_scope(self, scope: ScopedRecordScope) -> Result<(), Diagnostic> {
        validate_scope_domain(scope, T::DOMAIN)?;
        validate_ordered_section(self.count, self.head, self.tail)
    }
}

impl<T> Encode for OrderedSection<T>
where
    T: OrderedScopedToken + Encode,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        T::DOMAIN.encode(encoder)?;
        self.count.encode(encoder)?;
        encode_optional_typed(self.head, encoder)?;
        encode_optional_typed(self.tail, encoder)
    }
}

impl<Context, T> Decode<Context> for OrderedSection<T>
where
    T: OrderedScopedToken + Decode<Context>,
{
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        decode_expected_domain::<T, Context, D>(decoder)?;
        let count = u64::decode(decoder)?;
        validate_scoped_count("ordered section", count, true)
            .map_err(|error| DecodeError::OtherString(error.to_string()))?;
        let head = decode_optional_typed(decoder)?;
        let tail = decode_optional_typed(decoder)?;
        validate_ordered_section(count, head, tail)
            .map_err(|error| DecodeError::OtherString(error.to_string()))?;
        Ok(Self { count, head, tail })
    }
}

impl<'de, Context, T> bincode::BorrowDecode<'de, Context> for OrderedSection<T>
where
    T: OrderedScopedToken + Decode<Context>,
{
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Self::decode(decoder)
    }
}

/// Intrusive ordering links stored beside one exact ordered record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct OrderLinks<T> {
    pub(crate) previous: Option<T>,
    pub(crate) next: Option<T>,
}

impl<T: OrderedScopedToken> OrderLinks<T> {
    pub(crate) const fn new(previous: Option<T>, next: Option<T>) -> Self {
        Self { previous, next }
    }
}

impl<T> Encode for OrderLinks<T>
where
    T: OrderedScopedToken + Encode,
{
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        T::DOMAIN.encode(encoder)?;
        encode_optional_typed(self.previous, encoder)?;
        encode_optional_typed(self.next, encoder)
    }
}

impl<Context, T> Decode<Context> for OrderLinks<T>
where
    T: OrderedScopedToken + Decode<Context>,
{
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        decode_expected_domain::<T, Context, D>(decoder)?;
        Ok(Self::new(
            decode_optional_typed(decoder)?,
            decode_optional_typed(decoder)?,
        ))
    }
}

impl<'de, Context, T> bincode::BorrowDecode<'de, Context> for OrderLinks<T>
where
    T: OrderedScopedToken + Decode<Context>,
{
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
        decoder: &mut D,
    ) -> Result<Self, DecodeError> {
        Self::decode(decoder)
    }
}

/// Payload-independent body membership summary. Expressions and bindings remain unordered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BodyCommitment {
    pub(crate) root: ExpressionToken,
    pub(crate) expression_count: u64,
    pub(crate) binding_count: u64,
}

impl BodyCommitment {
    pub(crate) fn new(
        root: ExpressionToken,
        expression_count: u64,
        binding_count: u64,
    ) -> Result<Self, Diagnostic> {
        validate_scoped_count("body expression", expression_count, false)?;
        validate_scoped_count("body binding", binding_count, true)?;
        Ok(Self {
            root,
            expression_count,
            binding_count,
        })
    }
}

impl Encode for BodyCommitment {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        BODY_COMMITMENT_TAG.encode(encoder)?;
        self.root.encode(encoder)?;
        self.expression_count.encode(encoder)?;
        self.binding_count.encode(encoder)
    }
}

impl<Context> Decode<Context> for BodyCommitment {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let tag = u8::decode(decoder)?;
        if tag != BODY_COMMITMENT_TAG {
            return Err(DecodeError::OtherString(format!(
                "foreign body commitment tag {tag}; expected {BODY_COMMITMENT_TAG}"
            )));
        }
        Self::new(
            ExpressionToken::decode(decoder)?,
            u64::decode(decoder)?,
            u64::decode(decoder)?,
        )
        .map_err(|error| DecodeError::OtherString(error.to_string()))
    }
}

bincode::impl_borrow_decode!(BodyCommitment);

fn encode_optional_typed<T: Encode, E: Encoder>(
    token: Option<T>,
    encoder: &mut E,
) -> Result<(), EncodeError> {
    match token {
        None => 0_u8.encode(encoder),
        Some(token) => {
            1_u8.encode(encoder)?;
            token.encode(encoder)
        }
    }
}

fn decode_optional_typed<Context, T, D>(decoder: &mut D) -> Result<Option<T>, DecodeError>
where
    T: Decode<Context>,
    D: Decoder<Context = Context>,
{
    match u8::decode(decoder)? {
        0 => Ok(None),
        1 => T::decode(decoder).map(Some),
        tag => Err(DecodeError::OtherString(format!(
            "unknown scoped optional-token tag {tag}"
        ))),
    }
}

fn decode_expected_domain<T, Context, D>(decoder: &mut D) -> Result<(), DecodeError>
where
    T: ScopedTokenType,
    D: Decoder<Context = Context>,
{
    let observed = ScopedSectionDomain::decode(decoder)?;
    if observed == T::DOMAIN {
        Ok(())
    } else {
        Err(DecodeError::OtherString(format!(
            "foreign ordered token domain tag {}; expected {}",
            observed.tag(),
            T::DOMAIN.tag()
        )))
    }
}

fn validate_ordered_section<T: OrderedScopedToken>(
    count: u64,
    head: Option<T>,
    tail: Option<T>,
) -> Result<(), Diagnostic> {
    validate_scoped_count("ordered section", count, true)?;
    match (count, head, tail) {
        (0, None, None) => Ok(()),
        (0, _, _) => Err(section_error(
            "an empty ordered section must not have head or tail tokens",
        )),
        (1, Some(head), Some(tail)) if head == tail => Ok(()),
        (1, _, _) => Err(section_error(
            "a one-record ordered section must have equal head and tail tokens",
        )),
        (_, Some(head), Some(tail)) if head != tail => Ok(()),
        (_, Some(_), Some(_)) => Err(section_error(
            "a multi-record ordered section must have distinct head and tail tokens",
        )),
        (_, _, _) => Err(section_error(
            "a nonempty ordered section must have both head and tail tokens",
        )),
    }
}

fn validate_scoped_count(label: &str, count: u64, allow_zero: bool) -> Result<(), Diagnostic> {
    let maximum_count = u64::try_from(super::contract::MAXIMUM_CHILDREN).map_err(|_| {
        scoped_error(
            DiagnosticClass::Infrastructure,
            "scoped_count_configuration",
            "kernel child limit cannot be represented as a scoped record count",
        )
    })?;
    if (!allow_zero && count == 0) || count > maximum_count {
        let minimum_count = u8::from(!allow_zero);
        return Err(scoped_error(
            DiagnosticClass::Semantic,
            "scoped_count",
            format!("{label} count must be between {minimum_count} and {maximum_count}"),
        ));
    }
    Ok(())
}

fn validate_scope_domain(
    scope: ScopedRecordScope,
    domain: ScopedSectionDomain,
) -> Result<(), Diagnostic> {
    let valid = match scope {
        ScopedRecordScope::Declaration { .. } => matches!(
            domain,
            ScopedSectionDomain::TypeParameter
                | ScopedSectionDomain::Field
                | ScopedSectionDomain::Case
                | ScopedSectionDomain::InterfaceOperation
                | ScopedSectionDomain::FunctionParameter
                | ScopedSectionDomain::Requirement
                | ScopedSectionDomain::Port
        ),
        ScopedRecordScope::InterfaceOperation { .. } => {
            domain == ScopedSectionDomain::OperationParameter
        }
        ScopedRecordScope::Body(_) => {
            matches!(
                domain,
                ScopedSectionDomain::Expression | ScopedSectionDomain::Binding
            )
        }
    };
    if valid {
        Ok(())
    } else {
        Err(scoped_error(
            DiagnosticClass::Semantic,
            "scoped_section_parent_domain",
            "scoped section domain is not valid for its exact parent scope",
        ))
    }
}

fn decode_key_version(cursor: &mut KeyCursor<'_>) -> Result<(), Diagnostic> {
    let version = cursor.take_byte("version")?;
    if version == SCOPED_RECORD_KEY_VERSION {
        Ok(())
    } else {
        Err(key_error(format!(
            "scoped record key has unknown version {version}"
        )))
    }
}

fn decode_local_key(cursor: &mut KeyCursor<'_>) -> Result<ScopedRecordKey, Diagnostic> {
    let declaration = DeclarationId::from_bytes(cursor.take_array("declaration identity")?)
        .ok_or_else(|| {
            key_error("scoped record key contains the reserved zero declaration identity")
        })?;
    let scope = match cursor.take_byte("scope tag")? {
        DECLARATION_SCOPE_TAG => ScopedRecordScope::declaration(declaration),
        OPERATION_SCOPE_TAG => {
            let operation = InterfaceOperationToken::from_bytes(
                cursor.take_array("interface-operation token")?,
            )
            .ok_or_else(|| key_error("scoped record key contains a zero operation token"))?;
            ScopedRecordScope::interface_operation(declaration, operation)
        }
        BODY_SCOPE_TAG => {
            let role = decode_body_role(cursor)?;
            ScopedRecordScope::body(BodyScope::new(declaration, role))
        }
        tag => {
            return Err(key_error(format!(
                "scoped record key contains unknown scope tag {tag}"
            )));
        }
    };
    let domain = ScopedSectionDomain::from_tag(cursor.take_byte("section domain")?)
        .map_err(|error| key_error(error.to_string()))?;
    let token = ScopedToken::from_domain_bytes(domain, cursor.take_array("local token")?)
        .map_err(|error| key_error(error.to_string()))?;
    ScopedRecordKey::new(scope, token)
}

fn encode_scope_prefix(scope: ScopedRecordScope) -> Vec<u8> {
    let declaration = match scope {
        ScopedRecordScope::Declaration { declaration }
        | ScopedRecordScope::InterfaceOperation { declaration, .. } => declaration,
        ScopedRecordScope::Body(scope) => scope.declaration,
    };
    let mut bytes = Vec::with_capacity(35);
    bytes.push(SCOPED_RECORD_KEY_VERSION);
    bytes.extend_from_slice(&declaration.bytes());
    match scope {
        ScopedRecordScope::Declaration { .. } => bytes.push(DECLARATION_SCOPE_TAG),
        ScopedRecordScope::InterfaceOperation { operation, .. } => {
            bytes.push(OPERATION_SCOPE_TAG);
            bytes.extend_from_slice(&operation.bytes());
        }
        ScopedRecordScope::Body(scope) => encode_body_role(&mut bytes, scope.role),
    }
    bytes
}

fn encode_exact_scope_prefix(scope: ExactScopedRecordScope) -> Vec<u8> {
    let local = encode_scope_prefix(scope.scope);
    let mut bytes = Vec::with_capacity(local.len() + LOCAL_TOKEN_BYTES);
    bytes.push(SCOPED_RECORD_KEY_VERSION);
    bytes.extend_from_slice(&scope.package.bytes());
    bytes.extend(local.into_iter().skip(1));
    bytes
}

fn encode_body_role(bytes: &mut Vec<u8>, role: BodyRole) {
    bytes.push(BODY_SCOPE_TAG);
    match role {
        BodyRole::Function => bytes.push(1),
        BodyRole::ConstantValue => bytes.push(2),
        BodyRole::TestActual => bytes.push(3),
        BodyRole::TestExpected => bytes.push(4),
        BodyRole::PortImplementation(port) => {
            bytes.push(5);
            bytes.extend_from_slice(&port.bytes());
        }
    }
}

fn decode_body_role(cursor: &mut KeyCursor<'_>) -> Result<BodyRole, Diagnostic> {
    match cursor.take_byte("body role")? {
        1 => Ok(BodyRole::Function),
        2 => Ok(BodyRole::ConstantValue),
        3 => Ok(BodyRole::TestActual),
        4 => Ok(BodyRole::TestExpected),
        5 => PortToken::from_bytes(cursor.take_array("port token")?)
            .map(BodyRole::PortImplementation)
            .ok_or_else(|| key_error("scoped record key contains a zero port token")),
        tag => Err(key_error(format!(
            "scoped record key contains unknown body-role tag {tag}"
        ))),
    }
}

struct KeyCursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> KeyCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take_byte(&mut self, field: &str) -> Result<u8, Diagnostic> {
        let byte =
            self.bytes.get(self.position).copied().ok_or_else(|| {
                key_error(format!("scoped record key is truncated before {field}"))
            })?;
        self.position += 1;
        Ok(byte)
    }

    fn take_array<const N: usize>(&mut self, field: &str) -> Result<[u8; N], Diagnostic> {
        let end = self
            .position
            .checked_add(N)
            .ok_or_else(|| key_error("scoped record key length overflowed"))?;
        let source = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| key_error(format!("scoped record key is truncated inside {field}")))?;
        let mut bytes = [0_u8; N];
        bytes.copy_from_slice(source);
        self.position = end;
        Ok(bytes)
    }

    fn finish(self) -> Result<(), Diagnostic> {
        if self.position == self.bytes.len() {
            Ok(())
        } else {
            Err(key_error("scoped record key contains trailing bytes"))
        }
    }
}

fn decode_hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn invalid_token_hex(value: &str) -> Diagnostic {
    scoped_error(
        DiagnosticClass::Source,
        "scoped_token_hex",
        format!("scoped token '{value}' is not canonical lowercase hexadecimal"),
    )
}

fn section_error(message: impl Into<String>) -> Diagnostic {
    scoped_error(
        DiagnosticClass::Semantic,
        "scoped_section_commitment",
        message,
    )
}

fn key_error(message: impl Into<String>) -> Diagnostic {
    scoped_error(DiagnosticClass::Corrupt, "scoped_record_key", message)
}

fn scoped_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::packed;
    use crate::platform::persistent_map::{MapWork, MemoryPageStore, PersistentMap};
    use crate::platform::semantic_id::DeclarationId;

    const TEST_MAGIC: [u8; 8] = *b"LKJSCTK1";
    const TEST_DIGEST_DOMAIN: &str = "lkjscript.test.scoped-token.v1";

    fn package(ordinal: u64) -> PackageId {
        PackageId::migrate(b"scoped-test-package", ordinal)
    }

    fn declaration(ordinal: u64) -> DeclarationId {
        DeclarationId::migrate(b"scoped-test-declaration", ordinal)
    }

    fn request_digest(byte: u8) -> ChangeDigest {
        ChangeDigest::of(&[byte])
    }

    fn config() -> impl bincode::config::Config {
        bincode::config::standard()
            .with_little_endian()
            .with_variable_int_encoding()
    }

    #[test]
    fn typed_tokens_are_strictly_domain_tagged_and_exactly_parsed() {
        let scope =
            ExactScopedRecordScope::new(package(1), ScopedRecordScope::declaration(declaration(1)));
        let field = FieldToken::allocate(scope, request_digest(1), 3, 0).expect("allocate field");
        let text = field.to_string();
        assert_eq!(text.parse::<FieldToken>().expect("parse field"), field);
        assert_eq!(
            text.parse::<CaseToken>()
                .expect_err("foreign text domain")
                .code,
            "scoped_token_domain"
        );
        assert!(text.to_uppercase().parse::<FieldToken>().is_err());

        let encoded = bincode::encode_to_vec(field, config()).expect("encode field");
        assert!(bincode::decode_from_slice::<CaseToken, _>(&encoded, config()).is_err());

        let mut zero = vec![ScopedSectionDomain::Field.tag()];
        zero.extend_from_slice(&[0; LOCAL_TOKEN_BYTES]);
        assert!(bincode::decode_from_slice::<FieldToken, _>(&zero, config()).is_err());

        let mut unknown = vec![u8::MAX];
        unknown.extend_from_slice(&[1; LOCAL_TOKEN_BYTES]);
        assert!(bincode::decode_from_slice::<ScopedToken, _>(&unknown, config()).is_err());

        let mut packed = packed::encode(
            TEST_MAGIC,
            TEST_DIGEST_DOMAIN,
            &ScopedToken::from(field),
            256,
        )
        .expect("pack token");
        packed.push(0);
        assert_eq!(
            packed::decode::<ScopedToken>(&packed, TEST_MAGIC, TEST_DIGEST_DOMAIN, 256,)
                .expect_err("trailing envelope bytes")
                .code,
            "packed_length_mismatch"
        );
    }

    #[test]
    fn deterministic_allocation_binds_every_input_dimension() {
        let local = ScopedRecordScope::declaration(declaration(1));
        let parent_a = ExactScopedRecordScope::new(package(1), local);
        let parent_b = ExactScopedRecordScope::new(package(2), local);
        let parent_c =
            ExactScopedRecordScope::new(package(1), ScopedRecordScope::declaration(declaration(2)));
        let base = FieldToken::allocate(parent_a, request_digest(2), 7, 0).expect("allocate base");
        assert_eq!(
            FieldToken::allocate(parent_a, request_digest(2), 7, 0).expect("replay"),
            base
        );
        assert_ne!(
            FieldToken::allocate(parent_b, request_digest(2), 7, 0).expect("parent separation"),
            base
        );
        assert_ne!(
            FieldToken::allocate(parent_c, request_digest(2), 7, 0)
                .expect("local scope separation"),
            base
        );
        assert_ne!(
            FieldToken::allocate(parent_a, request_digest(3), 7, 0).expect("request separation"),
            base
        );
        assert_ne!(
            FieldToken::allocate(parent_a, request_digest(2), 8, 0).expect("ordinal separation"),
            base
        );
        assert_ne!(
            FieldToken::allocate(parent_a, request_digest(2), 7, 1).expect("collision separation"),
            base
        );
        let case =
            CaseToken::allocate(parent_a, request_digest(2), 7, 0).expect("domain separation");
        assert_ne!(base.bytes(), case.bytes());
        assert_eq!(
            ExpressionToken::allocate(parent_a, request_digest(2), 7, 0)
                .expect_err("invalid parent domain")
                .code,
            "scoped_section_parent_domain"
        );
    }

    #[test]
    fn scopes_roles_and_section_metadata_use_closed_validated_codecs() {
        let package = package(1);
        let declaration = declaration(1);
        let port = PortToken::from_bytes([6; 16]).expect("port");
        let body = BodyScope::new(declaration, BodyRole::PortImplementation(port));
        let body_bytes = bincode::encode_to_vec(body, config()).expect("encode body");
        assert_eq!(body_bytes[0], BODY_SCOPE_TAG);
        assert_eq!(
            bincode::decode_from_slice::<BodyScope, _>(&body_bytes, config())
                .expect("decode body")
                .0,
            body
        );

        let operation = InterfaceOperationToken::from_bytes([7; 16]).expect("operation");
        for (scope, expected_tag) in [
            (
                ScopedRecordScope::declaration(declaration),
                DECLARATION_SCOPE_TAG,
            ),
            (
                ScopedRecordScope::interface_operation(declaration, operation),
                OPERATION_SCOPE_TAG,
            ),
            (ScopedRecordScope::body(body), BODY_SCOPE_TAG),
        ] {
            let bytes = bincode::encode_to_vec(scope, config()).expect("encode scope");
            assert_eq!(bytes[0], expected_tag);
            assert_eq!(
                bincode::decode_from_slice::<ScopedRecordScope, _>(&bytes, config())
                    .expect("decode scope")
                    .0,
                scope
            );
        }
        assert!(bincode::decode_from_slice::<BodyRole, _>(&[u8::MAX], config()).is_err());
        assert!(bincode::decode_from_slice::<ScopedRecordScope, _>(&[u8::MAX], config()).is_err());

        let exact =
            ExactScopedRecordScope::new(package, ScopedRecordScope::declaration(declaration));
        let exact_bytes = bincode::encode_to_vec(exact, config()).expect("encode exact scope");
        assert_eq!(
            bincode::decode_from_slice::<ExactScopedRecordScope, _>(&exact_bytes, config())
                .expect("decode exact scope")
                .0,
            exact
        );

        let first = FieldToken::from_bytes([9; 16]).expect("first");
        let last = FieldToken::from_bytes([1; 16]).expect("last");
        let scope = ScopedRecordScope::declaration(declaration);
        let section = OrderedSection::<FieldToken>::new(2, Some(first), Some(last))
            .expect("descending logical order is valid");
        let section_bytes = bincode::encode_to_vec(section, config()).expect("encode section");
        let decoded =
            bincode::decode_from_slice::<OrderedSection<FieldToken>, _>(&section_bytes, config())
                .expect("decode section")
                .0;
        assert_eq!(decoded, section);
        assert!(decoded.validate_for_scope(scope).is_ok());
        assert!(
            decoded
                .validate_for_scope(ScopedRecordScope::body(BodyScope::new(
                    declaration,
                    BodyRole::Function,
                )))
                .is_err()
        );
        assert!(
            bincode::decode_from_slice::<OrderedSection<CaseToken>, _>(&section_bytes, config())
                .is_err(),
            "typed ordered sections reject a foreign token domain"
        );

        let links = OrderLinks::<FieldToken>::new(Some(first), Some(last));
        let link_bytes = bincode::encode_to_vec(links, config()).expect("encode links");
        assert_eq!(
            bincode::decode_from_slice::<OrderLinks<FieldToken>, _>(&link_bytes, config())
                .expect("decode links")
                .0,
            links
        );
        assert!(
            bincode::decode_from_slice::<OrderLinks<CaseToken>, _>(&link_bytes, config()).is_err(),
            "intrusive links are token-domain typed"
        );

        let expression = ExpressionToken::from_bytes([4; 16]).expect("expression");
        let body_commitment = BodyCommitment::new(expression, 3, 2).expect("body commitment");
        let body_commitment_bytes =
            bincode::encode_to_vec(body_commitment, config()).expect("encode body commitment");
        assert_eq!(
            bincode::decode_from_slice::<BodyCommitment, _>(&body_commitment_bytes, config())
                .expect("decode body commitment")
                .0,
            body_commitment
        );

        #[derive(Encode)]
        struct InvalidSection {
            domain: ScopedSectionDomain,
            count: u64,
            head_tag: u8,
            tail_tag: u8,
        }
        let invalid = bincode::encode_to_vec(
            InvalidSection {
                domain: ScopedSectionDomain::Field,
                count: 1,
                head_tag: 0,
                tail_tag: 0,
            },
            config(),
        )
        .expect("encode invalid section");
        assert!(
            bincode::decode_from_slice::<OrderedSection<FieldToken>, _>(&invalid, config())
                .is_err()
        );

        let maximum_count =
            u64::try_from(super::super::contract::MAXIMUM_CHILDREN).expect("child limit fits");
        let oversized = bincode::encode_to_vec(
            InvalidSection {
                domain: ScopedSectionDomain::Field,
                count: maximum_count + 1,
                head_tag: 0,
                tail_tag: 0,
            },
            config(),
        )
        .expect("encode oversized section");
        assert!(
            bincode::decode_from_slice::<OrderedSection<FieldToken>, _>(&oversized, config())
                .is_err()
        );
        let zero_expression_error =
            BodyCommitment::new(expression, 0, 0).expect_err("zero expressions must fail");
        assert!(
            zero_expression_error
                .to_string()
                .contains("body expression count must be between 1 and")
        );
        let oversized_section_error =
            OrderedSection::<FieldToken>::new(maximum_count + 1, None, None)
                .expect_err("oversized ordered section must fail");
        assert!(
            oversized_section_error
                .to_string()
                .contains("ordered section count must be between 0 and")
        );
        assert!(BodyCommitment::new(expression, maximum_count + 1, 0).is_err());
        assert!(BodyCommitment::new(expression, 1, maximum_count + 1).is_err());
    }

    #[test]
    fn keys_and_prefix_bounds_separate_parent_body_and_domain() {
        let package_a = package(1);
        let package_b = package(2);
        let declaration_a = declaration(1);
        let declaration_b = declaration(2);
        let field = FieldToken::from_bytes([9; 16]).expect("field");
        let case = CaseToken::from_bytes([9; 16]).expect("case");
        let declaration_scope = ScopedRecordScope::declaration(declaration_a);
        let field_key = ScopedRecordKey::new(declaration_scope, field).expect("field key");
        let field_bytes = field_key.encode();
        assert_eq!(
            ScopedRecordKey::decode(&field_bytes).expect("decode key"),
            field_key
        );
        let field_bounds = ScopedSectionPrefix::new(declaration_scope, ScopedSectionDomain::Field)
            .expect("field prefix")
            .bounds();
        assert!(field_bounds.contains(&field_bytes));

        let case_bytes = ScopedRecordKey::new(declaration_scope, case)
            .expect("case key")
            .encode();
        assert_ne!(field_bytes, case_bytes);
        assert!(!field_bounds.contains(&case_bytes));

        let exact_a = ExactScopedRecordKey::new(
            ExactScopedRecordScope::new(package_a, declaration_scope),
            field,
        )
        .expect("exact A");
        let exact_b = ExactScopedRecordKey::new(
            ExactScopedRecordScope::new(package_b, declaration_scope),
            field,
        )
        .expect("exact B");
        let local_scope_bytes =
            bincode::encode_to_vec(declaration_scope, config()).expect("encode local scope");
        assert_eq!(
            bincode::encode_to_vec(exact_a.scope().scope, config()).expect("encode scope A"),
            local_scope_bytes
        );
        assert_eq!(
            bincode::encode_to_vec(exact_b.scope().scope, config()).expect("encode scope B"),
            local_scope_bytes
        );
        assert_ne!(
            bincode::encode_to_vec(exact_a.scope(), config()).expect("encode exact scope A"),
            bincode::encode_to_vec(exact_b.scope(), config()).expect("encode exact scope B")
        );
        assert_eq!(exact_a.key.encode(), exact_b.key.encode());
        assert_ne!(exact_a.encode(), exact_b.encode());
        assert_eq!(
            exact_a.encode().len(),
            field_bytes.len() + LOCAL_TOKEN_BYTES
        );
        assert_eq!(
            ExactScopedRecordKey::decode(&exact_a.encode()).expect("decode exact key"),
            exact_a
        );
        assert_eq!(exact_a.scope().scope, declaration_scope);
        let exact_record_bytes =
            bincode::encode_to_vec(exact_a, config()).expect("encode exact record");
        assert_eq!(
            bincode::decode_from_slice::<ExactScopedRecordKey, _>(&exact_record_bytes, config())
                .expect("decode exact record")
                .0,
            exact_a
        );

        assert_ne!(
            field_bytes,
            ScopedRecordKey::new(ScopedRecordScope::declaration(declaration_b), field)
                .expect("package-separated key")
                .encode()
        );

        let expression = ExpressionToken::from_bytes([7; 16]).expect("expression");
        let function = ScopedRecordScope::body(BodyScope::new(declaration_a, BodyRole::Function));
        let expected =
            ScopedRecordScope::body(BodyScope::new(declaration_a, BodyRole::TestExpected));
        assert_ne!(
            ScopedRecordKey::new(function, expression)
                .expect("function expression")
                .encode(),
            ScopedRecordKey::new(expected, expression)
                .expect("expected expression")
                .encode()
        );

        let mut trailing = field_bytes.clone();
        trailing.push(0);
        assert_eq!(
            ScopedRecordKey::decode(&trailing)
                .expect_err("trailing key bytes")
                .code,
            "scoped_record_key"
        );

        let mut zero_package = exact_a.encode();
        zero_package[1..1 + LOCAL_TOKEN_BYTES].fill(0);
        assert!(ExactScopedRecordKey::decode(&zero_package).is_err());
        let mut trailing_exact = exact_a.encode();
        trailing_exact.push(0);
        assert!(ExactScopedRecordKey::decode(&trailing_exact).is_err());
    }

    #[test]
    fn ordered_sections_are_bounded_typed_and_not_token_sorted() {
        let scope = ScopedRecordScope::declaration(declaration(1));
        let first = FieldToken::from_bytes([1; 16]).expect("first");
        let last = FieldToken::from_bytes([2; 16]).expect("last");
        assert!(OrderedSection::<FieldToken>::new(2, Some(first), Some(last)).is_ok());
        assert!(OrderedSection::<FieldToken>::new(0, Some(first), Some(first)).is_err());
        assert!(
            OrderedSection::<FieldToken>::new(2, Some(last), Some(first)).is_ok(),
            "logical order must not depend on random token byte order"
        );
        assert!(OrderedSection::<FieldToken>::new(1, Some(first), Some(last)).is_err());
        assert!(OrderedSection::<FieldToken>::new(2, Some(first), Some(first)).is_err());
        assert!(
            OrderedSection::<FieldToken>::new(1, Some(first), Some(first))
                .expect("singleton")
                .validate_for_scope(scope)
                .is_ok()
        );
        assert!(
            OrderedSection::<FieldToken>::new(1, Some(first), Some(first))
                .expect("singleton")
                .validate_for_scope(ScopedRecordScope::body(BodyScope::new(
                    declaration(1),
                    BodyRole::Function,
                )))
                .is_err()
        );
        let maximum_count =
            u64::try_from(super::super::contract::MAXIMUM_CHILDREN).expect("child limit fits");
        assert_eq!(
            OrderedSection::<FieldToken>::new(maximum_count + 1, Some(first), Some(last))
                .expect_err("oversized section count")
                .code,
            "scoped_count"
        );
    }

    #[test]
    fn whole_map_content_commitment_ignores_physical_page_partition() {
        let local_scope = ScopedRecordScope::declaration(declaration(1));
        let exact_scope = ExactScopedRecordScope::new(package(1), local_scope);
        let mut entries = (0_u64..160)
            .map(|ordinal| {
                let token = FieldToken::allocate(exact_scope, request_digest(4), ordinal, 0)
                    .expect("allocate field");
                (
                    ScopedRecordKey::new(local_scope, token)
                        .expect("key")
                        .encode(),
                    vec![u8::try_from(ordinal % 251).expect("bounded"); 192],
                )
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.0.cmp(&right.0));

        let mut canonical_store = MemoryPageStore::default();
        let mut canonical_work = MapWork::default();
        let canonical =
            PersistentMap::from_sorted(&mut canonical_store, entries.clone(), &mut canonical_work)
                .expect("canonical map");
        let mut alternate_store = MemoryPageStore::default();
        let mut alternate_work = MapWork::default();
        let alternate = PersistentMap::from_sorted_with_leaf_target(
            &mut alternate_store,
            entries,
            512,
            &mut alternate_work,
        )
        .expect("alternate map");

        assert_ne!(canonical.root().page(), alternate.root().page());
        assert_eq!(
            canonical.root().content_root(),
            alternate.root().content_root()
        );
    }
}
