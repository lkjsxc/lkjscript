mod ceilings;
mod v1;
mod v2;

use std::fmt;
use std::str::FromStr;

use crate::budget::{ResourceCategory, RESOURCE_CATEGORY_COUNT};
use crate::sha256;

pub use ceilings::ResourceCeilings;
use ceilings::{BUILD, DEFAULT, DETERMINISTIC, MAXIMA, SANDBOX};

pub const RESOURCE_PROFILE_SCHEMA: &str = "lkjscript.resource-profile";
pub const RESOURCE_PROFILE_VERSION: u32 = 2;
pub const IMPLEMENTATION_MAXIMA_VERSION: u32 = 2;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResourceProfileName {
    Sandbox,
    Default,
    Build,
    TrustedLocal,
    Deterministic,
}

impl ResourceProfileName {
    pub const ALL: [Self; 5] = [
        Self::Sandbox,
        Self::Default,
        Self::Build,
        Self::TrustedLocal,
        Self::Deterministic,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sandbox => "sandbox",
            Self::Default => "default",
            Self::Build => "build",
            Self::TrustedLocal => "trusted-local",
            Self::Deterministic => "deterministic",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnknownResourceProfile {
    name: String,
}

impl UnknownResourceProfile {
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for UnknownResourceProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown resource profile {:?}", self.name)
    }
}

impl std::error::Error for UnknownResourceProfile {}

impl FromStr for ResourceProfileName {
    type Err = UnknownResourceProfile;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == name)
            .ok_or_else(|| UnknownResourceProfile {
                name: name.to_owned(),
            })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceProfileIdentity {
    pub schema: &'static str,
    pub version: u32,
    pub name: ResourceProfileName,
    pub implementation_maxima_version: u32,
    pub ceilings_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceProfile {
    name: ResourceProfileName,
    ceilings: ResourceCeilings,
}

impl ResourceProfile {
    pub const fn new(name: ResourceProfileName) -> Self {
        let values = match name {
            ResourceProfileName::Sandbox => SANDBOX,
            ResourceProfileName::Default => DEFAULT,
            ResourceProfileName::Build => BUILD,
            ResourceProfileName::TrustedLocal => MAXIMA,
            ResourceProfileName::Deterministic => DETERMINISTIC,
        };
        Self {
            name,
            ceilings: ResourceCeilings { values },
        }
    }

    pub fn named(name: &str) -> Result<Self, UnknownResourceProfile> {
        name.parse().map(Self::new)
    }

    pub const fn name(self) -> ResourceProfileName {
        self.name
    }

    pub const fn ceilings(self) -> ResourceCeilings {
        self.ceilings
    }

    pub fn identity(self) -> ResourceProfileIdentity {
        let mut encoded = [0u8; RESOURCE_CATEGORY_COUNT * 8];
        for (slot, limit) in encoded.chunks_exact_mut(8).zip(self.ceilings.values) {
            slot.copy_from_slice(&limit.to_be_bytes());
        }
        ResourceProfileIdentity {
            schema: RESOURCE_PROFILE_SCHEMA,
            version: RESOURCE_PROFILE_VERSION,
            name: self.name,
            implementation_maxima_version: IMPLEMENTATION_MAXIMA_VERSION,
            ceilings_sha256: sha256(&encoded),
        }
    }

    pub fn lowered(self, category: ResourceCategory, limit: u64) -> Result<Self, InvalidCeiling> {
        let current = self.ceilings.limit(category);
        if limit > current {
            return Err(InvalidCeiling {
                category,
                current,
                requested: limit,
            });
        }
        let mut lowered = self;
        lowered.ceilings.values[category.index()] = limit;
        Ok(lowered)
    }
}

impl Default for ResourceProfile {
    fn default() -> Self {
        Self::new(ResourceProfileName::Default)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidCeiling {
    pub category: ResourceCategory,
    pub current: u64,
    pub requested: u64,
}

impl fmt::Display for InvalidCeiling {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot raise {} ceiling from {} to {}",
            self.category.as_str(),
            self.current,
            self.requested
        )
    }
}

impl std::error::Error for InvalidCeiling {}

#[cfg(test)]
mod tests;
