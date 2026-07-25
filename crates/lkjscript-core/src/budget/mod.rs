mod category;

use std::fmt;

pub use category::ResourceCategory;
pub(crate) use category::RESOURCE_CATEGORY_COUNT;

use crate::{ResourceProfile, ResourceProfileIdentity};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceDiagnostic {
    pub profile: ResourceProfileIdentity,
    pub category: ResourceCategory,
    pub limit: u64,
    pub before: u64,
    pub increment: u64,
}

impl fmt::Display for ResourceDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "compiler resource limit: profile={}/{}:{}; category={}; unit={}; limit={}; before={}; increment={}",
            self.profile.schema,
            self.profile.version,
            self.profile.name.as_str(),
            self.category.as_str(),
            self.category.unit(),
            self.limit,
            self.before,
            self.increment
        )
    }
}

impl std::error::Error for ResourceDiagnostic {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceUsage {
    profile: ResourceProfileIdentity,
    used: [u64; RESOURCE_CATEGORY_COUNT],
}

impl ResourceUsage {
    pub const fn profile(self) -> ResourceProfileIdentity {
        self.profile
    }

    pub const fn used(self, category: ResourceCategory) -> u64 {
        self.used[category.index()]
    }
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            profile: ResourceProfile::default().identity(),
            used: [0; RESOURCE_CATEGORY_COUNT],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BudgetLedger {
    profile: ResourceProfile,
    used: [u64; RESOURCE_CATEGORY_COUNT],
}

impl BudgetLedger {
    pub const fn new(profile: ResourceProfile) -> Self {
        Self {
            profile,
            used: [0; RESOURCE_CATEGORY_COUNT],
        }
    }

    pub const fn profile(&self) -> ResourceProfile {
        self.profile
    }

    pub const fn used(&self, category: ResourceCategory) -> u64 {
        self.used[category.index()]
    }

    pub fn usage(&self) -> ResourceUsage {
        ResourceUsage {
            profile: self.profile.identity(),
            used: self.used,
        }
    }

    pub fn charge(
        &mut self,
        category: ResourceCategory,
        increment: u64,
    ) -> Result<(), ResourceDiagnostic> {
        let before = self.used(category);
        let limit = self.profile.ceilings().limit(category);
        let after = before
            .checked_add(increment)
            .ok_or_else(|| self.diagnostic(category, limit, before, increment))?;
        if after > limit {
            return Err(self.diagnostic(category, limit, before, increment));
        }
        self.used[category.index()] = after;
        Ok(())
    }

    fn diagnostic(
        &self,
        category: ResourceCategory,
        limit: u64,
        before: u64,
        increment: u64,
    ) -> ResourceDiagnostic {
        ResourceDiagnostic {
            profile: self.profile.identity(),
            category,
            limit,
            before,
            increment,
        }
    }
}

impl Default for BudgetLedger {
    fn default() -> Self {
        Self::new(ResourceProfile::default())
    }
}

#[cfg(test)]
mod tests;
