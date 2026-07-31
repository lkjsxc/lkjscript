use super::*;

impl FunctionBuilder {
    pub fn set_instruction_failure_cleanup(
        &mut self,
        value: ValueId,
        cleanup: Vec<FailureCleanupCall>,
    ) -> Result<(), PlanError> {
        self.check_value(value)?;
        let block_id = match self
            .values
            .get(value.index as usize)
            .map(|fact| &fact.definition)
        {
            Some(ValueDefinition::Instruction(block)) => *block,
            _ => return Err(PlanError::UnknownValue),
        };
        let instruction = self
            .block_mut(block_id)?
            .instructions
            .iter_mut()
            .find(|instruction| instruction.output == value)
            .ok_or(PlanError::UnknownValue)?;
        instruction.failure_cleanup = cleanup;
        Ok(())
    }

    pub fn set_instruction_unentered_cleanup(
        &mut self,
        value: ValueId,
        cleanup: Vec<FailureCleanupCall>,
    ) -> Result<(), PlanError> {
        self.check_value(value)?;
        let block_id = match self
            .values
            .get(value.index as usize)
            .map(|fact| &fact.definition)
        {
            Some(ValueDefinition::Instruction(block)) => *block,
            _ => return Err(PlanError::UnknownValue),
        };
        let instruction = self
            .block_mut(block_id)?
            .instructions
            .iter_mut()
            .find(|instruction| instruction.output == value)
            .ok_or(PlanError::UnknownValue)?;
        instruction.unentered_cleanup = cleanup;
        Ok(())
    }

    pub fn set_instruction_source(
        &mut self,
        value: ValueId,
        source: SourceOrigin,
    ) -> Result<(), PlanError> {
        self.check_value(value)?;
        let block_id = match self
            .values
            .get(value.index as usize)
            .map(|fact| &fact.definition)
        {
            Some(ValueDefinition::Instruction(block)) => *block,
            _ => return Err(PlanError::UnknownValue),
        };
        let block = self.block_mut(block_id)?;
        let instruction = block
            .instructions
            .iter_mut()
            .find(|instruction| instruction.output == value)
            .ok_or(PlanError::UnknownValue)?;
        instruction.source = Some(source);
        Ok(())
    }

    pub(in crate::plan) fn append(
        &mut self,
        block: BlockId,
        output_type: ValueType,
        operation: Operation,
        source: Option<SourceOrigin>,
    ) -> Result<ValueId, PlanError> {
        self.check_block(block)?;
        if self.block(block)?.terminator.is_some() {
            return Err(PlanError::BlockAlreadyTerminated);
        }
        let index = u32::try_from(self.values.len()).map_err(|_| PlanError::TooManyItems)?;
        let output = ValueId {
            function: self.function,
            index,
        };
        self.values.push(ValueFact {
            id: output,
            value_type: output_type,
            definition: ValueDefinition::Instruction(block),
        });
        self.block_mut(block)?.instructions.push(Instruction {
            output,
            output_type,
            operation,
            failure_cleanup: Vec::new(),
            unentered_cleanup: Vec::new(),
            source,
        });
        Ok(output)
    }

    pub(super) fn terminate(
        &mut self,
        block: BlockId,
        terminator: Terminator,
    ) -> Result<(), PlanError> {
        let block = self.block_mut(block)?;
        if block.terminator.is_some() {
            return Err(PlanError::BlockAlreadyTerminated);
        }
        block.terminator = Some(terminator);
        Ok(())
    }

    pub(super) fn check_block(&self, block: BlockId) -> Result<(), PlanError> {
        self.block(block).map(|_| ())
    }

    pub(super) fn block(&self, block: BlockId) -> Result<&Block, PlanError> {
        if block.function != self.function {
            return Err(PlanError::ForeignId("block ID"));
        }
        self.blocks
            .get(block.index as usize)
            .filter(|item| item.id == block)
            .ok_or(PlanError::UnknownBlock)
    }

    pub(super) fn block_mut(&mut self, block: BlockId) -> Result<&mut Block, PlanError> {
        if block.function != self.function {
            return Err(PlanError::ForeignId("block ID"));
        }
        self.blocks
            .get_mut(block.index as usize)
            .filter(|item| item.id == block)
            .ok_or(PlanError::UnknownBlock)
    }

    pub(in crate::plan) fn check_value(&self, value: ValueId) -> Result<(), PlanError> {
        if value.function != self.function {
            return Err(PlanError::ForeignId("value ID"));
        }
        self.values
            .get(value.index as usize)
            .filter(|fact| fact.id == value)
            .map(|_| ())
            .ok_or(PlanError::UnknownValue)
    }

    pub(in crate::plan) fn local_type(&self, local: LocalId) -> Result<ValueType, PlanError> {
        if local.function != self.function {
            return Err(PlanError::ForeignId("local ID"));
        }
        self.locals
            .get(local.index as usize)
            .filter(|fact| fact.id == local)
            .map(|fact| fact.value_type)
            .ok_or(PlanError::UnknownLocal)
    }
}
