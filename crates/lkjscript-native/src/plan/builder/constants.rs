use super::*;

impl FunctionBuilder {
    pub fn i64_const(&mut self, block: BlockId, value: i64) -> Result<ValueId, PlanError> {
        self.append(block, ValueType::I64, Operation::I64Const(value), None)
    }

    pub fn f64_const_bits(&mut self, block: BlockId, bits: u64) -> Result<ValueId, PlanError> {
        self.append(block, ValueType::F64, Operation::F64Const(bits), None)
    }

    pub fn bool_const(&mut self, block: BlockId, value: bool) -> Result<ValueId, PlanError> {
        self.append(block, ValueType::Bool, Operation::BoolConst(value), None)
    }

    pub fn unit(&mut self, block: BlockId) -> Result<ValueId, PlanError> {
        self.append(block, ValueType::Unit, Operation::Unit, None)
    }

    pub fn static_bytes_const(
        &mut self,
        block: BlockId,
        identity: StaticBytesIdentity,
    ) -> Result<ValueId, PlanError> {
        self.append(
            block,
            ValueType::StaticBytes,
            Operation::StaticBytesConst(identity),
            None,
        )
    }

    pub fn static_string_const(
        &mut self,
        block: BlockId,
        identity: StaticBytesIdentity,
        value_type: StructuralTypeIdentity,
    ) -> Result<ValueId, PlanError> {
        if value_type.kind() != StructuralKind::String {
            return Err(PlanError::InvalidStructuralCall);
        }
        self.append(
            block,
            ValueType::StaticString(value_type),
            Operation::StaticStringConst(identity, value_type),
            None,
        )
    }
}
