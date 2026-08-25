//! Revision-bound derived compilation manifests over canonical persistent maps.

use super::unit::{
    BYTECODE_CONTRACT_VERSION, COMPILER_UNIT_CONTRACT_VERSION, CompilationUnitKey,
    OptimizationPolicy,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    EncodedOwnerKey, OwnerKey, OwnerKind, PackageId, PackageInterfaceDigest, PackageRevisionDigest,
    SemanticStateDigest,
};
use crate::platform::persistent_map::MapRoot;
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use crate::platform::storage::object::{ObjectDomain, ObjectKey};
use bincode::{Decode, Encode};
use std::fmt;

pub const COMPILATION_MANIFEST_CONTRACT_IDENTITY: &str = "lkjscript-compilation-manifest-3";
pub const COMPILATION_MANIFEST_CONTRACT_VERSION: u16 = 3;
pub(crate) const COMPILATION_MANIFEST_MAGIC: [u8; 8] = *b"LKJCMF03";
pub(crate) const COMPILATION_MANIFEST_ENVELOPE_DOMAIN: &str =
    "lkjscript.compilation-manifest-envelope.v3";
pub(crate) const MAXIMUM_COMPILATION_MANIFEST_BYTES: usize = 64 * 1024;
pub(crate) const MAXIMUM_COMPILATION_UNITS: u64 = 10_000_000;

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompilationManifestDigest([u8; 32]);

impl CompilationManifestDigest {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }

    pub const fn object_key(self) -> ObjectKey {
        ObjectKey::from_digest(ObjectDomain::CompilationManifest, self.0)
    }
}

impl fmt::Display for CompilationManifestDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("compilation_manifest_")?;
        formatter.write_str(&crate::platform::semantic_id::encode_hex(&self.0))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompilerUnitObjectDigest([u8; 32]);

impl CompilerUnitObjectDigest {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }

    pub const fn object_key(self) -> ObjectKey {
        ObjectKey::from_digest(ObjectDomain::CompilerUnit, self.0)
    }
}

impl fmt::Display for CompilerUnitObjectDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("compiler_unit_object_")?;
        formatter.write_str(&crate::platform::semantic_id::encode_hex(&self.0))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompilationBinding {
    pub kind: OwnerKind,
    pub key: CompilationUnitKey,
    pub object: CompilerUnitObjectDigest,
}

impl CompilationBinding {
    pub const ENCODED_BYTES: usize = 65;

    pub fn encode(self, owner: OwnerKey) -> Result<Vec<u8>, Diagnostic> {
        self.validate(owner)?;
        let mut bytes = Vec::with_capacity(Self::ENCODED_BYTES);
        bytes.push(self.kind.tag());
        bytes.extend_from_slice(&self.key.bytes());
        bytes.extend_from_slice(&self.object.bytes());
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8], owner: OwnerKey) -> Result<Self, Diagnostic> {
        let encoded: [u8; Self::ENCODED_BYTES] = bytes.try_into().map_err(|_| {
            manifest_error(
                DiagnosticClass::Corrupt,
                "compilation_binding_length",
                "compiled-unit binding has a noncanonical byte length",
            )
        })?;
        let kind = OwnerKind::ALL
            .into_iter()
            .find(|kind| kind.tag() == encoded[0])
            .ok_or_else(|| {
                manifest_error(
                    DiagnosticClass::Corrupt,
                    "compilation_binding_kind",
                    "compiled-unit binding contains an unknown owner-kind tag",
                )
            })?;
        let mut key = [0_u8; 32];
        key.copy_from_slice(&encoded[1..33]);
        let mut object = [0_u8; 32];
        object.copy_from_slice(&encoded[33..]);
        let binding = Self {
            kind,
            key: CompilationUnitKey::from_bytes(key),
            object: CompilerUnitObjectDigest::from_bytes(object),
        };
        binding.validate(owner)?;
        Ok(binding)
    }

    fn validate(self, owner: OwnerKey) -> Result<(), Diagnostic> {
        if !self.kind.has_compilation_unit() || !self.kind.accepts_owner(owner) {
            return Err(manifest_error(
                DiagnosticClass::Corrupt,
                "compilation_binding_domain",
                "compiled-unit binding kind disagrees with its exact owner-map key",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompilationManifest {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub compiler_contract_version: u16,
    pub bytecode_contract_version: u16,
    pub repository_id: RepositoryId,
    pub package_id: PackageId,
    pub revision: RevisionId,
    pub package_revision: PackageRevisionDigest,
    pub semantic_state: SemanticStateDigest,
    pub package_interface: PackageInterfaceDigest,
    pub optimization: OptimizationPolicy,
    pub units: MapRoot,
}

impl CompilationManifest {
    pub fn encode(&self) -> Result<(CompilationManifestDigest, Vec<u8>), Diagnostic> {
        self.validate()?;
        let bytes = crate::platform::packed::encode(
            COMPILATION_MANIFEST_MAGIC,
            COMPILATION_MANIFEST_ENVELOPE_DOMAIN,
            self,
            MAXIMUM_COMPILATION_MANIFEST_BYTES,
        )?;
        let key = ObjectKey::for_bytes(ObjectDomain::CompilationManifest, &bytes);
        Ok((
            CompilationManifestDigest::from_bytes(key.digest.bytes()),
            bytes,
        ))
    }

    pub fn decode(bytes: &[u8], expected: CompilationManifestDigest) -> Result<Self, Diagnostic> {
        let key = ObjectKey::from_digest(ObjectDomain::CompilationManifest, expected.bytes());
        key.verify(bytes).map_err(store_diagnostic)?;
        let value: Self = crate::platform::packed::decode(
            bytes,
            COMPILATION_MANIFEST_MAGIC,
            COMPILATION_MANIFEST_ENVELOPE_DOMAIN,
            MAXIMUM_COMPILATION_MANIFEST_BYTES,
        )?;
        value.validate()?;
        if value.encode()?.1 != bytes {
            return Err(manifest_error(
                DiagnosticClass::Corrupt,
                "compilation_manifest_canonical",
                "compilation manifest does not use its canonical current encoding",
            ));
        }
        Ok(value)
    }

    pub(crate) fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != COMPILATION_MANIFEST_CONTRACT_VERSION
            || self.graph_contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
            || self.compiler_contract_version != COMPILER_UNIT_CONTRACT_VERSION
            || self.bytecode_contract_version != BYTECODE_CONTRACT_VERSION
        {
            return Err(manifest_error(
                DiagnosticClass::Source,
                "compilation_manifest_contract",
                "compilation manifest uses a predecessor or foreign contract",
            ));
        }
        if self.repository_id.bytes() == [0; 16] || self.package_id.bytes() == [0; 16] {
            return Err(manifest_error(
                DiagnosticClass::Corrupt,
                "compilation_manifest_identity",
                "compilation manifest contains a reserved zero semantic identity",
            ));
        }
        if self.units.entries() > MAXIMUM_COMPILATION_UNITS {
            return Err(manifest_error(
                DiagnosticClass::Resource,
                "compilation_manifest_unit_count",
                "compilation manifest exceeds the current derived-unit implementation bound",
            ));
        }
        Ok(())
    }
}

pub(crate) const fn compilation_map_key(owner: OwnerKey) -> [u8; 17] {
    EncodedOwnerKey::new(owner).bytes()
}

fn store_diagnostic(error: crate::platform::storage::object::StoreError) -> Diagnostic {
    manifest_error(
        match error.class {
            crate::platform::storage::object::StoreErrorClass::Input => DiagnosticClass::Source,
            crate::platform::storage::object::StoreErrorClass::Resource => {
                DiagnosticClass::Resource
            }
            crate::platform::storage::object::StoreErrorClass::Corrupt => DiagnosticClass::Corrupt,
            crate::platform::storage::object::StoreErrorClass::Io => {
                DiagnosticClass::Infrastructure
            }
        },
        error.code,
        error.message,
    )
}

pub(crate) fn manifest_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
