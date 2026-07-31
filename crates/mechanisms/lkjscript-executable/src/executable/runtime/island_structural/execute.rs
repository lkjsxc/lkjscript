use super::*;

pub(super) fn execute(
    state: &mut IslandCallState<'_>,
    descriptor: &StructuralCallDescriptor,
    first: u64,
    second: u64,
    third: u64,
) -> Result<NativeValue, NativeServiceError> {
    let services = &mut state.services;
    match descriptor.operation() {
        StructuralOperation::PublishStatic {
            value_type,
            payload,
        } => {
            let bytes = state
                .image
                .resolve_static_bytes(NativeStaticBytes::new(first))
                .ok_or(NativeServiceError::Trap)?;
            services
                .publish_structural_static(bytes, *value_type, *payload)
                .map(NativeValue::StructuralOwner)
        }
        StructuralOperation::PublishUnique {
            value_type,
            payload,
            unique,
        } => services
            .publish_structural_unique(NativeUnique::new(*unique, first), *value_type, *payload)
            .map(NativeValue::StructuralOwner),
        StructuralOperation::PublishI64(value_type) => services
            .publish_structural_i64(first as i64, *value_type)
            .map(NativeValue::StructuralOwner),
        StructuralOperation::PublishFormattedI64(value_type) => services
            .publish_structural_formatted_i64(first as i64, *value_type)
            .map(NativeValue::StructuralOwner),
        StructuralOperation::Copy(value_type) => services
            .copy_structural(NativeStructuralOwner::new(*value_type, first))
            .map(NativeValue::StructuralOwner),
        StructuralOperation::Move(value_type) => services
            .move_structural(NativeStructuralOwner::new(*value_type, first))
            .map(NativeValue::StructuralOwner),
        StructuralOperation::CopyView(view) => {
            services.copy_structural_view(NativeStructuralView::new(*view, first))
        }
        StructuralOperation::Borrow { projection } => services
            .borrow_structural(
                NativeStructuralOwner::new(projection.view_type().root(), first),
                projection,
                second as i64,
                third as i64,
            )
            .map(NativeValue::StructuralView),
        StructuralOperation::StringUtf8View { projection } => services
            .borrow_structural_utf8(
                NativeStructuralOwner::new(projection.view_type().root(), first),
                projection,
            )
            .map(NativeValue::StructuralView),
        StructuralOperation::EndView(view) => services
            .end_structural_view(NativeStructuralView::new(*view, first))
            .map(|()| NativeValue::Unit),
        StructuralOperation::Drop(value_type) => services
            .drop_structural(NativeStructuralOwner::new(*value_type, first))
            .map(|()| NativeValue::Unit),
        StructuralOperation::CaptureTrap(value_type) => services
            .capture_structural_trap(NativeStructuralOwner::new(*value_type, first))
            .map(|()| NativeValue::Unit),
        StructuralOperation::NumericConversion {
            kind,
            success,
            failure,
            errors,
        } => {
            let input = match kind {
                StructuralNumericConversion::F64FromI64Exact => NativeValue::I64(first as i64),
                StructuralNumericConversion::I64FromF64Exact
                | StructuralNumericConversion::I64FromF64Truncating => NativeValue::F64Bits(first),
            };
            services
                .convert_structural_numeric(input, *kind, success, failure, errors)
                .map(NativeValue::StructuralOwner)
        }
        StructuralOperation::DestinationCreate(aggregate) => services
            .create_structural_destination(aggregate)
            .map(NativeValue::StructuralDestination),
        StructuralOperation::DestinationInitialize { aggregate, field } => {
            let value_type = descriptor
                .signature()
                .parameters()
                .get(1)
                .copied()
                .ok_or(NativeServiceError::HostFailure)?;
            services
                .initialize_structural_destination(
                    NativeStructuralDestination::new(aggregate.destination(*field), first),
                    native_value(second, value_type)?,
                    aggregate,
                    *field,
                )
                .map(NativeValue::StructuralDestination)
        }
        StructuralOperation::DestinationFinish(aggregate) => {
            let initialized = u16::try_from(aggregate.fields().len())
                .map_err(|_| NativeServiceError::HostFailure)?;
            services
                .finish_structural_destination(
                    NativeStructuralDestination::new(aggregate.destination(initialized), first),
                    aggregate,
                )
                .map(NativeValue::StructuralOwner)
        }
        StructuralOperation::DestinationAbort {
            aggregate,
            initialized,
        } => services
            .abort_structural_destination(NativeStructuralDestination::new(
                aggregate.destination(*initialized),
                first,
            ))
            .map(|()| NativeValue::Unit),
        StructuralOperation::ObserveTag(view) => services
            .structural_tag(NativeStructuralView::new(*view, first))
            .map(NativeValue::I64),
        StructuralOperation::ObserveOwnedTag(value_type) => services
            .structural_owned_tag(NativeStructuralOwner::new(*value_type, first))
            .map(NativeValue::I64),
        StructuralOperation::PayloadLength(value_type) => services
            .structural_payload_length(NativeStructuralOwner::new(*value_type, first))
            .map(NativeValue::I64),
        StructuralOperation::ObserveI64(view) => services
            .structural_i64(NativeStructuralView::new(*view, first))
            .map(NativeValue::I64),
        StructuralOperation::ConsumePayload(aggregate) => services.consume_structural_payload(
            NativeStructuralOwner::new(aggregate.value_type(), first),
            aggregate,
        ),
        StructuralOperation::PayloadBytesEqual { left, right } => services
            .structural_payload_bytes_equal(
                NativeStructuralView::new(*left, first),
                NativeStructuralView::new(*right, second),
            )
            .map(NativeValue::Bool),
        StructuralOperation::PayloadUtf8Valid(view) => services
            .structural_payload_utf8_valid(NativeStructuralView::new(*view, first))
            .map(NativeValue::Bool),
    }
}

fn native_value(word: u64, value_type: ValueType) -> Result<NativeValue, NativeServiceError> {
    match value_type {
        ValueType::Unit if word == 0 => Ok(NativeValue::Unit),
        ValueType::Bool if word <= 1 => Ok(NativeValue::Bool(word == 1)),
        ValueType::I64 => Ok(NativeValue::I64(word as i64)),
        ValueType::F64 => Ok(NativeValue::F64Bits(word)),
        ValueType::StructuralOwner(value_type) if word != 0 => Ok(NativeValue::StructuralOwner(
            NativeStructuralOwner::new(value_type, word),
        )),
        _ => Err(NativeServiceError::Trap),
    }
}
