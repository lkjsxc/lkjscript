use crate::process::ProcessObservation;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

pub(super) const RECEIPT_SCHEMA: &str = "lkjscript-release-receipt";
pub(super) const RECEIPT_SCHEMA_VERSION: u32 = 2;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(super) struct Sha256Digest(String);

impl Sha256Digest {
    pub(super) fn new(value: String) -> Result<Self, String> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("SHA-256 digest must be 64 lowercase hexadecimal characters".to_owned());
        }
        Ok(Self(value))
    }

    pub(super) fn as_str(&self) -> &str {
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
pub(super) enum PublicationMode {
    DryRun,
    Release,
}

impl PublicationMode {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::Release => "release",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SchemaIdentity {
    pub(super) identity: String,
    pub(super) version: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProductIdentity {
    pub(super) name: String,
    pub(super) version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceIdentity {
    pub(super) repository: String,
    pub(super) expected_release_tag: String,
    pub(super) tagged_commit_sha: String,
    pub(super) commit_timestamp_unix_seconds: u64,
    pub(super) annotated_tag_object_sha: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ToolchainIdentity {
    pub(super) rustc: String,
    pub(super) cargo: String,
    pub(super) toolchain_channel: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ElfIdentity {
    pub(super) class: String,
    pub(super) machine: String,
    pub(super) object_type: String,
    pub(super) inspector: String,
    pub(super) program_headers: u32,
    pub(super) load_headers: u32,
    pub(super) dynamic_entries: u32,
    pub(super) interpreter_headers: u32,
    pub(super) needed_libraries: u32,
    pub(super) glibc_version_requirements: u32,
    pub(super) position_independent: bool,
    pub(super) runtime_linkage: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExecutableIdentity {
    pub(super) archive_mode: u32,
    pub(super) byte_length: u64,
    pub(super) sha256: Sha256Digest,
    pub(super) elf: ElfIdentity,
    pub(super) capabilities_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PayloadIdentity {
    pub(super) archive_mode: u32,
    pub(super) byte_length: u64,
    pub(super) sha256: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NoticeIdentity {
    pub(super) generator: String,
    pub(super) generator_version: String,
    pub(super) downloaded_archive_sha256: Sha256Digest,
    pub(super) executable_sha256: Sha256Digest,
    pub(super) invocation: Vec<String>,
    pub(super) archive_mode: u32,
    pub(super) byte_length: u64,
    pub(super) sha256: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArchiveMemberIdentity {
    pub(super) name: String,
    pub(super) mode: u32,
    pub(super) kind: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PackagingIdentity {
    pub(super) tar_format: String,
    pub(super) tar_version: String,
    pub(super) tar_invocation: Vec<String>,
    pub(super) gzip_version: String,
    pub(super) gzip_level: u8,
    pub(super) gzip_name_header: bool,
    pub(super) gzip_time_header: bool,
    pub(super) gzip_invocation: Vec<String>,
    pub(super) numeric_owner: u32,
    pub(super) numeric_group: u32,
    pub(super) source_timestamp_unix_seconds: u64,
    pub(super) members: Vec<ArchiveMemberIdentity>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReleaseManifest {
    pub(super) publication_mode: PublicationMode,
    pub(super) product: ProductIdentity,
    pub(super) source: SourceIdentity,
    pub(super) target_triple: String,
    pub(super) toolchain: ToolchainIdentity,
    pub(super) cargo_lock_sha256: Sha256Digest,
    pub(super) executable: ExecutableIdentity,
    pub(super) root_license: PayloadIdentity,
    pub(super) third_party_notices: NoticeIdentity,
    pub(super) packaging: PackagingIdentity,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum EvidenceClassification {
    FreshPassed,
    Reused,
    Skipped,
    Unavailable,
    Failed,
    NotProvided,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct VerificationClassification {
    pub(super) name: String,
    pub(super) classification: EvidenceClassification,
    pub(super) detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArtifactIdentity {
    pub(super) name: String,
    pub(super) byte_length: u64,
    pub(super) sha256: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExternalEvidence {
    pub(super) path: String,
    pub(super) byte_length: u64,
    pub(super) sha256: Sha256Digest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HostedContext {
    pub(super) github_actions: Option<String>,
    pub(super) repository: Option<String>,
    pub(super) workflow: Option<String>,
    pub(super) job: Option<String>,
    pub(super) run_id: Option<String>,
    pub(super) run_attempt: Option<String>,
    pub(super) run_url: Option<String>,
    pub(super) runner_os: Option<String>,
    pub(super) runner_architecture: Option<String>,
    pub(super) runner_image_os: Option<String>,
    pub(super) runner_image_version: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReleaseReceipt {
    pub(super) schema: SchemaIdentity,
    pub(super) publication_mode: PublicationMode,
    pub(super) release_tag: String,
    pub(super) commit_sha: String,
    pub(super) started_unix_nanoseconds: u128,
    pub(super) completed_unix_nanoseconds: u128,
    pub(super) elapsed_nanoseconds: u64,
    pub(super) hosted_context: HostedContext,
    pub(super) manifest_sha256: Sha256Digest,
    pub(super) archive: ArtifactIdentity,
    pub(super) checksum_file: ArtifactIdentity,
    pub(super) full_verification_receipt: Option<ExternalEvidence>,
    pub(super) target_admission_receipt: ExternalEvidence,
    pub(super) candidate_lifecycle: ProcessObservation,
    pub(super) classifications: Vec<VerificationClassification>,
}
