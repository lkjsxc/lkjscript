use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralNumericConversion {
    F64FromI64Exact,
    I64FromF64Exact,
    I64FromF64Truncating,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructuralOperation {
    PublishStatic {
        value_type: StructuralTypeIdentity,
        payload: StructuralPayloadKind,
    },
    PublishUnique {
        value_type: StructuralTypeIdentity,
        payload: StructuralPayloadKind,
        unique: UniqueType,
    },
    PublishI64(StructuralTypeIdentity),
    PublishFormattedI64(StructuralTypeIdentity),
    Copy(StructuralTypeIdentity),
    CopyView(StructuralViewType),
    Move(StructuralTypeIdentity),
    Borrow {
        projection: StructuralProjectionDescriptor,
    },
    StringUtf8View {
        projection: StructuralProjectionDescriptor,
    },
    EndView(StructuralViewType),
    Drop(StructuralTypeIdentity),
    DestinationCreate(StructuralAggregateDescriptor),
    DestinationInitialize {
        aggregate: StructuralAggregateDescriptor,
        field: u16,
    },
    DestinationFinish(StructuralAggregateDescriptor),
    DestinationAbort {
        aggregate: StructuralAggregateDescriptor,
        initialized: u16,
    },
    ObserveTag(StructuralViewType),
    ObserveOwnedTag(StructuralTypeIdentity),
    PayloadLength(StructuralTypeIdentity),
    CaptureTrap(StructuralTypeIdentity),
    NumericConversion {
        kind: StructuralNumericConversion,
        success: StructuralAggregateDescriptor,
        failure: StructuralAggregateDescriptor,
        errors: Vec<StructuralAggregateDescriptor>,
    },
    ObserveI64(StructuralViewType),
    ConsumePayload(StructuralAggregateDescriptor),
    PayloadBytesEqual {
        left: StructuralViewType,
        right: StructuralViewType,
    },
    PayloadUtf8Valid(StructuralViewType),
}

impl StructuralOperation {
    pub(crate) fn canonical(&self) -> bool {
        match self {
            Self::PublishStatic {
                value_type,
                payload,
            } => value_type.is_valid() && payload_matches(*value_type, *payload),
            Self::PublishUnique {
                value_type,
                payload,
                unique,
            } => {
                value_type.is_valid()
                    && payload_matches(*value_type, *payload)
                    && matches!(unique, UniqueType::Bytes | UniqueType::ByteVector)
            }
            Self::PublishI64(value_type) => {
                value_type.is_valid() && value_type.kind() == StructuralKind::I64
            }
            Self::PublishFormattedI64(value_type) => {
                value_type.is_valid() && value_type.kind() == StructuralKind::String
            }
            Self::Copy(value_type)
            | Self::Move(value_type)
            | Self::Drop(value_type)
            | Self::ObserveOwnedTag(value_type) => value_type.is_valid(),
            Self::PayloadLength(value_type) => {
                value_type.is_valid() && byte_payload(value_type.kind())
            }
            Self::CaptureTrap(value_type) => {
                value_type.is_valid() && value_type.kind() == StructuralKind::String
            }
            Self::NumericConversion {
                kind,
                success,
                failure,
                errors,
            } => numeric_conversion_canonical(*kind, success, failure, errors),
            Self::Borrow { projection } => projection.canonical(),
            Self::StringUtf8View { projection } => {
                projection.canonical()
                    && projection.kind() == StructuralProjectionKind::Utf8
                    && projection.path().is_empty()
                    && projection.view_type().root() == projection.view_type().projected()
                    && projection.view_type().root().kind() == StructuralKind::String
            }
            Self::CopyView(view)
            | Self::EndView(view)
            | Self::ObserveTag(view)
            | Self::ObserveI64(view)
            | Self::PayloadUtf8Valid(view) => view.is_valid(),
            Self::DestinationCreate(aggregate) | Self::DestinationFinish(aggregate) => {
                aggregate.canonical()
            }
            Self::ConsumePayload(aggregate) => {
                aggregate.canonical()
                    && matches!(aggregate.kind(), StructuralAggregateKind::Enum(_))
                    && aggregate.fields().len() == 1
            }
            Self::DestinationInitialize { aggregate, field } => {
                aggregate.canonical() && usize::from(*field) < aggregate.fields().len()
            }
            Self::DestinationAbort {
                aggregate,
                initialized,
            } => aggregate.canonical() && usize::from(*initialized) <= aggregate.fields().len(),
            Self::PayloadBytesEqual { left, right } => {
                left.is_valid()
                    && right.is_valid()
                    && byte_payload(left.projected().kind())
                    && byte_payload(right.projected().kind())
            }
        }
    }

    #[must_use]
    pub const fn is_observation(&self) -> bool {
        matches!(
            self,
            Self::Copy(_)
                | Self::CopyView(_)
                | Self::StringUtf8View { .. }
                | Self::ObserveTag(_)
                | Self::ObserveOwnedTag(_)
                | Self::PayloadLength(_)
                | Self::ObserveI64(_)
                | Self::PayloadBytesEqual { .. }
                | Self::PayloadUtf8Valid(_)
        )
    }
}
