use super::{
    BudgetAuthority, BudgetCause, BudgetError, BudgetErrorKind, BudgetJournal, BudgetPath,
    BudgetPrefix, BudgetRejectedEvent, BudgetScope, ResourceCategory, ResourceDiagnostic,
    RESOURCE_CATEGORY_COUNT,
};
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
    legacy_used: [u64; RESOURCE_CATEGORY_COUNT],
    pub(crate) next_reservation: u64,
    journal: BudgetJournal,
}

impl BudgetLedger {
    pub const fn new(profile: ResourceProfile) -> Self {
        Self {
            profile,
            used: [0; RESOURCE_CATEGORY_COUNT],
            legacy_used: [0; RESOURCE_CATEGORY_COUNT],
            next_reservation: 0,
            journal: BudgetJournal::new(),
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

    pub fn prefix(&self) -> BudgetPrefix {
        self.journal
            .prefix(self.profile.identity(), &self.legacy_used, None)
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
            journal: &mut self.journal,
            prefix_base: self.legacy_used,
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
            let event = BudgetRejectedEvent {
                kind: BudgetErrorKind::MissingAuthority,
                category,
                authority: None,
                path: BudgetPath::empty(),
                cause,
                limit: self.profile.ceilings().limit(category),
                reserved: 0,
                attempted: amount,
                observed: self.used(category),
                allocated_before_rejection: false,
            };
            return Err(BudgetError::new(
                self.profile.identity(),
                event,
                &self.journal,
                &self.legacy_used,
            ));
        };
        self.scope(authority)
            .reserve(category, amount, cause)?
            .commit();
        Ok(())
    }

    /// Legacy post-phase accounting. This separate diagnostic does not claim
    /// that allocation was prevented and does not carry a journal prefix.
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
        let Some(legacy_after) = self.legacy_used[category.index()].checked_add(increment) else {
            return Err(self.legacy_diagnostic(category, limit, before, increment));
        };
        if after > limit {
            return Err(self.legacy_diagnostic(category, limit, before, increment));
        }
        self.used[category.index()] = after;
        self.legacy_used[category.index()] = legacy_after;
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
