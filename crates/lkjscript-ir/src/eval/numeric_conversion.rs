use super::*;
use crate::numeric_contract::{self as algorithm, NumericError};

impl Evaluator<'_> {
    pub(crate) fn numeric_conversion(
        &mut self,
        instruction: &InstructionKind,
        input: &EvalValue,
        result_type: &crate::SsaType,
    ) -> std::result::Result<EvalValue, Flow> {
        match (instruction, input) {
            (InstructionKind::F64FromI64Rounded { .. }, EvalValue::I64(value)) => {
                Ok(EvalValue::F64(algorithm::f64_from_i64_rounded(*value)))
            }
            (InstructionKind::F64FromI64Exact { .. }, EvalValue::I64(value)) => self.result(
                algorithm::f64_from_i64_exact(*value).map(EvalValue::F64),
                result_type,
            ),
            (InstructionKind::I64FromF64Exact { .. }, EvalValue::F64(value)) => self.result(
                algorithm::i64_from_f64_exact(*value).map(EvalValue::I64),
                result_type,
            ),
            (InstructionKind::I64FromF64Trunc { .. }, EvalValue::F64(value)) => self.result(
                algorithm::i64_from_f64_trunc(*value).map(EvalValue::I64),
                result_type,
            ),
            _ => Err(Flow::Trap(
                "numeric conversion operand type mismatch".into(),
            )),
        }
    }

    fn result(
        &mut self,
        result: std::result::Result<EvalValue, NumericError>,
        result_type: &crate::SsaType,
    ) -> std::result::Result<EvalValue, Flow> {
        match result {
            Ok(value) => self.allocate_result(result_type, value, true),
            Err(error) => {
                let (_, fields, _) = enum_variant(
                    self.program.program(),
                    result_type,
                    crate::VariantId::new(crate::prelude_contract::RESULT_ERR_ID),
                )
                .map_err(Flow::Trap)?;
                let error_type = fields
                    .first()
                    .ok_or_else(|| Flow::Trap("numeric result error metadata missing".into()))?;
                let payload = self.allocate_enum(error_type, error.variant_id(), Vec::new())?;
                self.allocate_result(result_type, payload, false)
            }
        }
    }
}
