use std::fmt;

use super::ResourceCategory;
use crate::ResourceProfileIdentity;

/// Legacy compiler post-phase diagnostic.
///
/// This does not carry a journal prefix or claim that allocation was prevented.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceDiagnostic {
    pub profile: Box<ResourceProfileIdentity>,
    pub category: ResourceCategory,
    pub limit: u64,
    pub before: u64,
    pub increment: u64,
}

impl fmt::Display for ResourceDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            concat!(
                "compiler resource limit: profile={}:{}; contract={}; category={}; ",
                "unit={}; limit={}; before={}; increment={}"
            ),
            self.profile.schema,
            self.profile.name.as_str(),
            self.profile.contract,
            self.category.as_str(),
            self.category.unit(),
            self.limit,
            self.before,
            self.increment
        )
    }
}

impl std::error::Error for ResourceDiagnostic {}
