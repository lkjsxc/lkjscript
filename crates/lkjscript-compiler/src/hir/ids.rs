macro_rules! dense_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);

        impl $name {
            pub(crate) const fn new(raw: u64) -> Self {
                Self(raw)
            }

            #[allow(dead_code)]
            pub const fn raw(self) -> u64 {
                self.0
            }

            #[allow(dead_code)]
            pub(crate) fn index(self) -> Option<usize> {
                usize::try_from(self.0).ok()
            }
        }
    };
}

dense_id!(TraitId);
dense_id!(ImplId);
dense_id!(PlaceId);
dense_id!(LoanId);
dense_id!(LoopId);
dense_id!(SourceId);
dense_id!(BindingId);

#[cfg(test)]
mod tests {
    use super::{BindingId, ImplId, LoanId, PlaceId, SourceId, TraitId};

    #[test]
    fn dense_identity_domains_preserve_values_above_u32() {
        let high = u64::from(u32::MAX) + 1;
        assert_eq!(SourceId::new(high).raw(), high);
        assert_eq!(BindingId::new(high + 1).raw(), high + 1);
        assert_eq!(PlaceId::new(high + 2).raw(), high + 2);
        assert_eq!(LoanId::new(high + 3).raw(), high + 3);
        assert_eq!(TraitId::new(high + 4).raw(), high + 4);
        assert_eq!(ImplId::new(high + 5).raw(), high + 5);
        assert_ne!(BindingId::new(high).raw(), BindingId::new(0).raw());
    }
}
