use super::*;

mod structural;
mod tokens;

pub use structural::*;
pub use tokens::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeExecutionDomain {
    CollectorFree,
    InvocationRegion,
}

/// Copyable, worker-local runtime-adapter token. The opaque word is never
/// interpreted as an object address by the native ABI. The ownership marker
/// intentionally makes this token non-Send and non-Sync; it is not a source
/// reference or an independently owned heap value.
///
/// ```compile_fail
/// use lkjscript_native::{LayoutIdentity, NativeReference, ReferenceType};
/// let reference = NativeReference::new(
///     ReferenceType::List(LayoutIdentity::new(0), 1, LayoutIdentity::new(2), 3),
///     7,
/// );
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
    StaticString(NativeStaticString),
    Capability(lkjscript_contracts::CapabilityKind),
    Resource(NativeResource),
    Unique(NativeUnique),
    Loan(NativeLoan),
    StructuralKey(u64),
    StructuralOwner(NativeStructuralOwner),
    StructuralView(NativeStructuralView),
    StructuralDestination(NativeStructuralDestination),
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
            Self::StaticString(value) => ValueType::StaticString(value.structural_type()),
            Self::Capability(kind) => ValueType::Capability(kind),
            Self::Resource(resource) => ValueType::Resource(resource.resource_kind()),
            Self::Unique(unique) => ValueType::Unique(unique.unique_type()),
            Self::Loan(loan) => ValueType::Loan(loan.loan_type()),
            Self::StructuralKey(_) => ValueType::StructuralKey,
            Self::StructuralOwner(owner) => ValueType::StructuralOwner(owner.structural_type()),
            Self::StructuralView(view) => ValueType::StructuralView(view.view_type()),
            Self::StructuralDestination(destination) => {
                ValueType::StructuralDestination(destination.destination_type())
            }
            Self::Reference(reference) => ValueType::Reference(reference.reference_type()),
        }
    }
}
