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
                    && u16::try_from(*product).is_ok()
                    && product_reference_matches(result, *product)
                    && inputs.iter().copied().all(is_region_product_field)
            }
            Op::ProductField {
                product,
                field,
                field_type,
            } => {
                *field < 15
                    && u16::try_from(*product).is_ok()
                    && is_region_product_field(*field_type)
                    && result == *field_type
                    && matches!(inputs, [input]
                        if product_reference_matches(*input, *product)
                            && (!matches!(input, Ty::Reference(Ref::RegionProduct(_, _)))
                                || is_region_product_field(*field_type)))
            }
            Op::WithProductField {
                product,
                field,
                field_type,
            } => {
                *field < 15
                    && u16::try_from(*product).is_ok()
                    && is_region_product_field(*field_type)
                    && matches!(inputs, [input, replacement]
                        if product_reference_matches(*input, *product)
                            && replacement == field_type
                            && *input == result
                            && (!matches!(input, Ty::Reference(Ref::RegionProduct(_, _)))
                                || is_region_product_field(*field_type)))
            }
            Op::Cons => matches!(inputs, [payload, list]
                if is_list_element(*payload)
                    && *list == result
                    && matches!(result, Ty::Reference(Ref::List(_, element))
                        if element == payload.layout_identity())),
            Op::Car => matches!(inputs, [Ty::Reference(Ref::List(_, element))]
                if is_list_element(result) && *element == result.layout_identity()),
            Op::Cdr => {
                matches!(inputs, [list] if *list == result && matches!(result, Ty::Reference(Ref::List(_, _))))
            }
            Op::IsEmptyList => {
                matches!(inputs, [Ty::Reference(Ref::List(_, _))]) && result == Ty::Bool
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

fn product_reference_matches(value_type: ValueType, product: u32) -> bool {
    matches!(
        value_type,
        ValueType::Reference(ReferenceType::RegionProduct(layout, _))
            if layout == LayoutIdentity::product(product)
    )
}

const fn is_list_element(value_type: ValueType) -> bool {
    is_region_product_field(value_type)
        || matches!(
            value_type,
            ValueType::StaticString(_) | ValueType::StructuralOwner(_)
        )
}

const fn is_region_product_field(value_type: ValueType) -> bool {
    matches!(
        value_type,
        ValueType::Unit
            | ValueType::Bool
            | ValueType::I64
            | ValueType::F64
            | ValueType::Reference(ReferenceType::List(_, _) | ReferenceType::RegionProduct(_, _))
    )
}
