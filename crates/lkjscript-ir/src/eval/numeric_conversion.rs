use super::*;
use crate::numeric_contract::{self as algorithm, NumericError};

impl Evaluator<'_> {
    pub(crate) fn numeric_conversion(
        &mut self,
        instruction: &InstructionKind,
        input: &EvalValue,
    ) -> std::result::Result<EvalValue, Flow> {
        match (instruction, input) {
            (InstructionKind::F64FromI64Rounded { .. }, EvalValue::I64(value)) => {
                Ok(EvalValue::F64(algorithm::f64_from_i64_rounded(*value)))
            }
            (InstructionKind::F64FromI64Exact { .. }, EvalValue::I64(value)) => {
                self.result(algorithm::f64_from_i64_exact(*value).map(EvalValue::F64))
            }
            (InstructionKind::I64FromF64Exact { .. }, EvalValue::F64(value)) => {
                self.result(algorithm::i64_from_f64_exact(*value).map(EvalValue::I64))
            }
            (InstructionKind::I64FromF64Trunc { .. }, EvalValue::F64(value)) => {
                self.result(algorithm::i64_from_f64_trunc(*value).map(EvalValue::I64))
            }
            _ => Err(Flow::Trap(
                "numeric conversion operand type mismatch".into(),
            )),
        }
    }

    fn result(
        &mut self,
        result: std::result::Result<EvalValue, NumericError>,
    ) -> std::result::Result<EvalValue, Flow> {
        match result {
            Ok(value) => self.allocate_result(value, true),
            Err(error) => {
                let payload =
                    self.allocate_enum(algorithm::ERROR_ID, error.variant_id(), Vec::new())?;
                self.allocate_result(payload, false)
            }
        }
    }
}
