use super::{
    BudgetCause, BudgetError, BudgetErrorKind, BudgetJournal, BudgetPath, BudgetPrefix,
    BudgetRejectedEvent, BudgetScope, ResourceCategory, RESOURCE_CATEGORY_COUNT,
};
use crate::ResourceProfileIdentity;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReservationId(u64);

impl ReservationId {
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReservationState {
    Active,
    PartiallyConsumed,
    Consumed,
    Returned,
}

#[derive(Debug)]
pub struct Reservation<'a> {
    id: ReservationId,
    profile: ResourceProfileIdentity,
    owner: BudgetPath,
    category: ResourceCategory,
    cause: BudgetCause,
    amount: u64,
    remaining: u64,
    limit: u64,
    state: ReservationState,
    used: &'a mut u64,
    reserved: &'a mut u64,
    journal: &'a mut BudgetJournal,
    journal_index: usize,
    prefix_base: [u64; RESOURCE_CATEGORY_COUNT],
}

impl<'a> Reservation<'a> {
    pub(crate) fn new(
        scope: &'a mut BudgetScope<'_>,
        id: u64,
        journal_index: usize,
        category: ResourceCategory,
        amount: u64,
        cause: BudgetCause,
    ) -> Self {
        let index = category.index();
        Self {
            id: ReservationId::new(id),
            profile: scope.profile.identity(),
            owner: scope.path,
            category,
            cause,
            amount,
            remaining: amount,
            limit: scope.grant[index],
            state: ReservationState::Active,
            used: &mut scope.used[index],
            reserved: &mut scope.reserved[index],
            journal: scope.journal,
            journal_index,
            prefix_base: scope.prefix_base,
        }
    }

    pub const fn id(&self) -> ReservationId {
        self.id
    }

    pub const fn owner(&self) -> BudgetPath {
        self.owner
    }

    pub const fn category(&self) -> ResourceCategory {
        self.category
    }

    pub const fn cause(&self) -> BudgetCause {
        self.cause
    }

    pub const fn amount(&self) -> u64 {
        self.amount
    }

    pub const fn remaining(&self) -> u64 {
        self.remaining
    }

    pub const fn state(&self) -> ReservationState {
        self.state
    }

    pub fn prefix(&self) -> BudgetPrefix {
        self.journal.prefix(self.profile, &self.prefix_base, None)
    }

    #[allow(
        clippy::result_large_err,
        reason = "budget rejection must not allocate"
    )]
    pub fn consume(&mut self, amount: u64) -> Result<(), BudgetError> {
        if amount > self.remaining {
            return Err(self.error(amount));
        }
        *self.reserved -= amount;
        *self.used += amount;
        self.journal.consume(self.journal_index, amount);
        self.remaining -= amount;
        self.state = if self.remaining == 0 {
            ReservationState::Consumed
        } else if self.remaining == self.amount {
            ReservationState::Active
        } else {
            ReservationState::PartiallyConsumed
        };
        Ok(())
    }

    pub fn commit(mut self) {
        let amount = self.remaining;
        self.apply_commit(amount);
        self.state = ReservationState::Consumed;
    }

    pub fn return_unused(mut self) {
        *self.reserved -= self.remaining;
        self.journal
            .return_units(self.journal_index, self.remaining);
        self.remaining = 0;
        self.state = ReservationState::Returned;
    }

    fn apply_commit(&mut self, amount: u64) {
        *self.reserved -= amount;
        *self.used += amount;
        self.journal.consume(self.journal_index, amount);
        self.remaining -= amount;
    }

    fn error(&self, attempted: u64) -> BudgetError {
        let event = BudgetRejectedEvent {
            kind: BudgetErrorKind::ReservationExceeded,
            category: self.category,
            authority: self.owner.authority(),
            path: self.owner,
            cause: self.cause,
            limit: self.limit,
            reserved: *self.reserved,
            attempted,
            observed: *self.used,
            allocated_before_rejection: false,
        };
        BudgetError::new(self.profile, event, self.journal, &self.prefix_base)
    }
}

impl Drop for Reservation<'_> {
    fn drop(&mut self) {
        if self.remaining != 0 {
            let amount = self.remaining;
            self.apply_commit(amount);
            self.journal.mark_conservative_drop(self.journal_index);
            self.state = ReservationState::Consumed;
        }
    }
}
