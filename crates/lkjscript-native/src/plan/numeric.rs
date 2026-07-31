use super::*;

impl FunctionBuilder {
    pub fn i64_bit_xor(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64BitXor(left, right),
            left,
            right,
        )
    }

    pub fn i64_to_f64(&mut self, block: BlockId, value: ValueId) -> Result<ValueId, PlanError> {
        self.check_value(value)?;
        self.append(block, ValueType::F64, Operation::I64ToF64(value), None)
    }

    pub fn f64_add(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::F64,
            Operation::F64Add(left, right),
            left,
            right,
        )
    }

    pub fn f64_sub(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::F64,
            Operation::F64Sub(left, right),
            left,
            right,
        )
    }

    pub fn f64_mul(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::F64,
            Operation::F64Mul(left, right),
            left,
            right,
        )
    }

    pub fn f64_div(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::F64,
            Operation::F64Div(left, right),
            left,
            right,
        )
    }

    pub fn i64_compare(
        &mut self,
        block: BlockId,
        comparison: I64Comparison,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::Bool,
            Operation::I64Compare(comparison, left, right),
            left,
            right,
        )
    }

    pub fn f64_compare(
        &mut self,
        block: BlockId,
        comparison: F64Comparison,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::Bool,
            Operation::F64Compare(comparison, left, right),
            left,
            right,
        )
    }

    pub fn f64_bits_equal(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::Bool,
            Operation::F64BitsEqual(left, right),
            left,
            right,
        )
    }

    pub fn bool_compare(
        &mut self,
        block: BlockId,
        comparison: BoolComparison,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::Bool,
            Operation::BoolCompare(comparison, left, right),
            left,
            right,
        )
    }

    pub fn bool_not(&mut self, block: BlockId, value: ValueId) -> Result<ValueId, PlanError> {
        self.check_value(value)?;
        self.append(block, ValueType::Bool, Operation::BoolNot(value), None)
    }

    pub fn read_local(&mut self, block: BlockId, local: LocalId) -> Result<ValueId, PlanError> {
        let value_type = self.local_type(local)?;
        self.append(block, value_type, Operation::ReadLocal(local), None)
    }

    pub fn observe_local(&mut self, block: BlockId, local: LocalId) -> Result<ValueId, PlanError> {
        let value_type = self.local_type(local)?;
        let observable = match value_type {
            ValueType::StructuralOwner(_) => true,
            ValueType::StructuralView(view) => !view.exclusive(),
            _ => false,
        };
        if !observable {
            return Err(PlanError::InvalidStructuralCall);
        }
        self.append(block, value_type, Operation::ObserveLocal(local), None)
    }

    pub fn write_local(
        &mut self,
        block: BlockId,
        local: LocalId,
        value: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.local_type(local)?;
        self.check_value(value)?;
        self.append(
            block,
            ValueType::Unit,
            Operation::WriteLocal(local, value),
            None,
        )
    }
}
