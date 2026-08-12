#![allow(clippy::panic)]

use lkjscript_native::{
    encode, AllocationClass, BackendLimits, CapabilityKind, FailureCleanupCall, HeapCallDescriptor,
    HeapOperation, InternalMachineArgument, InternalMachineResult, LayoutIdentity,
    MachinePlanBuilder, NativeError, PlanError, ReferenceType, ResourceKind, RuntimeCallSlot,
    Signature, SourceFunctionId, StoreClass, StructuralAggregateDescriptor,
    StructuralAggregateKind, StructuralCallDescriptor, StructuralKind, StructuralOperation,
    StructuralPayloadKind, StructuralProjectionDescriptor, StructuralProjectionKind,
    StructuralStorageRoute, StructuralTypeIdentity, StructuralViewType, TrapCode, UniqueType,
    ValueType, VerificationError,
};

mod control;
mod heap_facts;
mod heap_sites;
mod layouts;
mod limits;
mod runtime_abi;
mod runtime_structural;
mod structural;
