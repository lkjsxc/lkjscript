use super::*;

impl FunctionEncoder<'_> {
    pub(super) fn emit_instruction(
        &mut self,
        instruction: &Instruction,
    ) -> Result<(), NativeError> {
        match &instruction.operation {
            Operation::I64Const(value) => {
                self.load_rax_immediate(*value as u64)?;
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
            Operation::F64Const(bits) => {
                self.load_rax_immediate(*bits)?;
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
            Operation::BoolConst(value) => {
                self.load_rax_immediate(u64::from(*value))?;
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
            Operation::Unit => {
                self.zero_rax()?;
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
            Operation::I64Add(left, right) => {
                self.emit_checked_i64_binary(instruction.output, *left, *right, 0x03, None)?;
            }
            Operation::I64Sub(left, right) => {
                self.emit_checked_i64_binary(instruction.output, *left, *right, 0x2b, None)?;
            }
            Operation::I64Mul(left, right) => {
                self.emit_checked_i64_binary(
                    instruction.output,
                    *left,
                    *right,
                    0xaf,
                    Some(&[0x48, 0x0f]),
                )?;
            }
            Operation::I64Div(left, right) => {
                self.emit_checked_i64_division(instruction.output, *left, *right)?;
            }
            Operation::I64BitAnd(left, right) => {
                self.emit_i64_bitwise(instruction.output, *left, *right, 0x23)?;
            }
            Operation::I64BitOr(left, right) => {
                self.emit_i64_bitwise(instruction.output, *left, *right, 0x0b)?;
            }
            Operation::I64BitXor(left, right) => {
                self.emit_i64_bitwise(instruction.output, *left, *right, 0x33)?;
            }
            Operation::I64ToF64(value) => {
                self.emit_i64_to_f64(instruction.output, *value)?;
            }
            Operation::F64Add(left, right) => {
                self.emit_f64_binary(instruction.output, *left, *right, 0x58)?;
            }
            Operation::F64Sub(left, right) => {
                self.emit_f64_binary(instruction.output, *left, *right, 0x5c)?;
            }
            Operation::F64Mul(left, right) => {
                self.emit_f64_binary(instruction.output, *left, *right, 0x59)?;
            }
            Operation::F64Div(left, right) => {
                self.emit_f64_binary(instruction.output, *left, *right, 0x5e)?;
            }
            Operation::I64Compare(comparison, left, right) => {
                self.emit_integer_comparison(
                    instruction.output,
                    *left,
                    *right,
                    integer_condition(*comparison),
                )?;
            }
            Operation::BoolCompare(comparison, left, right) => {
                let condition = match comparison {
                    BoolComparison::Equal => 0x94,
                    BoolComparison::NotEqual => 0x95,
                };
                self.emit_integer_comparison(instruction.output, *left, *right, condition)?;
            }
            Operation::F64Compare(comparison, left, right) => {
                self.emit_f64_comparison(instruction.output, *left, *right, *comparison)?;
            }
            Operation::F64BitsEqual(left, right) => {
                self.emit_integer_comparison(instruction.output, *left, *right, 0x94)?;
            }
            Operation::BoolNot(value) => {
                self.load_rax(self.value_offset(*value)?)?;
                self.emit(&[0x48, 0x83, 0xf0, 0x01])?;
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
            Operation::ReadLocal(local) => {
                self.load_rax(self.local_offset(*local)?)?;
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
            Operation::WriteLocal(local, value) => {
                self.load_rax(self.value_offset(*value)?)?;
                self.store_rax(self.local_offset(*local)?)?;
                self.zero_rax()?;
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
            Operation::Call(callee, arguments) => {
                let signature = self
                    .find_signature(*callee)
                    .ok_or(NativeError::Encode(EncodeError::InvalidCall))?
                    .clone();
                self.emit_call(
                    instruction.output,
                    &signature,
                    arguments,
                    RelocationTarget::Function(*callee),
                )?;
            }
            Operation::RuntimeCall(slot, arguments) => {
                let signature = slot
                    .plan_signature()
                    .ok_or(NativeError::Encode(EncodeError::InvalidCall))?;
                self.runtime_calls.insert(*slot);
                self.emit_call(
                    instruction.output,
                    &signature,
                    arguments,
                    RelocationTarget::Runtime(*slot),
                )?;
            }
            Operation::HeapCall(descriptor, arguments) => {
                self.emit_heap_call(instruction, descriptor, arguments)?;
            }
        }
        Ok(())
    }
}
