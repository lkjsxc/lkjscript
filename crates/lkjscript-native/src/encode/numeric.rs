use super::*;

impl FunctionEncoder<'_> {
    pub(super) fn emit_checked_i64_binary(
        &mut self,
        instruction: &Instruction,
        left: ValueId,
        right: ValueId,
        opcode: u8,
        prefix: Option<&[u8]>,
    ) -> Result<(), NativeError> {
        self.load_rax(self.value_offset(left)?)?;
        if let Some(prefix) = prefix {
            self.emit(prefix)?;
            self.emit(&[opcode, 0x85])?;
        } else {
            self.emit(&[0x48, opcode, 0x85])?;
        }
        self.emit_displacement(self.value_offset(right)?)?;
        self.emit_instruction_trap_branch(0x80, TrapCode::I64Overflow, instruction)?;
        self.store_rax(self.value_offset(instruction.output)?)
    }

    pub(super) fn emit_checked_i64_division(
        &mut self,
        instruction: &Instruction,
        left: ValueId,
        right: ValueId,
    ) -> Result<(), NativeError> {
        self.emit(&[0x48, 0x83, 0xbd])?;
        self.emit_displacement(self.value_offset(right)?)?;
        self.emit(&[0x00])?;
        self.emit_instruction_trap_branch(0x84, TrapCode::DivisionByZero, instruction)?;
        self.load_rax(self.value_offset(left)?)?;
        self.emit(&[0x48, 0xb9])?;
        self.emit(&(i64::MIN as u64).to_le_bytes())?;
        self.emit(&[0x48, 0x39, 0xc8])?;
        self.emit(&[0x0f, 0x85])?;
        let normal_displacement = self.reserve_i32()?;
        self.emit(&[0x48, 0x83, 0xbd])?;
        self.emit_displacement(self.value_offset(right)?)?;
        self.emit(&[0xff])?;
        self.emit_instruction_trap_branch(0x84, TrapCode::I64Overflow, instruction)?;
        let normal = self.bytes.len();
        patch_relative(self.bytes, normal_displacement, normal)?;
        self.emit(&[0x48, 0x99])?;
        self.emit(&[0x48, 0xf7, 0xbd])?;
        self.emit_displacement(self.value_offset(right)?)?;
        self.store_rax(self.value_offset(instruction.output)?)
    }

    pub(super) fn emit_i64_bitwise(
        &mut self,
        output: ValueId,
        left: ValueId,
        right: ValueId,
        opcode: u8,
    ) -> Result<(), NativeError> {
        self.load_rax(self.value_offset(left)?)?;
        self.emit(&[0x48, opcode, 0x85])?;
        self.emit_displacement(self.value_offset(right)?)?;
        self.store_rax(self.value_offset(output)?)
    }

    pub(super) fn emit_i64_to_f64(
        &mut self,
        output: ValueId,
        value: ValueId,
    ) -> Result<(), NativeError> {
        self.emit(&[0xf2, 0x48, 0x0f, 0x2a, 0x85])?;
        self.emit_displacement(self.value_offset(value)?)?;
        self.store_xmm0(self.value_offset(output)?)
    }

    pub(super) fn emit_f64_binary(
        &mut self,
        output: ValueId,
        left: ValueId,
        right: ValueId,
        opcode: u8,
    ) -> Result<(), NativeError> {
        self.load_xmm0(self.value_offset(left)?)?;
        self.emit(&[0xf2, 0x0f, opcode, 0x85])?;
        self.emit_displacement(self.value_offset(right)?)?;
        self.store_xmm0(self.value_offset(output)?)
    }

    pub(super) fn emit_integer_comparison(
        &mut self,
        output: ValueId,
        left: ValueId,
        right: ValueId,
        condition: u8,
    ) -> Result<(), NativeError> {
        self.load_rax(self.value_offset(left)?)?;
        self.emit(&[0x48, 0x3b, 0x85])?;
        self.emit_displacement(self.value_offset(right)?)?;
        self.emit(&[0x0f, condition, 0xc0, 0x0f, 0xb6, 0xc0])?;
        self.store_rax(self.value_offset(output)?)
    }

    pub(super) fn emit_f64_comparison(
        &mut self,
        output: ValueId,
        left: ValueId,
        right: ValueId,
        comparison: F64Comparison,
    ) -> Result<(), NativeError> {
        self.load_xmm0(self.value_offset(left)?)?;
        self.emit(&[0x66, 0x0f, 0x2e, 0x85])?;
        self.emit_displacement(self.value_offset(right)?)?;
        let condition = match comparison {
            F64Comparison::OrderedEqual => 0x94,
            F64Comparison::OrderedNotEqual => 0x95,
            F64Comparison::OrderedLessThan => 0x92,
            F64Comparison::OrderedLessThanOrEqual => 0x96,
            F64Comparison::OrderedGreaterThan => 0x97,
            F64Comparison::OrderedGreaterThanOrEqual => 0x93,
        };
        self.emit(&[0x0f, condition, 0xc0])?;
        self.emit(&[0x0f, 0x9b, 0xc1])?;
        self.emit(&[0x20, 0xc8, 0x0f, 0xb6, 0xc0])?;
        self.store_rax(self.value_offset(output)?)
    }
}
