use super::*;

impl FunctionBuilder {
    pub(in crate::plan) fn new(
        function: FunctionId,
        signature: Signature,
        source_function: SourceFunctionId,
        signatures: Vec<(FunctionId, Signature)>,
    ) -> Self {
        let values = signature
            .parameters()
            .iter()
            .copied()
            .enumerate()
            .map(|(index, value_type)| ValueFact {
                id: ValueId {
                    function,
                    index: u32::try_from(index).unwrap_or(u32::MAX),
                },
                value_type,
                definition: ValueDefinition::Parameter(index),
            })
            .collect();
        Self {
            function,
            signature,
            source_function,
            signatures,
            blocks: Vec::new(),
            entry: None,
            values,
            locals: Vec::new(),
        }
    }

    #[must_use]
    pub fn function_id(&self) -> FunctionId {
        self.function
    }

    pub fn parameter(&self, index: usize) -> Result<ValueId, PlanError> {
        if index >= self.signature.parameters().len() {
            return Err(PlanError::UnknownValue);
        }
        self.values
            .get(index)
            .map(|fact| fact.id)
            .ok_or(PlanError::UnknownValue)
    }

    pub fn create_block(&mut self) -> Result<BlockId, PlanError> {
        let index = u32::try_from(self.blocks.len()).map_err(|_| PlanError::TooManyItems)?;
        let id = BlockId {
            function: self.function,
            index,
        };
        self.blocks.push(Block {
            id,
            instructions: Vec::new(),
            terminator: None,
        });
        Ok(id)
    }

    pub fn set_entry(&mut self, block: BlockId) -> Result<(), PlanError> {
        self.check_block(block)?;
        self.entry = Some(block);
        Ok(())
    }

    pub fn create_local(&mut self, value_type: ValueType) -> Result<LocalId, PlanError> {
        let index = u32::try_from(self.locals.len()).map_err(|_| PlanError::TooManyItems)?;
        let id = LocalId {
            function: self.function,
            index,
        };
        self.locals.push(LocalFact { id, value_type });
        Ok(id)
    }

    pub fn i64_add(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64Add(left, right),
            left,
            right,
        )
    }

    pub fn i64_sub(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64Sub(left, right),
            left,
            right,
        )
    }

    pub fn i64_mul(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64Mul(left, right),
            left,
            right,
        )
    }

    pub fn i64_div(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64Div(left, right),
            left,
            right,
        )
    }

    pub fn i64_bit_and(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64BitAnd(left, right),
            left,
            right,
        )
    }

    pub fn i64_bit_or(
        &mut self,
        block: BlockId,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.append_binary(
            block,
            ValueType::I64,
            Operation::I64BitOr(left, right),
            left,
            right,
        )
    }
}
