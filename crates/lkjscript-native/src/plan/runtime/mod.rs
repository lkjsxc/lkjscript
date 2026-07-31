use super::*;

mod signatures;
mod slots;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeCallSlot {
    IdentityI64,
    /// Cooperative deadline and native fuel poll. The execution context is the
    /// implicit first ABI argument; no language value is boxed for this call.
    Poll,
    /// Consumes the marker set when a generated callee rejects before entry.
    TakeRejectedEntry,
    /// Records entry to a source function for exact native-tier accounting.
    EnterFunction,
    /// Installs or reuses the invocation-owned borrowed standard-input resource.
    StdinHandle,
    ByteVectorNew,
    ByteVectorMove,
    ByteVectorBorrowShared,
    ByteVectorBorrowExclusive,
    ByteSliceLength,
    ByteSliceByteAt,
    ByteSliceReadU32Le,
    ByteSliceMutSetByte,
    ByteSliceMutWriteU32Le,
    ByteSliceEnd,
    ByteSliceMutEnd,
    ByteVectorDrop,
    StaticBytesLength,
    StaticBytesByteAt,
    StaticBytesClone,
    StaticBytesCopySlice,
    StaticBytesThaw,
    BytesMove,
    BytesBorrowShared,
    BytesLength,
    BytesByteAt,
    BytesClone,
    BytesCopySlice,
    BytesEndBorrow,
    BytesDrop,
    FreezeByteVector,
    ThawBytes,
    /// Generic direct structural operation selected by verified site metadata.
    StructuralDispatch,
    /// Generic verified-frame-home runtime-value dispatch. Plans create it only through
    /// `FunctionBuilder::heap_call`; ordinary runtime-call construction cannot
    /// forge its site metadata.
    HeapDispatch,
    /// Encoder-owned frame-chain operations. Plans cannot name these slots.
    ReserveFrame,
    RegisterFrame,
    UnregisterFrame,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InternalMachineArgument {
    InvocationContext,
    FunctionOrdinal,
    FrameBytes,
    FramePointer,
    HeapSiteId,
    StructuralSiteId,
    StructuralArgument0,
    StructuralArgument1,
    StructuralArgument2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InternalMachineResult {
    Unit,
    Integer,
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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeOutcome {
    DeadlineExceeded,
    ResourceLimitExceeded,
    HostFailure,
}
