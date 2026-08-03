use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralCallDescriptor {
    operation: StructuralOperation,
    signature: Signature,
}

impl StructuralCallDescriptor {
    pub fn new(operation: StructuralOperation) -> Result<Self, PlanError> {
        if !operation.canonical() {
            return Err(PlanError::InvalidStructuralCall);
        }
        let signature = operation_signature(&operation)?;
        Ok(Self {
            operation,
            signature,
        })
    }

    #[must_use]
    pub const fn operation(&self) -> &StructuralOperation {
        &self.operation
    }

    #[must_use]
    pub const fn signature(&self) -> &Signature {
        &self.signature
    }

    pub(crate) fn canonical(&self) -> bool {
        self.operation.canonical()
            && operation_signature(&self.operation).ok().as_ref() == Some(&self.signature)
    }
}

fn operation_signature(operation: &StructuralOperation) -> Result<Signature, PlanError> {
    let owner = |value_type| ValueType::StructuralOwner(value_type);
    let signature = match operation {
        StructuralOperation::PublishStatic {
            value_type,
            payload,
            storage: _,
        } => {
            let input = if *payload == StructuralPayloadKind::String {
                ValueType::StaticString(*value_type)
            } else {
                ValueType::StaticBytes
            };
            Signature::new(vec![input], owner(*value_type))?
        }
        StructuralOperation::PublishUnique {
            value_type, unique, ..
        } => Signature::new(vec![ValueType::Unique(*unique)], owner(*value_type))?,
        StructuralOperation::PublishI64 {
            value_type,
            storage: _,
        }
        | StructuralOperation::PublishFormattedI64(value_type) => {
            Signature::new(vec![ValueType::I64], owner(*value_type))?
        }
        StructuralOperation::PublishOwner {
            value_type,
            storage: _,
        }
        | StructuralOperation::Copy(value_type)
        | StructuralOperation::Move(value_type) => {
            Signature::new(vec![owner(*value_type)], owner(*value_type))?
        }
        StructuralOperation::WitnessIndependentOwner => Signature::new(
            vec![ValueType::MemoryWitnessLocator, ValueType::StructuralKey],
            ValueType::StructuralKey,
        )?,
        StructuralOperation::WitnessDispose => Signature::new(
            vec![ValueType::MemoryWitnessLocator, ValueType::StructuralKey],
            ValueType::Unit,
        )?,
        StructuralOperation::WitnessDisposeStatic(_) => {
            Signature::new(vec![ValueType::StructuralKey], ValueType::Unit)?
        }
        StructuralOperation::CopyView(view) => Signature::new(
            vec![ValueType::StructuralView(*view)],
            structural_value_type(view.projected()),
        )?,
        StructuralOperation::Borrow { projection } => {
            let view = projection.view_type();
            let mut parameters = vec![owner(view.root())];
            if projection.kind() == StructuralProjectionKind::Utf8 {
                parameters.extend([ValueType::I64, ValueType::I64]);
            }
            Signature::new(parameters, ValueType::StructuralView(view))?
        }
        StructuralOperation::StringUtf8View { projection } => Signature::new(
            vec![owner(projection.view_type().root())],
            ValueType::StructuralView(projection.view_type()),
        )?,
        StructuralOperation::EndView(view) => {
            Signature::new(vec![ValueType::StructuralView(*view)], ValueType::Unit)?
        }
        StructuralOperation::Drop(value_type) | StructuralOperation::CaptureTrap(value_type) => {
            Signature::new(vec![owner(*value_type)], ValueType::Unit)?
        }
        StructuralOperation::DestinationCreate { aggregate, storage } => Signature::new(
            Vec::new(),
            ValueType::StructuralDestination(aggregate.destination(*storage, 0)),
        )?,
        StructuralOperation::DestinationInitialize {
            aggregate,
            storage,
            field,
        } => {
            let next = field
                .checked_add(1)
                .ok_or(PlanError::InvalidStructuralCall)?;
            Signature::new(
                vec![
                    ValueType::StructuralDestination(aggregate.destination(*storage, *field)),
                    structural_value_type(aggregate.fields()[usize::from(*field)]),
                ],
                ValueType::StructuralDestination(aggregate.destination(*storage, next)),
            )?
        }
        StructuralOperation::DestinationFinish { aggregate, storage } => {
            let initialized = u16::try_from(aggregate.fields().len())
                .map_err(|_| PlanError::InvalidStructuralCall)?;
            Signature::new(
                vec![ValueType::StructuralDestination(
                    aggregate.destination(*storage, initialized),
                )],
                owner(aggregate.value_type()),
            )?
        }
        StructuralOperation::DestinationAbort {
            aggregate,
            storage,
            initialized,
        } => Signature::new(
            vec![ValueType::StructuralDestination(
                aggregate.destination(*storage, *initialized),
            )],
            ValueType::Unit,
        )?,
        StructuralOperation::ObserveTag(view) | StructuralOperation::ObserveI64(view) => {
            Signature::new(vec![ValueType::StructuralView(*view)], ValueType::I64)?
        }
        StructuralOperation::ObserveOwnedTag(value_type)
        | StructuralOperation::PayloadLength(value_type) => {
            Signature::new(vec![owner(*value_type)], ValueType::I64)?
        }
        StructuralOperation::NumericConversion { kind, success, .. } => {
            let input = match kind {
                StructuralNumericConversion::F64FromI64Exact => ValueType::I64,
                StructuralNumericConversion::I64FromF64Exact
                | StructuralNumericConversion::I64FromF64Truncating => ValueType::F64,
            };
            Signature::new(vec![input], owner(success.value_type()))?
        }
        StructuralOperation::ConsumePayload(aggregate) => Signature::new(
            vec![owner(aggregate.value_type())],
            structural_value_type(aggregate.fields()[0]),
        )?,
        StructuralOperation::PayloadBytesEqual { left, right } => Signature::new(
            vec![
                ValueType::StructuralView(*left),
                ValueType::StructuralView(*right),
            ],
            ValueType::Bool,
        )?,
        StructuralOperation::PayloadUtf8Valid(view) => {
            Signature::new(vec![ValueType::StructuralView(*view)], ValueType::Bool)?
        }
    };
    Ok(signature)
}

fn structural_value_type(value_type: StructuralTypeIdentity) -> ValueType {
    match value_type.kind() {
        StructuralKind::Unit => ValueType::Unit,
        StructuralKind::Bool => ValueType::Bool,
        StructuralKind::I64 => ValueType::I64,
        StructuralKind::F64 => ValueType::F64,
        StructuralKind::String
        | StructuralKind::Path
        | StructuralKind::Bytes
        | StructuralKind::ByteVector
        | StructuralKind::Product
        | StructuralKind::Enum
        | StructuralKind::Static => ValueType::StructuralOwner(value_type),
    }
}
