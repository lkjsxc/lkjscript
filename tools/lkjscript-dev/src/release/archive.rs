use super::model::Sha256Digest;
use crate::error::DevError;
use crate::process;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) const ARCHIVE_NAME: &str = super::target::ARCHIVE_NAME;
pub(super) const CHECKSUM_NAME: &str = "SHA256SUMS";
pub(super) const RECEIPT_NAME: &str = "release-receipt.json";
pub(super) use lkjscript::release_container::{
    EXECUTABLE_MEMBER, LICENSE_MEMBER, MANIFEST_MEMBER, NOTICE_MEMBER, TOP_DIRECTORY,
    VerifiedArchive, manifest_members, normalized_gzip_invocation, normalized_tar_invocation,
};
#[cfg(test)]
use lkjscript::release_container::{ParsedTar, parse_tar_bytes};
#[cfg(test)]
const TAR_BLOCK_BYTES: usize = 512;

pub(super) fn stage_payload(
    stage: &Path,
    candidate: &Path,
    license: &Path,
    notice: &Path,
    manifest_bytes: &[u8],
) -> Result<(), DevError> {
    fs::create_dir(stage).map_err(|error| {
        DevError::infrastructure(format!(
            "create release payload stage '{}': {error}",
            stage.display()
        ))
    })?;
    fs::set_permissions(stage, fs::Permissions::from_mode(0o755)).map_err(|error| {
        DevError::infrastructure(format!("set release payload stage mode: {error}"))
    })?;
    copy_regular(candidate, &stage.join("lkjscript"), 0o755)?;
    copy_regular(license, &stage.join("LICENSE"), 0o644)?;
    copy_regular(notice, &stage.join("THIRD-PARTY-LICENSES.html"), 0o644)?;
    write_new(&stage.join("RELEASE-MANIFEST.json"), manifest_bytes, 0o644)?;
    synchronize_directory(stage)
}

pub(super) fn create_archive(
    payload_parent: &Path,
    tar_path: &Path,
    timestamp: u64,
) -> Result<PathBuf, DevError> {
    reject_existing(tar_path, "tar output")?;
    let mut arguments = vec![
        "--format=ustar".to_owned(),
        "--create".to_owned(),
        format!("--file={}", tar_path.display()),
        format!("--directory={}", payload_parent.display()),
        "--no-recursion".to_owned(),
        "--numeric-owner".to_owned(),
        "--owner=0".to_owned(),
        "--group=0".to_owned(),
        format!("--mtime=@{timestamp}"),
        "--no-xattrs".to_owned(),
        "--no-acls".to_owned(),
        "--no-selinux".to_owned(),
    ];
    arguments.extend([
        TOP_DIRECTORY.trim_end_matches('/').to_owned(),
        EXECUTABLE_MEMBER.to_owned(),
        LICENSE_MEMBER.to_owned(),
        NOTICE_MEMBER.to_owned(),
        MANIFEST_MEMBER.to_owned(),
    ]);
    run_quiet("tar", &arguments, payload_parent)?;
    ensure_regular(tar_path, "created tar")?;
    let gzip_arguments = vec![
        "--no-name".to_owned(),
        "--best".to_owned(),
        tar_path.to_string_lossy().into_owned(),
    ];
    run_quiet("gzip", &gzip_arguments, payload_parent)?;
    let archive = tar_path.with_extension("tar.gz");
    ensure_regular(&archive, "created gzip archive")?;
    fs::set_permissions(&archive, fs::Permissions::from_mode(0o644)).map_err(|error| {
        DevError::infrastructure(format!("set archive mode '{}': {error}", archive.display()))
    })?;
    File::open(&archive)
        .and_then(|file| file.sync_all())
        .map_err(|error| {
            DevError::infrastructure(format!(
                "synchronize archive '{}': {error}",
                archive.display()
            ))
        })?;
    Ok(archive)
}

pub(super) fn verify_archive(
    archive: &Path,
    working_directory: &Path,
    candidate: Option<&Path>,
) -> Result<VerifiedArchive, DevError> {
    verify_archive_controlled(archive, working_directory, candidate, None)
}

pub(super) fn verify_archive_controlled(
    archive: &Path,
    working_directory: &Path,
    candidate: Option<&Path>,
    control: Option<&process::ProcessControl>,
) -> Result<VerifiedArchive, DevError> {
    let extraction = tempfile::Builder::new()
        .prefix("lkjscript-release-extract-")
        .tempdir_in(working_directory)
        .map_err(|error| {
            DevError::infrastructure(format!("create verification extraction: {error}"))
        })?;
    let observed = verify_archive_into(
        archive,
        working_directory,
        candidate,
        extraction.path(),
        control,
    );
    extraction.close()?;
    observed
}

fn verify_archive_into(
    archive: &Path,
    working_directory: &Path,
    candidate: Option<&Path>,
    extraction: &Path,
    control: Option<&process::ProcessControl>,
) -> Result<VerifiedArchive, DevError> {
    let _ = working_directory;
    if control.is_some_and(process::ProcessControl::cancelled) {
        return Err(DevError::corrupt("archive admission cancelled"));
    }
    ensure_regular(archive, "release archive")?;
    let mut input = File::open(archive)?;
    let bytes = lkjscript::release_container::read_bounded(
        &mut input,
        lkjscript::release_container::MAXIMUM_COMPRESSED_BYTES,
    )?;
    let admitted = lkjscript::release_container::admit_gzip(&bytes)?;
    if let Some(candidate) = candidate {
        let metadata = ensure_regular(candidate, "candidate executable")?;
        let (digest, length) = sha256_file(candidate)?;
        if digest != admitted.verified.manifest.executable.sha256
            || length != admitted.verified.manifest.executable.byte_length
            || metadata.permissions().mode() & 0o111 == 0
            || super::target::inspect_static_elf(candidate)?
                != admitted.verified.manifest.executable.elf
        {
            return Err(DevError::corrupt(
                "candidate executable does not match archived executable",
            ));
        }
    }
    if control.is_some_and(process::ProcessControl::cancelled) {
        return Err(DevError::corrupt("archive admission cancelled"));
    }
    let payload = extraction.join("lkjscript");
    fs::create_dir(&payload)?;
    fs::set_permissions(&payload, fs::Permissions::from_mode(0o755))?;
    for (member, bytes) in admitted.payloads() {
        write_new(&extraction.join(&member.name), bytes, member.mode)?;
    }
    synchronize_directory(&payload)?;
    synchronize_directory(extraction)?;
    Ok(admitted.verified)
}

pub(super) fn extract_verified_archive(
    archive: &Path,
    working_directory: &Path,
    output: &Path,
    expected: &VerifiedArchive,
) -> Result<(), DevError> {
    reject_existing(output, "verified extraction output")?;
    let output_parent = output.parent().ok_or_else(|| {
        DevError::usage("verified extraction output must have an existing parent directory")
    })?;
    ensure_directory(output_parent, "verified extraction parent")?;
    let work = tempfile::Builder::new()
        .prefix("lkjscript-release-reextract-work-")
        .tempdir_in(working_directory)
        .map_err(|error| {
            DevError::infrastructure(format!("create verified extraction work: {error}"))
        })?;
    let stage = tempfile::Builder::new()
        .prefix(".lkjscript-release-extract-stage-")
        .tempdir_in(output_parent)
        .map_err(|error| {
            DevError::infrastructure(format!("create verified extraction stage: {error}"))
        })?;
    let observed = verify_archive_into(archive, work.path(), None, stage.path(), None)?;
    super::validate_manifest(&observed.manifest)?;
    if &observed != expected {
        return Err(DevError::corrupt(
            "verified extraction replay disagrees with the accepted archive identity",
        ));
    }
    let source = stage.path().join(TOP_DIRECTORY.trim_end_matches('/'));
    ensure_directory(&source, "verified extraction stage")?;
    synchronize_directory(&source)?;
    publish_directory_no_replace(&source, output)?;
    synchronize_directory(output_parent)
}

// A create-new pair admission uses the exact same parser, manifest and static-linkage checks.
// Temporary decompression/extraction state is explicitly closed on success and failure.
pub(super) fn admit_archive(
    archive: &Path,
    checksums: &Path,
    working_directory: &Path,
    output: &Path,
    control: &process::ProcessControl,
) -> Result<VerifiedArchive, DevError> {
    super::require_absolute_extraction_output(output)?;
    let work = tempfile::Builder::new()
        .prefix(".admission-")
        .tempdir_in(working_directory)?;
    let stage = tempfile::Builder::new()
        .prefix(".extraction-")
        .tempdir_in(working_directory)?;
    let result = (|| {
        let verified =
            verify_archive_into(archive, work.path(), None, stage.path(), Some(control))?;
        super::validate_manifest(&verified.manifest)?;
        super::verify_checksum_bytes(
            &process::read_bounded(checksums, 1024)?,
            &verified.archive_sha256,
        )?;
        if control.cancelled() {
            return Err(DevError::corrupt("pair admission cancelled"));
        }
        let source = stage.path().join(TOP_DIRECTORY.trim_end_matches('/'));
        synchronize_directory(&source)?;
        publish_directory_no_replace(&source, output)?;
        synchronize_directory(working_directory)?;
        Ok(verified)
    })();
    let stage_closed = stage.close();
    let work_closed = work.close();
    stage_closed?;
    work_closed?;
    result
}

fn copy_regular(source: &Path, destination: &Path, mode: u32) -> Result<(), DevError> {
    ensure_regular(source, "release payload input")?;
    let mut input = File::open(source).map_err(|error| {
        DevError::infrastructure(format!(
            "open payload input '{}': {error}",
            source.display()
        ))
    })?;
    let mut options = OpenOptions::new();
    options.create_new(true).write(true).mode(mode);
    let mut output = options.open(destination).map_err(|error| {
        DevError::infrastructure(format!(
            "create payload '{}': {error}",
            destination.display()
        ))
    })?;
    std::io::copy(&mut input, &mut output).map_err(|error| {
        DevError::infrastructure(format!("copy payload '{}': {error}", destination.display()))
    })?;
    output
        .set_permissions(fs::Permissions::from_mode(mode))
        .map_err(|error| {
            DevError::infrastructure(format!(
                "set payload mode '{}': {error}",
                destination.display()
            ))
        })?;
    output.sync_all().map_err(|error| {
        DevError::infrastructure(format!(
            "synchronize payload '{}': {error}",
            destination.display()
        ))
    })
}

pub(super) fn write_new(path: &Path, bytes: &[u8], mode: u32) -> Result<(), DevError> {
    let mut options = OpenOptions::new();
    options.create_new(true).write(true).mode(mode);
    let mut output = options.open(path).map_err(|error| {
        DevError::infrastructure(format!("create '{}': {error}", path.display()))
    })?;
    output.write_all(bytes).map_err(|error| {
        DevError::infrastructure(format!("write '{}': {error}", path.display()))
    })?;
    output
        .set_permissions(fs::Permissions::from_mode(mode))
        .map_err(|error| {
            DevError::infrastructure(format!("set mode '{}': {error}", path.display()))
        })?;
    output.sync_all().map_err(|error| {
        DevError::infrastructure(format!("synchronize '{}': {error}", path.display()))
    })
}

pub(super) fn copy_new(source: &Path, destination: &Path, mode: u32) -> Result<(), DevError> {
    copy_regular(source, destination, mode)
}

pub(super) fn sha256_file(path: &Path) -> Result<(Sha256Digest, u64), DevError> {
    let metadata = ensure_regular(path, "SHA-256 input")?;
    let mut input = File::open(path).map_err(|error| {
        DevError::infrastructure(format!("open SHA-256 input '{}': {error}", path.display()))
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut observed = 0_u64;
    loop {
        let count = input.read(&mut buffer).map_err(|error| {
            DevError::infrastructure(format!("read SHA-256 input '{}': {error}", path.display()))
        })?;
        if count == 0 {
            break;
        }
        observed = observed
            .checked_add(count as u64)
            .ok_or_else(|| DevError::infrastructure("SHA-256 byte length overflow"))?;
        hasher.update(&buffer[..count]);
    }
    if observed != metadata.len() {
        return Err(DevError::infrastructure(format!(
            "SHA-256 input '{}' changed while reading",
            path.display()
        )));
    }
    Ok((digest_from_hasher(hasher)?, observed))
}

pub(super) fn sha256_bytes(bytes: &[u8]) -> Result<Sha256Digest, DevError> {
    Ok(lkjscript::release_container::sha256_bytes(bytes)?)
}

pub(super) fn canonical_json<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, DevError> {
    Ok(lkjscript::release_container::canonical_json(value)?)
}

pub(super) fn reject_existing(path: &Path, label: &str) -> Result<(), DevError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(DevError::infrastructure(format!(
            "{label} '{}' already exists",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(DevError::infrastructure(format!(
            "inspect {label} '{}': {error}",
            path.display()
        ))),
    }
}

pub(super) fn ensure_regular(path: &Path, label: &str) -> Result<fs::Metadata, DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!("inspect {label} '{}': {error}", path.display()))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DevError::infrastructure(format!(
            "{label} '{}' is not a regular non-symlink file",
            path.display()
        )));
    }
    Ok(metadata)
}

pub(super) fn ensure_directory(path: &Path, label: &str) -> Result<fs::Metadata, DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!("inspect {label} '{}': {error}", path.display()))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(DevError::infrastructure(format!(
            "{label} '{}' is not a regular non-symlink directory",
            path.display()
        )));
    }
    Ok(metadata)
}

pub(super) fn synchronize_directory(path: &Path) -> Result<(), DevError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            DevError::infrastructure(format!(
                "synchronize directory '{}': {error}",
                path.display()
            ))
        })
}

fn publish_directory_no_replace(stage: &Path, output: &Path) -> Result<(), DevError> {
    ensure_directory(stage, "verified extraction stage")?;
    reject_existing(output, "verified extraction output")?;
    let stage_parent = stage
        .parent()
        .ok_or_else(|| DevError::infrastructure("verified extraction stage has no parent"))?;
    let output_parent = output
        .parent()
        .ok_or_else(|| DevError::usage("verified extraction output has no parent"))?;
    ensure_directory(stage_parent, "verified extraction stage parent")?;
    ensure_directory(output_parent, "verified extraction output parent")?;
    let stage_name = stage
        .file_name()
        .ok_or_else(|| DevError::infrastructure("verified extraction stage has no name"))?;
    let output_name = output
        .file_name()
        .ok_or_else(|| DevError::usage("verified extraction output has no name"))?;
    let stage_directory = File::open(stage_parent).map_err(|error| {
        DevError::infrastructure(format!("open verified extraction stage parent: {error}"))
    })?;
    let output_directory = File::open(output_parent).map_err(|error| {
        DevError::infrastructure(format!("open verified extraction output parent: {error}"))
    })?;
    rustix::fs::renameat_with(
        &stage_directory,
        stage_name,
        &output_directory,
        output_name,
        rustix::fs::RenameFlags::NOREPLACE,
    )
    .map_err(|error| {
        DevError::infrastructure(format!(
            "publish verified extraction '{}' without replacement: {error}",
            output.display()
        ))
    })
}

fn digest_from_hasher(hasher: Sha256) -> Result<Sha256Digest, DevError> {
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in digest {
        encoded.push(char::from(HEX[(byte >> 4) as usize]));
        encoded.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    Sha256Digest::new(encoded).map_err(DevError::infrastructure)
}

fn run_quiet(program: &str, arguments: &[String], cwd: &Path) -> Result<(), DevError> {
    let output = Command::new(program)
        .args(arguments)
        .current_dir(cwd)
        .env_clear()
        .envs(process::environment())
        .output()
        .map_err(|error| DevError::infrastructure(format!("start {program}: {error}")))?;
    if output.stdout.len() > 64 * 1024 || output.stderr.len() > 64 * 1024 {
        return Err(DevError::infrastructure(format!(
            "{program} output exceeded 64 KiB"
        )));
    }
    if !output.status.success() {
        return Err(DevError::infrastructure(format!(
            "{program} failed with {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[derive(Clone)]
    pub(in crate::release) struct TestMember {
        pub(in crate::release) name: String,
        pub(in crate::release) mode: u32,
        pub(in crate::release) kind: u8,
        pub(in crate::release) bytes: Vec<u8>,
    }

    pub(in crate::release) fn test_members() -> Vec<TestMember> {
        vec![
            TestMember {
                name: TOP_DIRECTORY.to_owned(),
                mode: 0o755,
                kind: b'5',
                bytes: Vec::new(),
            },
            TestMember {
                name: EXECUTABLE_MEMBER.to_owned(),
                mode: 0o755,
                kind: b'0',
                bytes: b"elf".to_vec(),
            },
            TestMember {
                name: LICENSE_MEMBER.to_owned(),
                mode: 0o644,
                kind: b'0',
                bytes: b"license".to_vec(),
            },
            TestMember {
                name: NOTICE_MEMBER.to_owned(),
                mode: 0o644,
                kind: b'0',
                bytes: b"notice".to_vec(),
            },
            TestMember {
                name: MANIFEST_MEMBER.to_owned(),
                mode: 0o644,
                kind: b'0',
                bytes: b"{}\n".to_vec(),
            },
        ]
    }

    fn encode_octal(field: &mut [u8], value: u64) {
        field.fill(b'0');
        let digits = format!("{value:o}");
        let start = field.len() - 1 - digits.len();
        field[start..start + digits.len()].copy_from_slice(digits.as_bytes());
        field[field.len() - 1] = 0;
    }

    fn header(member: &TestMember) -> [u8; TAR_BLOCK_BYTES] {
        let mut header = [0_u8; TAR_BLOCK_BYTES];
        header[..member.name.len()].copy_from_slice(member.name.as_bytes());
        encode_octal(&mut header[100..108], member.mode as u64);
        encode_octal(&mut header[108..116], 0);
        encode_octal(&mut header[116..124], 0);
        encode_octal(&mut header[124..136], member.bytes.len() as u64);
        encode_octal(&mut header[136..148], 1_700_000_000);
        header[148..156].fill(b' ');
        header[156] = member.kind;
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        let checksum = header.iter().map(|byte| *byte as u64).sum::<u64>();
        let digits = format!("{checksum:06o}");
        header[148..154].copy_from_slice(digits.as_bytes());
        header[154] = 0;
        header[155] = b' ';
        header
    }

    pub(in crate::release) fn test_tar(members: &[TestMember]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for member in members {
            bytes.extend_from_slice(&header(member));
            bytes.extend_from_slice(&member.bytes);
            let remainder = member.bytes.len() % TAR_BLOCK_BYTES;
            if remainder != 0 {
                bytes.resize(bytes.len() + TAR_BLOCK_BYTES - remainder, 0);
            }
        }
        bytes.resize(bytes.len() + TAR_BLOCK_BYTES * 2, 0);
        bytes
    }

    fn parse_fixture(bytes: &[u8]) -> Result<ParsedTar, DevError> {
        Ok(parse_tar_bytes(bytes)?)
    }

    #[test]
    fn release_manifest_inventory_is_exact_and_ordered() {
        let members = manifest_members();
        assert_eq!(members.len(), 5);
        assert_eq!(members[0].name, TOP_DIRECTORY);
        assert_eq!(members[1].name, EXECUTABLE_MEMBER);
        assert_eq!(members[4].name, MANIFEST_MEMBER);
        assert_eq!(members[1].mode, 0o755);
        assert_eq!(members[2].mode, 0o644);
    }

    #[test]
    fn release_tar_parser_accepts_only_the_exact_inventory() {
        let valid = test_tar(&test_members());
        let parsed = parse_fixture(&valid).expect("valid strict ustar fixture");
        assert_eq!(parsed.members.len(), 5);

        let mut wrong_order = test_members();
        wrong_order.swap(1, 2);
        assert!(parse_fixture(&test_tar(&wrong_order)).is_err());

        let mut wrong_mode = test_members();
        wrong_mode[1].mode = 0o644;
        assert!(parse_fixture(&test_tar(&wrong_mode)).is_err());

        let mut link = test_members();
        link[2].kind = b'2';
        assert!(parse_fixture(&test_tar(&link)).is_err());

        let mut extra = test_members();
        extra.push(TestMember {
            name: "lkjscript/extra".to_owned(),
            mode: 0o644,
            kind: b'0',
            bytes: b"extra".to_vec(),
        });
        assert!(parse_fixture(&test_tar(&extra)).is_err());
    }

    #[test]
    fn release_tar_parser_rejects_truncation_and_checksum_mutation() {
        let valid = test_tar(&test_members());
        assert!(parse_fixture(&valid[..valid.len() - TAR_BLOCK_BYTES * 2]).is_err());
        let mut corrupt = valid;
        corrupt[42] ^= 1;
        assert!(parse_fixture(&corrupt).is_err());
    }

    #[test]
    fn release_output_conflicts_reject_files_directories_and_symlinks() {
        let temporary = tempfile::tempdir().expect("temporary conflict fixtures");
        let file = temporary.path().join("file");
        fs::write(&file, b"retained").expect("write conflict file");
        let directory = temporary.path().join("directory");
        fs::create_dir(&directory).expect("create conflict directory");
        let link = temporary.path().join("link");
        symlink(&file, &link).expect("create conflict symlink");
        for path in [&file, &directory, &link] {
            assert!(reject_existing(path, "fixture").is_err());
        }
        assert_eq!(fs::read(file).expect("retained file"), b"retained");
    }
}
