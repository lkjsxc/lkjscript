mod authority;
mod category;
mod category_names;
mod category_units;
mod diagnostic;
mod legacy;
mod path;
mod reservation;
mod scope;

pub use authority::BudgetAuthority;
pub use category::ResourceCategory;
pub(crate) use category::{RESOURCE_CATEGORY_COUNT, V1_RESOURCE_CATEGORY_COUNT};
pub use diagnostic::{BudgetCause, BudgetError, BudgetErrorKind};
pub use legacy::ResourceDiagnostic;
pub use path::{BudgetPath, MAX_BUDGET_PATH_DEPTH};
pub use reservation::{Reservation, ReservationId, ReservationState};
pub use scope::BudgetScope;

use crate::{ResourceProfile, ResourceProfileIdentity};

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
    pub(crate) used: [u64; RESOURCE_CATEGORY_COUNT],
    next_reservation: u64,
}

impl BudgetLedger {
    pub const fn new(profile: ResourceProfile) -> Self {
        Self {
            profile,
            used: [0; RESOURCE_CATEGORY_COUNT],
            next_reservation: 0,
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

    pub fn scope(&mut self, authority: BudgetAuthority) -> BudgetScope<'_> {
        let mut grant = [0; RESOURCE_CATEGORY_COUNT];
        for category in ResourceCategory::ALL {
            let index = category.index();
            grant[index] = self
                .profile
                .ceilings()
                .limit(category)
                .saturating_sub(self.used[index]);
        }
        BudgetScope {
            profile: self.profile,
            path: BudgetPath::root(authority),
            grant,
            used: [0; RESOURCE_CATEGORY_COUNT],
            reserved: [0; RESOURCE_CATEGORY_COUNT],
            sink: &mut self.used,
            next_reservation: &mut self.next_reservation,
        }
    }

    #[allow(
        clippy::result_large_err,
        reason = "budget rejection must not allocate"
    )]
    pub fn charge_with_authority(
        &mut self,
        authority: Option<BudgetAuthority>,
        category: ResourceCategory,
        amount: u64,
        cause: BudgetCause,
    ) -> Result<(), BudgetError> {
        let Some(authority) = authority else {
            return Err(BudgetError {
                kind: BudgetErrorKind::MissingAuthority,
                profile: self.profile.identity(),
                category,
                authority: None,
                path: BudgetPath::empty(),
                cause,
                limit: self.profile.ceilings().limit(category),
                reserved: 0,
                attempted: amount,
                observed: self.used(category),
                allocated_before_rejection: false,
            });
        };
        let mut scope = self.scope(authority);
        scope.reserve(category, amount, cause)?.commit();
        Ok(())
    }

    /// Legacy Current compiler post-phase wrapper. New allocation authorities
    /// must use a typed scope and reserve before allocation.
    pub fn charge(
        &mut self,
        category: ResourceCategory,
        increment: u64,
    ) -> Result<(), ResourceDiagnostic> {
        let before = self.used(category);
        let limit = self.profile.ceilings().limit(category);
        let Some(after) = before.checked_add(increment) else {
            return Err(self.legacy_diagnostic(category, limit, before, increment));
        };
        if after > limit {
            return Err(self.legacy_diagnostic(category, limit, before, increment));
        }
        self.used[category.index()] = after;
        Ok(())
    }

    fn legacy_diagnostic(
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
mod hierarchy_tests;
#[cfg(test)]
mod tests;
