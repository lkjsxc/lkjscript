use super::{CallTarget, InstructionKind, ValueId};

impl InstructionKind {
    pub fn operands(&self) -> Vec<ValueId> {
        match self {
            Self::Constant(_) | Self::PlaceEnd { .. } | Self::FunctionRef(_) => Vec::new(),
            Self::Copy(value)
            | Self::PlaceInit { value, .. }
            | Self::EndBorrow { value, .. }
            | Self::Drop { value, .. }
            | Self::Move { value, .. }
            | Self::Borrow { value, .. }
            | Self::F64FromI64Exact { value }
            | Self::F64FromI64Rounded { value }
            | Self::I64FromF64Exact { value }
            | Self::I64FromF64Trunc { value } => vec![*value],
            Self::Runtime { arguments, .. }
            | Self::Call {
                target: CallTarget::Direct(_),
                arguments,
                ..
            } => arguments.clone(),
            Self::Call {
                target: CallTarget::Indirect(target),
                arguments,
                ..
            } => {
                let mut operands = Vec::with_capacity(arguments.len().saturating_add(1));
                operands.push(*target);
                operands.extend(arguments.iter().copied());
                operands
            }
            Self::ProductValue { fields, .. } | Self::EnumValue { fields, .. } => fields.clone(),
            Self::ProductField { value, .. }
            | Self::EnumIsVariant { value, .. }
            | Self::EnumField { value, .. } => vec![*value],
            Self::WithProductField {
                value, replacement, ..
            } => vec![*value, *replacement],
        }
    }
}
