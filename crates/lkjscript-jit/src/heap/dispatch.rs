use crate::*;

impl JitHeapServices<'_> {
    pub(crate) fn execute(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        match site.descriptor().operation() {
            HeapOperation::EmptyList
            | HeapOperation::ProductValue { .. }
            | HeapOperation::ProductField { .. }
            | HeapOperation::WithProductField { .. } => self.execute_products(site, arguments),
            HeapOperation::EnumValue { .. }
            | HeapOperation::EnumIsVariant { .. }
            | HeapOperation::EnumField { .. } => self.execute_enums(site, arguments),
            HeapOperation::Cons
            | HeapOperation::Car
            | HeapOperation::Cdr
            | HeapOperation::IsEmptyList => self.execute_lists(site, arguments),
            HeapOperation::F64FromI64Exact { .. }
            | HeapOperation::I64FromF64Exact { .. }
            | HeapOperation::I64FromF64Trunc { .. } => {
                self.execute_numeric_conversion(site, arguments)
            }
            HeapOperation::EqualValue | HeapOperation::ListEqual => {
                self.execute_equality(site, arguments)
            }
        }
    }
}
