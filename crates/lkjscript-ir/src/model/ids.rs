macro_rules! dense_id {
    ($name:ident, $raw:ty) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name($raw);

        impl $name {
            pub const fn new(raw: $raw) -> Self {
                Self(raw)
            }

            pub const fn raw(self) -> $raw {
                self.0
            }

            pub fn index(self) -> Option<usize> {
                usize::try_from(self.0).ok()
            }
        }
    };
}

dense_id!(FunctionId, u64);
dense_id!(BlockId, u64);
dense_id!(ValueId, u64);
dense_id!(ProductId, u64);
dense_id!(BindingId, u64);
dense_id!(TraitId, u64);
dense_id!(ImplId, u64);
dense_id!(PlaceId, u64);
dense_id!(LoanId, u64);
dense_id!(FailureCleanupId, u64);
dense_id!(StructuralTypeId, u64);
dense_id!(StructuralLayoutId, u64);
dense_id!(StructuralRepresentationId, u64);

macro_rules! stable_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            pub const fn new(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            pub const fn bytes(self) -> [u8; 32] {
                self.0
            }

            pub fn is_resolved(self) -> bool {
                self.0 != [0; 32]
            }
        }
    };
}

stable_id!(EnumId);
stable_id!(VariantId);
stable_id!(VariantFieldId);
stable_id!(RuntimeLayoutId);
stable_id!(MemoryPlanId);
stable_id!(MemoryWitnessGroupId);
stable_id!(MemoryWitnessId);

#[cfg(test)]
mod tests {
    use super::{BlockId, FunctionId, LoanId, PlaceId, ValueId};

    #[test]
    fn dense_identity_domains_preserve_values_above_u32() {
        let high = u64::from(u32::MAX) + 1;
        assert_eq!(FunctionId::new(high).raw(), high);
        assert_eq!(BlockId::new(high + 1).raw(), high + 1);
        assert_eq!(ValueId::new(high + 2).raw(), high + 2);
        assert_eq!(PlaceId::new(high + 3).raw(), high + 3);
        assert_eq!(LoanId::new(high + 4).raw(), high + 4);
        assert_ne!(ValueId::new(high), ValueId::new(0));
    }
}
