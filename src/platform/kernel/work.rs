//! Passive validation work observation with independent exact optional admission.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ValidationWork {
    pub consumed: usize,
    /// When true, `consumed` is a lower bound rather than an exact count.
    pub saturated: bool,
}

impl ValidationWork {
    pub(crate) fn exact(consumed: usize) -> Self {
        Self {
            consumed,
            saturated: false,
        }
    }

    /// Charge a modeled product without making overflow a hidden unbounded quota.
    pub(crate) fn charge_product(
        &mut self,
        count: usize,
        width: usize,
        maximum: Option<usize>,
    ) -> bool {
        self.charge_factors(&[count, width], maximum)
    }

    pub(crate) fn charge_factors(&mut self, factors: &[usize], maximum: Option<usize>) -> bool {
        let amount = if factors.contains(&0) {
            Some(0)
        } else {
            factors
                .iter()
                .try_fold(1_usize, |product, factor| product.checked_mul(*factor))
        };
        match amount {
            Some(amount) => self.charge(amount, maximum),
            None if maximum.is_some() => false,
            None => {
                self.consumed = usize::MAX;
                self.saturated = true;
                true
            }
        }
    }

    /// Admit before work occurs. Failed finite admission does not consume work.
    /// `None`, `Some(0)` and `Some(usize::MAX)` remain distinct policies.
    pub(crate) fn charge(&mut self, count: usize, maximum: Option<usize>) -> bool {
        match maximum {
            Some(maximum) => {
                if self.saturated {
                    return false;
                }
                let Some(next) = self.consumed.checked_add(count) else {
                    return false;
                };
                if next > maximum {
                    return false;
                }
                self.consumed = next;
            }
            None => match self.consumed.checked_add(count) {
                Some(next) => self.consumed = next,
                None => {
                    self.consumed = usize::MAX;
                    self.saturated = true;
                }
            },
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::ValidationWork;

    #[test]
    fn factor_products_preserve_zero_exact_and_maximum_policies() {
        for maximum in [None, Some(0), Some(5), Some(6), Some(usize::MAX)] {
            let mut work = ValidationWork::default();
            assert!(work.charge_factors(&[usize::MAX, usize::MAX, 0], maximum));
            assert_eq!(work, ValidationWork::default());
            let admitted = maximum.is_none_or(|limit| limit >= 6);
            assert_eq!(work.charge_factors(&[2, 3], maximum), admitted);
            assert_eq!(work, ValidationWork::exact(if admitted { 6 } else { 0 }));
        }
    }

    #[test]
    fn factor_overflow_is_passive_only_without_finite_admission() {
        for maximum in [0, usize::MAX] {
            let mut work = ValidationWork::default();
            assert!(!work.charge_product(usize::MAX, 2, Some(maximum)));
            assert_eq!(work, ValidationWork::default());
        }
        let mut work = ValidationWork::default();
        assert!(work.charge_product(usize::MAX, 2, None));
        assert_eq!(work.consumed, usize::MAX);
        assert!(work.saturated);
        assert!(work.charge_factors(&[usize::MAX, usize::MAX, 0], None));
        assert!(work.saturated);
        assert!(!work.charge_factors(&[0], Some(usize::MAX)));
    }

    #[test]
    fn zero_and_exact_finite_admission_precede_consumption() {
        let mut work = ValidationWork::default();
        assert!(work.charge(0, Some(0)));
        assert!(!work.charge(1, Some(0)));
        assert_eq!(work, ValidationWork::default());
        assert!(work.charge(2, Some(2)));
        assert!(!work.charge(1, Some(2)));
        assert_eq!(work, ValidationWork::exact(2));
    }

    #[test]
    fn maximum_finite_policy_is_not_unlimited() {
        let mut work = ValidationWork::exact(usize::MAX - 1);
        assert!(work.charge(1, Some(usize::MAX)));
        assert!(!work.charge(1, Some(usize::MAX)));
        assert_eq!(work, ValidationWork::exact(usize::MAX));
    }

    #[test]
    fn unbounded_work_exceeds_the_old_ceiling_and_reports_saturation() {
        let mut work = ValidationWork::exact(10_000_000);
        assert!(work.charge(1, None));
        assert_eq!(work, ValidationWork::exact(10_000_001));
        work = ValidationWork::exact(usize::MAX);
        assert!(work.charge(1, None));
        assert_eq!(work.consumed, usize::MAX);
        assert!(work.saturated);
        assert!(work.charge(1, None));
        assert!(!work.charge(0, Some(usize::MAX)));
    }
}
