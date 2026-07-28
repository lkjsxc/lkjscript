mod ceiling_sets;
mod ceilings;
mod identity;

use std::fmt;
use std::str::FromStr;

use crate::budget::ResourceCategory;

use ceiling_sets::{BUILD, DEFAULT, DETERMINISTIC, MAXIMA, SANDBOX};
pub use ceilings::ResourceCeilings;
pub use identity::ResourceProfileIdentity;

pub const RESOURCE_PROFILE_SCHEMA: &str = "lkjscript.resource-profile";

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
        identity::identity(self)
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
mod scheduler_tests;
#[cfg(test)]
mod tests;
