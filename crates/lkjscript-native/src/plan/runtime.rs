use super::*;

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
