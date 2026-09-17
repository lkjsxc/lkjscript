//! The two accepted container encodings are closed and disjoint. Legacy serialization exists
//! only to preserve canonical read admission and retained installation receipts unchanged.
use super::model::*;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Deserialize)]
enum NeutralFormat {
    #[serde(rename = "lkjscript-release-content-1")]
    PublicationNeutral,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NeutralWire {
    format: NeutralFormat,
    build: BuildIdentity,
    product: ProductIdentity,
    source: SourceIdentity,
    target_triple: String,
    toolchain: ToolchainIdentity,
    cargo_lock_sha256: Sha256Digest,
    executable: ExecutableIdentity,
    root_license: PayloadIdentity,
    third_party_notices: NoticeIdentity,
    packaging: PackagingIdentity,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySource {
    repository: String,
    expected_release_tag: String,
    tagged_commit_sha: String,
    commit_timestamp_unix_seconds: u64,
    annotated_tag_object_sha: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyWire {
    publication_mode: PublicationMode,
    product: ProductIdentity,
    source: LegacySource,
    target_triple: String,
    toolchain: ToolchainIdentity,
    cargo_lock_sha256: Sha256Digest,
    executable: ExecutableIdentity,
    root_license: PayloadIdentity,
    third_party_notices: NoticeIdentity,
    packaging: PackagingIdentity,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Wire {
    Neutral(NeutralWire),
    Legacy(LegacyWire),
}

impl<'de> Deserialize<'de> for ReleaseManifest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Neutral(wire) => {
                let NeutralFormat::PublicationNeutral = wire.format;
                Self {
                    encoding: ManifestEncoding::PublicationNeutral { build: wire.build },
                    product: wire.product,
                    source: wire.source,
                    target_triple: wire.target_triple,
                    toolchain: wire.toolchain,
                    cargo_lock_sha256: wire.cargo_lock_sha256,
                    executable: wire.executable,
                    root_license: wire.root_license,
                    third_party_notices: wire.third_party_notices,
                    packaging: wire.packaging,
                }
            }
            Wire::Legacy(wire) => Self {
                encoding: ManifestEncoding::Legacy {
                    publication_mode: wire.publication_mode,
                    annotated_tag_object_sha: wire.source.annotated_tag_object_sha,
                },
                product: wire.product,
                source: SourceIdentity {
                    repository: wire.source.repository,
                    expected_release_tag: wire.source.expected_release_tag,
                    tagged_commit_sha: wire.source.tagged_commit_sha,
                    commit_timestamp_unix_seconds: wire.source.commit_timestamp_unix_seconds,
                },
                target_triple: wire.target_triple,
                toolchain: wire.toolchain,
                cargo_lock_sha256: wire.cargo_lock_sha256,
                executable: wire.executable,
                root_license: wire.root_license,
                third_party_notices: wire.third_party_notices,
                packaging: wire.packaging,
            },
        })
    }
}

#[derive(Serialize)]
struct LegacySourceRef<'a> {
    repository: &'a str,
    expected_release_tag: &'a str,
    tagged_commit_sha: &'a str,
    commit_timestamp_unix_seconds: u64,
    annotated_tag_object_sha: &'a Option<String>,
}

impl Serialize for ReleaseManifest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let neutral = self.is_publication_neutral();
        let mut state =
            serializer.serialize_struct("ReleaseManifest", if neutral { 11 } else { 10 })?;
        match &self.encoding {
            ManifestEncoding::PublicationNeutral { build } => {
                state.serialize_field("format", PUBLICATION_NEUTRAL_FORMAT)?;
                state.serialize_field("build", build)?;
            }
            ManifestEncoding::Legacy {
                publication_mode, ..
            } => state.serialize_field("publication_mode", publication_mode)?,
        }
        state.serialize_field("product", &self.product)?;
        match &self.encoding {
            ManifestEncoding::PublicationNeutral { .. } => {
                state.serialize_field("source", &self.source)?;
            }
            ManifestEncoding::Legacy {
                annotated_tag_object_sha,
                ..
            } => state.serialize_field(
                "source",
                &LegacySourceRef {
                    repository: &self.source.repository,
                    expected_release_tag: &self.source.expected_release_tag,
                    tagged_commit_sha: &self.source.tagged_commit_sha,
                    commit_timestamp_unix_seconds: self.source.commit_timestamp_unix_seconds,
                    annotated_tag_object_sha,
                },
            )?,
        }
        state.serialize_field("target_triple", &self.target_triple)?;
        state.serialize_field("toolchain", &self.toolchain)?;
        state.serialize_field("cargo_lock_sha256", &self.cargo_lock_sha256)?;
        state.serialize_field("executable", &self.executable)?;
        state.serialize_field("root_license", &self.root_license)?;
        state.serialize_field("third_party_notices", &self.third_party_notices)?;
        state.serialize_field("packaging", &self.packaging)?;
        state.end()
    }
}
