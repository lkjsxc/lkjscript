use super::*;

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
            Op::None => inputs.is_empty() && matches!(result, Ty::Reference(Ref::Option(_, _))),
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
            Op::Some => matches!(inputs, [payload]
                if matches!(result, Ty::Reference(Ref::Option(_, value))
                    if value == payload.layout_identity())),
            Op::IsSome => {
                matches!(inputs, [Ty::Reference(Ref::Option(_, _))]) && result == Ty::Bool
            }
            Op::UnwrapSome => matches!(inputs, [Ty::Reference(Ref::Option(_, payload))]
                if *payload == result.layout_identity()),
            Op::Ok => matches!(inputs, [payload]
                if matches!(result, Ty::Reference(Ref::Result(_, ok, _)) if ok == payload.layout_identity())),
            Op::Err => matches!(inputs, [payload]
                if matches!(result, Ty::Reference(Ref::Result(_, _, error)) if error == payload.layout_identity())),
            Op::IsOk => {
                matches!(inputs, [Ty::Reference(Ref::Result(_, _, _))]) && result == Ty::Bool
            }
            Op::UnwrapOk => matches!(inputs, [Ty::Reference(Ref::Result(_, ok, _))]
                if *ok == result.layout_identity()),
            Op::UnwrapErr => matches!(inputs, [Ty::Reference(Ref::Result(_, _, error))]
                if *error == result.layout_identity()),
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
            Op::BufToStr => {
                inputs == [Ty::Reference(Ref::Buf)]
                    && result
                        == Ty::Reference(Ref::Result(
                            result.layout_identity(),
                            Ty::Reference(Ref::Str).layout_identity(),
                            Ty::Reference(Ref::Str).layout_identity(),
                        ))
            }
            Op::BufSlice => {
                inputs == [Ty::Reference(Ref::Buf), Ty::I64, Ty::I64]
                    && result
                        == Ty::Reference(Ref::Result(
                            result.layout_identity(),
                            Ty::Reference(Ref::Buf).layout_identity(),
                            Ty::Reference(Ref::Str).layout_identity(),
                        ))
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
            Op::F64FromI64Rounded => inputs == [Ty::I64] && result == Ty::F64,
            Op::F64FromI64Exact => {
                inputs == [Ty::I64]
                    && matches!(result, Ty::Reference(Ref::Result(_, ok, _))
                        if ok == Ty::F64.layout_identity())
            }
            Op::I64FromF64Exact | Op::I64FromF64Trunc => {
                inputs == [Ty::F64]
                    && matches!(result, Ty::Reference(Ref::Result(_, ok, _))
                        if ok == Ty::I64.layout_identity())
            }
            Op::EqualValue => {
                matches!(
                    inputs,
                    [left @ Ty::Reference(Ref::Str | Ref::Option(_, _) | Ref::Result(_, _, _)), right]
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
