use super::model::{ReleaseManifest, Sha256Digest};
use crate::error::DevError;
use crate::process::{self, ProcessSpec, ProcessStatus};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub(super) const ARCHIVE_NAME: &str = "lkjscript-x86_64-unknown-linux-gnu.tar.gz";
pub(super) const CHECKSUM_NAME: &str = "SHA256SUMS";
pub(super) const RECEIPT_NAME: &str = "release-receipt.json";
pub(super) const TOP_DIRECTORY: &str = "lkjscript/";
pub(super) const EXECUTABLE_MEMBER: &str = "lkjscript/lkjscript";
pub(super) const LICENSE_MEMBER: &str = "lkjscript/LICENSE";
pub(super) const NOTICE_MEMBER: &str = "lkjscript/THIRD-PARTY-LICENSES.html";
pub(super) const MANIFEST_MEMBER: &str = "lkjscript/RELEASE-MANIFEST.json";
const TAR_BLOCK_BYTES: usize = 512;
const MAXIMUM_COMPRESSED_BYTES: u64 = 128 * 1024 * 1024;
const MAXIMUM_UNCOMPRESSED_BYTES: u64 = 256 * 1024 * 1024;
const MAXIMUM_MANIFEST_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MemberKind {
    Directory,
    File,
}

#[derive(Clone, Copy, Debug)]
struct ExpectedMember {
    name: &'static str,
    mode: u32,
    kind: MemberKind,
}

const EXPECTED_MEMBERS: [ExpectedMember; 5] = [
    ExpectedMember {
        name: TOP_DIRECTORY,
        mode: 0o755,
        kind: MemberKind::Directory,
    },
    ExpectedMember {
        name: EXECUTABLE_MEMBER,
        mode: 0o755,
        kind: MemberKind::File,
    },
    ExpectedMember {
        name: LICENSE_MEMBER,
        mode: 0o644,
        kind: MemberKind::File,
    },
    ExpectedMember {
        name: NOTICE_MEMBER,
        mode: 0o644,
        kind: MemberKind::File,
    },
    ExpectedMember {
        name: MANIFEST_MEMBER,
        mode: 0o644,
        kind: MemberKind::File,
    },
];

#[derive(Clone, Debug)]
pub(super) struct ObservedMember {
    pub(super) name: String,
    pub(super) mode: u32,
    pub(super) byte_length: u64,
    pub(super) sha256: Option<Sha256Digest>,
}

#[derive(Clone, Debug)]
pub(super) struct VerifiedArchive {
    pub(super) manifest: ReleaseManifest,
    pub(super) manifest_sha256: Sha256Digest,
    pub(super) archive_byte_length: u64,
    pub(super) archive_sha256: Sha256Digest,
    pub(super) source_timestamp_unix_seconds: u64,
    pub(super) members: Vec<ObservedMember>,
}

pub(super) fn manifest_members() -> Vec<super::model::ArchiveMemberIdentity> {
    EXPECTED_MEMBERS
        .iter()
        .map(|member| super::model::ArchiveMemberIdentity {
            name: member.name.to_owned(),
            mode: member.mode,
            kind: match member.kind {
                MemberKind::Directory => "directory",
                MemberKind::File => "regular-file",
            }
            .to_owned(),
        })
        .collect()
}

pub(super) fn normalized_tar_invocation(timestamp: u64) -> Vec<String> {
    vec![
        "tar".to_owned(),
        "--format=ustar".to_owned(),
        "--create".to_owned(),
        "--file=$TAR".to_owned(),
        "--directory=$STAGE".to_owned(),
        "--no-recursion".to_owned(),
        "--numeric-owner".to_owned(),
        "--owner=0".to_owned(),
        "--group=0".to_owned(),
        format!("--mtime=@{timestamp}"),
        "--no-xattrs".to_owned(),
        "--no-acls".to_owned(),
        "--no-selinux".to_owned(),
        TOP_DIRECTORY.trim_end_matches('/').to_owned(),
        EXECUTABLE_MEMBER.to_owned(),
        LICENSE_MEMBER.to_owned(),
        NOTICE_MEMBER.to_owned(),
        MANIFEST_MEMBER.to_owned(),
    ]
}

pub(super) fn normalized_gzip_invocation() -> Vec<String> {
    vec![
        "gzip".to_owned(),
        "--no-name".to_owned(),
        "--best".to_owned(),
        "$TAR".to_owned(),
    ]
}

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
    let archive_metadata = ensure_regular(archive, "release archive")?;
    if archive_metadata.len() > MAXIMUM_COMPRESSED_BYTES {
        return Err(DevError::corrupt(format!(
            "release archive exceeds {MAXIMUM_COMPRESSED_BYTES} bytes"
        )));
    }
    verify_gzip_header(archive)?;
    let archive_sha256 = sha256_file(archive)?.0;
    let tar_path = working_directory.join("verified.tar");
    let stderr_path = working_directory.join("gzip.stderr");
    let observation = process::run(
        &ProcessSpec {
            command: vec![
                "gzip".to_owned(),
                "--decompress".to_owned(),
                "--stdout".to_owned(),
                archive.to_string_lossy().into_owned(),
            ],
            cwd: working_directory.to_path_buf(),
            environment: process::environment(),
            timeout: Duration::from_secs(120),
            maximum_stdout_bytes: MAXIMUM_UNCOMPRESSED_BYTES,
            maximum_stderr_bytes: 64 * 1024,
            stdout_path: tar_path.clone(),
            stderr_path,
            unavailable_exit_code: None,
        },
        working_directory,
    );
    if observation.status != ProcessStatus::Passed {
        return Err(DevError::corrupt(format!(
            "gzip decompression failed with {:?}: {}",
            observation.status,
            observation.reason.as_deref().unwrap_or("no reason")
        )));
    }
    let extraction = tempfile::Builder::new()
        .prefix("lkjscript-release-extract-")
        .tempdir_in(working_directory)
        .map_err(|error| {
            DevError::infrastructure(format!("create verification extraction: {error}"))
        })?;
    let parsed = parse_tar(&tar_path, extraction.path())?;
    let manifest_member = parsed
        .members
        .iter()
        .find(|member| member.name == MANIFEST_MEMBER)
        .ok_or_else(|| DevError::corrupt("release manifest member is missing"))?;
    if manifest_member.byte_length > MAXIMUM_MANIFEST_BYTES {
        return Err(DevError::corrupt("release manifest exceeds its byte limit"));
    }
    let manifest_path = extraction.path().join(MANIFEST_MEMBER);
    let manifest_bytes = fs::read(&manifest_path).map_err(|error| {
        DevError::infrastructure(format!("read extracted release manifest: {error}"))
    })?;
    let manifest: ReleaseManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| DevError::corrupt(format!("decode release manifest: {error}")))?;
    let canonical = canonical_json(&manifest)?;
    if canonical != manifest_bytes {
        return Err(DevError::corrupt(
            "release manifest is not in canonical first-party encoding",
        ));
    }
    let manifest_sha256 = sha256_bytes(&manifest_bytes)?;
    if parsed.source_timestamp_unix_seconds != manifest.packaging.source_timestamp_unix_seconds {
        return Err(DevError::corrupt(
            "archive member timestamp disagrees with release manifest",
        ));
    }
    cross_check_payloads(&manifest, &parsed.members)?;
    if let Some(candidate) = candidate {
        let metadata = ensure_regular(candidate, "candidate executable")?;
        let (digest, length) = sha256_file(candidate)?;
        if length != manifest.executable.byte_length
            || digest != manifest.executable.sha256
            || metadata.permissions().mode() & 0o111 == 0
        {
            return Err(DevError::corrupt(
                "candidate executable does not match archived executable",
            ));
        }
    }
    Ok(VerifiedArchive {
        manifest,
        manifest_sha256,
        archive_byte_length: archive_metadata.len(),
        archive_sha256,
        source_timestamp_unix_seconds: parsed.source_timestamp_unix_seconds,
        members: parsed.members,
    })
}

struct ParsedTar {
    source_timestamp_unix_seconds: u64,
    members: Vec<ObservedMember>,
}

fn parse_tar(path: &Path, extraction: &Path) -> Result<ParsedTar, DevError> {
    let metadata = ensure_regular(path, "decompressed tar")?;
    if metadata.len() > MAXIMUM_UNCOMPRESSED_BYTES {
        return Err(DevError::corrupt("decompressed tar exceeds its byte limit"));
    }
    let mut input = File::open(path)
        .map_err(|error| DevError::infrastructure(format!("open decompressed tar: {error}")))?;
    let mut observed = Vec::with_capacity(EXPECTED_MEMBERS.len());
    let mut names = BTreeSet::new();
    let mut source_timestamp = None;
    for expected in EXPECTED_MEMBERS {
        let mut header = [0_u8; TAR_BLOCK_BYTES];
        input.read_exact(&mut header).map_err(|error| {
            DevError::corrupt(format!("read tar header for '{}': {error}", expected.name))
        })?;
        if header.iter().all(|byte| *byte == 0) {
            return Err(DevError::corrupt(format!(
                "archive ended before expected member '{}'",
                expected.name
            )));
        }
        validate_header_checksum(&header)?;
        let name = tar_name(&header)?;
        validate_member_name(&name)?;
        if !names.insert(name.clone()) {
            return Err(DevError::corrupt(format!(
                "duplicate archive member '{name}'"
            )));
        }
        if name != expected.name {
            return Err(DevError::corrupt(format!(
                "noncanonical archive member order: expected '{}', observed '{name}'",
                expected.name
            )));
        }
        let mode = parse_octal(&header[100..108], "member mode")? as u32;
        let uid = parse_octal(&header[108..116], "member uid")?;
        let gid = parse_octal(&header[116..124], "member gid")?;
        let size = parse_octal(&header[124..136], "member size")?;
        let timestamp = parse_octal(&header[136..148], "member mtime")?;
        let type_flag = header[156];
        if mode != expected.mode || uid != 0 || gid != 0 {
            return Err(DevError::corrupt(format!(
                "archive metadata mismatch for '{name}'"
            )));
        }
        if &header[257..263] != b"ustar\0" || &header[263..265] != b"00" {
            return Err(DevError::corrupt(format!(
                "archive member '{name}' is not POSIX ustar"
            )));
        }
        if header[157..257].iter().any(|byte| *byte != 0) {
            return Err(DevError::corrupt(format!(
                "archive member '{name}' contains a link target"
            )));
        }
        match expected.kind {
            MemberKind::Directory if type_flag != b'5' || size != 0 => {
                return Err(DevError::corrupt(format!(
                    "archive member '{name}' is not the expected directory"
                )));
            }
            MemberKind::File if type_flag != b'0' => {
                return Err(DevError::corrupt(format!(
                    "archive member '{name}' is not a regular file"
                )));
            }
            _ => {}
        }
        if let Some(previous) = source_timestamp {
            if previous != timestamp {
                return Err(DevError::corrupt(
                    "archive members have inconsistent timestamps",
                ));
            }
        } else {
            source_timestamp = Some(timestamp);
        }
        let sha256 = match expected.kind {
            MemberKind::Directory => {
                fs::create_dir(extraction.join(name.trim_end_matches('/'))).map_err(|error| {
                    DevError::infrastructure(format!("create extracted directory: {error}"))
                })?;
                fs::set_permissions(
                    extraction.join(name.trim_end_matches('/')),
                    fs::Permissions::from_mode(expected.mode),
                )
                .map_err(|error| {
                    DevError::infrastructure(format!("set extracted directory mode: {error}"))
                })?;
                None
            }
            MemberKind::File => {
                let destination = extraction.join(&name);
                let parent = destination.parent().ok_or_else(|| {
                    DevError::corrupt(format!("archive member '{name}' has no parent"))
                })?;
                ensure_directory(parent, "extraction parent")?;
                let mut options = OpenOptions::new();
                options.create_new(true).write(true).mode(expected.mode);
                let mut output = options.open(&destination).map_err(|error| {
                    DevError::infrastructure(format!("create extracted member '{name}': {error}"))
                })?;
                let digest = copy_exact(&mut input, &mut output, size, &name)?;
                output
                    .set_permissions(fs::Permissions::from_mode(expected.mode))
                    .map_err(|error| {
                        DevError::infrastructure(format!(
                            "set extracted member mode '{name}': {error}"
                        ))
                    })?;
                output.sync_all().map_err(|error| {
                    DevError::infrastructure(format!(
                        "synchronize extracted member '{name}': {error}"
                    ))
                })?;
                drop(output);
                skip_zero_padding(&mut input, size, &name)?;
                Some(digest)
            }
        };
        observed.push(ObservedMember {
            name,
            mode,
            byte_length: size,
            sha256,
        });
    }
    let mut trailing = Vec::new();
    input
        .read_to_end(&mut trailing)
        .map_err(|error| DevError::infrastructure(format!("read trailing tar blocks: {error}")))?;
    if trailing.len() < TAR_BLOCK_BYTES * 2
        || trailing.len() % TAR_BLOCK_BYTES != 0
        || trailing.iter().any(|byte| *byte != 0)
    {
        return Err(DevError::corrupt(
            "archive has missing, malformed, or nonzero terminal blocks",
        ));
    }
    synchronize_directory(extraction)?;
    Ok(ParsedTar {
        source_timestamp_unix_seconds: source_timestamp
            .ok_or_else(|| DevError::corrupt("archive contains no members"))?,
        members: observed,
    })
}

fn cross_check_payloads(
    manifest: &ReleaseManifest,
    members: &[ObservedMember],
) -> Result<(), DevError> {
    if manifest.packaging.members != manifest_members() {
        return Err(DevError::corrupt(
            "release manifest archive inventory is not canonical",
        ));
    }
    cross_check_member(
        members,
        EXECUTABLE_MEMBER,
        manifest.executable.archive_mode,
        manifest.executable.byte_length,
        &manifest.executable.sha256,
    )?;
    cross_check_member(
        members,
        LICENSE_MEMBER,
        manifest.root_license.archive_mode,
        manifest.root_license.byte_length,
        &manifest.root_license.sha256,
    )?;
    cross_check_member(
        members,
        NOTICE_MEMBER,
        manifest.third_party_notices.archive_mode,
        manifest.third_party_notices.byte_length,
        &manifest.third_party_notices.sha256,
    )
}

fn cross_check_member(
    members: &[ObservedMember],
    name: &str,
    mode: u32,
    byte_length: u64,
    sha256: &Sha256Digest,
) -> Result<(), DevError> {
    let member = members
        .iter()
        .find(|member| member.name == name)
        .ok_or_else(|| DevError::corrupt(format!("archive member '{name}' is missing")))?;
    if member.mode != mode
        || member.byte_length != byte_length
        || member.sha256.as_ref() != Some(sha256)
    {
        return Err(DevError::corrupt(format!(
            "archive member '{name}' disagrees with release manifest"
        )));
    }
    Ok(())
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
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    digest_from_hasher(hasher)
}

pub(super) fn canonical_json<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, DevError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        DevError::infrastructure(format!("encode canonical release JSON: {error}"))
    })?;
    bytes.push(b'\n');
    Ok(bytes)
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

fn verify_gzip_header(path: &Path) -> Result<(), DevError> {
    let mut input = File::open(path)
        .map_err(|error| DevError::infrastructure(format!("open gzip header: {error}")))?;
    let mut header = [0_u8; 10];
    input
        .read_exact(&mut header)
        .map_err(|error| DevError::corrupt(format!("read gzip header: {error}")))?;
    if header[0..3] != [0x1f, 0x8b, 0x08]
        || header[3] != 0
        || header[4..8] != [0, 0, 0, 0]
        || header[8] != 2
    {
        return Err(DevError::corrupt(
            "gzip header is not canonical level-9 output with name/time disabled",
        ));
    }
    Ok(())
}

fn validate_header_checksum(header: &[u8; TAR_BLOCK_BYTES]) -> Result<(), DevError> {
    let expected = parse_octal(&header[148..156], "header checksum")?;
    let observed = header
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            if (148..156).contains(&index) {
                b' ' as u64
            } else {
                *byte as u64
            }
        })
        .sum::<u64>();
    if expected != observed {
        return Err(DevError::corrupt("archive header checksum mismatch"));
    }
    Ok(())
}

fn tar_name(header: &[u8; TAR_BLOCK_BYTES]) -> Result<String, DevError> {
    let name = nul_terminated(&header[0..100], "member name")?;
    let prefix = nul_terminated(&header[345..500], "member prefix")?;
    let bytes = if prefix.is_empty() {
        name
    } else {
        let mut combined = prefix;
        combined.push(b'/');
        combined.extend(name);
        combined
    };
    String::from_utf8(bytes)
        .map_err(|_| DevError::corrupt("archive member name is not portable UTF-8"))
}

fn nul_terminated(field: &[u8], label: &str) -> Result<Vec<u8>, DevError> {
    let end = field
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(field.len());
    if field[end..].iter().any(|byte| *byte != 0) {
        return Err(DevError::corrupt(format!(
            "archive {label} has nonzero bytes after terminator"
        )));
    }
    Ok(field[..end].to_vec())
}

fn validate_member_name(name: &str) -> Result<(), DevError> {
    if name.is_empty() || name.starts_with('/') || name.contains('\\') {
        return Err(DevError::corrupt(format!(
            "unsafe archive member name '{name}'"
        )));
    }
    let trimmed = name.trim_end_matches('/');
    let path = Path::new(trimmed);
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(DevError::corrupt(format!(
            "unsafe archive member name '{name}'"
        )));
    }
    Ok(())
}

fn parse_octal(field: &[u8], label: &str) -> Result<u64, DevError> {
    let trimmed = field
        .iter()
        .copied()
        .skip_while(|byte| *byte == b' ' || *byte == 0)
        .take_while(|byte| *byte != b' ' && *byte != 0)
        .collect::<Vec<_>>();
    if trimmed.is_empty() || trimmed.iter().any(|byte| !(b'0'..=b'7').contains(byte)) {
        return Err(DevError::corrupt(format!("invalid octal {label}")));
    }
    let mut value = 0_u64;
    for byte in trimmed {
        value = value
            .checked_mul(8)
            .and_then(|current| current.checked_add((byte - b'0') as u64))
            .ok_or_else(|| DevError::corrupt(format!("octal {label} overflow")))?;
    }
    Ok(value)
}

fn copy_exact(
    input: &mut File,
    output: &mut File,
    bytes: u64,
    name: &str,
) -> Result<Sha256Digest, DevError> {
    let mut remaining = bytes;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    while remaining > 0 {
        let requested = usize::try_from(remaining.min(buffer.len() as u64))
            .map_err(|_| DevError::corrupt("archive member size conversion overflow"))?;
        input
            .read_exact(&mut buffer[..requested])
            .map_err(|error| {
                DevError::corrupt(format!("truncated archive member '{name}': {error}"))
            })?;
        output.write_all(&buffer[..requested]).map_err(|error| {
            DevError::infrastructure(format!("write extracted member '{name}': {error}"))
        })?;
        hasher.update(&buffer[..requested]);
        remaining -= requested as u64;
    }
    digest_from_hasher(hasher)
}

fn skip_zero_padding(input: &mut File, size: u64, name: &str) -> Result<(), DevError> {
    let remainder = size % TAR_BLOCK_BYTES as u64;
    let padding = if remainder == 0 {
        0
    } else {
        TAR_BLOCK_BYTES as u64 - remainder
    };
    let padding = usize::try_from(padding)
        .map_err(|_| DevError::corrupt("archive padding conversion overflow"))?;
    let mut bytes = vec![0_u8; padding];
    input.read_exact(&mut bytes).map_err(|error| {
        DevError::corrupt(format!("truncated archive padding for '{name}': {error}"))
    })?;
    if bytes.iter().any(|byte| *byte != 0) {
        return Err(DevError::corrupt(format!(
            "archive padding for '{name}' is nonzero"
        )));
    }
    Ok(())
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
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[derive(Clone)]
    struct TestMember {
        name: String,
        mode: u32,
        kind: u8,
        bytes: Vec<u8>,
    }

    fn test_members() -> Vec<TestMember> {
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

    fn test_tar(members: &[TestMember]) -> Vec<u8> {
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
        let temporary = tempfile::tempdir().expect("temporary archive fixture");
        let tar = temporary.path().join("fixture.tar");
        fs::write(&tar, bytes).expect("write tar fixture");
        let extraction = temporary.path().join("extract");
        fs::create_dir(&extraction).expect("create fixture extraction");
        parse_tar(&tar, &extraction)
    }

    #[test]
    fn release_member_names_reject_traversal_and_absolute_paths() {
        for name in ["../escape", "/absolute", "lkjscript/../escape", "a\\b"] {
            assert!(validate_member_name(name).is_err(), "accepted {name}");
        }
        assert!(validate_member_name(EXECUTABLE_MEMBER).is_ok());
    }

    #[test]
    fn release_octal_parser_rejects_malformed_and_overflowing_values() {
        assert_eq!(parse_octal(b"0000755\0", "mode").expect("mode"), 0o755);
        assert!(parse_octal(b"00008\0", "mode").is_err());
        assert!(parse_octal(b"77777777777777777777777777777777", "size").is_err());
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
