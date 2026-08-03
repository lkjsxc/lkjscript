use super::*;

#[derive(Clone, Copy)]
pub(super) enum MachineArgument {
    Integer(u64),
    Float(f64),
}

#[derive(Clone, Copy)]
pub(super) enum RawReturn {
    Integer(u64),
    Float(f64),
    Unit,
}

impl RawReturn {
    pub(super) fn into_value(self, value_type: ValueType) -> Result<NativeValue, InvocationError> {
        match (self, value_type) {
            (Self::Integer(value), ValueType::I64) => Ok(NativeValue::I64(value as i64)),
            (Self::Integer(value), ValueType::Bool) if value <= 1 => {
                Ok(NativeValue::Bool(value == 1))
            }
            (Self::Integer(value), ValueType::Bool) => {
                Err(InvocationError::InvalidBoolReturn(value))
            }
            (Self::Float(value), ValueType::F64) => Ok(NativeValue::F64Bits(value.to_bits())),
            (Self::Unit, ValueType::Unit) => Ok(NativeValue::Unit),
            (Self::Integer(value), ValueType::StaticBytes) if value != 0 => {
                Ok(NativeValue::StaticBytes(NativeStaticBytes::new(value)))
            }
            (Self::Integer(_), ValueType::StaticBytes) => {
                Err(InvocationError::UnsupportedSignature)
            }
            (Self::Integer(value), ValueType::StaticString(value_type)) if value != 0 => Ok(
                NativeValue::StaticString(NativeStaticString::new(value_type, value)),
            ),
            (Self::Integer(_), ValueType::StaticString(_)) => {
                Err(InvocationError::UnsupportedSignature)
            }
            (Self::Integer(value), ValueType::Capability(kind))
                if value == capability_word(kind) =>
            {
                Ok(NativeValue::Capability(kind))
            }
            (Self::Integer(_), ValueType::Capability(_)) => {
                Err(InvocationError::UnsupportedSignature)
            }
            (Self::Integer(value), ValueType::Resource(kind)) if value != 0 => {
                Ok(NativeValue::Resource(NativeResource::new(kind, value)))
            }
            (Self::Integer(_), ValueType::Resource(_)) => {
                Err(InvocationError::UnsupportedSignature)
            }
            (Self::Integer(value), ValueType::Unique(kind)) if value != 0 => {
                Ok(NativeValue::Unique(NativeUnique::new(kind, value)))
            }
            (Self::Integer(_), ValueType::Unique(_)) => Err(InvocationError::UnsupportedSignature),
            (Self::Integer(value), ValueType::Loan(kind)) if value != 0 => {
                Ok(NativeValue::Loan(NativeLoan::new(kind, value)))
            }
            (Self::Integer(_), ValueType::Loan(_)) => Err(InvocationError::UnsupportedSignature),
            (Self::Integer(value), ValueType::StructuralKey) if value != 0 => {
                Ok(NativeValue::StructuralKey(value))
            }
            (Self::Integer(value), ValueType::StructuralOwner(value_type)) if value != 0 => Ok(
                NativeValue::StructuralOwner(NativeStructuralOwner::new(value_type, value)),
            ),
            (Self::Integer(value), ValueType::StructuralView(view_type)) if value != 0 => Ok(
                NativeValue::StructuralView(NativeStructuralView::new(view_type, value)),
            ),
            (Self::Integer(value), ValueType::StructuralDestination(destination_type))
                if value != 0 =>
            {
                Ok(NativeValue::StructuralDestination(
                    NativeStructuralDestination::new(destination_type, value),
                ))
            }
            (
                Self::Integer(_),
                ValueType::StructuralKey
                | ValueType::StructuralOwner(_)
                | ValueType::StructuralView(_)
                | ValueType::StructuralDestination(_),
            ) => Err(InvocationError::UnsupportedSignature),
            (Self::Integer(value), ValueType::Reference(reference_type)) => Ok(
                NativeValue::Reference(NativeReference::new(reference_type, value)),
            ),
            _ => Err(InvocationError::UnsupportedSignature),
        }
    }
}

pub(super) fn native_value_word(value: NativeValue, expected: ValueType) -> Option<u64> {
    if value.value_type() != expected {
        return None;
    }
    Some(match value {
        NativeValue::I64(value) => value as u64,
        NativeValue::F64Bits(bits) => bits,
        NativeValue::Bool(value) => u64::from(value),
        NativeValue::Unit => 0,
        NativeValue::StaticBytes(identity) => identity.opaque_word(),
        NativeValue::StaticString(identity) => identity.opaque_word(),
        NativeValue::Capability(kind) => capability_word(kind),
        NativeValue::Resource(resource) => resource.opaque_word(),
        NativeValue::Unique(unique) => unique.opaque_word(),
        NativeValue::Loan(loan) => loan.opaque_word(),
        NativeValue::StructuralKey(key) => key,
        NativeValue::StructuralOwner(owner) => owner.opaque_word(),
        NativeValue::StructuralView(view) => view.opaque_word(),
        NativeValue::StructuralDestination(destination) => destination.opaque_word(),
        NativeValue::Reference(reference) => reference.opaque_word(),
    })
}

pub(super) fn validate_arguments(
    signature: &Signature,
    arguments: &[NativeValue],
) -> Result<(), InvocationError> {
    if signature.parameters().len() != arguments.len() {
        return Err(InvocationError::ArgumentCount {
            expected: signature.parameters().len(),
            actual: arguments.len(),
        });
    }
    for (index, (expected, actual)) in signature
        .parameters()
        .iter()
        .copied()
        .zip(arguments.iter().copied())
        .enumerate()
    {
        if expected != actual.value_type() {
            return Err(InvocationError::ArgumentType {
                index,
                expected: Box::new(expected),
                actual: Box::new(actual.value_type()),
            });
        }
    }
    Ok(())
}

pub(super) fn machine_arguments(arguments: &[NativeValue]) -> Vec<MachineArgument> {
    arguments
        .iter()
        .filter_map(|argument| match argument {
            NativeValue::I64(value) => Some(MachineArgument::Integer(*value as u64)),
            NativeValue::F64Bits(bits) => Some(MachineArgument::Float(f64::from_bits(*bits))),
            NativeValue::Bool(value) => Some(MachineArgument::Integer(u64::from(*value))),
            NativeValue::Unit => None,
            NativeValue::StaticBytes(identity) => {
                Some(MachineArgument::Integer(identity.opaque_word()))
            }
            NativeValue::StaticString(identity) => {
                Some(MachineArgument::Integer(identity.opaque_word()))
            }
            NativeValue::Capability(kind) => Some(MachineArgument::Integer(capability_word(*kind))),
            NativeValue::Resource(resource) => {
                Some(MachineArgument::Integer(resource.opaque_word()))
            }
            NativeValue::Unique(unique) => Some(MachineArgument::Integer(unique.opaque_word())),
            NativeValue::Loan(loan) => Some(MachineArgument::Integer(loan.opaque_word())),
            NativeValue::StructuralKey(key) => Some(MachineArgument::Integer(*key)),
            NativeValue::StructuralOwner(owner) => {
                Some(MachineArgument::Integer(owner.opaque_word()))
            }
            NativeValue::StructuralView(view) => Some(MachineArgument::Integer(view.opaque_word())),
            NativeValue::StructuralDestination(destination) => {
                Some(MachineArgument::Integer(destination.opaque_word()))
            }
            NativeValue::Reference(reference) => {
                Some(MachineArgument::Integer(reference.opaque_word()))
            }
        })
        .collect()
}

pub(super) const fn capability_word(kind: lkjscript_native::CapabilityKind) -> u64 {
    kind as u64 + 1
}
