use crate::*;
use lkjscript_core::{
    InlineStructuralValue, LayoutIdentity, NumericError, SemanticPayload, SemanticTypeIdentity,
    SemanticValue, StructuralDestinationKey, StructuralError, StructuralFieldPath, StructuralKind,
    StructuralNode, StructuralNodeView, StructuralProjection, StructuralRootTableError,
    StructuralType, StructuralValueError, StructuralValueKey, StructuralValueRuntime,
    StructuralValueRuntimeLimits, StructuralViewKey, Value,
};
use lkjscript_executable::NativeServiceError;
use lkjscript_native::{
    NativeStructuralDestination, NativeStructuralOwner, NativeStructuralView, NativeValue,
    StructuralAggregateDescriptor, StructuralAggregateKind, StructuralNumericConversion,
    StructuralPayloadKind, StructuralProjectionDescriptor, StructuralProjectionKind,
    StructuralStorageRoute, StructuralTypeIdentity,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NativeOwnerRecord {
    value_type: StructuralTypeIdentity,
    storage: StructuralStorageRoute,
}

mod access;
mod conversion;
mod destination;
mod equality;
mod lifecycle;
mod lists;
mod model;
mod numeric;
mod payload;
mod services;
#[cfg(test)]
mod tests;
mod witness_services;

use conversion::*;
pub(in crate::island) use model::JitStructuralRuntime;
