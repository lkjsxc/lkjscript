use super::*;

pub const CURRENT_SEMANTIC_ABI_VERSION: u16 = 1;
pub const CURRENT_NATIVE_ABI_VERSION: u16 = 2;
pub const CURRENT_RUNTIME_ABI_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbiVersions {
    pub(super) semantic: u16,
    pub(super) native: u16,
    pub(super) runtime: u16,
}

impl AbiVersions {
    #[must_use]
    pub const fn new(semantic: u16, native: u16, runtime: u16) -> Self {
        Self {
            semantic,
            native,
            runtime,
        }
    }

    #[must_use]
    pub const fn current() -> Self {
        Self::new(
            CURRENT_SEMANTIC_ABI_VERSION,
            CURRENT_NATIVE_ABI_VERSION,
            CURRENT_RUNTIME_ABI_VERSION,
        )
    }

    #[must_use]
    pub const fn semantic(self) -> u16 {
        self.semantic
    }

    #[must_use]
    pub const fn native(self) -> u16 {
        self.native
    }

    #[must_use]
    pub const fn runtime(self) -> u16 {
        self.runtime
    }
}

impl Default for AbiVersions {
    fn default() -> Self {
        Self::current()
    }
}

/// Copyable, worker-local runtime-adapter token. The opaque word is never
/// interpreted as an object address by the native ABI. The ownership marker
/// intentionally makes this token non-Send and non-Sync; it is not a source
/// reference or an independently owned heap value.
///
/// ```compile_fail
/// use lkjscript_native::NativeReference;
/// let reference = NativeReference::buf(7);
/// std::thread::spawn(move || reference.opaque_word());
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeReference {
    pub(super) reference_type: ReferenceType,
    pub(super) opaque_word: u64,
    pub(super) worker_owner: PhantomData<Rc<()>>,
}

impl NativeReference {
    #[must_use]
    pub const fn new(reference_type: ReferenceType, opaque_word: u64) -> Self {
        Self {
            reference_type,
            opaque_word,
            worker_owner: PhantomData,
        }
    }

    #[must_use]
    pub const fn buf(opaque_word: u64) -> Self {
        Self::new(ReferenceType::Buf, opaque_word)
    }

    #[must_use]
    pub const fn reference_type(self) -> ReferenceType {
        self.reference_type
    }

    #[must_use]
    pub const fn opaque_word(self) -> u64 {
        self.opaque_word
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NativeValue {
    I64(i64),
    F64Bits(u64),
    Bool(bool),
    Unit,
    Reference(NativeReference),
}

impl NativeValue {
    #[must_use]
    pub fn f64(value: f64) -> Self {
        Self::F64Bits(value.to_bits())
    }

    #[must_use]
    pub const fn value_type(self) -> ValueType {
        match self {
            Self::I64(_) => ValueType::I64,
            Self::F64Bits(_) => ValueType::F64,
            Self::Bool(_) => ValueType::Bool,
            Self::Unit => ValueType::Unit,
            Self::Reference(reference) => ValueType::Reference(reference.reference_type()),
        }
    }
}
