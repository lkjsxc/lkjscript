use super::*;

macro_rules! structural_word {
    ($name:ident, $identity:ty, $field:ident) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub struct $name {
            $field: $identity,
            opaque_word: u64,
            worker_owner: PhantomData<Rc<()>>,
        }

        impl $name {
            #[must_use]
            pub const fn new($field: $identity, opaque_word: u64) -> Self {
                Self {
                    $field,
                    opaque_word,
                    worker_owner: PhantomData,
                }
            }

            #[must_use]
            pub const fn $field(self) -> $identity {
                self.$field
            }

            #[must_use]
            pub const fn opaque_word(self) -> u64 {
                self.opaque_word
            }
        }
    };
}

structural_word!(NativeStaticString, StructuralTypeIdentity, structural_type);
structural_word!(
    NativeStructuralOwner,
    StructuralTypeIdentity,
    structural_type
);
structural_word!(NativeStructuralView, StructuralViewType, view_type);
structural_word!(
    NativeStructuralDestination,
    StructuralDestinationType,
    destination_type
);
