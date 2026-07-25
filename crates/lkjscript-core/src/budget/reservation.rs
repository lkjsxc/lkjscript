use super::{BudgetCause, BudgetError, BudgetErrorKind, BudgetPath, BudgetScope, ResourceCategory};
use crate::ResourceProfileIdentity;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReservationId(u64);

impl ReservationId {
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
}

impl<'a> Reservation<'a> {
    pub(crate) fn new(
        scope: &'a mut BudgetScope<'_>,
        id: u64,
        category: ResourceCategory,
        amount: u64,
        cause: BudgetCause,
    ) -> Self {
        let index = category.index();
        Self {
            id: ReservationId(id),
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

    #[allow(
        clippy::result_large_err,
        reason = "budget rejection must not allocate"
    )]
    pub fn consume(&mut self, amount: u64) -> Result<(), BudgetError> {
        if amount > self.remaining {
            return Err(self.error(amount));
        }
        *self.reserved -= amount;
        *self.used = self.used.saturating_add(amount);
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
        self.remaining = 0;
        self.state = ReservationState::Returned;
    }

    fn apply_commit(&mut self, amount: u64) {
        *self.reserved -= amount;
        *self.used = self.used.saturating_add(amount);
        self.remaining -= amount;
    }

    fn error(&self, attempted: u64) -> BudgetError {
        BudgetError {
            kind: BudgetErrorKind::ReservationExceeded,
            profile: self.profile,
            category: self.category,
            authority: self.owner.authority(),
            path: self.owner,
            cause: self.cause,
            limit: self.limit,
            reserved: *self.reserved,
            attempted,
            observed: *self.used,
            allocated_before_rejection: false,
        }
    }
}

impl Drop for Reservation<'_> {
    fn drop(&mut self) {
        if self.remaining != 0 {
            let amount = self.remaining;
            self.apply_commit(amount);
            self.state = ReservationState::Consumed;
        }
    }
}
