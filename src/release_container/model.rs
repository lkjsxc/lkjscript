use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Sha256Digest(String);

impl Sha256Digest {
    pub fn new(value: String) -> Result<Self, String> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("SHA-256 digest must be 64 lowercase hexadecimal characters".to_owned());
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Sha256Digest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PublicationMode {
    DryRun,
    Release,
}

impl PublicationMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::Release => "release",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProductIdentity {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    pub repository: String,
    pub expected_release_tag: String,
    pub tagged_commit_sha: String,
    pub commit_timestamp_unix_seconds: u64,
    pub annotated_tag_object_sha: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ToolchainIdentity {
    pub rustc: String,
    pub cargo: String,
    pub toolchain_channel: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ElfIdentity {
    pub class: String,
    pub machine: String,
    pub object_type: String,
    pub inspector: String,
    pub program_headers: u32,
    pub load_headers: u32,
    pub dynamic_entries: u32,
    pub interpreter_headers: u32,
    pub needed_libraries: u32,
    pub glibc_version_requirements: u32,
    pub position_independent: bool,
    pub runtime_linkage: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutableIdentity {
    pub archive_mode: u32,
    pub byte_length: u64,
    pub sha256: Sha256Digest,
    pub elf: ElfIdentity,
    pub capabilities_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PayloadIdentity {
    pub archive_mode: u32,
    pub byte_length: u64,
    pub sha256: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NoticeIdentity {
    pub generator: String,
    pub generator_version: String,
    pub downloaded_archive_sha256: Sha256Digest,
    pub executable_sha256: Sha256Digest,
    pub invocation: Vec<String>,
    pub archive_mode: u32,
    pub byte_length: u64,
    pub sha256: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveMemberIdentity {
    pub name: String,
    pub mode: u32,
    pub kind: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackagingIdentity {
    pub tar_format: String,
    pub tar_version: String,
    pub tar_invocation: Vec<String>,
    pub gzip_version: String,
    pub gzip_level: u8,
    pub gzip_name_header: bool,
    pub gzip_time_header: bool,
    pub gzip_invocation: Vec<String>,
    pub numeric_owner: u32,
    pub numeric_group: u32,
    pub source_timestamp_unix_seconds: u64,
    pub members: Vec<ArchiveMemberIdentity>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseManifest {
    pub publication_mode: PublicationMode,
    pub product: ProductIdentity,
    pub source: SourceIdentity,
    pub target_triple: String,
    pub toolchain: ToolchainIdentity,
    pub cargo_lock_sha256: Sha256Digest,
    pub executable: ExecutableIdentity,
    pub root_license: PayloadIdentity,
    pub third_party_notices: NoticeIdentity,
    pub packaging: PackagingIdentity,
}
