use std::num::NonZeroU32;

use lkjscript_core::{
    EnumId, ErrorClass, InlineStructuralValue, ResourceKind, RuntimeLayoutId, SemanticPayload,
    SemanticValue, StructuralFieldMetadata, StructuralFieldRoute, StructuralKind,
    StructuralLayoutKind, StructuralTypeId, StructuralValueCategory, SystemErrorKind, Utf8Failure,
    Value, VariantId,
};

use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::run) enum HostValueType {
    Unit,
    Bool,
    I64,
    F64,
    String,
    Path,
    Bytes,
    Option(Box<Self>),
    Result(Box<Self>, Box<Self>),
    Resource(ResourceKind),
    SystemError,
    Utf8Error,
    NumericError,
}

#[derive(Debug)]
pub(in crate::run) enum HostValue {
    Unit,
    Bool(bool),
    I64(i64),
    F64Bits(u64),
    String(String),
    Path(Vec<u8>),
    Bytes(Vec<u8>),
    Option {
        element: HostValueType,
        value: Option<Box<Self>>,
    },
    Resource {
        kind: ResourceKind,
        value: Value,
    },
    SystemError {
        kind: SystemErrorKind,
        detail: String,
    },
    SystemUtf8(Utf8Failure),
    Utf8Error(Utf8Failure),
    NumericError(lkjscript_core::NumericError),
    Result {
        ok: HostValueType,
        error: HostValueType,
        value: std::result::Result<Box<Self>, Box<Self>>,
    },
}

impl HostValue {
    pub(in crate::run) fn value_type(&self) -> HostValueType {
        match self {
            Self::Unit => HostValueType::Unit,
            Self::Bool(_) => HostValueType::Bool,
            Self::I64(_) => HostValueType::I64,
            Self::F64Bits(_) => HostValueType::F64,
            Self::String(_) => HostValueType::String,
            Self::Path(_) => HostValueType::Path,
            Self::Bytes(_) => HostValueType::Bytes,
            Self::Option { .. } => unreachable!("option value type requires its element type"),
            Self::Resource { kind, .. } => HostValueType::Resource(*kind),
            Self::SystemError { .. } | Self::SystemUtf8(_) => HostValueType::SystemError,
            Self::Utf8Error(_) => HostValueType::Utf8Error,
            Self::NumericError(_) => HostValueType::NumericError,
            Self::Result { .. } => unreachable!("result value type requires its arguments"),
        }
    }

    pub(in crate::run) fn option(element: HostValueType, value: Option<Self>) -> Self {
        Self::Option {
            element,
            value: value.map(Box::new),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AdapterPayload {
    Resource { value: Value, kind: ResourceKind },
    Structural(Value),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AdapterRecord {
    enum_id: EnumId,
    layout: RuntimeLayoutId,
    variant: VariantId,
    physical_tag: u64,
    payload: AdapterPayload,
}

#[derive(Debug)]
enum AdapterSlot {
    Vacant(NonZeroU32),
    Live {
        generation: NonZeroU32,
        record: AdapterRecord,
    },
    Retired,
}

pub(super) struct AggregateAdapters {
    slots: Vec<AdapterSlot>,
    free: Vec<u32>,
    allocations: u64,
}

include!("adapter/storage.rs");
include!("adapter/conversion.rs");
include!("adapter/publication.rs");
include!("adapter/matching.rs");
include!("adapter/errors.rs");
include!("adapter/semantic.rs");
include!("adapter/fields.rs");
include!("adapter/operations.rs");
