//! Exact optional allowances, separated from passive cumulative observations.
//!
//! These scalar helpers contain no semantic proof, cancellation, allocation, or
//! execution policy. Callers choose their metric and check physical extents.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Observation {
    pub consumed: u64,
    /// The count is a lower bound, unsuitable for sizing or exact proof bounds.
    pub saturated: bool,
}

impl Observation {
    pub(crate) const fn exact(consumed: u64) -> Self {
        Self {
            consumed,
            saturated: false,
        }
    }

    pub(crate) fn from_usize(consumed: usize) -> Self {
        match u64::try_from(consumed) {
            Ok(consumed) => Self::exact(consumed),
            Err(_) => Self {
                consumed: u64::MAX,
                saturated: true,
            },
        }
    }

    pub(crate) fn product(left: usize, right: usize) -> Self {
        if left == 0 || right == 0 {
            return Self::exact(0);
        }
        let left = Self::from_usize(left);
        let right = Self::from_usize(right);
        match left.consumed.checked_mul(right.consumed) {
            Some(consumed) if !left.saturated && !right.saturated => Self::exact(consumed),
            _ => Self {
                consumed: u64::MAX,
                saturated: true,
            },
        }
    }

    pub(crate) fn sum_counts(counts: &[usize]) -> Self {
        let mut total = Self::default();
        for count in counts {
            total.observe(Self::from_usize(*count));
        }
        total
    }

    pub(crate) fn observe(&mut self, other: Self) {
        self.saturated |= other.saturated;
        match self.consumed.checked_add(other.consumed) {
            Some(consumed) => self.consumed = consumed,
            None => {
                self.consumed = u64::MAX;
                self.saturated = true;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AllowanceExceeded;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Meter {
    remaining: Option<u64>,
    observation: Observation,
}

impl Meter {
    pub(crate) const fn new(remaining: Option<u64>) -> Self {
        Self {
            remaining,
            observation: Observation::exact(0),
        }
    }

    pub(crate) const fn remaining(self) -> Option<u64> {
        self.remaining
    }

    pub(crate) const fn observation(self) -> Observation {
        self.observation
    }

    /// Debit permission before the caller performs work. Refusal changes neither
    /// remaining allowance nor observation. Saturation never grants permission;
    /// an unbounded caller still proceeds when its observation saturates.
    pub(crate) fn charge(&mut self, amount: Observation) -> Result<(), AllowanceExceeded> {
        if let Some(remaining) = self.remaining {
            if amount.saturated {
                return Err(AllowanceExceeded);
            }
            let next = remaining
                .checked_sub(amount.consumed)
                .ok_or(AllowanceExceeded)?;
            self.remaining = Some(next);
        }
        self.observation.observe(amount);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_meter_none_zero_one_short_exact_and_max_are_distinct() {
        let mut unbounded = Meter::new(None);
        assert!(unbounded.charge(Observation::exact(u64::MAX)).is_ok());
        assert_eq!(unbounded.observation(), Observation::exact(u64::MAX));
        assert!(unbounded.charge(Observation::exact(1)).is_ok());
        assert_eq!(unbounded.remaining(), None);
        assert!(unbounded.observation().saturated);

        for (limit, succeeds) in [(0, false), (6, false), (7, true), (u64::MAX, true)] {
            let mut meter = Meter::new(Some(limit));
            let before = meter;
            assert_eq!(meter.charge(Observation::exact(7)).is_ok(), succeeds);
            if succeeds {
                assert_eq!(meter.remaining(), Some(limit - 7));
                assert_eq!(meter.observation(), Observation::exact(7));
            } else {
                assert_eq!(meter, before);
            }
        }
        let mut maximum = Meter::new(Some(u64::MAX));
        assert!(maximum.charge(Observation::exact(u64::MAX)).is_ok());
        assert_eq!(maximum.remaining(), Some(0));
        assert_eq!(maximum.observation(), Observation::exact(u64::MAX));
        assert!(maximum.charge(Observation::exact(0)).is_ok());
        assert!(maximum.charge(Observation::exact(1)).is_err());
        assert_eq!(maximum.observation(), Observation::exact(u64::MAX));
    }

    #[test]
    fn admission_observation_overflow_is_passive_and_never_an_exact_amount() {
        let mut amount = Observation::exact(u64::MAX);
        amount.observe(Observation::exact(1));
        assert!(amount.saturated);
        for limit in [0, 1, u64::MAX] {
            let mut meter = Meter::new(Some(limit));
            let before = meter;
            assert!(meter.charge(amount).is_err());
            assert_eq!(meter, before);
        }
        let mut meter = Meter::new(None);
        assert!(meter.charge(amount).is_ok());
        assert!(meter.charge(Observation::exact(1)).is_ok());
        assert!(meter.observation().saturated);
        assert_eq!(meter.remaining(), None);
    }

    #[test]
    fn admission_observation_sums_and_products_preserve_exact_zero_and_bounds() {
        assert_eq!(Observation::product(0, usize::MAX), Observation::exact(0));
        assert_eq!(Observation::product(3, 7), Observation::exact(21));
        assert_eq!(Observation::sum_counts(&[3, 7, 11]), Observation::exact(21));
        let product = Observation::product(usize::MAX, usize::MAX);
        if usize::BITS >= 64 {
            assert!(product.saturated);
        } else {
            assert!(!product.saturated);
            assert_eq!(
                Some(product.consumed),
                (usize::MAX as u64).checked_mul(usize::MAX as u64)
            );
        }
        let mut observed = Observation {
            consumed: 4,
            saturated: true,
        };
        observed.observe(Observation::exact(3));
        assert_eq!(observed.consumed, 7);
        assert!(observed.saturated);
    }
}
