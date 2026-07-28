use super::*;

impl RuntimeCallSlot {
    /// Returns the typed machine-plan signature. Encoder-owned slots have no
    /// plan signature; use `internal_abi_signature` for their private ABI.
    #[must_use]
    pub fn plan_signature(self) -> Option<Signature> {
        signatures::plan_signature(self)
    }

    #[must_use]
    pub const fn internal_abi_signature(self) -> Option<InternalRuntimeSignature> {
        const FRAME_PARAMETERS: &[InternalMachineArgument] = &[
            InternalMachineArgument::InvocationContext,
            InternalMachineArgument::FunctionOrdinal,
            InternalMachineArgument::FramePointer,
        ];
        match self {
            Self::ReserveFrame => Some(InternalRuntimeSignature {
                parameters: &[
                    InternalMachineArgument::InvocationContext,
                    InternalMachineArgument::FunctionOrdinal,
                    InternalMachineArgument::FrameBytes,
                    InternalMachineArgument::FramePointer,
                ],
                result: InternalMachineResult::InvocationContext,
            }),
            Self::RegisterFrame | Self::UnregisterFrame => Some(InternalRuntimeSignature {
                parameters: FRAME_PARAMETERS,
                result: InternalMachineResult::Unit,
            }),
            Self::PublishSafepoint => Some(InternalRuntimeSignature {
                parameters: &[
                    InternalMachineArgument::InvocationContext,
                    InternalMachineArgument::SafepointId,
                ],
                result: InternalMachineResult::Unit,
            }),
            Self::HeapDispatch => Some(InternalRuntimeSignature {
                parameters: &[
                    InternalMachineArgument::InvocationContext,
                    InternalMachineArgument::HeapSiteId,
                ],
                result: InternalMachineResult::Unit,
            }),
            Self::IdentityI64
            | Self::Poll
            | Self::EnterFunction
            | Self::StdinHandle
            | Self::ByteVectorNew
            | Self::ByteVectorMove
            | Self::ByteVectorBorrowShared
            | Self::ByteVectorBorrowExclusive
            | Self::ByteSliceLength
            | Self::ByteSliceByteAt
            | Self::ByteSliceReadU32Le
            | Self::ByteSliceMutSetByte
            | Self::ByteSliceMutWriteU32Le
            | Self::ByteSliceEnd
            | Self::ByteSliceMutEnd
            | Self::ByteVectorDrop
            | Self::StaticBytesLength
            | Self::StaticBytesByteAt
            | Self::StaticBytesClone
            | Self::StaticBytesCopySlice
            | Self::StaticBytesThaw
            | Self::BytesMove
            | Self::BytesBorrowShared
            | Self::BytesLength
            | Self::BytesByteAt
            | Self::BytesClone
            | Self::BytesCopySlice
            | Self::BytesEndBorrow
            | Self::BytesDrop
            | Self::FreezeByteVector
            | Self::ThawBytes
            | Self::CollectReference => None,
        }
    }

    #[must_use]
    pub const fn may_collect(self) -> bool {
        matches!(self, Self::CollectReference | Self::HeapDispatch)
    }

    pub(crate) const fn plan_callable(self) -> bool {
        matches!(
            self,
            Self::IdentityI64
                | Self::Poll
                | Self::EnterFunction
                | Self::StdinHandle
                | Self::ByteVectorNew
                | Self::ByteVectorMove
                | Self::ByteVectorBorrowShared
                | Self::ByteVectorBorrowExclusive
                | Self::ByteSliceLength
                | Self::ByteSliceByteAt
                | Self::ByteSliceReadU32Le
                | Self::ByteSliceMutSetByte
                | Self::ByteSliceMutWriteU32Le
                | Self::ByteSliceEnd
                | Self::ByteSliceMutEnd
                | Self::ByteVectorDrop
                | Self::StaticBytesLength
                | Self::StaticBytesByteAt
                | Self::StaticBytesClone
                | Self::StaticBytesCopySlice
                | Self::StaticBytesThaw
                | Self::BytesMove
                | Self::BytesBorrowShared
                | Self::BytesLength
                | Self::BytesByteAt
                | Self::BytesClone
                | Self::BytesCopySlice
                | Self::BytesEndBorrow
                | Self::BytesDrop
                | Self::FreezeByteVector
                | Self::ThawBytes
                | Self::CollectReference
        )
    }
}
