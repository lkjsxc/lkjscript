use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImageContracts {
    language: lkjscript_contracts::ContractDigest,
    verified_ssa: lkjscript_contracts::ContractDigest,
    runtime_calls: lkjscript_contracts::ContractDigest,
    native_layout: lkjscript_contracts::ContractDigest,
}

impl ImageContracts {
    #[must_use]
    pub const fn new(
        language: lkjscript_contracts::ContractDigest,
        verified_ssa: lkjscript_contracts::ContractDigest,
        runtime_calls: lkjscript_contracts::ContractDigest,
        native_layout: lkjscript_contracts::ContractDigest,
    ) -> Self {
        Self {
            language,
            verified_ssa,
            runtime_calls,
            native_layout,
        }
    }

    #[must_use]
    pub const fn current() -> Self {
        Self::new(
            lkjscript_contracts::LANGUAGE_DIGEST,
            lkjscript_contracts::VERIFIED_SSA_DIGEST,
            lkjscript_contracts::RUNTIME_CALLS_DIGEST,
            lkjscript_contracts::NATIVE_LAYOUT_DIGEST,
        )
    }

    #[must_use]
    pub const fn language(self) -> lkjscript_contracts::ContractDigest {
        self.language
    }

    #[must_use]
    pub const fn verified_ssa(self) -> lkjscript_contracts::ContractDigest {
        self.verified_ssa
    }

    #[must_use]
    pub const fn runtime_calls(self) -> lkjscript_contracts::ContractDigest {
        self.runtime_calls
    }

    #[must_use]
    pub const fn native_layout(self) -> lkjscript_contracts::ContractDigest {
        self.native_layout
    }
}

impl Default for ImageContracts {
    fn default() -> Self {
        Self::current()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeExecutionDomain {
    CollectorFree,
    LegacyHeap,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeResource {
    resource_kind: lkjscript_contracts::ResourceKind,
    opaque_word: u64,
    worker_owner: PhantomData<Rc<()>>,
}

impl NativeResource {
    #[must_use]
    pub const fn new(resource_kind: lkjscript_contracts::ResourceKind, opaque_word: u64) -> Self {
        Self {
            resource_kind,
            opaque_word,
            worker_owner: PhantomData,
        }
    }

    #[must_use]
    pub const fn resource_kind(self) -> lkjscript_contracts::ResourceKind {
        self.resource_kind
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
    StaticBytes(NativeStaticBytes),
    Capability(lkjscript_contracts::CapabilityKind),
    Resource(NativeResource),
    Unique(NativeUnique),
    Loan(NativeLoan),
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
            Self::StaticBytes(_) => ValueType::StaticBytes,
            Self::Capability(kind) => ValueType::Capability(kind),
            Self::Resource(resource) => ValueType::Resource(resource.resource_kind()),
            Self::Unique(unique) => ValueType::Unique(unique.unique_type()),
            Self::Loan(loan) => ValueType::Loan(loan.loan_type()),
            Self::Reference(reference) => ValueType::Reference(reference.reference_type()),
        }
    }
}
