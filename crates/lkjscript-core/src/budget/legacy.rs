use std::fmt;

use super::ResourceCategory;
use crate::ResourceProfileIdentity;

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
            "legacy compiler resource limit: profile={}/{}:{}; category={}; unit={}; limit={}; before={}; increment={}",
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
