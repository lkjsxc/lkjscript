use super::{
    BudgetAuthority, BudgetCause, BudgetErrorKind, BudgetPath, ReservationId, ResourceCategory,
    RESOURCE_CATEGORY_COUNT,
};
use crate::ResourceProfileIdentity;

pub const MAX_BUDGET_JOURNAL_ENTRIES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReservationJournalRecord {
    pub id: ReservationId,
    pub owner: BudgetPath,
    pub category: ResourceCategory,
    pub cause: BudgetCause,
    pub amount: u64,
    pub consumed: u64,
    pub returned: u64,
    pub conservative_drop: bool,
}

impl ReservationJournalRecord {
    const EMPTY: Self = Self {
        id: ReservationId::new(0),
        owner: BudgetPath::empty(),
        category: ResourceCategory::SourceBytes,
        cause: BudgetCause::Request,
        amount: 0,
        consumed: 0,
        returned: 0,
        conservative_drop: false,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BudgetRejectedEvent {
    pub kind: BudgetErrorKind,
    pub category: ResourceCategory,
    pub authority: Option<BudgetAuthority>,
    pub path: BudgetPath,
    pub cause: BudgetCause,
    pub limit: u64,
    pub reserved: u64,
    pub attempted: u64,
    pub observed: u64,
    pub allocated_before_rejection: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BudgetPrefix {
    profile: ResourceProfileIdentity,
    committed: [u64; RESOURCE_CATEGORY_COUNT],
    records: [ReservationJournalRecord; MAX_BUDGET_JOURNAL_ENTRIES],
    record_count: u16,
    rejected: Option<BudgetRejectedEvent>,
}

impl BudgetPrefix {
    pub const fn profile(&self) -> ResourceProfileIdentity {
        self.profile
    }

    pub const fn committed(&self, category: ResourceCategory) -> u64 {
        self.committed[category.index()]
    }

    pub const fn reservation_count(&self) -> usize {
        self.record_count as usize
    }

    pub fn reservations(&self) -> &[ReservationJournalRecord] {
        &self.records[..self.reservation_count()]
    }

    pub const fn rejected(&self) -> Option<&BudgetRejectedEvent> {
        self.rejected.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BudgetJournal {
    records: [ReservationJournalRecord; MAX_BUDGET_JOURNAL_ENTRIES],
    len: u16,
}

impl BudgetJournal {
    pub(crate) const fn new() -> Self {
        Self {
            records: [ReservationJournalRecord::EMPTY; MAX_BUDGET_JOURNAL_ENTRIES],
            len: 0,
        }
    }

    pub(crate) const fn has_capacity(&self) -> bool {
        (self.len as usize) < MAX_BUDGET_JOURNAL_ENTRIES
    }

    pub(crate) fn committed(
        &self,
        base: &[u64; RESOURCE_CATEGORY_COUNT],
    ) -> [u64; RESOURCE_CATEGORY_COUNT] {
        let mut committed = *base;
        for record in &self.records[..self.len as usize] {
            committed[record.category.index()] =
                committed[record.category.index()].saturating_add(record.consumed);
        }
        committed
    }

    pub(crate) fn clear(&mut self) {
        self.records[..self.len as usize].fill(ReservationJournalRecord::EMPTY);
        self.len = 0;
    }

    pub(crate) fn push(
        &mut self,
        id: ReservationId,
        owner: BudgetPath,
        category: ResourceCategory,
        cause: BudgetCause,
        amount: u64,
    ) -> usize {
        let index = self.len as usize;
        self.records[index] = ReservationJournalRecord {
            id,
            owner,
            category,
            cause,
            amount,
            consumed: 0,
            returned: 0,
            conservative_drop: false,
        };
        self.len += 1;
        index
    }

    pub(crate) fn consume(&mut self, index: usize, amount: u64) {
        self.records[index].consumed += amount;
    }

    pub(crate) fn return_units(&mut self, index: usize, amount: u64) {
        self.records[index].returned += amount;
    }

    pub(crate) fn mark_conservative_drop(&mut self, index: usize) {
        self.records[index].conservative_drop = true;
    }

    pub(crate) fn commit_owner_remainders(&mut self, owner: BudgetPath) {
        for record in &mut self.records[..self.len as usize] {
            if record.owner == owner {
                let settled = record.consumed.saturating_add(record.returned);
                if settled < record.amount {
                    record.consumed += record.amount - settled;
                    record.conservative_drop = true;
                }
            }
        }
    }

    pub(crate) fn prefix(
        &self,
        profile: ResourceProfileIdentity,
        base: &[u64; RESOURCE_CATEGORY_COUNT],
        rejected: Option<BudgetRejectedEvent>,
    ) -> BudgetPrefix {
        let committed = self.committed(base);
        BudgetPrefix {
            profile,
            committed,
            records: self.records,
            record_count: self.len,
            rejected,
        }
    }
}
