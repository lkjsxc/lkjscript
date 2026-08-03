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

dense_id!(FunctionId, u32);
dense_id!(BlockId, u32);
dense_id!(ValueId, u32);
dense_id!(ProductId, u16);
dense_id!(BindingId, u32);
dense_id!(TraitId, u32);
dense_id!(ImplId, u32);
dense_id!(PlaceId, u32);
dense_id!(LoanId, u32);
dense_id!(FailureCleanupId, u32);
dense_id!(StructuralTypeId, u16);
dense_id!(StructuralLayoutId, u16);
dense_id!(StructuralRepresentationId, u16);

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
