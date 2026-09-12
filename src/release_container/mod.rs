//! Release-container format and integrity. This owner admits historical producer identities;
//! contributor release production adds its current pinned tool and provenance policy.

mod elf;
pub mod model;
pub use elf::inspect_static_elf_bytes;
use model::{ReleaseManifest, Sha256Digest};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::ops::Range;

pub const TARGET_TRIPLE: &str = "x86_64-unknown-linux-musl";
pub const ARCHIVE_NAME: &str = "lkjscript-x86_64-unknown-linux-musl.tar.gz";
pub const LINKAGE_MODEL: &str = "static-musl";
pub const ELF_INSPECTOR: &str = "lkjscript-elf64-little-endian-inspector-1";

#[derive(Debug)]
pub struct ContainerError {
    pub message: String,
    pub infrastructure: bool,
}
impl ContainerError {
    pub fn corrupt(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            infrastructure: false,
        }
    }
    pub fn infrastructure(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            infrastructure: true,
        }
    }
}
impl std::fmt::Display for ContainerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for ContainerError {}

pub const TOP_DIRECTORY: &str = "lkjscript/";
pub const EXECUTABLE_MEMBER: &str = "lkjscript/lkjscript";
pub const LICENSE_MEMBER: &str = "lkjscript/LICENSE";
pub const NOTICE_MEMBER: &str = "lkjscript/THIRD-PARTY-LICENSES.html";
pub const MANIFEST_MEMBER: &str = "lkjscript/RELEASE-MANIFEST.json";
const TAR_BLOCK_BYTES: usize = 512;
pub const MAXIMUM_COMPRESSED_BYTES: u64 = 128 * 1024 * 1024;
pub const MAXIMUM_UNCOMPRESSED_BYTES: u64 = 256 * 1024 * 1024;
pub const MAXIMUM_MANIFEST_BYTES: u64 = 1024 * 1024;

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

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedMember {
    pub name: String,
    pub mode: u32,
    pub byte_length: u64,
    pub sha256: Option<Sha256Digest>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedArchive {
    pub manifest: ReleaseManifest,
    pub manifest_sha256: Sha256Digest,
    pub archive_byte_length: u64,
    pub archive_sha256: Sha256Digest,
    pub source_timestamp_unix_seconds: u64,
    pub members: Vec<ObservedMember>,
}

pub fn manifest_members() -> Vec<model::ArchiveMemberIdentity> {
    EXPECTED_MEMBERS
        .iter()
        .map(|member| model::ArchiveMemberIdentity {
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

pub fn normalized_tar_invocation(timestamp: u64) -> Vec<String> {
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

pub fn normalized_gzip_invocation() -> Vec<String> {
    vec![
        "gzip".to_owned(),
        "--no-name".to_owned(),
        "--best".to_owned(),
        "$TAR".to_owned(),
    ]
}

fn cross_check_payloads(
    manifest: &ReleaseManifest,
    members: &[ObservedMember],
) -> Result<(), ContainerError> {
    if manifest.packaging.members != manifest_members() {
        return Err(ContainerError::corrupt(
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
) -> Result<(), ContainerError> {
    let member = members
        .iter()
        .find(|member| member.name == name)
        .ok_or_else(|| ContainerError::corrupt(format!("archive member '{name}' is missing")))?;
    if member.mode != mode
        || member.byte_length != byte_length
        || member.sha256.as_ref() != Some(sha256)
    {
        return Err(ContainerError::corrupt(format!(
            "archive member '{name}' disagrees with release manifest"
        )));
    }
    Ok(())
}

pub fn validate_manifest(manifest: &ReleaseManifest) -> Result<(), ContainerError> {
    if manifest.product.name != "lkjscript"
        || manifest.target_triple != TARGET_TRIPLE
        || manifest.source.repository != "lkjsxc/lkjscript"
        || manifest.executable.archive_mode != 0o755
        || manifest.root_license.archive_mode != 0o644
        || manifest.third_party_notices.archive_mode != 0o644
        || manifest.packaging.tar_format != "posix-ustar"
        || manifest.packaging.gzip_level != 9
        || manifest.packaging.gzip_name_header
        || manifest.packaging.gzip_time_header
        || manifest.packaging.numeric_owner != 0
        || manifest.packaging.numeric_group != 0
        || manifest.packaging.members != manifest_members()
        || manifest.packaging.source_timestamp_unix_seconds
            != manifest.source.commit_timestamp_unix_seconds
    {
        return Err(ContainerError::corrupt(
            "release manifest contains a noncanonical fixed product field",
        ));
    }
    validate_strict_tag(
        &manifest.source.expected_release_tag,
        &manifest.product.version,
    )?;
    validate_git_sha(&manifest.source.tagged_commit_sha, "manifest commit SHA")?;
    validate_capabilities_digest(&manifest.executable.capabilities_digest)?;
    match (
        manifest.publication_mode,
        manifest.source.annotated_tag_object_sha.as_deref(),
    ) {
        (model::PublicationMode::DryRun, None) => {}
        (model::PublicationMode::Release, Some(object)) => {
            validate_git_sha(object, "manifest annotated tag object SHA")?;
        }
        _ => {
            return Err(ContainerError::corrupt(
                "manifest tag-object state disagrees with publication mode",
            ));
        }
    }
    if manifest.executable.elf.class != "ELF64"
        || manifest.executable.elf.machine != "x86-64"
        || manifest.executable.elf.inspector != ELF_INSPECTOR
        || manifest.executable.elf.runtime_linkage != LINKAGE_MODEL
        || manifest.executable.elf.program_headers == 0
        || manifest.executable.elf.load_headers == 0
        || manifest.executable.elf.interpreter_headers != 0
        || manifest.executable.elf.needed_libraries != 0
        || manifest.executable.elf.glibc_version_requirements != 0
    {
        return Err(ContainerError::corrupt(
            "release manifest contains an invalid measured identity",
        ));
    }
    Ok(())
}

pub fn validate_strict_tag(tag: &str, product_version: &str) -> Result<(), ContainerError> {
    let expected = format!("v{product_version}");
    if tag.len() > 64 || tag != expected {
        return Err(ContainerError::corrupt(format!(
            "release tag '{tag}' does not equal product version tag '{expected}'"
        )));
    }
    let version = tag
        .strip_prefix('v')
        .ok_or_else(|| ContainerError::corrupt("release tag must start with 'v'"))?;
    let parts = version.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
        })
    {
        return Err(ContainerError::corrupt(format!(
            "release tag '{tag}' is not strict vMAJOR.MINOR.PATCH"
        )));
    }
    Ok(())
}

fn validate_git_sha(value: &str, label: &str) -> Result<(), ContainerError> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ContainerError::corrupt(format!(
            "{label} is not a full lowercase Git object SHA"
        )));
    }
    Ok(())
}

fn validate_capabilities_digest(value: &str) -> Result<(), ContainerError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ContainerError::corrupt(
            "capabilities digest is not 64 lowercase hexadecimal characters",
        ));
    }
    Ok(())
}

pub fn sha256_bytes(bytes: &[u8]) -> Result<Sha256Digest, ContainerError> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    digest_from_hasher(hasher)
}

pub fn canonical_json<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, ContainerError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        ContainerError::infrastructure(format!("encode canonical release JSON: {error}"))
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn digest_from_hasher(hasher: Sha256) -> Result<Sha256Digest, ContainerError> {
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in digest {
        encoded.push(char::from(HEX[(byte >> 4) as usize]));
        encoded.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    Sha256Digest::new(encoded).map_err(ContainerError::infrastructure)
}

/// Ranges address only the validated immutable decompressed buffer.
pub struct ParsedTar {
    pub source_timestamp_unix_seconds: u64,
    pub members: Vec<ObservedMember>,
    ranges: Vec<Range<usize>>,
}

pub struct AdmittedArchive {
    pub verified: VerifiedArchive,
    tar: Vec<u8>,
    ranges: Vec<Range<usize>>,
}

impl AdmittedArchive {
    pub fn payloads(&self) -> impl Iterator<Item = (&ObservedMember, &[u8])> {
        self.verified
            .members
            .iter()
            .zip(&self.ranges)
            .skip(1)
            .map(|(member, range)| (member, &self.tar[range.clone()]))
    }
}

/// The input must already be an owned snapshot. No paths are reopened during admission.
pub fn admit_gzip(bytes: &[u8]) -> Result<AdmittedArchive, ContainerError> {
    if bytes.len() as u64 > MAXIMUM_COMPRESSED_BYTES {
        return Err(ContainerError::corrupt(
            "release archive exceeds its compressed byte limit",
        ));
    }
    if bytes.get(..10) != Some(&[0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 2, 3]) {
        return Err(ContainerError::corrupt(
            "gzip header is not canonical Linux level-9 output with name/time disabled",
        ));
    }
    let mut decoder = flate2::bufread::GzDecoder::new(bytes);
    let tar = read_bounded(&mut decoder, MAXIMUM_UNCOMPRESSED_BYTES)?;
    if !decoder.into_inner().is_empty() {
        return Err(ContainerError::corrupt(
            "gzip has another member or trailing material",
        ));
    }
    let parsed = parse_tar_bytes(&tar)?;
    let manifest_bytes = &tar[parsed.ranges[4].clone()];
    let manifest = decode_manifest(manifest_bytes)?;
    if parsed.source_timestamp_unix_seconds != manifest.packaging.source_timestamp_unix_seconds {
        return Err(ContainerError::corrupt(
            "archive member timestamp disagrees with release manifest",
        ));
    }
    cross_check_payloads(&manifest, &parsed.members)?;
    let elf = inspect_static_elf_bytes(&tar[parsed.ranges[1].clone()])?;
    if elf != manifest.executable.elf {
        return Err(ContainerError::corrupt(
            "executable static linkage disagrees with release manifest",
        ));
    }
    Ok(AdmittedArchive {
        verified: VerifiedArchive {
            manifest,
            manifest_sha256: sha256_bytes(manifest_bytes)?,
            archive_byte_length: bytes.len() as u64,
            archive_sha256: sha256_bytes(bytes)?,
            source_timestamp_unix_seconds: parsed.source_timestamp_unix_seconds,
            members: parsed.members,
        },
        tar,
        ranges: parsed.ranges,
    })
}

pub fn decode_manifest(bytes: &[u8]) -> Result<ReleaseManifest, ContainerError> {
    if bytes.len() as u64 > MAXIMUM_MANIFEST_BYTES {
        return Err(ContainerError::corrupt(
            "release manifest exceeds its byte limit",
        ));
    }
    let manifest: ReleaseManifest = serde_json::from_slice(bytes)
        .map_err(|error| ContainerError::corrupt(format!("decode release manifest: {error}")))?;
    if canonical_json(&manifest)? != bytes {
        return Err(ContainerError::corrupt(
            "release manifest is not in canonical first-party encoding",
        ));
    }
    validate_manifest(&manifest)?;
    // Retained manifests must describe a container that could have passed archive admission.
    // Include each padded payload and every header/end block without allocating declared sizes.
    let mut minimum_tar_bytes = 7_u64 * 512;
    for length in [
        manifest.executable.byte_length,
        manifest.root_license.byte_length,
        manifest.third_party_notices.byte_length,
        bytes.len() as u64,
    ] {
        minimum_tar_bytes = length
            .checked_add(511)
            .map(|n| n / 512 * 512)
            .and_then(|padded| minimum_tar_bytes.checked_add(padded))
            .filter(|total| *total <= MAXIMUM_UNCOMPRESSED_BYTES)
            .ok_or_else(|| {
                ContainerError::corrupt(
                    "manifest payloads cannot fit the admitted container byte limit",
                )
            })?;
    }
    Ok(manifest)
}

/// Reserve bounded memory before growth, including when a stream lies about its length.
pub fn read_bounded(input: &mut impl Read, maximum: u64) -> Result<Vec<u8>, ContainerError> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = input
            .read(&mut buffer)
            .map_err(|error| ContainerError::corrupt(format!("read bounded container: {error}")))?;
        if count == 0 {
            break;
        }
        let length = bytes
            .len()
            .checked_add(count)
            .filter(|length| *length as u64 <= maximum)
            .ok_or_else(|| ContainerError::corrupt("container exceeds its byte limit"))?;
        bytes
            .try_reserve_exact(length - bytes.len())
            .map_err(|error| {
                ContainerError::infrastructure(format!("reserve container storage: {error}"))
            })?;
        bytes.extend_from_slice(&buffer[..count]);
    }
    Ok(bytes)
}

/// Closed ustar inventory. No member name ever becomes a filesystem path here.
pub fn parse_tar_bytes(bytes: &[u8]) -> Result<ParsedTar, ContainerError> {
    if bytes.len() as u64 > MAXIMUM_UNCOMPRESSED_BYTES {
        return Err(ContainerError::corrupt(
            "decompressed tar exceeds its byte limit",
        ));
    }
    let mut offset = 0_usize;
    let mut members = Vec::with_capacity(5);
    let mut ranges = Vec::with_capacity(5);
    let mut timestamp = None;
    for expected in EXPECTED_MEMBERS {
        let end = offset
            .checked_add(TAR_BLOCK_BYTES)
            .ok_or_else(|| ContainerError::corrupt("tar header offset overflow"))?;
        let header = bytes
            .get(offset..end)
            .ok_or_else(|| ContainerError::corrupt("truncated tar header"))?;
        let name = expected.name.as_bytes();
        if &header[..name.len()] != name
            || header[name.len()..100].iter().any(|byte| *byte != 0)
            || header[157..257].iter().any(|byte| *byte != 0)
            || &header[257..265] != b"ustar\x0000"
            || header[265..].iter().any(|byte| *byte != 0)
        {
            return Err(ContainerError::corrupt(format!(
                "noncanonical tar name, order, link, extension or metadata: expected {}",
                expected.name
            )));
        }
        let checksum = header
            .iter()
            .enumerate()
            .map(|(index, byte)| {
                u64::from(if (148..156).contains(&index) {
                    b' '
                } else {
                    *byte
                })
            })
            .sum::<u64>();
        if header[154..156] != [0, b' ']
            || header[148..154] != format!("{checksum:06o}").as_bytes()[..]
        {
            return Err(ContainerError::corrupt("archive header checksum mismatch"));
        }
        let mode = parse_octal(&header[100..108], "mode")?;
        let uid = parse_octal(&header[108..116], "uid")?;
        let gid = parse_octal(&header[116..124], "gid")?;
        let size = parse_octal(&header[124..136], "size")?;
        let time = parse_octal(&header[136..148], "mtime")?;
        let expected_kind = match expected.kind {
            MemberKind::Directory => b'5',
            MemberKind::File => b'0',
        };
        if mode != u64::from(expected.mode)
            || uid != 0
            || gid != 0
            || header[156] != expected_kind
            || (expected.kind == MemberKind::Directory && size != 0)
            || (expected.name == MANIFEST_MEMBER && size > MAXIMUM_MANIFEST_BYTES)
            || timestamp.is_some_and(|previous| previous != time)
        {
            return Err(ContainerError::corrupt(
                "archive member mode, kind, size or timestamp mismatch",
            ));
        }
        timestamp = Some(time);
        let size = usize::try_from(size)
            .map_err(|_| ContainerError::corrupt("tar member size overflow"))?;
        let payload_end = end
            .checked_add(size)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| ContainerError::corrupt("truncated or oversized tar member"))?;
        let padding = (TAR_BLOCK_BYTES - size % TAR_BLOCK_BYTES) % TAR_BLOCK_BYTES;
        offset = payload_end
            .checked_add(padding)
            .ok_or_else(|| ContainerError::corrupt("tar padding overflow"))?;
        if bytes
            .get(payload_end..offset)
            .is_none_or(|padding| padding.iter().any(|byte| *byte != 0))
        {
            return Err(ContainerError::corrupt("truncated or nonzero tar padding"));
        }
        members.push(ObservedMember {
            name: expected.name.to_owned(),
            mode: expected.mode,
            byte_length: size as u64,
            sha256: if expected.kind == MemberKind::File {
                Some(sha256_bytes(&bytes[end..payload_end])?)
            } else {
                None
            },
        });
        ranges.push(end..payload_end);
    }
    let trailing = &bytes[offset..];
    // GNU ustar emits records of 20 blocks; independent fixtures may end at the required two.
    let record_padding = (10240 - offset % 10240) % 10240;
    let canonical_padding = if record_padding < 1024 {
        record_padding + 10240
    } else {
        record_padding
    };
    if (trailing.len() != 1024 && trailing.len() != canonical_padding)
        || trailing.iter().any(|byte| *byte != 0)
    {
        return Err(ContainerError::corrupt(
            "archive has noncanonical terminal material",
        ));
    }
    Ok(ParsedTar {
        source_timestamp_unix_seconds: timestamp
            .ok_or_else(|| ContainerError::corrupt("empty archive"))?,
        members,
        ranges,
    })
}

fn parse_octal(field: &[u8], label: &str) -> Result<u64, ContainerError> {
    if field.last() != Some(&0)
        || field[..field.len() - 1]
            .iter()
            .any(|byte| !(b'0'..=b'7').contains(byte))
    {
        return Err(ContainerError::corrupt(format!(
            "noncanonical octal {label}"
        )));
    }
    field[..field.len() - 1]
        .iter()
        .try_fold(0_u64, |value, byte| {
            value
                .checked_mul(8)
                .and_then(|value| value.checked_add(u64::from(*byte - b'0')))
                .ok_or_else(|| ContainerError::corrupt(format!("octal {label} overflow")))
        })
}

#[cfg(test)]
pub(crate) mod tests;
