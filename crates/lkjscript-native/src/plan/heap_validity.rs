use super::*;

const OPTION_LAYOUT: [u8; 32] = [
    0x59, 0x65, 0x30, 0x02, 0x24, 0xcf, 0x57, 0x10, 0x7d, 0x47, 0x39, 0x82, 0x08, 0x01, 0x10, 0xd8,
    0x1c, 0x6f, 0x0f, 0x5e, 0xb3, 0x9b, 0x6f, 0xb0, 0xd4, 0x8e, 0x41, 0x02, 0xe4, 0xdd, 0xc5, 0xd1,
];
const RESULT_LAYOUT: [u8; 32] = [
    0xad, 0x73, 0x35, 0x0f, 0xf0, 0x48, 0xd2, 0xf4, 0x87, 0xd7, 0xe6, 0x1e, 0x52, 0x6a, 0xb7, 0x3c,
    0x50, 0x20, 0xf6, 0x48, 0x37, 0xb8, 0xfd, 0xdf, 0xd1, 0x0e, 0xbb, 0x35, 0x09, 0x84, 0xc9, 0xf0,
];
const SYSTEM_ERROR_LAYOUT: [u8; 32] = [
    0x99, 0xb9, 0x2b, 0x22, 0xaa, 0x82, 0xb0, 0xd6, 0x58, 0xd2, 0x08, 0xd8, 0xfa, 0x80, 0x4c, 0xe7,
    0x78, 0xf4, 0x28, 0x08, 0xfa, 0x63, 0xa8, 0x9a, 0xa9, 0x62, 0x90, 0x59, 0x60, 0xa6, 0x71, 0x55,
];
const UTF8_ERROR_LAYOUT: [u8; 32] = [
    0x26, 0xe6, 0xa3, 0x42, 0xf9, 0x98, 0xfb, 0x19, 0x2e, 0xca, 0xf6, 0x6e, 0x53, 0xc8, 0x37, 0xff,
    0xcc, 0x66, 0x24, 0x36, 0x08, 0x01, 0x83, 0x30, 0xf5, 0x26, 0xc1, 0x40, 0x18, 0xcc, 0xa1, 0x02,
];

impl HeapCallDescriptor {
    pub(super) fn operation_types_are_valid(&self) -> bool {
        use HeapOperation as Op;
        use ReferenceType as Ref;
        use ValueType as Ty;

        let inputs = self.input_types.as_slice();
        let result = self.result_type;
        match &self.operation {
            Op::ConstantStr(_) | Op::EmptyStr => {
                inputs.is_empty() && result == Ty::Reference(Ref::Str)
            }
            Op::EmptyList => inputs.is_empty() && matches!(result, Ty::Reference(Ref::List(_, _))),
            Op::ProductValue { product, fields } => {
                usize::from(*fields) == inputs.len()
                    && usize::from(*fields) <= 15
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
                if *list == result
                    && matches!(result, Ty::Reference(Ref::List(_, element))
                        if element == payload.layout_identity())),
            Op::Car => matches!(inputs, [Ty::Reference(Ref::List(_, element))]
                if *element == result.layout_identity()),
            Op::Cdr => {
                matches!(inputs, [list] if *list == result && matches!(result, Ty::Reference(Ref::List(_, _))))
            }
            Op::IsEmptyList => {
                matches!(inputs, [Ty::Reference(Ref::List(_, _))]) && result == Ty::Bool
            }
            Op::BufNew => inputs == [Ty::I64] && result == Ty::Reference(Ref::Buf),
            Op::BufLen => inputs == [Ty::Reference(Ref::Buf)] && result == Ty::I64,
            Op::BufRef | Op::BufGetU32 => {
                inputs == [Ty::Reference(Ref::Buf), Ty::I64] && result == Ty::I64
            }
            Op::BufSet | Op::BufSetU32 => {
                inputs == [Ty::Reference(Ref::Buf), Ty::I64, Ty::I64] && result == Ty::Unit
            }
            Op::BufClone => {
                inputs == [Ty::Reference(Ref::Buf)] && result == Ty::Reference(Ref::Buf)
            }
            Op::BufFromStr => {
                inputs == [Ty::Reference(Ref::Str)] && result == Ty::Reference(Ref::Buf)
            }
            Op::BufToStr { error_type } => {
                inputs == [Ty::Reference(Ref::Buf)]
                    && matches!(result, Ty::Reference(Ref::Enum(_, layout)) if layout == RESULT_LAYOUT)
                    && matches!(error_type, Ref::Enum(_, layout) if *layout == UTF8_ERROR_LAYOUT)
            }
            Op::BufSlice {
                error_type,
                code_option_type,
                detail_option_type,
            } => {
                inputs == [Ty::Reference(Ref::Buf), Ty::I64, Ty::I64]
                    && matches!(result, Ty::Reference(Ref::Enum(_, layout)) if layout == RESULT_LAYOUT)
                    && matches!(error_type, Ref::Enum(_, layout) if *layout == SYSTEM_ERROR_LAYOUT)
                    && matches!(code_option_type, Ref::Enum(_, layout) if *layout == OPTION_LAYOUT)
                    && matches!(detail_option_type, Ref::Enum(_, layout) if *layout == OPTION_LAYOUT)
                    && code_option_type != detail_option_type
            }
            Op::StrLen => inputs == [Ty::Reference(Ref::Str)] && result == Ty::I64,
            Op::StrRef => inputs == [Ty::Reference(Ref::Str), Ty::I64] && result == Ty::I64,
            Op::StrAppend => {
                inputs == [Ty::Reference(Ref::Str), Ty::Reference(Ref::Str)]
                    && result == Ty::Reference(Ref::Str)
            }
            Op::StrSlice => {
                inputs == [Ty::Reference(Ref::Str), Ty::I64, Ty::I64]
                    && result == Ty::Reference(Ref::Str)
            }
            Op::StrFromByte | Op::StrFromI64 => {
                inputs == [Ty::I64] && result == Ty::Reference(Ref::Str)
            }
            Op::StrFromF64 => inputs == [Ty::F64] && result == Ty::Reference(Ref::Str),
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
                    [left @ Ty::Reference(Ref::Str | Ref::Enum(_, _)), right]
                        if left == right
                ) && result == Ty::Bool
            }
            Op::SameObject => {
                inputs == [Ty::Reference(Ref::Buf), Ty::Reference(Ref::Buf)] && result == Ty::Bool
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
