use std::path::PathBuf;

use lkjscript_core::ResourceProfileIdentity;
use lkjscript_native::InstallableImage;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CacheTier {
    Baseline,
    Optimizing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheContext {
    pub package_root: PathBuf,
    pub module_path: String,
    pub source_sha256: [u8; 32],
    pub module_sha256: [u8; 32],
    pub package_sha256: [u8; 32],
    pub lock_sha256: [u8; 32],
    pub profile: ResourceProfileIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactKey {
    digest: [u8; 32],
}

impl ArtifactKey {
    pub(crate) const fn new(digest: [u8; 32]) -> Self {
        Self { digest }
    }

    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }

    pub fn hex(&self) -> String {
        lkjscript_contracts::ContractDigest::from_bytes(self.digest).to_hex()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissReason {
    NotFound,
    Corrupt,
    OverLimit,
}

pub enum Lookup {
    Hit {
        image: Box<InstallableImage>,
        bytes: u64,
    },
    Miss(MissReason),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Publication {
    Published { bytes: u64 },
    Duplicate { bytes: u64 },
    SkippedFull,
    SkippedBusy,
}
