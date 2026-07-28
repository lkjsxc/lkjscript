use super::*;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeCallSlot {
    IdentityI64,
    /// Cooperative deadline and native fuel poll. The execution context is the
    /// implicit first ABI argument; no language value is boxed for this call.
    Poll,
    /// Records entry to a source function for exact native-tier accounting.
    EnterFunction,
    /// Installs or reuses the invocation-owned borrowed standard-input resource.
    StdinHandle,
    /// Collecting reference round trip used by the closed runtime contract.
    CollectReference,
    /// Generic verified-frame-home heap dispatch. Plans create it only through
    /// `FunctionBuilder::heap_call`; ordinary runtime-call construction cannot
    /// forge its site metadata.
    HeapDispatch,
    /// Encoder-owned frame-chain operations. Plans cannot name these slots.
    ReserveFrame,
    RegisterFrame,
    PublishSafepoint,
    UnregisterFrame,
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
            Self::IdentityI64 => Some(Signature {
                parameters: vec![ValueType::I64],
                result: ValueType::I64,
            }),
            Self::Poll => Some(Signature {
                parameters: Vec::new(),
                result: ValueType::Unit,
            }),
            Self::EnterFunction => Some(Signature {
                parameters: vec![ValueType::I64],
                result: ValueType::Unit,
            }),
            Self::StdinHandle => Some(Signature {
                parameters: vec![ValueType::Capability(
                    lkjscript_contracts::CapabilityKind::Stdio,
                )],
                result: ValueType::Resource(lkjscript_contracts::ResourceKind::InputStream),
            }),
            Self::CollectReference => Some(Signature {
                parameters: vec![ValueType::Reference(ReferenceType::Buf)],
                result: ValueType::Reference(ReferenceType::Buf),
            }),
            Self::HeapDispatch
            | Self::ReserveFrame
            | Self::RegisterFrame
            | Self::PublishSafepoint
            | Self::UnregisterFrame => None,
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
                | Self::CollectReference
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeOutcome {
    DeadlineExceeded,
    ResourceLimitExceeded,
    HostFailure,
}
