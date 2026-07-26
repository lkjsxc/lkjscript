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
            (HeapOperation::F64FromI64Exact { error_type }, NativeValue::I64(value)) => {
                let result = lkjscript_core::f64_from_i64_exact(value)
                    .map(|value| NativeValue::F64Bits(value.to_bits()));
                self.numeric_result(result, *error_type, site.descriptor().result_type())
            }
            (HeapOperation::I64FromF64Exact { error_type }, NativeValue::F64Bits(bits)) => {
                let result =
                    lkjscript_core::i64_from_f64_exact(f64::from_bits(bits)).map(NativeValue::I64);
                self.numeric_result(result, *error_type, site.descriptor().result_type())
            }
            (HeapOperation::I64FromF64Trunc { error_type }, NativeValue::F64Bits(bits)) => {
                let result =
                    lkjscript_core::i64_from_f64_trunc(f64::from_bits(bits)).map(NativeValue::I64);
                self.numeric_result(result, *error_type, site.descriptor().result_type())
            }
            _ => self.trap("numeric conversion runtime metadata mismatch"),
        }
    }

    fn numeric_result(
        &mut self,
        result: std::result::Result<NativeValue, NumericError>,
        error_type: ValueType,
        result_type: ValueType,
    ) -> Result<NativeValue, NativeServiceError> {
        let result_reference = match result_type.reference_type() {
            Some(reference @ ReferenceType::Enum(_, _)) => reference,
            _ => return Err(NativeServiceError::HostFailure),
        };
        let (physical_tag, payload) = match result {
            Ok(value) => (0, self.value_from_native(value)?),
            Err(error) => {
                let error_reference = match error_type.reference_type() {
                    Some(reference @ ReferenceType::Enum(_, _)) => reference,
                    _ => return Err(NativeServiceError::HostFailure),
                };
                let payload = self.allocate(
                    HeapObj::Enum {
                        layout: lkjscript_core::RuntimeLayoutId::new(
                            lkjscript_core::NUMERIC_ERROR_LAYOUT,
                        ),
                        physical_tag: error.physical_tag(),
                        active_payload: Vec::new(),
                    },
                    error_reference,
                )?;
                (1, payload)
            }
        };
        let value = self.allocate(
            HeapObj::Enum {
                layout: lkjscript_core::RuntimeLayoutId::new(lkjscript_core::RESULT_LAYOUT),
                physical_tag,
                active_payload: vec![payload],
            },
            result_reference,
        )?;
        self.native_from_value(value, result_type)
    }
}
