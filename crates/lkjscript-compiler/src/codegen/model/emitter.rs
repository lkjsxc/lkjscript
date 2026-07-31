use super::*;

pub(in crate::codegen) struct Emitter<'a> {
    pub(in crate::codegen) chunk: &'a mut Chunk,
    pub(in crate::codegen) globals: &'a HashMap<FunctionId, u16>,
    pub(in crate::codegen) function: &'a Function,
    pub(in crate::codegen) slots: HashMap<ValueId, u8>,
    pub(in crate::codegen) code_base: u16,
    pub(in crate::codegen) proto: FunctionProto,
    pub(in crate::codegen) block_offsets: HashMap<BlockId, u16>,
    pub(in crate::codegen) patches: Vec<(usize, BlockId)>,
    pub(in crate::codegen) block_links: Vec<BytecodeBlockLink>,
    pub(in crate::codegen) instruction_links: Vec<BytecodeInstructionLink>,
    pub(in crate::codegen) failure_cleanup_map: Vec<u16>,
}

impl Emitter<'_> {
    pub(in crate::codegen) fn record_failure_range(
        &mut self,
        start: u16,
        end: u16,
        plan: Option<u32>,
        unentered_plan: Option<u16>,
    ) -> Result<()> {
        if start >= end {
            return Err(Error::msg("failure cleanup has an empty bytecode range"));
        }
        let plan = plan
            .map(|plan| {
                self.failure_cleanup_map
                    .get(usize::try_from(plan).unwrap_or(usize::MAX))
                    .copied()
                    .ok_or_else(|| Error::msg("failure-cleanup plan index is out of range"))
            })
            .transpose()?;
        self.proto.failure_cleanup_ranges.push(FailureCleanupRange {
            start,
            end,
            plan,
            unentered_plan,
        });
        Ok(())
    }

    pub(in crate::codegen) fn intern_unentered_cleanup(
        &mut self,
        instruction: &Instruction,
    ) -> Result<Option<u16>> {
        let actions =
            compile_unentered_cleanup(self.function, instruction, &self.slots, self.chunk)?;
        if actions.is_empty() {
            return Ok(None);
        }
        if let Some(index) = self
            .proto
            .failure_cleanups
            .iter()
            .position(|plan| plan.actions == actions)
        {
            return u16::try_from(index)
                .map(Some)
                .map_err(|_| Error::msg("failure-cleanup plan index exceeds u16"));
        }
        let index = u16::try_from(self.proto.failure_cleanups.len())
            .map_err(|_| Error::msg("failure-cleanup plan index exceeds u16"))?;
        self.proto
            .failure_cleanups
            .push(BytecodeFailureCleanupPlan { actions });
        Ok(Some(index))
    }
}
