#![allow(clippy::panic)]

use lkjscript_native::{
    encode, AllocationClass, BackendLimits, EncodingConfig, FrameHomeKind, HeapCallDescriptor,
    HeapOperation, InternalMachineArgument, InternalMachineResult, LayoutIdentity,
    MachinePlanBuilder, NativeError, PlanError, ReferenceType, RuntimeCallSlot, Signature,
    SourceFunctionId, StoreClass, ValueType, VerificationError,
};

mod control;
mod enum_heap;
mod heap_facts;
mod heap_sites;
mod layouts;
mod limits;
mod references;
mod root_limits;
mod roots;
mod runtime_abi;
