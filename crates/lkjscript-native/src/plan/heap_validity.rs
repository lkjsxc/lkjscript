use super::*;

impl HeapCallDescriptor {
    pub(super) fn operation_types_are_valid(&self) -> bool {
        use HeapOperation as Op;
        use ReferenceType as Ref;
        use ValueType as Ty;

        let inputs = self.input_types.as_slice();
        let result = self.result_type;
        match &self.operation {
            Op::EmptyList => inputs.is_empty() && valid_list_reference(result),
            Op::ProductValue { product, fields } => {
                usize::try_from(*fields).ok() == Some(inputs.len())
                    && *fields <= 15
                    && product_reference_matches(result, *product)
                    && inputs.iter().copied().all(is_region_product_field)
            }
            Op::ProductField {
                product,
                field,
                field_type,
            } => {
                *field < 15
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
                    && list_reference_matches(result, *payload)),
            Op::Car => matches!(inputs, [list]
                if is_list_element(result) && list_reference_matches(*list, result)),
            Op::Cdr => matches!(inputs, [list] if *list == result && valid_list_reference(result)),
            Op::IsEmptyList => {
                matches!(inputs, [list] if valid_list_reference(*list)) && result == Ty::Bool
            }
            Op::ListEqual => {
                matches!(inputs, [left, right] if left == right && valid_list_reference(*left))
                    && result == Ty::Bool
            }
        }
    }
}

fn valid_list_reference(value_type: ValueType) -> bool {
    matches!(
        value_type,
        ValueType::Reference(ReferenceType::List(_, list_semantic, _, element_semantic))
            if list_semantic != 0 && element_semantic != 0
    )
}

fn list_reference_matches(list: ValueType, element: ValueType) -> bool {
    matches!(
        list,
        ValueType::Reference(ReferenceType::List(
            _,
            list_semantic,
            element_layout,
            element_semantic,
        )) if list_semantic != 0
            && element_layout == element.layout_identity()
            && semantic_identity(element) == Some(element_semantic)
    )
}

fn semantic_identity(value_type: ValueType) -> Option<u64> {
    match value_type {
        ValueType::Unit => Some(scalar_semantic(1)),
        ValueType::Bool => Some(scalar_semantic(2)),
        ValueType::I64 => Some(scalar_semantic(3)),
        ValueType::F64 => Some(scalar_semantic(4)),
        ValueType::StaticString(value_type) | ValueType::StructuralOwner(value_type) => {
            Some(value_type.semantic_type())
        }
        ValueType::Reference(ReferenceType::List(_, semantic, _, _)) => Some(semantic),
        ValueType::Reference(ReferenceType::RegionProduct(_, identity)) => {
            Some(product_semantic(identity))
        }
        _ => None,
    }
}

const fn scalar_semantic(tag: u8) -> u64 {
    nonzero((0x8f3f_73b5_cf1c_9ade ^ tag as u64).wrapping_mul(0x0000_0100_0000_01b3))
}

fn product_semantic(identity: [u8; 32]) -> u64 {
    let mut state = 0x8f3f_73b5_cf1c_9ade;
    for byte in identity {
        state = (state ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    nonzero(state)
}

const fn nonzero(value: u64) -> u64 {
    if value == 0 {
        1
    } else {
        value
    }
}

fn product_reference_matches(value_type: ValueType, product: u64) -> bool {
    matches!(
        value_type,
        ValueType::Reference(ReferenceType::RegionProduct(layout, _))
            if layout == LayoutIdentity::product(product)
    )
}

const fn is_list_element(value_type: ValueType) -> bool {
    if is_region_product_field(value_type) {
        return true;
    }
    match value_type {
        ValueType::StaticString(value_type) | ValueType::StructuralOwner(value_type) => {
            value_type.copyable()
                || matches!(
                    value_type.kind(),
                    StructuralKind::String | StructuralKind::Path
                )
        }
        _ => false,
    }
}

const fn is_region_product_field(value_type: ValueType) -> bool {
    matches!(
        value_type,
        ValueType::Unit
            | ValueType::Bool
            | ValueType::I64
            | ValueType::F64
            | ValueType::Reference(
                ReferenceType::List(_, _, _, _) | ReferenceType::RegionProduct(_, _)
            )
    )
}
