use super::*;

impl HeapCallDescriptor {
    pub(super) fn operation_types_are_valid(&self) -> bool {
        use HeapOperation as Op;
        use ReferenceType as Ref;
        use ValueType as Ty;

        let inputs = self.input_types.as_slice();
        let result = self.result_type;
        match &self.operation {
            Op::EmptyList => inputs.is_empty() && matches!(result, Ty::Reference(Ref::List(_, _))),
            Op::ProductValue { product, fields } => {
                usize::from(*fields) == inputs.len()
                    && usize::from(*fields) <= 15
                    && inputs.iter().copied().all(is_legacy_heap_value)
                    && u16::try_from(*product).is_ok()
                    && result == Ty::Reference(Ref::Product(LayoutIdentity::product(*product)))
            }
            Op::ProductField {
                product,
                field,
                field_type,
            } => {
                *field < 15
                    && u16::try_from(*product).is_ok()
                    && is_legacy_heap_value(*field_type)
                    && result == *field_type
                    && matches!(inputs, [Ty::Reference(Ref::Product(layout))]
                        if *layout == LayoutIdentity::product(*product))
            }
            Op::WithProductField {
                product,
                field,
                field_type,
            } => {
                *field < 15
                    && u16::try_from(*product).is_ok()
                    && is_legacy_heap_value(*field_type)
                    && matches!(inputs, [Ty::Reference(Ref::Product(layout)), replacement]
                        if *layout == LayoutIdentity::product(*product) && replacement == field_type)
                    && result == Ty::Reference(Ref::Product(LayoutIdentity::product(*product)))
            }
            Op::EnumValue { .. } | Op::EnumIsVariant { .. } | Op::EnumField { .. } => {
                super::enum_heap_validity::enum_operation_types_are_valid(
                    &self.operation,
                    inputs,
                    result,
                )
            }
            Op::Cons => matches!(inputs, [payload, list]
                if is_legacy_heap_value(*payload)
                    && *list == result
                    && matches!(result, Ty::Reference(Ref::List(_, element))
                        if element == payload.layout_identity())),
            Op::Car => matches!(inputs, [Ty::Reference(Ref::List(_, element))]
                if is_legacy_heap_value(result) && *element == result.layout_identity()),
            Op::Cdr => {
                matches!(inputs, [list] if *list == result && matches!(result, Ty::Reference(Ref::List(_, _))))
            }
            Op::IsEmptyList => {
                matches!(inputs, [Ty::Reference(Ref::List(_, _))]) && result == Ty::Bool
            }
            Op::F64FromI64Exact { error_type } => {
                inputs == [Ty::I64]
                    && matches!(result, Ty::Reference(Ref::Enum(_, _)))
                    && matches!(error_type, Ty::Reference(Ref::Enum(_, _)))
            }
            Op::I64FromF64Exact { error_type } | Op::I64FromF64Trunc { error_type } => {
                inputs == [Ty::F64]
                    && matches!(result, Ty::Reference(Ref::Enum(_, _)))
                    && matches!(error_type, Ty::Reference(Ref::Enum(_, _)))
            }
            Op::EqualValue => {
                matches!(
                    inputs,
                    [left @ Ty::Reference(Ref::Enum(_, _)), right]
                        if left == right
                ) && result == Ty::Bool
            }
            Op::ListEqual => {
                matches!(
                    inputs,
                    [
                        Ty::Reference(Ref::List(left, _)),
                        Ty::Reference(Ref::List(right, _))
                    ] if left == right
                ) && result == Ty::Bool
            }
        }
    }
}

pub(super) const fn is_legacy_heap_value(value_type: ValueType) -> bool {
    matches!(
        value_type,
        ValueType::Unit
            | ValueType::Bool
            | ValueType::I64
            | ValueType::F64
            | ValueType::Reference(_)
    )
}
