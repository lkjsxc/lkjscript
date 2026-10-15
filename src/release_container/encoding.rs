//! The release container has one canonical, publication-neutral JSON encoding.
use super::model::*;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Deserialize)]
enum Format {
    #[serde(rename = "lkjscript-release-content-1")]
    PublicationNeutral,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    format: Format,
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

impl<'de> Deserialize<'de> for ReleaseManifest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = Wire::deserialize(deserializer)?;
        let Format::PublicationNeutral = wire.format;
        Ok(Self {
            build: wire.build,
            product: wire.product,
            source: wire.source,
            target_triple: wire.target_triple,
            toolchain: wire.toolchain,
            cargo_lock_sha256: wire.cargo_lock_sha256,
            executable: wire.executable,
            root_license: wire.root_license,
            third_party_notices: wire.third_party_notices,
            packaging: wire.packaging,
        })
    }
}

impl Serialize for ReleaseManifest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ReleaseManifest", 11)?;
        state.serialize_field("format", PUBLICATION_NEUTRAL_FORMAT)?;
        state.serialize_field("build", &self.build)?;
        state.serialize_field("product", &self.product)?;
        state.serialize_field("source", &self.source)?;
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
