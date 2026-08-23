//! Strict canonical codecs for owner summaries and witness manifests.

use super::contract::{
    MAXIMUM_OWNER_SUMMARY_BYTES, MAXIMUM_WITNESS_MANIFEST_BYTES, OWNER_SUMMARY_CONTRACT_VERSION,
    OWNER_SUMMARY_ENVELOPE_DOMAIN, OWNER_SUMMARY_MAGIC, WITNESS_CONTRACT_VERSION,
    WITNESS_ENVELOPE_DOMAIN, WITNESS_MAGIC,
};
use super::summary::{CertificateCore, OwnerSummary, ValidationWitnessManifest};
use super::{OwnerSummaryDigest, ValidationCertificateDigest, ValidationWitnessDigest};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::packed;

pub fn encode_owner_summary(
    summary: &OwnerSummary,
) -> Result<(OwnerSummaryDigest, Vec<u8>), Diagnostic> {
    validate_summary(summary)?;
    let bytes = packed::encode(
        OWNER_SUMMARY_MAGIC,
        OWNER_SUMMARY_ENVELOPE_DOMAIN,
        summary,
        MAXIMUM_OWNER_SUMMARY_BYTES,
    )?;
    Ok((OwnerSummaryDigest::of(&bytes), bytes))
}

pub fn decode_owner_summary(
    bytes: &[u8],
    expected_digest: OwnerSummaryDigest,
) -> Result<OwnerSummary, Diagnostic> {
    if OwnerSummaryDigest::of(bytes) != expected_digest {
        return Err(codec_error(
            "witness_summary_digest",
            "owner-summary bytes do not match the expected digest",
        ));
    }
    let summary: OwnerSummary = packed::decode(
        bytes,
        OWNER_SUMMARY_MAGIC,
        OWNER_SUMMARY_ENVELOPE_DOMAIN,
        MAXIMUM_OWNER_SUMMARY_BYTES,
    )?;
    validate_summary(&summary)?;
    let (digest, canonical) = encode_owner_summary(&summary)?;
    if digest != expected_digest || canonical != bytes {
        return Err(codec_error(
            "witness_summary_canonical",
            "owner summary is not canonically encoded",
        ));
    }
    Ok(summary)
}

pub fn encode_witness_manifest(
    manifest: &ValidationWitnessManifest,
) -> Result<(ValidationWitnessDigest, Vec<u8>), Diagnostic> {
    validate_manifest(manifest)?;
    let bytes = packed::encode(
        WITNESS_MAGIC,
        WITNESS_ENVELOPE_DOMAIN,
        manifest,
        MAXIMUM_WITNESS_MANIFEST_BYTES,
    )?;
    Ok((ValidationWitnessDigest::of(&bytes), bytes))
}

pub fn decode_witness_manifest(
    bytes: &[u8],
    expected_digest: ValidationWitnessDigest,
) -> Result<ValidationWitnessManifest, Diagnostic> {
    if ValidationWitnessDigest::of(bytes) != expected_digest {
        return Err(codec_error(
            "witness_manifest_digest",
            "validation-witness bytes do not match the expected digest",
        ));
    }
    let manifest: ValidationWitnessManifest = packed::decode(
        bytes,
        WITNESS_MAGIC,
        WITNESS_ENVELOPE_DOMAIN,
        MAXIMUM_WITNESS_MANIFEST_BYTES,
    )?;
    validate_manifest(&manifest)?;
    let (digest, canonical) = encode_witness_manifest(&manifest)?;
    if digest != expected_digest || canonical != bytes {
        return Err(codec_error(
            "witness_manifest_canonical",
            "validation witness is not canonically encoded",
        ));
    }
    Ok(manifest)
}

pub(crate) fn certificate_digest(
    core: &CertificateCore,
) -> Result<ValidationCertificateDigest, Diagnostic> {
    let configuration = bincode::config::standard()
        .with_little_endian()
        .with_variable_int_encoding();
    let bytes = bincode::encode_to_vec(core, configuration).map_err(|error| {
        codec_error(
            "witness_certificate_encode",
            format!("validation certificate encoding failed: {error}"),
        )
    })?;
    Ok(ValidationCertificateDigest::of(&bytes))
}

fn validate_summary(summary: &OwnerSummary) -> Result<(), Diagnostic> {
    if summary.contract_version != OWNER_SUMMARY_CONTRACT_VERSION {
        return Err(codec_error(
            "witness_summary_contract",
            "owner summary uses a foreign summary contract",
        ));
    }
    if !summary.kind.accepts_owner(summary.owner) {
        return Err(codec_error(
            "witness_summary_owner_kind",
            "owner-summary identity domain disagrees with its owner kind",
        ));
    }
    Ok(())
}

fn validate_manifest(manifest: &ValidationWitnessManifest) -> Result<(), Diagnostic> {
    if manifest.contract_version != WITNESS_CONTRACT_VERSION {
        return Err(codec_error(
            "witness_manifest_contract",
            "validation witness uses a foreign witness contract",
        ));
    }
    if !manifest.contract_is_current() {
        return Err(codec_error(
            "witness_validator_contract",
            "validation witness is not reusable under the current graph and validator contracts",
        ));
    }
    let certificate = certificate_digest(&manifest.core())?;
    if certificate != manifest.certificate {
        return Err(codec_error(
            "witness_certificate_mismatch",
            "validation witness certificate does not match its committed roots",
        ));
    }
    Ok(())
}

fn codec_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}
