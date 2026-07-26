use crate::*;

impl JitHeapServices<'_> {
    pub(crate) fn execute(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        match site.descriptor().operation() {
            HeapOperation::ConstantStr(_)
            | HeapOperation::EmptyStr
            | HeapOperation::EmptyList
            | HeapOperation::None
            | HeapOperation::ProductValue { .. }
            | HeapOperation::ProductField { .. }
            | HeapOperation::WithProductField { .. } => self.execute_products(site, arguments),
            HeapOperation::EnumValue { .. }
            | HeapOperation::EnumIsVariant { .. }
            | HeapOperation::EnumField { .. } => self.execute_enums(site, arguments),
            HeapOperation::Cons
            | HeapOperation::Car
            | HeapOperation::Cdr
            | HeapOperation::IsEmptyList
            | HeapOperation::Some
            | HeapOperation::Ok
            | HeapOperation::Err
            | HeapOperation::IsSome
            | HeapOperation::UnwrapSome
            | HeapOperation::IsOk
            | HeapOperation::UnwrapOk
            | HeapOperation::UnwrapErr => self.execute_lists(site, arguments),
            HeapOperation::BufNew
            | HeapOperation::BufLen
            | HeapOperation::BufRef
            | HeapOperation::BufGetU32
            | HeapOperation::BufSet
            | HeapOperation::BufSetU32 => self.execute_buffer_access(site, arguments),
            HeapOperation::BufClone
            | HeapOperation::BufFromStr
            | HeapOperation::BufToStr
            | HeapOperation::BufSlice => self.execute_buffer_transfer(site, arguments),
            HeapOperation::StrLen
            | HeapOperation::StrRef
            | HeapOperation::StrAppend
            | HeapOperation::StrSlice
            | HeapOperation::StrFromByte
            | HeapOperation::StrFromI64
            | HeapOperation::StrFromF64 => self.execute_strings(site, arguments),
            HeapOperation::F64FromI64Exact
            | HeapOperation::F64FromI64Rounded
            | HeapOperation::I64FromF64Exact
            | HeapOperation::I64FromF64Trunc => self.execute_numeric_conversion(site, arguments),
            HeapOperation::EqualValue | HeapOperation::SameObject | HeapOperation::ListEqual => {
                self.execute_equality(site, arguments)
            }
        }
    }
}
