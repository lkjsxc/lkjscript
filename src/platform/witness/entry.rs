//! Typed derived namespace, ownership, relation, and test-dependency entries.

use super::contract::{MAXIMUM_OWNERSHIP_VALUE_BYTES, WITNESS_CONTRACT_VERSION};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::ExpressionChildRole;
use crate::platform::kernel::{
    EncodedOwnerKey, ExactOwnerKey, Name, NamespaceClass, OwnerKey, PackageId, RelationEdge,
    RelationEndpoint, RelationKind,
};
use crate::platform::packed;
use bincode::{Decode, Encode};

const OWNERSHIP_VALUE_MAGIC: [u8; 8] = *b"LKJOWNW1";
const OWNERSHIP_VALUE_DOMAIN: &str = "lkjscript.witness.ownership-entry.v1";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct NamespaceKey {
    pub parent: Option<OwnerKey>,
    pub class: NamespaceClass,
    pub name: Name,
}

impl NamespaceKey {
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1 + 17 + 1 + 2 + self.name.as_str().len());
        match self.parent {
            None => bytes.push(1),
            Some(owner) => {
                bytes.push(2);
                bytes.extend_from_slice(&EncodedOwnerKey::new(owner).bytes());
            }
        }
        bytes.push(self.class.tag());
        let name = self.name.as_str().as_bytes();
        let length = name.len() as u16;
        bytes.extend_from_slice(&length.to_be_bytes());
        bytes.extend_from_slice(name);
        bytes
    }
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, Ord, PartialEq, PartialOrd)]
pub enum OwnershipParent {
    Package,
    Owner(OwnerKey),
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExpressionRootRole {
    FunctionBody,
    ConstantValue,
    TestActual,
    TestExpected,
    BindingValue,
    PortImplementation,
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, Ord, PartialEq, PartialOrd)]
pub enum BindingContainerRole {
    Let,
    MatchPayload,
    Transaction,
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, Ord, PartialEq, PartialOrd)]
pub enum OwnershipRole {
    PackageModule,
    PackageTarget,
    ModuleDeclaration,
    DeclarationTypeParameter,
    DeclarationField,
    DeclarationCase,
    DeclarationOperation,
    DeclarationParameter,
    OperationParameter,
    DeclarationRequirement,
    DeclarationPort,
    ExpressionRoot(ExpressionRootRole),
    ExpressionChild {
        role: ExpressionChildRole,
        ordinal: u32,
    },
    ExpressionBinding {
        role: BindingContainerRole,
        ordinal: u32,
    },
    Documentation,
    Annotation,
}

impl OwnershipRole {
    /// Whether this child's semantic dimensions contribute to its parent's owner summary.
    /// Module membership and retained attachments remain separate witness facts so local edits do
    /// not create module-sized or documentation-driven compiler invalidation.
    pub const fn aggregates_into_parent(self) -> bool {
        matches!(
            self,
            Self::DeclarationTypeParameter
                | Self::DeclarationField
                | Self::DeclarationCase
                | Self::DeclarationOperation
                | Self::DeclarationParameter
                | Self::OperationParameter
                | Self::DeclarationRequirement
                | Self::DeclarationPort
                | Self::ExpressionRoot(_)
                | Self::ExpressionChild { .. }
                | Self::ExpressionBinding { .. }
        )
    }
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, Ord, PartialEq, PartialOrd)]
pub struct OwnershipEntry {
    pub contract_version: u16,
    pub parent: OwnershipParent,
    pub role: OwnershipRole,
}

impl OwnershipEntry {
    pub const fn new(parent: OwnershipParent, role: OwnershipRole) -> Self {
        Self {
            contract_version: WITNESS_CONTRACT_VERSION,
            parent,
            role,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TestDependency {
    pub test: OwnerKey,
    pub kind: RelationKind,
    pub target: RelationEndpoint,
}

pub fn owner_key_bytes(owner: OwnerKey) -> Vec<u8> {
    EncodedOwnerKey::new(owner).bytes().to_vec()
}

pub fn owner_value_bytes(owner: OwnerKey) -> Vec<u8> {
    owner_key_bytes(owner)
}

pub fn encode_ownership(entry: &OwnershipEntry) -> Result<Vec<u8>, Diagnostic> {
    if entry.contract_version != WITNESS_CONTRACT_VERSION {
        return Err(entry_error(
            "witness_ownership_contract",
            "ownership entry uses a foreign witness contract",
        ));
    }
    packed::encode(
        OWNERSHIP_VALUE_MAGIC,
        OWNERSHIP_VALUE_DOMAIN,
        entry,
        MAXIMUM_OWNERSHIP_VALUE_BYTES,
    )
}

pub fn decode_ownership(bytes: &[u8]) -> Result<OwnershipEntry, Diagnostic> {
    let entry: OwnershipEntry = packed::decode(
        bytes,
        OWNERSHIP_VALUE_MAGIC,
        OWNERSHIP_VALUE_DOMAIN,
        MAXIMUM_OWNERSHIP_VALUE_BYTES,
    )?;
    if entry.contract_version != WITNESS_CONTRACT_VERSION {
        return Err(entry_error(
            "witness_ownership_contract",
            "ownership entry uses a foreign witness contract",
        ));
    }
    let canonical = encode_ownership(&entry)?;
    if canonical != bytes {
        return Err(entry_error(
            "witness_ownership_canonical",
            "ownership entry is not canonically encoded",
        ));
    }
    Ok(entry)
}

pub fn forward_relation_key(edge: RelationEdge) -> Vec<u8> {
    relation_key(edge.source, edge.kind, edge.target)
}

pub fn reverse_relation_key(edge: RelationEdge) -> Vec<u8> {
    relation_key(edge.target, edge.kind, edge.source)
}

pub fn forward_relation_prefix(source: RelationEndpoint, kind: Option<RelationKind>) -> Vec<u8> {
    relation_prefix(source, kind)
}

pub fn reverse_relation_prefix(target: RelationEndpoint, kind: Option<RelationKind>) -> Vec<u8> {
    relation_prefix(target, kind)
}

pub fn decode_forward_relation_key(bytes: &[u8]) -> Result<RelationEdge, Diagnostic> {
    decode_relation_key(bytes, false)
}

pub fn decode_reverse_relation_key(bytes: &[u8]) -> Result<RelationEdge, Diagnostic> {
    decode_relation_key(bytes, true)
}

pub fn test_dependency_keys(dependency: TestDependency) -> [Vec<u8>; 2] {
    let mut forward = vec![1];
    forward.extend_from_slice(&EncodedOwnerKey::new(dependency.test).bytes());
    forward.push(dependency.kind.tag());
    encode_endpoint(&mut forward, dependency.target);

    let mut reverse = vec![2];
    encode_endpoint(&mut reverse, dependency.target);
    reverse.push(dependency.kind.tag());
    reverse.extend_from_slice(&EncodedOwnerKey::new(dependency.test).bytes());
    [forward, reverse]
}

pub fn test_dependency_forward_prefix(test: OwnerKey) -> Vec<u8> {
    let mut prefix = Vec::with_capacity(18);
    prefix.push(1);
    prefix.extend_from_slice(&EncodedOwnerKey::new(test).bytes());
    prefix
}

pub fn decode_test_dependency_forward_key(bytes: &[u8]) -> Result<TestDependency, Diagnostic> {
    if bytes.first().copied() != Some(1) {
        return Err(entry_error(
            "witness_test_dependency_direction",
            "test-dependency key is not in the forward domain",
        ));
    }
    let owner_bytes = bytes.get(1..18).ok_or_else(|| {
        entry_error(
            "witness_test_dependency_owner",
            "test-dependency key has no complete test owner identity",
        )
    })?;
    let test = EncodedOwnerKey::decode(owner_bytes)?;
    if !matches!(test, OwnerKey::Declaration(_)) {
        return Err(entry_error(
            "witness_test_dependency_test",
            "test-dependency key does not identify a declaration owner",
        ));
    }
    let mut cursor = 18;
    let kind = bytes
        .get(cursor)
        .and_then(|tag| RelationKind::from_tag(*tag))
        .ok_or_else(|| {
            entry_error(
                "witness_test_dependency_kind",
                "test-dependency key contains an unknown relation kind",
            )
        })?;
    cursor += 1;
    let target = decode_endpoint(bytes, &mut cursor)?;
    if cursor != bytes.len() {
        return Err(entry_error(
            "witness_test_dependency_trailing",
            "test-dependency key contains trailing bytes",
        ));
    }
    Ok(TestDependency { test, kind, target })
}

fn relation_key(first: RelationEndpoint, kind: RelationKind, second: RelationEndpoint) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(70);
    encode_endpoint(&mut bytes, first);
    bytes.push(kind.tag());
    encode_endpoint(&mut bytes, second);
    bytes
}

fn relation_prefix(endpoint: RelationEndpoint, kind: Option<RelationKind>) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(35);
    encode_endpoint(&mut bytes, endpoint);
    if let Some(kind) = kind {
        bytes.push(kind.tag());
    }
    bytes
}

fn decode_relation_key(bytes: &[u8], reverse: bool) -> Result<RelationEdge, Diagnostic> {
    let mut cursor = 0;
    let first = decode_endpoint(bytes, &mut cursor)?;
    let kind = bytes
        .get(cursor)
        .and_then(|tag| RelationKind::from_tag(*tag))
        .ok_or_else(|| {
            entry_error(
                "witness_relation_kind",
                "relation key contains a missing or unknown relation-kind tag",
            )
        })?;
    cursor += 1;
    let second = decode_endpoint(bytes, &mut cursor)?;
    if cursor != bytes.len() {
        return Err(entry_error(
            "witness_relation_trailing",
            "relation key contains trailing bytes",
        ));
    }
    let (source, target) = if reverse {
        (second, first)
    } else {
        (first, second)
    };
    Ok(RelationEdge {
        source,
        kind,
        target,
    })
}

fn decode_endpoint(bytes: &[u8], cursor: &mut usize) -> Result<RelationEndpoint, Diagnostic> {
    let tag = *bytes.get(*cursor).ok_or_else(|| {
        entry_error(
            "witness_relation_endpoint",
            "relation key ends before an endpoint domain tag",
        )
    })?;
    *cursor += 1;
    let package_end = cursor.checked_add(16).ok_or_else(|| {
        entry_error(
            "witness_relation_endpoint",
            "relation endpoint length overflows",
        )
    })?;
    let package_bytes: [u8; 16] = bytes
        .get(*cursor..package_end)
        .ok_or_else(|| {
            entry_error(
                "witness_relation_endpoint",
                "relation key ends inside a package identity",
            )
        })?
        .try_into()
        .map_err(|_| {
            entry_error(
                "witness_relation_endpoint",
                "relation package identity has a noncanonical length",
            )
        })?;
    let package = PackageId::from_bytes(package_bytes).ok_or_else(|| {
        entry_error(
            "witness_relation_package",
            "relation endpoint contains the reserved all-zero package identity",
        )
    })?;
    *cursor = package_end;
    match tag {
        1 => Ok(RelationEndpoint::Package(package)),
        2 => {
            let owner_end = cursor.checked_add(17).ok_or_else(|| {
                entry_error(
                    "witness_relation_endpoint",
                    "relation owner endpoint length overflows",
                )
            })?;
            let owner =
                EncodedOwnerKey::decode(bytes.get(*cursor..owner_end).ok_or_else(|| {
                    entry_error(
                        "witness_relation_endpoint",
                        "relation key ends inside an owner identity",
                    )
                })?)?;
            *cursor = owner_end;
            Ok(RelationEndpoint::Owner(ExactOwnerKey { package, owner }))
        }
        other => Err(entry_error(
            "witness_relation_endpoint_domain",
            format!("relation endpoint contains unknown domain tag {other}"),
        )),
    }
}

pub fn encode_endpoint(bytes: &mut Vec<u8>, endpoint: RelationEndpoint) {
    match endpoint {
        RelationEndpoint::Package(package) => {
            bytes.push(1);
            bytes.extend_from_slice(&package.bytes());
        }
        RelationEndpoint::Owner(ExactOwnerKey { package, owner }) => {
            bytes.push(2);
            bytes.extend_from_slice(&package.bytes());
            bytes.extend_from_slice(&EncodedOwnerKey::new(owner).bytes());
        }
    }
}

pub fn exact_owner(package: PackageId, owner: OwnerKey) -> RelationEndpoint {
    RelationEndpoint::Owner(ExactOwnerKey { package, owner })
}

fn entry_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}
