use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::verify::{verify_plan, VerifiedMachinePlan};
use crate::{BackendLimits, NativeError};

static NEXT_PLAN_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LayoutIdentity(u32);

impl LayoutIdentity {
    const PRODUCT_BASE: u32 = 32;

    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn product(product: u32) -> Self {
        Self(Self::PRODUCT_BASE + product)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Exact runtime layout identity for a worker-local stable reference handle.
/// Layout-bearing variants leave the machine ABI unchanged while preventing
/// unrelated heap layouts from being interchanged.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReferenceType {
    Buf,
    Str,
    /// Complete interned list identity followed by its element identity.
    List(LayoutIdentity, LayoutIdentity),
    /// Complete interned option identity followed by its payload identity.
    Option(LayoutIdentity, LayoutIdentity),
    /// Complete interned result identity followed by its Ok and Err identities.
    Result(LayoutIdentity, LayoutIdentity, LayoutIdentity),
    Product(LayoutIdentity),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ValueType {
    I64,
    F64,
    Bool,
    Unit,
    Reference(ReferenceType),
}

impl ValueType {
    #[must_use]
    pub const fn reference_type(self) -> Option<ReferenceType> {
        match self {
            Self::Reference(reference_type) => Some(reference_type),
            Self::I64 | Self::F64 | Self::Bool | Self::Unit => None,
        }
    }

    /// Exact structural identity used by List/Option/Result payload facts.
    /// Nested reference variants retain their pre-interned complete identity.
    #[must_use]
    pub const fn layout_identity(self) -> LayoutIdentity {
        match self {
            Self::Unit => LayoutIdentity::new(1),
            Self::Bool => LayoutIdentity::new(2),
            Self::I64 => LayoutIdentity::new(3),
            Self::F64 => LayoutIdentity::new(4),
            Self::Reference(ReferenceType::Str) => LayoutIdentity::new(5),
            Self::Reference(ReferenceType::Buf) => LayoutIdentity::new(7),
            Self::Reference(ReferenceType::Product(layout))
            | Self::Reference(ReferenceType::List(layout, _))
            | Self::Reference(ReferenceType::Option(layout, _))
            | Self::Reference(ReferenceType::Result(layout, _, _)) => layout,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Signature {
    parameters: Vec<ValueType>,
    result: ValueType,
}

impl Signature {
    pub fn new(parameters: Vec<ValueType>, result: ValueType) -> Result<Self, PlanError> {
        if parameters.len() > 16 {
            return Err(PlanError::TooManyParameters {
                count: parameters.len(),
                maximum: 16,
            });
        }
        Ok(Self { parameters, result })
    }

    #[must_use]
    pub fn parameters(&self) -> &[ValueType] {
        &self.parameters
    }

    #[must_use]
    pub const fn result(&self) -> ValueType {
        self.result
    }

    pub(crate) fn machine_parameter_count(&self) -> usize {
        self.parameters
            .iter()
            .filter(|parameter| **parameter != ValueType::Unit)
            .count()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceFunctionId(u32);

impl SourceFunctionId {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SourceOrigin(u32);

impl SourceOrigin {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FunctionId {
    pub(crate) plan: u64,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BlockId {
    pub(crate) function: FunctionId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ValueId {
    pub(crate) function: FunctionId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LocalId {
    pub(crate) function: FunctionId,
    pub(crate) index: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum I64Comparison {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum F64Comparison {
    OrderedEqual,
    OrderedNotEqual,
    OrderedLessThan,
    OrderedLessThanOrEqual,
    OrderedGreaterThan,
    OrderedGreaterThanOrEqual,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BoolComparison {
    Equal,
    NotEqual,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u32)]
pub enum TrapCode {
    I64Overflow = 1,
    DivisionByZero = 2,
    Explicit = 3,
}

impl TrapCode {
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AllocationClass {
    None,
    Bounded,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StoreClass {
    None,
    Initialization,
    Scalar,
    Reference,
    ReferenceClearing,
}

/// Host-independent heap semantics retained as exact versioned runtime-site
/// identity. Literal bytes and nominal identities are bounded image metadata,
/// never source pointers.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum HeapOperation {
    ConstantStr(String),
    EmptyStr,
    EmptyList,
    None,
    ProductValue {
        product: u32,
        fields: u8,
    },
    ProductField {
        product: u32,
        field: u8,
        field_type: ValueType,
    },
    WithProductField {
        product: u32,
        field: u8,
        field_type: ValueType,
    },
    Cons,
    Car,
    Cdr,
    IsEmptyList,
    Some,
    IsSome,
    UnwrapSome,
    Ok,
    Err,
    IsOk,
    UnwrapOk,
    UnwrapErr,
    BufNew,
    BufLen,
    BufRef,
    BufSet,
    BufClone,
    BufFromStr,
    BufToStr,
    BufSlice,
    BufGetU32,
    BufSetU32,
    StrLen,
    StrRef,
    StrAppend,
    StrSlice,
    StrFromByte,
    StrFromI64,
    StrFromF64,
    EqualValue,
    SameObject,
    ListEqual,
}

impl HeapOperation {
    pub(crate) fn expected_arity(&self) -> usize {
        match self {
            Self::EmptyStr | Self::EmptyList | Self::None | Self::ConstantStr(_) => 0,
            Self::ProductValue { fields, .. } => usize::from(*fields),
            Self::BufSet | Self::BufSlice | Self::BufSetU32 | Self::StrSlice => 3,
            Self::ProductField { .. }
            | Self::Car
            | Self::Cdr
            | Self::IsEmptyList
            | Self::Some
            | Self::IsSome
            | Self::UnwrapSome
            | Self::Ok
            | Self::Err
            | Self::IsOk
            | Self::UnwrapOk
            | Self::UnwrapErr
            | Self::BufNew
            | Self::BufLen
            | Self::BufClone
            | Self::BufFromStr
            | Self::BufToStr
            | Self::StrLen
            | Self::StrFromByte
            | Self::StrFromI64
            | Self::StrFromF64 => 1,
            Self::WithProductField { .. }
            | Self::Cons
            | Self::BufRef
            | Self::BufGetU32
            | Self::StrRef
            | Self::StrAppend
            | Self::EqualValue
            | Self::SameObject
            | Self::ListEqual => 2,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct HeapCallDescriptor {
    operation: HeapOperation,
    input_types: Vec<ValueType>,
    result_type: ValueType,
    allocation: AllocationClass,
    store: StoreClass,
}

impl HeapCallDescriptor {
    pub fn new(
        operation: HeapOperation,
        input_types: Vec<ValueType>,
        result_type: ValueType,
        allocation: AllocationClass,
        store: StoreClass,
    ) -> Result<Self, PlanError> {
        if input_types.len() > 16 || input_types.len() != operation.expected_arity() {
            return Err(PlanError::InvalidHeapCall);
        }
        let descriptor = Self {
            operation,
            input_types,
            result_type,
            allocation,
            store,
        };
        if !descriptor.canonical_facts_are_valid() {
            return Err(PlanError::InvalidHeapCall);
        }
        Ok(descriptor)
    }

    #[must_use]
    pub fn operation(&self) -> &HeapOperation {
        &self.operation
    }

    #[must_use]
    pub fn input_types(&self) -> &[ValueType] {
        &self.input_types
    }

    #[must_use]
    pub const fn result_type(&self) -> ValueType {
        self.result_type
    }

    #[must_use]
    pub const fn allocation(&self) -> AllocationClass {
        self.allocation
    }

    #[must_use]
    pub const fn store(&self) -> StoreClass {
        self.store
    }

    pub(crate) fn canonical_facts_are_valid(&self) -> bool {
        let allocates = matches!(
            self.operation,
            HeapOperation::ConstantStr(_)
                | HeapOperation::EmptyStr
                | HeapOperation::ProductValue { .. }
                | HeapOperation::WithProductField { .. }
                | HeapOperation::Cons
                | HeapOperation::Some
                | HeapOperation::Ok
                | HeapOperation::Err
                | HeapOperation::BufNew
                | HeapOperation::BufClone
                | HeapOperation::BufFromStr
                | HeapOperation::BufToStr
                | HeapOperation::BufSlice
                | HeapOperation::StrAppend
                | HeapOperation::StrSlice
                | HeapOperation::StrFromByte
                | HeapOperation::StrFromI64
                | HeapOperation::StrFromF64
        );
        let expected_allocation = if allocates {
            AllocationClass::Bounded
        } else {
            AllocationClass::None
        };
        let expected_store = match self.operation {
            HeapOperation::BufSet | HeapOperation::BufSetU32 => StoreClass::Scalar,
            _ if allocates => StoreClass::Initialization,
            _ => StoreClass::None,
        };
        self.allocation == expected_allocation
            && self.store == expected_store
            && self.operation_types_are_valid()
    }

    fn operation_types_are_valid(&self) -> bool {
        use HeapOperation as Op;
        use ReferenceType as Ref;
        use ValueType as Ty;

        let inputs = self.input_types.as_slice();
        let result = self.result_type;
        match &self.operation {
            Op::ConstantStr(_) | Op::EmptyStr => {
                inputs.is_empty() && result == Ty::Reference(Ref::Str)
            }
            Op::EmptyList => inputs.is_empty() && matches!(result, Ty::Reference(Ref::List(_, _))),
            Op::None => inputs.is_empty() && matches!(result, Ty::Reference(Ref::Option(_, _))),
            Op::ProductValue { product, fields } => {
                usize::from(*fields) == inputs.len()
                    && usize::from(*fields) <= 15
                    && u16::try_from(*product).is_ok()
                    && result == Ty::Reference(Ref::Product(LayoutIdentity::product(*product)))
            }
            Op::ProductField {
                product,
                field,
                field_type,
            } => {
                *field < 15
                    && u16::try_from(*product).is_ok()
                    && result == *field_type
                    && matches!(inputs, [Ty::Reference(Ref::Product(layout))]
                        if *layout == LayoutIdentity::product(*product))
            }
            Op::WithProductField {
                product,
                field,
                field_type,
            } => {
                *field < 15
                    && u16::try_from(*product).is_ok()
                    && matches!(inputs, [Ty::Reference(Ref::Product(layout)), replacement]
                        if *layout == LayoutIdentity::product(*product) && replacement == field_type)
                    && result == Ty::Reference(Ref::Product(LayoutIdentity::product(*product)))
            }
            Op::Cons => matches!(inputs, [payload, list]
                if *list == result
                    && matches!(result, Ty::Reference(Ref::List(_, element))
                        if element == payload.layout_identity())),
            Op::Car => matches!(inputs, [Ty::Reference(Ref::List(_, element))]
                if *element == result.layout_identity()),
            Op::Cdr => {
                matches!(inputs, [list] if *list == result && matches!(result, Ty::Reference(Ref::List(_, _))))
            }
            Op::IsEmptyList => {
                matches!(inputs, [Ty::Reference(Ref::List(_, _))]) && result == Ty::Bool
            }
            Op::Some => matches!(inputs, [payload]
                if matches!(result, Ty::Reference(Ref::Option(_, value))
                    if value == payload.layout_identity())),
            Op::IsSome => {
                matches!(inputs, [Ty::Reference(Ref::Option(_, _))]) && result == Ty::Bool
            }
            Op::UnwrapSome => matches!(inputs, [Ty::Reference(Ref::Option(_, payload))]
                if *payload == result.layout_identity()),
            Op::Ok => matches!(inputs, [payload]
                if matches!(result, Ty::Reference(Ref::Result(_, ok, _)) if ok == payload.layout_identity())),
            Op::Err => matches!(inputs, [payload]
                if matches!(result, Ty::Reference(Ref::Result(_, _, error)) if error == payload.layout_identity())),
            Op::IsOk => {
                matches!(inputs, [Ty::Reference(Ref::Result(_, _, _))]) && result == Ty::Bool
            }
            Op::UnwrapOk => matches!(inputs, [Ty::Reference(Ref::Result(_, ok, _))]
                if *ok == result.layout_identity()),
            Op::UnwrapErr => matches!(inputs, [Ty::Reference(Ref::Result(_, _, error))]
                if *error == result.layout_identity()),
            Op::BufNew => inputs == [Ty::I64] && result == Ty::Reference(Ref::Buf),
            Op::BufLen => inputs == [Ty::Reference(Ref::Buf)] && result == Ty::I64,
            Op::BufRef | Op::BufGetU32 => {
                inputs == [Ty::Reference(Ref::Buf), Ty::I64] && result == Ty::I64
            }
            Op::BufSet | Op::BufSetU32 => {
                inputs == [Ty::Reference(Ref::Buf), Ty::I64, Ty::I64] && result == Ty::Unit
            }
            Op::BufClone => {
                inputs == [Ty::Reference(Ref::Buf)] && result == Ty::Reference(Ref::Buf)
            }
            Op::BufFromStr => {
                inputs == [Ty::Reference(Ref::Str)] && result == Ty::Reference(Ref::Buf)
            }
            Op::BufToStr => {
                inputs == [Ty::Reference(Ref::Buf)]
                    && result
                        == Ty::Reference(Ref::Result(
                            result.layout_identity(),
                            Ty::Reference(Ref::Str).layout_identity(),
                            Ty::Reference(Ref::Str).layout_identity(),
                        ))
            }
            Op::BufSlice => {
                inputs == [Ty::Reference(Ref::Buf), Ty::I64, Ty::I64]
                    && result
                        == Ty::Reference(Ref::Result(
                            result.layout_identity(),
                            Ty::Reference(Ref::Buf).layout_identity(),
                            Ty::Reference(Ref::Str).layout_identity(),
                        ))
            }
            Op::StrLen => inputs == [Ty::Reference(Ref::Str)] && result == Ty::I64,
            Op::StrRef => inputs == [Ty::Reference(Ref::Str), Ty::I64] && result == Ty::I64,
            Op::StrAppend => {
                inputs == [Ty::Reference(Ref::Str), Ty::Reference(Ref::Str)]
                    && result == Ty::Reference(Ref::Str)
            }
            Op::StrSlice => {
                inputs == [Ty::Reference(Ref::Str), Ty::I64, Ty::I64]
                    && result == Ty::Reference(Ref::Str)
            }
            Op::StrFromByte | Op::StrFromI64 => {
                inputs == [Ty::I64] && result == Ty::Reference(Ref::Str)
            }
            Op::StrFromF64 => inputs == [Ty::F64] && result == Ty::Reference(Ref::Str),
            Op::EqualValue => {
                matches!(
                    inputs,
                    [left @ Ty::Reference(Ref::Str | Ref::Option(_, _) | Ref::Result(_, _, _)), right]
                        if left == right
                ) && result == Ty::Bool
            }
            Op::SameObject => {
                inputs == [Ty::Reference(Ref::Buf), Ty::Reference(Ref::Buf)] && result == Ty::Bool
            }
            Op::ListEqual => {
                matches!(inputs, [Ty::Reference(Ref::List(left, _)), Ty::Reference(Ref::List(right, _))] if left == right)
                    && result == Ty::Bool
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeCallSlot {
    IdentityI64V1,
    /// Cooperative deadline and native fuel poll. The execution context is the
    /// implicit first ABI argument; no language value is boxed for this call.
    PollV1,
    /// Records entry to a source function for exact native-tier accounting.
    EnterFunctionV1,
    /// Collecting reference round trip used by the closed ABI-2 plan slice.
    CollectReferenceV1,
    /// Generic verified-frame-home heap dispatch. Plans create it only through
    /// `FunctionBuilder::heap_call`; ordinary runtime-call construction cannot
    /// forge its site metadata.
    HeapDispatchV1,
    /// Encoder-owned frame-chain operations. Plans cannot name these slots.
    ReserveFrameV1,
    RegisterFrameV1,
    PublishSafepointV1,
    UnregisterFrameV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InternalMachineArgument {
    InvocationContext,
    FunctionOrdinal,
    FrameBytes,
    FramePointer,
    SafepointId,
    HeapSiteId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InternalMachineResult {
    Unit,
    InvocationContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InternalRuntimeSignature {
    parameters: &'static [InternalMachineArgument],
    result: InternalMachineResult,
}

impl InternalRuntimeSignature {
    #[must_use]
    pub const fn parameters(self) -> &'static [InternalMachineArgument] {
        self.parameters
    }

    #[must_use]
    pub const fn result(self) -> InternalMachineResult {
        self.result
    }
}

impl RuntimeCallSlot {
    /// Returns the typed machine-plan signature. Encoder-owned slots have no
    /// plan signature; use `internal_abi_signature` for their private ABI.
    #[must_use]
    pub fn plan_signature(self) -> Option<Signature> {
        match self {
            Self::IdentityI64V1 => Some(Signature {
                parameters: vec![ValueType::I64],
                result: ValueType::I64,
            }),
            Self::PollV1 => Some(Signature {
                parameters: Vec::new(),
                result: ValueType::Unit,
            }),
            Self::EnterFunctionV1 => Some(Signature {
                parameters: vec![ValueType::I64],
                result: ValueType::Unit,
            }),
            Self::CollectReferenceV1 => Some(Signature {
                parameters: vec![ValueType::Reference(ReferenceType::Buf)],
                result: ValueType::Reference(ReferenceType::Buf),
            }),
            Self::HeapDispatchV1
            | Self::ReserveFrameV1
            | Self::RegisterFrameV1
            | Self::PublishSafepointV1
            | Self::UnregisterFrameV1 => None,
        }
    }

    #[must_use]
    pub const fn internal_abi_signature(self) -> Option<InternalRuntimeSignature> {
        const FRAME_PARAMETERS: &[InternalMachineArgument] = &[
            InternalMachineArgument::InvocationContext,
            InternalMachineArgument::FunctionOrdinal,
            InternalMachineArgument::FramePointer,
        ];
        match self {
            Self::ReserveFrameV1 => Some(InternalRuntimeSignature {
                parameters: &[
                    InternalMachineArgument::InvocationContext,
                    InternalMachineArgument::FunctionOrdinal,
                    InternalMachineArgument::FrameBytes,
                    InternalMachineArgument::FramePointer,
                ],
                result: InternalMachineResult::InvocationContext,
            }),
            Self::RegisterFrameV1 | Self::UnregisterFrameV1 => Some(InternalRuntimeSignature {
                parameters: FRAME_PARAMETERS,
                result: InternalMachineResult::Unit,
            }),
            Self::PublishSafepointV1 => Some(InternalRuntimeSignature {
                parameters: &[
                    InternalMachineArgument::InvocationContext,
                    InternalMachineArgument::SafepointId,
                ],
                result: InternalMachineResult::Unit,
            }),
            Self::HeapDispatchV1 => Some(InternalRuntimeSignature {
                parameters: &[
                    InternalMachineArgument::InvocationContext,
                    InternalMachineArgument::HeapSiteId,
                ],
                result: InternalMachineResult::Unit,
            }),
            Self::IdentityI64V1
            | Self::PollV1
            | Self::EnterFunctionV1
            | Self::CollectReferenceV1 => None,
        }
    }

    #[must_use]
    pub const fn version(self) -> u16 {
        1
    }

    #[must_use]
    pub const fn may_collect(self) -> bool {
        matches!(self, Self::CollectReferenceV1 | Self::HeapDispatchV1)
    }

    pub(crate) const fn plan_callable(self) -> bool {
        matches!(
            self,
            Self::IdentityI64V1 | Self::PollV1 | Self::EnterFunctionV1 | Self::CollectReferenceV1
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeOutcome {
    DeadlineExceeded,
    ResourceLimitExceeded,
    HostFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanError {
    TooManyParameters { count: usize, maximum: usize },
    TooManyItems,
    ForeignId(&'static str),
    UnknownFunction,
    FunctionAlreadyDefined,
    UnknownBlock,
    UnknownValue,
    UnknownLocal,
    BlockAlreadyTerminated,
    EncoderOwnedRuntimeCall,
    InvalidHeapCall,
}

impl fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyParameters { count, maximum } => {
                write!(
                    formatter,
                    "signature has {count} parameters; maximum is {maximum}"
                )
            }
            Self::TooManyItems => formatter.write_str("machine plan exceeds its ID space"),
            Self::ForeignId(kind) => {
                write!(formatter, "{kind} belongs to a different plan or function")
            }
            Self::UnknownFunction => formatter.write_str("unknown machine-plan function"),
            Self::FunctionAlreadyDefined => {
                formatter.write_str("machine-plan function is already defined")
            }
            Self::UnknownBlock => formatter.write_str("unknown machine-plan block"),
            Self::UnknownValue => formatter.write_str("unknown machine-plan value"),
            Self::UnknownLocal => formatter.write_str("unknown machine-plan local"),
            Self::BlockAlreadyTerminated => {
                formatter.write_str("machine-plan block is already terminated")
            }
            Self::EncoderOwnedRuntimeCall => {
                formatter.write_str("runtime call is owned by the native encoder")
            }
            Self::InvalidHeapCall => formatter.write_str("heap runtime call metadata is invalid"),
        }
    }
}

impl std::error::Error for PlanError {}

#[derive(Clone, Debug)]
pub(crate) enum Operation {
    I64Const(i64),
    F64Const(u64),
    BoolConst(bool),
    Unit,
    I64Add(ValueId, ValueId),
    I64Sub(ValueId, ValueId),
    I64Mul(ValueId, ValueId),
    I64Div(ValueId, ValueId),
    I64BitAnd(ValueId, ValueId),
    I64BitOr(ValueId, ValueId),
    I64BitXor(ValueId, ValueId),
    I64ToF64(ValueId),
    F64Add(ValueId, ValueId),
    F64Sub(ValueId, ValueId),
    F64Mul(ValueId, ValueId),
    F64Div(ValueId, ValueId),
    I64Compare(I64Comparison, ValueId, ValueId),
    F64Compare(F64Comparison, ValueId, ValueId),
    F64BitsEqual(ValueId, ValueId),
    BoolCompare(BoolComparison, ValueId, ValueId),
    BoolNot(ValueId),
    ReadLocal(LocalId),
    WriteLocal(LocalId, ValueId),
    Call(FunctionId, Vec<ValueId>),
    RuntimeCall(RuntimeCallSlot, Vec<ValueId>),
    HeapCall(HeapCallDescriptor, Vec<ValueId>),
}

impl Operation {
    pub(crate) fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::I64Const(_)
            | Self::F64Const(_)
            | Self::BoolConst(_)
            | Self::Unit
            | Self::ReadLocal(_) => Vec::new(),
            Self::BoolNot(value) | Self::I64ToF64(value) | Self::WriteLocal(_, value) => {
                vec![*value]
            }
            Self::I64Add(left, right)
            | Self::I64Sub(left, right)
            | Self::I64Mul(left, right)
            | Self::I64Div(left, right)
            | Self::I64BitAnd(left, right)
            | Self::I64BitOr(left, right)
            | Self::I64BitXor(left, right)
            | Self::F64Add(left, right)
            | Self::F64Sub(left, right)
            | Self::F64Mul(left, right)
            | Self::F64Div(left, right)
            | Self::I64Compare(_, left, right)
            | Self::F64Compare(_, left, right)
            | Self::F64BitsEqual(left, right)
            | Self::BoolCompare(_, left, right) => vec![*left, *right],
            Self::Call(_, arguments)
            | Self::RuntimeCall(_, arguments)
            | Self::HeapCall(_, arguments) => arguments.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Instruction {
    pub(crate) output: ValueId,
    pub(crate) output_type: ValueType,
    pub(crate) operation: Operation,
    pub(crate) source: Option<SourceOrigin>,
}

#[derive(Clone, Debug)]
pub(crate) enum Terminator {
    Branch(BlockId),
    BranchIf {
        condition: ValueId,
        when_true: BlockId,
        when_false: BlockId,
    },
    Return(ValueId),
    Trap {
        trap: TrapCode,
        site: Option<u32>,
    },
    Exit(ValueId),
    Outcome(RuntimeOutcome),
}

impl Terminator {
    pub(crate) fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::Branch(_) | Self::Trap { .. } | Self::Outcome(_) => Vec::new(),
            Self::BranchIf { condition, .. } | Self::Return(condition) | Self::Exit(condition) => {
                vec![*condition]
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Block {
    pub(crate) id: BlockId,
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) terminator: Option<Terminator>,
}

#[derive(Clone, Debug)]
pub(crate) enum ValueDefinition {
    Parameter(usize),
    Instruction(BlockId),
}

#[derive(Clone, Debug)]
pub(crate) struct ValueFact {
    pub(crate) id: ValueId,
    pub(crate) value_type: ValueType,
    pub(crate) definition: ValueDefinition,
}

#[derive(Clone, Debug)]
pub(crate) struct LocalFact {
    pub(crate) id: LocalId,
    pub(crate) value_type: ValueType,
}

#[derive(Clone, Debug)]
pub struct FunctionPlan {
    pub(crate) id: FunctionId,
    pub(crate) signature: Signature,
    pub(crate) source_function: SourceFunctionId,
    pub(crate) blocks: Vec<Block>,
    pub(crate) entry: Option<BlockId>,
    pub(crate) values: Vec<ValueFact>,
    pub(crate) locals: Vec<LocalFact>,
}

#[derive(Clone, Debug)]
pub(crate) struct FunctionDeclaration {
    pub(crate) id: FunctionId,
    pub(crate) signature: Signature,
    pub(crate) source_function: SourceFunctionId,
    pub(crate) body: Option<FunctionPlan>,
}

#[derive(Debug)]
pub struct MachinePlanBuilder {
    plan: u64,
    functions: Vec<FunctionDeclaration>,
}

impl MachinePlanBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            plan: NEXT_PLAN_ID.fetch_add(1, Ordering::Relaxed),
            functions: Vec::new(),
        }
    }

    pub fn declare_function(
        &mut self,
        source_function: SourceFunctionId,
        signature: Signature,
    ) -> Result<FunctionId, PlanError> {
        let index = u32::try_from(self.functions.len()).map_err(|_| PlanError::TooManyItems)?;
        let id = FunctionId {
            plan: self.plan,
            index,
        };
        self.functions.push(FunctionDeclaration {
            id,
            signature,
            source_function,
            body: None,
        });
        Ok(id)
    }

    pub fn function_builder(&self, function: FunctionId) -> Result<FunctionBuilder, PlanError> {
        let declaration = self.declaration(function)?;
        if declaration.body.is_some() {
            return Err(PlanError::FunctionAlreadyDefined);
        }
        Ok(FunctionBuilder::new(
            declaration.id,
            declaration.signature.clone(),
            declaration.source_function,
            self.functions
                .iter()
                .map(|item| (item.id, item.signature.clone()))
                .collect(),
        ))
    }

    pub fn define_function(&mut self, function: FunctionPlan) -> Result<(), PlanError> {
        let declaration = self.declaration_mut(function.id)?;
        if declaration.body.is_some() {
            return Err(PlanError::FunctionAlreadyDefined);
        }
        if declaration.signature != function.signature
            || declaration.source_function != function.source_function
        {
            return Err(PlanError::ForeignId("function definition"));
        }
        declaration.body = Some(function);
        Ok(())
    }

    pub fn verify(self, limits: BackendLimits) -> Result<VerifiedMachinePlan, NativeError> {
        verify_plan(self.plan, self.functions, limits)
    }

    fn declaration(&self, function: FunctionId) -> Result<&FunctionDeclaration, PlanError> {
        if function.plan != self.plan {
            return Err(PlanError::ForeignId("function ID"));
        }
        self.functions
            .get(function.index as usize)
            .filter(|item| item.id == function)
            .ok_or(PlanError::UnknownFunction)
    }

    fn declaration_mut(
        &mut self,
        function: FunctionId,
    ) -> Result<&mut FunctionDeclaration, PlanError> {
        if function.plan != self.plan {
            return Err(PlanError::ForeignId("function ID"));
        }
        self.functions
            .get_mut(function.index as usize)
            .filter(|item| item.id == function)
            .ok_or(PlanError::UnknownFunction)
    }
}

impl Default for MachinePlanBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct FunctionBuilder {
    function: FunctionId,
    signature: Signature,
    source_function: SourceFunctionId,
    signatures: Vec<(FunctionId, Signature)>,
    blocks: Vec<Block>,
    entry: Option<BlockId>,
    values: Vec<ValueFact>,
    locals: Vec<LocalFact>,
}

impl FunctionBuilder {
    fn new(
        function: FunctionId,
        signature: Signature,
        source_function: SourceFunctionId,
        signatures: Vec<(FunctionId, Signature)>,
    ) -> Self {
        let values = signature
            .parameters()
            .iter()
            .copied()
            .enumerate()
            .map(|(index, value_type)| ValueFact {
                id: ValueId {
                    function,
                    index: u32::try_from(index).unwrap_or(u32::MAX),
                },
                value_type,
                definition: ValueDefinition::Parameter(index),
            })
            .collect();
        Self {
            function,
            signature,
            source_function,
            signatures,
            blocks: Vec::new(),
            entry: None,
            values,
            locals: Vec::new(),
        }
    }

    #[must_use]
    pub fn function_id(&self) -> FunctionId {
        self.function
    }

    pub fn parameter(&self, index: usize) -> Result<ValueId, PlanError> {
        if index >= self.signature.parameters().len() {
            return Err(PlanError::UnknownValue);
        }
        self.values
            .get(index)
            .map(|fact| fact.id)
            .ok_or(PlanError::UnknownValue)
    }

    pub fn create_block(&mut self) -> Result<BlockId, PlanError> {
        let index = u32::try_from(self.blocks.len()).map_err(|_| PlanError::TooManyItems)?;
        let id = BlockId {
            function: self.function,
            index,
        };
        self.blocks.push(Block {
            id,
            instructions: Vec::new(),
            terminator: None,
        });
        Ok(id)
    }

    pub fn set_entry(&mut self, block: BlockId) -> Result<(), PlanError> {
        self.check_block(block)?;
        self.entry = Some(block);
        Ok(())
    }

    pub fn create_local(&mut self, value_type: ValueType) -> Result<LocalId, PlanError> {
        let index = u32::try_from(self.locals.len()).map_err(|_| PlanError::TooManyItems)?;
        let id = LocalId {
            function: self.function,
            index,
        };
        self.locals.push(LocalFact { id, value_type });
        Ok(id)
    }

    pub fn i64_const(&mut self, block: BlockId, value: i64) -> Result<ValueId, PlanError> {
        self.append(block, ValueType::I64, Operation::I64Const(value), None)
    }

    pub fn f64_const_bits(&mut self, block: BlockId, bits: u64) -> Result<ValueId, PlanError> {
        self.append(block, ValueType::F64, Operation::F64Const(bits), None)
    }

    pub fn bool_const(&mut self, block: BlockId, value: bool) -> Result<ValueId, PlanError> {
        self.append(block, ValueType::Bool, Operation::BoolConst(value), None)
    }

    pub fn unit(&mut self, block: BlockId) -> Result<ValueId, PlanError> {
        self.append(block, ValueType::Unit, Operation::Unit, None)
    }

    pub fn i64_add(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64Add(left, right),
            left,
            right,
        )
    }

    pub fn i64_sub(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64Sub(left, right),
            left,
            right,
        )
    }

    pub fn i64_mul(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64Mul(left, right),
            left,
            right,
        )
    }

    pub fn i64_div(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64Div(left, right),
            left,
            right,
        )
    }

    pub fn i64_bit_and(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64BitAnd(left, right),
            left,
            right,
        )
    }

    pub fn i64_bit_or(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64BitOr(left, right),
            left,
            right,
        )
    }

    pub fn i64_bit_xor(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64BitXor(left, right),
            left,
            right,
        )
    }

    pub fn i64_to_f64(&mut self, block: BlockId, value: ValueId) -> Result<ValueId, PlanError> {
        self.check_value(value)?;
        self.append(block, ValueType::F64, Operation::I64ToF64(value), None)
    }

    pub fn f64_add(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::F64,
            Operation::F64Add(left, right),
            left,
            right,
        )
    }

    pub fn f64_sub(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::F64,
            Operation::F64Sub(left, right),
            left,
            right,
        )
    }

    pub fn f64_mul(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::F64,
            Operation::F64Mul(left, right),
            left,
            right,
        )
    }

    pub fn f64_div(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::F64,
            Operation::F64Div(left, right),
            left,
            right,
        )
    }

    pub fn i64_compare(
        &mut self,
        block: BlockId,
        comparison: I64Comparison,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::Bool,
            Operation::I64Compare(comparison, left, right),
            left,
            right,
        )
    }

    pub fn f64_compare(
        &mut self,
        block: BlockId,
        comparison: F64Comparison,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::Bool,
            Operation::F64Compare(comparison, left, right),
            left,
            right,
        )
    }

    pub fn f64_bits_equal(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::Bool,
            Operation::F64BitsEqual(left, right),
            left,
            right,
        )
    }

    pub fn bool_compare(
        &mut self,
        block: BlockId,
        comparison: BoolComparison,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::Bool,
            Operation::BoolCompare(comparison, left, right),
            left,
            right,
        )
    }

    pub fn bool_not(&mut self, block: BlockId, value: ValueId) -> Result<ValueId, PlanError> {
        self.check_value(value)?;
        self.append(block, ValueType::Bool, Operation::BoolNot(value), None)
    }

    pub fn read_local(&mut self, block: BlockId, local: LocalId) -> Result<ValueId, PlanError> {
        let value_type = self.local_type(local)?;
        self.append(block, value_type, Operation::ReadLocal(local), None)
    }

    pub fn write_local(
        &mut self,
        block: BlockId,
        local: LocalId,
        value: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.local_type(local)?;
        self.check_value(value)?;
        self.append(
            block,
            ValueType::Unit,
            Operation::WriteLocal(local, value),
            None,
        )
    }

    pub fn call(
        &mut self,
        block: BlockId,
        callee: FunctionId,
        arguments: Vec<ValueId>,
    ) -> Result<ValueId, PlanError> {
        if callee.plan != self.function.plan {
            return Err(PlanError::ForeignId("callee"));
        }
        for argument in &arguments {
            self.check_value(*argument)?;
        }
        let signature = self
            .signatures
            .iter()
            .find(|(id, _)| *id == callee)
            .map(|(_, signature)| signature)
            .ok_or(PlanError::UnknownFunction)?;
        self.append(
            block,
            signature.result(),
            Operation::Call(callee, arguments),
            None,
        )
    }

    pub fn heap_call(
        &mut self,
        block: BlockId,
        descriptor: HeapCallDescriptor,
        arguments: Vec<ValueId>,
    ) -> Result<ValueId, PlanError> {
        if arguments.len() != descriptor.input_types().len()
            || arguments
                .iter()
                .zip(descriptor.input_types())
                .any(|(argument, expected)| {
                    self.values
                        .get(argument.index as usize)
                        .filter(|fact| fact.id == *argument)
                        .is_none_or(|fact| fact.value_type != *expected)
                })
        {
            return Err(PlanError::InvalidHeapCall);
        }
        for argument in &arguments {
            self.check_value(*argument)?;
        }
        self.append(
            block,
            descriptor.result_type(),
            Operation::HeapCall(descriptor, arguments),
            None,
        )
    }

    pub fn runtime_call(
        &mut self,
        block: BlockId,
        slot: RuntimeCallSlot,
        arguments: Vec<ValueId>,
    ) -> Result<ValueId, PlanError> {
        for argument in &arguments {
            self.check_value(*argument)?;
        }
        let signature = slot
            .plan_signature()
            .ok_or(PlanError::EncoderOwnedRuntimeCall)?;
        self.append(
            block,
            signature.result(),
            Operation::RuntimeCall(slot, arguments),
            None,
        )
    }

    pub fn set_instruction_source(
        &mut self,
        value: ValueId,
        source: SourceOrigin,
    ) -> Result<(), PlanError> {
        self.check_value(value)?;
        let block_id = match self
            .values
            .get(value.index as usize)
            .map(|fact| &fact.definition)
        {
            Some(ValueDefinition::Instruction(block)) => *block,
            _ => return Err(PlanError::UnknownValue),
        };
        let block = self.block_mut(block_id)?;
        let instruction = block
            .instructions
            .iter_mut()
            .find(|instruction| instruction.output == value)
            .ok_or(PlanError::UnknownValue)?;
        instruction.source = Some(source);
        Ok(())
    }

    pub fn branch(&mut self, block: BlockId, target: BlockId) -> Result<(), PlanError> {
        self.check_block(target)?;
        self.terminate(block, Terminator::Branch(target))
    }

    pub fn branch_if(
        &mut self,
        block: BlockId,
        condition: ValueId,
        when_true: BlockId,
        when_false: BlockId,
    ) -> Result<(), PlanError> {
        self.check_value(condition)?;
        self.check_block(when_true)?;
        self.check_block(when_false)?;
        self.terminate(
            block,
            Terminator::BranchIf {
                condition,
                when_true,
                when_false,
            },
        )
    }

    pub fn return_value(&mut self, block: BlockId, value: ValueId) -> Result<(), PlanError> {
        self.check_value(value)?;
        self.terminate(block, Terminator::Return(value))
    }

    pub fn trap(&mut self, block: BlockId, trap: TrapCode) -> Result<(), PlanError> {
        self.terminate(block, Terminator::Trap { trap, site: None })
    }

    pub fn trap_at(&mut self, block: BlockId, trap: TrapCode, site: u32) -> Result<(), PlanError> {
        self.terminate(
            block,
            Terminator::Trap {
                trap,
                site: Some(site),
            },
        )
    }

    pub fn exit(&mut self, block: BlockId, code: ValueId) -> Result<(), PlanError> {
        self.check_value(code)?;
        self.terminate(block, Terminator::Exit(code))
    }

    pub fn outcome(&mut self, block: BlockId, outcome: RuntimeOutcome) -> Result<(), PlanError> {
        self.terminate(block, Terminator::Outcome(outcome))
    }

    #[must_use]
    pub fn finish(self) -> FunctionPlan {
        FunctionPlan {
            id: self.function,
            signature: self.signature,
            source_function: self.source_function,
            blocks: self.blocks,
            entry: self.entry,
            values: self.values,
            locals: self.locals,
        }
    }

    fn append_binary(
        &mut self,
        block: BlockId,
        output_type: ValueType,
        operation: Operation,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.check_value(left)?;
        self.check_value(right)?;
        self.append(block, output_type, operation, None)
    }

    fn append(
        &mut self,
        block: BlockId,
        output_type: ValueType,
        operation: Operation,
        source: Option<SourceOrigin>,
    ) -> Result<ValueId, PlanError> {
        self.check_block(block)?;
        if self.block(block)?.terminator.is_some() {
            return Err(PlanError::BlockAlreadyTerminated);
        }
        let index = u32::try_from(self.values.len()).map_err(|_| PlanError::TooManyItems)?;
        let output = ValueId {
            function: self.function,
            index,
        };
        self.values.push(ValueFact {
            id: output,
            value_type: output_type,
            definition: ValueDefinition::Instruction(block),
        });
        self.block_mut(block)?.instructions.push(Instruction {
            output,
            output_type,
            operation,
            source,
        });
        Ok(output)
    }

    fn terminate(&mut self, block: BlockId, terminator: Terminator) -> Result<(), PlanError> {
        let block = self.block_mut(block)?;
        if block.terminator.is_some() {
            return Err(PlanError::BlockAlreadyTerminated);
        }
        block.terminator = Some(terminator);
        Ok(())
    }

    fn check_block(&self, block: BlockId) -> Result<(), PlanError> {
        self.block(block).map(|_| ())
    }

    fn block(&self, block: BlockId) -> Result<&Block, PlanError> {
        if block.function != self.function {
            return Err(PlanError::ForeignId("block ID"));
        }
        self.blocks
            .get(block.index as usize)
            .filter(|item| item.id == block)
            .ok_or(PlanError::UnknownBlock)
    }

    fn block_mut(&mut self, block: BlockId) -> Result<&mut Block, PlanError> {
        if block.function != self.function {
            return Err(PlanError::ForeignId("block ID"));
        }
        self.blocks
            .get_mut(block.index as usize)
            .filter(|item| item.id == block)
            .ok_or(PlanError::UnknownBlock)
    }

    fn check_value(&self, value: ValueId) -> Result<(), PlanError> {
        if value.function != self.function {
            return Err(PlanError::ForeignId("value ID"));
        }
        self.values
            .get(value.index as usize)
            .filter(|fact| fact.id == value)
            .map(|_| ())
            .ok_or(PlanError::UnknownValue)
    }

    fn local_type(&self, local: LocalId) -> Result<ValueType, PlanError> {
        if local.function != self.function {
            return Err(PlanError::ForeignId("local ID"));
        }
        self.locals
            .get(local.index as usize)
            .filter(|fact| fact.id == local)
            .map(|fact| fact.value_type)
            .ok_or(PlanError::UnknownLocal)
    }
}
