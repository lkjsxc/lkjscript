//! Strict canonical Graph 6 record codecs.

use super::contract::{
    DEPENDENCY_ENVELOPE_DOMAIN, DEPENDENCY_MAGIC, MAXIMUM_DEPENDENCY_BYTES,
    MAXIMUM_OWNER_OBJECT_BYTES, MAXIMUM_RETIREMENT_BYTES, MAXIMUM_ROOT_BYTES,
    MAXIMUM_TYPE_OBJECT_BYTES, OWNER_ENVELOPE_DOMAIN, OWNER_MAGIC, RETIREMENT_ENVELOPE_DOMAIN,
    RETIREMENT_MAGIC, ROOT_ENVELOPE_DOMAIN, ROOT_MAGIC, TYPE_OBJECT_ENVELOPE_DOMAIN,
    TYPE_OBJECT_MAGIC,
};
use super::digest::{
    DependencyObjectDigest, OwnerObjectDigest, RetirementObjectDigest, SemanticRootDigest,
    TypeObjectDigest,
};
use super::id::{OwnerKey, OwnerKind, PackageId};
use super::owner::{OwnerBinding, OwnerRecord};
use super::root::{
    DependencyBinding, DependencyRecord, RetirementBinding, RetirementRecord, SemanticRoot,
};
use super::type_object::TypeObject;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::packed;

pub const OWNER_BINDING_BYTES: usize = 33;
pub const DEPENDENCY_BINDING_BYTES: usize = 32;
pub const RETIREMENT_BINDING_BYTES: usize = 32;

pub fn encode_owner_binding(binding: &OwnerBinding) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(OWNER_BINDING_BYTES);
    bytes.push(binding.kind.tag());
    bytes.extend_from_slice(&binding.object.bytes());
    bytes
}

pub fn decode_owner_binding(
    bytes: &[u8],
    expected_owner: OwnerKey,
) -> Result<OwnerBinding, Diagnostic> {
    if bytes.len() != OWNER_BINDING_BYTES {
        return Err(codec_error(
            "kernel_owner_binding_length",
            "owner binding has a noncanonical byte length",
        ));
    }
    let kind = OwnerKind::ALL
        .into_iter()
        .find(|kind| kind.tag() == bytes[0])
        .ok_or_else(|| {
            codec_error(
                "kernel_owner_binding_kind",
                "owner binding contains an unknown owner-kind tag",
            )
        })?;
    if !kind.accepts_owner(expected_owner) {
        return Err(codec_error(
            "kernel_owner_binding_domain",
            "owner binding kind disagrees with its map-key identity domain",
        ));
    }
    let object = bytes[1..].try_into().map_err(|_| {
        codec_error(
            "kernel_owner_binding_length",
            "owner binding has a noncanonical digest length",
        )
    })?;
    Ok(OwnerBinding {
        kind,
        object: OwnerObjectDigest::from_bytes(object),
    })
}

pub fn encode_dependency_binding(binding: &DependencyBinding) -> Vec<u8> {
    binding.object.bytes().to_vec()
}

pub fn decode_dependency_binding(bytes: &[u8]) -> Result<DependencyBinding, Diagnostic> {
    let object = bytes.try_into().map_err(|_| {
        codec_error(
            "kernel_dependency_binding_length",
            "dependency binding has a noncanonical byte length",
        )
    })?;
    Ok(DependencyBinding {
        object: DependencyObjectDigest::from_bytes(object),
    })
}

pub fn encode_retirement_binding(binding: &RetirementBinding) -> Vec<u8> {
    binding.object.bytes().to_vec()
}

pub fn decode_retirement_binding(bytes: &[u8]) -> Result<RetirementBinding, Diagnostic> {
    let object = bytes.try_into().map_err(|_| {
        codec_error(
            "kernel_retirement_binding_length",
            "retirement binding has a noncanonical byte length",
        )
    })?;
    Ok(RetirementBinding {
        object: RetirementObjectDigest::from_bytes(object),
    })
}

pub fn encode_owner(record: &OwnerRecord) -> Result<(OwnerObjectDigest, Vec<u8>), Diagnostic> {
    record.validate_local()?;
    let bytes = packed::encode(
        OWNER_MAGIC,
        OWNER_ENVELOPE_DOMAIN,
        record,
        MAXIMUM_OWNER_OBJECT_BYTES,
    )?;
    Ok((OwnerObjectDigest::of(&bytes), bytes))
}

pub fn decode_owner(
    bytes: &[u8],
    expected_owner: OwnerKey,
    expected_kind: OwnerKind,
    expected_digest: OwnerObjectDigest,
) -> Result<OwnerRecord, Diagnostic> {
    verify_digest(
        expected_digest.bytes(),
        OwnerObjectDigest::of(bytes).bytes(),
        "owner",
    )?;
    let record: OwnerRecord = packed::decode(
        bytes,
        OWNER_MAGIC,
        OWNER_ENVELOPE_DOMAIN,
        MAXIMUM_OWNER_OBJECT_BYTES,
    )?;
    record.validate_local()?;
    if record.owner() != expected_owner || record.kind() != expected_kind {
        return Err(codec_error(
            "kernel_owner_key_mismatch",
            "owner map key or binding kind does not match the decoded owner header",
        ));
    }
    let (digest, canonical) = encode_owner(&record)?;
    verify_canonical(
        bytes,
        &canonical,
        digest.bytes(),
        expected_digest.bytes(),
        "owner",
    )?;
    Ok(record)
}

pub fn encode_type_object(object: &TypeObject) -> Result<(TypeObjectDigest, Vec<u8>), Diagnostic> {
    object.validate_local()?;
    let bytes = packed::encode(
        TYPE_OBJECT_MAGIC,
        TYPE_OBJECT_ENVELOPE_DOMAIN,
        object,
        MAXIMUM_TYPE_OBJECT_BYTES,
    )?;
    Ok((TypeObjectDigest::of(&bytes), bytes))
}

pub fn decode_type_object(
    bytes: &[u8],
    expected_digest: TypeObjectDigest,
) -> Result<TypeObject, Diagnostic> {
    verify_digest(
        expected_digest.bytes(),
        TypeObjectDigest::of(bytes).bytes(),
        "type",
    )?;
    let object: TypeObject = packed::decode(
        bytes,
        TYPE_OBJECT_MAGIC,
        TYPE_OBJECT_ENVELOPE_DOMAIN,
        MAXIMUM_TYPE_OBJECT_BYTES,
    )?;
    object.validate_local()?;
    let (digest, canonical) = encode_type_object(&object)?;
    verify_canonical(
        bytes,
        &canonical,
        digest.bytes(),
        expected_digest.bytes(),
        "type",
    )?;
    Ok(object)
}

pub fn encode_root(root: &SemanticRoot) -> Result<(SemanticRootDigest, Vec<u8>), Diagnostic> {
    root.validate_local()?;
    let bytes = packed::encode(ROOT_MAGIC, ROOT_ENVELOPE_DOMAIN, root, MAXIMUM_ROOT_BYTES)?;
    Ok((SemanticRootDigest::of(&bytes), bytes))
}

pub fn decode_root(
    bytes: &[u8],
    expected_digest: SemanticRootDigest,
) -> Result<SemanticRoot, Diagnostic> {
    verify_digest(
        expected_digest.bytes(),
        SemanticRootDigest::of(bytes).bytes(),
        "root",
    )?;
    let root: SemanticRoot =
        packed::decode(bytes, ROOT_MAGIC, ROOT_ENVELOPE_DOMAIN, MAXIMUM_ROOT_BYTES)?;
    root.validate_local()?;
    let (digest, canonical) = encode_root(&root)?;
    verify_canonical(
        bytes,
        &canonical,
        digest.bytes(),
        expected_digest.bytes(),
        "root",
    )?;
    Ok(root)
}

pub fn encode_dependency(
    dependency: &DependencyRecord,
) -> Result<(DependencyObjectDigest, Vec<u8>), Diagnostic> {
    dependency.validate_local()?;
    let bytes = packed::encode(
        DEPENDENCY_MAGIC,
        DEPENDENCY_ENVELOPE_DOMAIN,
        dependency,
        MAXIMUM_DEPENDENCY_BYTES,
    )?;
    Ok((DependencyObjectDigest::of(&bytes), bytes))
}

pub fn decode_dependency(
    bytes: &[u8],
    expected_package: &PackageId,
    expected_digest: DependencyObjectDigest,
) -> Result<DependencyRecord, Diagnostic> {
    verify_digest(
        expected_digest.bytes(),
        DependencyObjectDigest::of(bytes).bytes(),
        "dependency",
    )?;
    let dependency: DependencyRecord = packed::decode(
        bytes,
        DEPENDENCY_MAGIC,
        DEPENDENCY_ENVELOPE_DOMAIN,
        MAXIMUM_DEPENDENCY_BYTES,
    )?;
    dependency.validate_local()?;
    if &dependency.package != expected_package {
        return Err(codec_error(
            "kernel_dependency_key_mismatch",
            "dependency map key does not match the dependency record",
        ));
    }
    let (digest, canonical) = encode_dependency(&dependency)?;
    verify_canonical(
        bytes,
        &canonical,
        digest.bytes(),
        expected_digest.bytes(),
        "dependency",
    )?;
    Ok(dependency)
}

pub fn encode_retirement(
    retirement: &RetirementRecord,
) -> Result<(RetirementObjectDigest, Vec<u8>), Diagnostic> {
    retirement.validate_local()?;
    let bytes = packed::encode(
        RETIREMENT_MAGIC,
        RETIREMENT_ENVELOPE_DOMAIN,
        retirement,
        MAXIMUM_RETIREMENT_BYTES,
    )?;
    Ok((RetirementObjectDigest::of(&bytes), bytes))
}

pub fn decode_retirement(
    bytes: &[u8],
    expected_owner: OwnerKey,
    expected_digest: RetirementObjectDigest,
) -> Result<RetirementRecord, Diagnostic> {
    verify_digest(
        expected_digest.bytes(),
        RetirementObjectDigest::of(bytes).bytes(),
        "retirement",
    )?;
    let retirement: RetirementRecord = packed::decode(
        bytes,
        RETIREMENT_MAGIC,
        RETIREMENT_ENVELOPE_DOMAIN,
        MAXIMUM_RETIREMENT_BYTES,
    )?;
    retirement.validate_local()?;
    if retirement.owner != expected_owner {
        return Err(codec_error(
            "kernel_retirement_key_mismatch",
            "retirement map key does not match the retirement record",
        ));
    }
    let (digest, canonical) = encode_retirement(&retirement)?;
    verify_canonical(
        bytes,
        &canonical,
        digest.bytes(),
        expected_digest.bytes(),
        "retirement",
    )?;
    Ok(retirement)
}

fn verify_digest(expected: [u8; 32], actual: [u8; 32], label: &str) -> Result<(), Diagnostic> {
    if expected != actual {
        return Err(codec_error(
            "kernel_object_digest",
            format!("{label} object digest does not match its requested identity"),
        ));
    }
    Ok(())
}

fn verify_canonical(
    input: &[u8],
    canonical: &[u8],
    canonical_digest: [u8; 32],
    expected_digest: [u8; 32],
    label: &str,
) -> Result<(), Diagnostic> {
    if input != canonical || canonical_digest != expected_digest {
        return Err(codec_error(
            "kernel_noncanonical_encoding",
            format!("{label} object does not use the canonical Graph 6 encoding"),
        ));
    }
    Ok(())
}

fn codec_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}
