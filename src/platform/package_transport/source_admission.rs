//! Exact optional source permission and independent passive observations.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SourceObservation {
    pub consumed: u64,
    /// When true, consumed is a lower bound, not an exact count. A transformed
    /// observation may be below u64::MAX while retaining this flag.
    pub saturated: bool,
}

impl SourceObservation {
    pub(crate) const fn exact(consumed: u64) -> Self {
        Self {
            consumed,
            saturated: false,
        }
    }

    pub(crate) fn product(factors: &[u64]) -> Self {
        if factors.contains(&0) {
            return Self::exact(0);
        }
        match factors
            .iter()
            .try_fold(1_u64, |value, factor| value.checked_mul(*factor))
        {
            Some(consumed) => Self::exact(consumed),
            None => Self {
                consumed: u64::MAX,
                saturated: true,
            },
        }
    }

    pub(crate) fn observe(&mut self, additional: Self) {
        self.saturated |= additional.saturated;
        match self.consumed.checked_add(additional.consumed) {
            Some(consumed) => self.consumed = consumed,
            None => {
                self.consumed = u64::MAX;
                self.saturated = true;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SourceAllowance {
    remaining: Option<u64>,
}

impl SourceAllowance {
    pub(crate) const fn new(remaining: Option<u64>) -> Self {
        Self { remaining }
    }

    pub(crate) const fn remaining(self) -> Option<u64> {
        self.remaining
    }

    pub(crate) fn reserve(&mut self, amount: u64) -> bool {
        if let Some(remaining) = self.remaining {
            let Some(next) = remaining.checked_sub(amount) else {
                return false;
            };
            self.remaining = Some(next);
        }
        true
    }

    /// Commit work already admitted by a nested phase under this exact allowance.
    /// An inexact phase observation cannot discharge a finite permission ledger.
    pub(crate) fn reserve_observed(&mut self, amount: SourceObservation) -> bool {
        if amount.saturated && self.remaining.is_some() {
            return false;
        }
        self.reserve(amount.consumed)
    }

    /// Debit a jointly admitted read using its checked permission difference.
    /// The read's passive counters are deliberately not inputs to this operation.
    pub(crate) fn charged_from(
        mut self,
        joint_initial: Option<u64>,
        joint_remaining: Option<u64>,
    ) -> Option<Self> {
        if self.remaining.is_none() {
            return Some(self);
        }
        let consumed = joint_initial?.checked_sub(joint_remaining?)?;
        self.reserve(consumed).then_some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{SourceAllowance, SourceObservation};

    #[test]
    fn optional_source_allowance_keeps_zero_exact_and_maximum_finite() {
        for maximum in [None, Some(0), Some(2), Some(3), Some(u64::MAX)] {
            let mut allowance = SourceAllowance::new(maximum);
            assert!(allowance.reserve(0));
            let admitted = maximum.is_none_or(|limit| limit >= 3);
            assert_eq!(allowance.reserve(3), admitted);
            assert_eq!(
                allowance.remaining(),
                maximum.map(|limit| if admitted { limit - 3 } else { limit })
            );
        }
        let mut finite = SourceAllowance::new(Some(u64::MAX));
        assert!(finite.reserve(u64::MAX));
        assert_eq!(finite.remaining(), Some(0));
        assert!(!finite.reserve(1));
        assert_eq!(finite.remaining(), Some(0));
    }

    #[test]
    fn passive_saturation_does_not_end_an_unbounded_source_operation() {
        let mut allowance = SourceAllowance::default();
        let mut observed = SourceObservation::exact(u64::MAX);
        observed.observe(SourceObservation::exact(1));
        assert!(observed.saturated);
        assert!(allowance.reserve_observed(observed));
        assert!(allowance.reserve(1));
        assert_eq!(allowance.remaining(), None);
        let mut finite = SourceAllowance::new(Some(u64::MAX));
        assert!(!finite.reserve_observed(observed));
        assert_eq!(finite.remaining(), Some(u64::MAX));
    }

    #[test]
    fn source_products_and_nested_read_debits_preserve_exact_permission() {
        assert_eq!(
            SourceObservation::product(&[u64::MAX, u64::MAX, 0]),
            SourceObservation::exact(0)
        );
        assert_eq!(
            SourceObservation::product(&[2, 3]),
            SourceObservation::exact(6)
        );
        assert!(SourceObservation::product(&[u64::MAX, 2]).saturated);
        assert_eq!(
            SourceAllowance::new(Some(10)).charged_from(Some(7), Some(3)),
            Some(SourceAllowance::new(Some(6)))
        );
        assert_eq!(
            SourceAllowance::new(Some(u64::MAX)).charged_from(Some(u64::MAX), Some(0)),
            Some(SourceAllowance::new(Some(0)))
        );
        assert_eq!(
            SourceAllowance::new(Some(1)).charged_from(Some(2), Some(0)),
            None
        );
        assert_eq!(
            SourceAllowance::new(Some(1)).charged_from(Some(0), Some(1)),
            None
        );
        assert_eq!(SourceAllowance::new(Some(1)).charged_from(None, None), None);
        assert_eq!(
            SourceAllowance::default().charged_from(None, None),
            Some(SourceAllowance::default())
        );
    }
}
