use super::{
    BudgetAuthority, BudgetCause, BudgetError, BudgetErrorKind, BudgetJournal, BudgetPath,
    BudgetPrefix, BudgetRejectedEvent, Reservation, ReservationId, ResourceCategory,
    RESOURCE_CATEGORY_COUNT,
};
use crate::ResourceProfile;

pub struct BudgetScope<'a> {
    pub(crate) profile: ResourceProfile,
    pub(crate) path: BudgetPath,
    pub(crate) grant: [u64; RESOURCE_CATEGORY_COUNT],
    pub(crate) used: [u64; RESOURCE_CATEGORY_COUNT],
    pub(crate) reserved: [u64; RESOURCE_CATEGORY_COUNT],
    pub(crate) sink: &'a mut [u64; RESOURCE_CATEGORY_COUNT],
    pub(crate) next_reservation: &'a mut u64,
    pub(crate) journal: &'a mut BudgetJournal,
    pub(crate) prefix_base: [u64; RESOURCE_CATEGORY_COUNT],
}

impl<'a> BudgetScope<'a> {
    pub const fn path(&self) -> BudgetPath {
        self.path
    }

    pub const fn authority(&self) -> BudgetAuthority {
        match self.path.authority() {
            Some(authority) => authority,
            None => BudgetAuthority::CompileRequest,
        }
    }

    pub const fn grant(&self, category: ResourceCategory) -> u64 {
        self.grant[category.index()]
    }

    pub const fn observed(&self, category: ResourceCategory) -> u64 {
        self.used[category.index()]
    }

    pub const fn reserved(&self, category: ResourceCategory) -> u64 {
        self.reserved[category.index()]
    }

    pub fn prefix(&self) -> BudgetPrefix {
        self.journal
            .prefix(self.profile.identity(), &self.prefix_base, None)
    }

    #[allow(
        clippy::result_large_err,
        reason = "budget rejection must not allocate"
    )]
    pub fn lower_grant(
        &mut self,
        category: ResourceCategory,
        limit: u64,
    ) -> Result<(), BudgetError> {
        let index = category.index();
        let occupied = self.used[index].saturating_add(self.reserved[index]);
        if limit > self.grant[index] || limit < occupied {
            let kind = if limit > self.grant[index] {
                BudgetErrorKind::GrantExceedsParent
            } else {
                BudgetErrorKind::GrantBelowObserved
            };
            return Err(self.error(kind, category, limit, BudgetCause::Request));
        }
        self.grant[index] = limit;
        Ok(())
    }

    #[allow(
        clippy::result_large_err,
        reason = "budget rejection must not allocate"
    )]
    pub fn child(&mut self, authority: BudgetAuthority) -> Result<BudgetScope<'_>, BudgetError> {
        let Some(path) = self.path.pushed(authority) else {
            return Err(self.error(
                BudgetErrorKind::PathTooDeep,
                ResourceCategory::PathWork,
                1,
                BudgetCause::Request,
            ));
        };
        let mut grant = [0; RESOURCE_CATEGORY_COUNT];
        for category in ResourceCategory::ALL {
            let index = category.index();
            grant[index] = self.grant[index]
                .saturating_sub(self.used[index])
                .saturating_sub(self.reserved[index]);
        }
        Ok(BudgetScope {
            profile: self.profile,
            path,
            grant,
            used: [0; RESOURCE_CATEGORY_COUNT],
            reserved: [0; RESOURCE_CATEGORY_COUNT],
            sink: &mut self.used,
            next_reservation: self.next_reservation,
            journal: self.journal,
            prefix_base: self.prefix_base,
        })
    }

    #[allow(
        clippy::result_large_err,
        reason = "budget rejection must not allocate"
    )]
    pub fn reserve(
        &mut self,
        category: ResourceCategory,
        amount: u64,
        cause: BudgetCause,
    ) -> Result<Reservation<'_>, BudgetError> {
        let index = category.index();
        let remaining = self.grant[index]
            .checked_sub(self.used[index])
            .and_then(|value| value.checked_sub(self.reserved[index]));
        if remaining.is_none_or(|value| amount > value) {
            return Err(self.error(BudgetErrorKind::LimitExceeded, category, amount, cause));
        }
        if !self.journal.has_capacity() {
            return Err(self.error(BudgetErrorKind::JournalExhausted, category, amount, cause));
        }
        let Some(id) = self.next_reservation.checked_add(1) else {
            return Err(self.error(
                BudgetErrorKind::ReservationIdExhausted,
                category,
                amount,
                cause,
            ));
        };
        let journal_index =
            self.journal
                .push(ReservationId::new(id), self.path, category, cause, amount);
        *self.next_reservation = id;
        self.reserved[index] += amount;
        Ok(Reservation::new(
            self,
            id,
            journal_index,
            category,
            amount,
            cause,
        ))
    }

    pub(crate) fn error(
        &self,
        kind: BudgetErrorKind,
        category: ResourceCategory,
        attempted: u64,
        cause: BudgetCause,
    ) -> BudgetError {
        let index = category.index();
        let event = BudgetRejectedEvent {
            kind,
            category,
            authority: self.path.authority(),
            path: self.path,
            cause,
            limit: self.grant[index],
            reserved: self.reserved[index],
            attempted,
            observed: self.used[index],
            allocated_before_rejection: false,
        };
        BudgetError::new(
            self.profile.identity(),
            event,
            self.journal,
            &self.prefix_base,
        )
    }
}

impl Drop for BudgetScope<'_> {
    fn drop(&mut self) {
        self.journal.commit_owner_remainders(self.path);
        for index in 0..RESOURCE_CATEGORY_COUNT {
            self.used[index] = self.used[index].saturating_add(self.reserved[index]);
            self.sink[index] = self.sink[index].saturating_add(self.used[index]);
            self.reserved[index] = 0;
        }
    }
}
