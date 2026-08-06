impl Emitter<'_> {
    fn emit_call_instruction(&mut self, instruction: &Instruction) -> Result<()> {
        let InstructionKind::Call {
            target,
            arguments,
            consuming,
            instantiation,
            ..
        } = &instruction.kind
        else {
            return Err(Error::msg("non-call instruction reached call emitter"));
        };
        for (argument, consuming) in arguments.iter().zip(consuming) {
            if *consuming {
                self.load(*argument)?;
            } else {
                self.load_observed_structural(*argument)?;
            }
        }
        match target {
            CallTarget::Direct(function) => {
                let global = self.global(*function)?;
                self.proto.emit_op_u16(Op::LoadGlobal, global);
            }
            CallTarget::Indirect(value) => self.load(*value)?,
        }
        let arity = arguments.len();
        if let Some(instantiation) = instantiation
            .as_ref()
            .filter(|item| !item.memory_witnesses.is_empty())
        {
            self.record_call_witness(target, instantiation)?;
        }
        self.emit_index(Op::Call, arity)?;
        Ok(())
    }

    fn record_call_witness(
        &mut self,
        target: &CallTarget,
        instantiation: &lkjscript_ir::GenericInstantiation,
    ) -> Result<()> {
        let CallTarget::Direct(function) = target else {
            return Err(Error::msg(
                "indirect generic call cannot carry hidden memory witnesses",
            ));
        };
        let global = self.global(*function)?;
        let callee = self
            .chunk
            .global_prototypes
            .get(usize::from(global))
            .copied()
            .flatten()
            .ok_or_else(|| Error::msg("witnessed call has no exact prototype"))?;
        let bindings = instantiation
            .memory_witnesses
            .iter()
            .map(|binding| self.call_witness_binding(instantiation, binding))
            .collect::<Result<Vec<_>>>()?;
        self.proto.call_witnesses.push(lkjscript_core::CallWitnessSite {
            offset: u32::try_from(self.offset()?)
                .map_err(|_| Error::msg("call witness offset exceeds u32"))?,
            callee,
            bindings,
        });
        Ok(())
    }

    fn call_witness_binding(
        &self,
        instantiation: &lkjscript_ir::GenericInstantiation,
        binding: &lkjscript_ir::MemoryWitnessBinding,
    ) -> Result<lkjscript_core::MemoryWitnessBinding> {
        let parameter = instantiation
            .substitutions
            .iter()
            .position(|item| item.parameter == binding.parameter)
            .and_then(|index| u16::try_from(index).ok())
            .ok_or_else(|| Error::msg("call witness parameter is stale"))?;
        let witness = self
            .chunk
            .memory_witnesses
            .binary_search_by_key(&binding.witness.bytes(), |item| item.id.bytes())
            .ok()
            .and_then(|index| u16::try_from(index).ok())
            .ok_or_else(|| Error::msg("call witness is not installed"))?;
        Ok(lkjscript_core::MemoryWitnessBinding { parameter, witness })
    }
}
