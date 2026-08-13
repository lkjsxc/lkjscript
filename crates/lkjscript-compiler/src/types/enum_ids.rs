macro_rules! stable_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; 32]);

        impl $name {
            pub const fn new(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            #[allow(dead_code)]
            pub const fn bytes(self) -> [u8; 32] {
                self.0
            }

            #[allow(dead_code)]
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
