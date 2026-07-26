use crate::*;
use lkjscript_core::NumericError;

impl JitHeapServices<'_> {
    pub(crate) fn execute_numeric_conversion(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        let input = arguments
            .first()
            .copied()
            .ok_or(NativeServiceError::HostFailure)?;
        match (site.descriptor().operation(), input) {
            (HeapOperation::F64FromI64Rounded, NativeValue::I64(value)) => Ok(
                NativeValue::F64Bits(lkjscript_core::f64_from_i64_rounded(value).to_bits()),
            ),
            (HeapOperation::F64FromI64Exact, NativeValue::I64(value)) => {
                let result = lkjscript_core::f64_from_i64_exact(value)
                    .map(|value| NativeValue::F64Bits(value.to_bits()));
                self.numeric_result(result, site.descriptor().result_type())
            }
            (HeapOperation::I64FromF64Exact, NativeValue::F64Bits(bits)) => {
                let result =
                    lkjscript_core::i64_from_f64_exact(f64::from_bits(bits)).map(NativeValue::I64);
                self.numeric_result(result, site.descriptor().result_type())
            }
            (HeapOperation::I64FromF64Trunc, NativeValue::F64Bits(bits)) => {
                let result =
                    lkjscript_core::i64_from_f64_trunc(f64::from_bits(bits)).map(NativeValue::I64);
                self.numeric_result(result, site.descriptor().result_type())
            }
            _ => self.trap("numeric conversion runtime metadata mismatch"),
        }
    }

    fn numeric_result(
        &mut self,
        result: std::result::Result<NativeValue, NumericError>,
        result_type: ValueType,
    ) -> Result<NativeValue, NativeServiceError> {
        let ReferenceType::Result(_, _, error_layout) = result_type
            .reference_type()
            .ok_or(NativeServiceError::HostFailure)?
        else {
            return Err(NativeServiceError::HostFailure);
        };
        let object = match result {
            Ok(value) => HeapObj::ResultOk(self.value_from_native(value)?),
            Err(error) => {
                let payload = self.allocate(
                    HeapObj::Enum {
                        layout: lkjscript_core::RuntimeLayoutId::new(
                            lkjscript_core::NUMERIC_ERROR_LAYOUT,
                        ),
                        physical_tag: error.physical_tag(),
                        active_payload: Vec::new(),
                    },
                    ReferenceType::Enum(error_layout),
                )?;
                HeapObj::ResultErr(payload)
            }
        };
        let result_reference = result_type
            .reference_type()
            .ok_or(NativeServiceError::HostFailure)?;
        let value = self.allocate(object, result_reference)?;
        self.native_from_value(value, result_type)
    }
}
