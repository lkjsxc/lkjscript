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
    StructuralTypeIdentity,
};

mod access;
mod conversion;
mod destination;
mod lifecycle;
mod model;
mod numeric;
mod payload;
mod services;
#[cfg(test)]
mod tests;

use conversion::*;
pub(in crate::island) use model::JitStructuralRuntime;
