use crate::*;

impl JitValueServices<'_> {
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
            HeapOperation::Cons
            | HeapOperation::Car
            | HeapOperation::Cdr
            | HeapOperation::IsEmptyList => self.execute_lists(site, arguments),
            HeapOperation::ListEqual => self.execute_equality(site, arguments),
        }
    }
}
