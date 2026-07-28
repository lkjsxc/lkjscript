use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeUnique {
    unique_type: UniqueType,
    opaque_word: u64,
    worker_owner: PhantomData<Rc<()>>,
}

impl NativeUnique {
    #[must_use]
    pub const fn new(unique_type: UniqueType, opaque_word: u64) -> Self {
        Self {
            unique_type,
            opaque_word,
            worker_owner: PhantomData,
        }
    }

    #[must_use]
    pub const fn byte_vector(opaque_word: u64) -> Self {
        Self::new(UniqueType::ByteVector, opaque_word)
    }

    #[must_use]
    pub const fn bytes(opaque_word: u64) -> Self {
        Self::new(UniqueType::Bytes, opaque_word)
    }

    #[must_use]
    pub const fn unique_type(self) -> UniqueType {
        self.unique_type
    }

    #[must_use]
    pub const fn opaque_word(self) -> u64 {
        self.opaque_word
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeLoan {
    loan_type: LoanType,
    opaque_word: u64,
    worker_owner: PhantomData<Rc<()>>,
}

impl NativeLoan {
    #[must_use]
    pub const fn new(loan_type: LoanType, opaque_word: u64) -> Self {
        Self {
            loan_type,
            opaque_word,
            worker_owner: PhantomData,
        }
    }

    #[must_use]
    pub const fn byte_slice(opaque_word: u64) -> Self {
        Self::new(LoanType::ByteSlice, opaque_word)
    }

    #[must_use]
    pub const fn byte_slice_mut(opaque_word: u64) -> Self {
        Self::new(LoanType::ByteSliceMut, opaque_word)
    }

    #[must_use]
    pub const fn bytes(opaque_word: u64) -> Self {
        Self::new(LoanType::Bytes, opaque_word)
    }

    #[must_use]
    pub const fn loan_type(self) -> LoanType {
        self.loan_type
    }

    #[must_use]
    pub const fn opaque_word(self) -> u64 {
        self.opaque_word
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeStaticBytes {
    opaque_word: u64,
    worker_owner: PhantomData<Rc<()>>,
}

impl NativeStaticBytes {
    #[must_use]
    pub const fn new(opaque_word: u64) -> Self {
        Self {
            opaque_word,
            worker_owner: PhantomData,
        }
    }

    #[must_use]
    pub const fn opaque_word(self) -> u64 {
        self.opaque_word
    }
}
