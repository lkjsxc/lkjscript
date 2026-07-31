use super::*;

impl FunctionBuilder {
    pub fn call(
        &mut self,
        block: BlockId,
        callee: FunctionId,
        arguments: Vec<ValueId>,
    ) -> Result<ValueId, PlanError> {
        if callee.plan != self.function.plan {
            return Err(PlanError::ForeignId("callee"));
        }
        for argument in &arguments {
            self.check_value(*argument)?;
        }
        let signature = self
            .signatures
            .iter()
            .find(|(id, _)| *id == callee)
            .map(|(_, signature)| signature)
            .ok_or(PlanError::UnknownFunction)?;
        self.append(
            block,
            signature.result(),
            Operation::Call(callee, arguments),
            None,
        )
    }

    pub fn heap_call(
        &mut self,
        block: BlockId,
        descriptor: HeapCallDescriptor,
        arguments: Vec<ValueId>,
    ) -> Result<ValueId, PlanError> {
        if arguments.len() != descriptor.input_types().len()
            || arguments
                .iter()
                .zip(descriptor.input_types())
                .any(|(argument, expected)| {
                    self.values
                        .get(argument.index as usize)
                        .filter(|fact| fact.id == *argument)
                        .is_none_or(|fact| fact.value_type != *expected)
                })
        {
            return Err(PlanError::InvalidHeapCall);
        }
        for argument in &arguments {
            self.check_value(*argument)?;
        }
        self.append(
            block,
            descriptor.result_type(),
            Operation::HeapCall(Box::new(descriptor), arguments),
            None,
        )
    }

    pub fn structural_call(
        &mut self,
        block: BlockId,
        descriptor: StructuralCallDescriptor,
        arguments: Vec<ValueId>,
    ) -> Result<ValueId, PlanError> {
        if arguments.len() != descriptor.signature().parameters().len() {
            return Err(PlanError::InvalidStructuralCall);
        }
        for argument in &arguments {
            self.check_value(*argument)?;
        }
        self.append(
            block,
            descriptor.signature().result(),
            Operation::StructuralCall(Box::new(descriptor), arguments),
            None,
        )
    }

    pub fn runtime_call(
        &mut self,
        block: BlockId,
        slot: RuntimeCallSlot,
        arguments: Vec<ValueId>,
    ) -> Result<ValueId, PlanError> {
        for argument in &arguments {
            self.check_value(*argument)?;
        }
        let signature = slot
            .plan_signature()
            .ok_or(PlanError::EncoderOwnedRuntimeCall)?;
        self.append(
            block,
            signature.result(),
            Operation::RuntimeCall(slot, arguments),
            None,
        )
    }

    pub fn branch(&mut self, block: BlockId, target: BlockId) -> Result<(), PlanError> {
        self.check_block(target)?;
        self.terminate(block, Terminator::Branch(target))
    }

    pub fn branch_if(
        &mut self,
        block: BlockId,
        condition: ValueId,
        when_true: BlockId,
        when_false: BlockId,
    ) -> Result<(), PlanError> {
        self.check_value(condition)?;
        self.check_block(when_true)?;
        self.check_block(when_false)?;
        self.terminate(
            block,
            Terminator::BranchIf {
                condition,
                when_true,
                when_false,
            },
        )
    }

    pub fn return_value(&mut self, block: BlockId, value: ValueId) -> Result<(), PlanError> {
        self.check_value(value)?;
        self.terminate(block, Terminator::Return(value))
    }

    pub fn trap(&mut self, block: BlockId, trap: TrapCode) -> Result<(), PlanError> {
        self.terminate(block, Terminator::Trap { trap, site: None })
    }

    pub fn trap_at(&mut self, block: BlockId, trap: TrapCode, site: u32) -> Result<(), PlanError> {
        self.terminate(
            block,
            Terminator::Trap {
                trap,
                site: Some(site),
            },
        )
    }

    pub fn exit(&mut self, block: BlockId, code: ValueId) -> Result<(), PlanError> {
        self.check_value(code)?;
        self.terminate(block, Terminator::Exit(code))
    }

    pub fn outcome(&mut self, block: BlockId, outcome: RuntimeOutcome) -> Result<(), PlanError> {
        self.terminate(block, Terminator::Outcome(outcome))
    }

    #[must_use]
    pub fn finish(self) -> FunctionPlan {
        FunctionPlan {
            id: self.function,
            signature: self.signature,
            source_function: self.source_function,
            blocks: self.blocks,
            entry: self.entry,
            values: self.values,
            locals: self.locals,
        }
    }

    pub(in crate::plan) fn append_binary(
        &mut self,
        block: BlockId,
        output_type: ValueType,
        operation: Operation,
        left: ValueId,
        right: ValueId,
    ) -> Result<ValueId, PlanError> {
        self.check_value(left)?;
        self.check_value(right)?;
        self.append(block, output_type, operation, None)
    }
}
