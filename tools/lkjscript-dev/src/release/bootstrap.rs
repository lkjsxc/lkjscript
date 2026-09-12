//! The only bootstrap renderer. Shell owns acquisition; the shipped Rust executable owns installation.
use super::{archive, model::ArtifactIdentity};
use crate::error::DevError;
use std::path::Path;
pub(super) const NAME: &str = "install.sh";
pub(super) const MAXIMUM_BYTES: u64 = 16 * 1024;
const TEMPLATE: &str = include_str!("bootstrap/install.sh.in");

pub(super) fn render(archive: &archive::VerifiedArchive) -> Result<Vec<u8>, DevError> {
    super::validate_manifest(&archive.manifest)?;
    let manifest = &archive.manifest;
    let mut script = TEMPLATE.to_owned();
    for (placeholder, value) in [
        ("@REPOSITORY@", manifest.source.repository.clone()),
        ("@TAG@", manifest.source.expected_release_tag.clone()),
        ("@TARGET@", manifest.target_triple.clone()),
        ("@ARCHIVE@", archive::ARCHIVE_NAME.to_owned()),
        ("@ARCHIVE_BYTES@", archive.archive_byte_length.to_string()),
        (
            "@ARCHIVE_SHA256@",
            archive.archive_sha256.as_str().to_owned(),
        ),
        (
            "@EXECUTABLE_BYTES@",
            manifest.executable.byte_length.to_string(),
        ),
        (
            "@EXECUTABLE_SHA256@",
            manifest.executable.sha256.as_str().to_owned(),
        ),
    ] {
        script = script.replace(placeholder, &value);
    }
    if script.contains('@') || script.len() as u64 > MAXIMUM_BYTES {
        return Err(DevError::corrupt(
            "bootstrap rendering has an unresolved field or exceeds its bound",
        ));
    }
    Ok(script.into_bytes())
}
pub(super) fn identity(archive: &archive::VerifiedArchive) -> Result<ArtifactIdentity, DevError> {
    let bytes = render(archive)?;
    Ok(ArtifactIdentity {
        name: NAME.to_owned(),
        byte_length: bytes.len() as u64,
        sha256: archive::sha256_bytes(&bytes)?,
    })
}
pub(super) fn verify(path: &Path, archive: &archive::VerifiedArchive) -> Result<(), DevError> {
    archive::ensure_regular(path, "release bootstrap")?;
    let bytes = crate::process::read_bounded(path, MAXIMUM_BYTES)?;
    if bytes != render(archive)? {
        return Err(DevError::corrupt(
            "install.sh differs from the deterministic exact-archive bootstrap",
        ));
    }
    Ok(())
}
