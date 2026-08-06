use super::*;

pub(in crate::codegen) struct Emitter<'a> {
    pub(in crate::codegen) chunk: &'a mut Chunk,
    pub(in crate::codegen) globals: &'a HashMap<FunctionId, u16>,
    pub(in crate::codegen) function: &'a Function,
    pub(in crate::codegen) slots: HashMap<ValueId, usize>,
    pub(in crate::codegen) code_base: u64,
    pub(in crate::codegen) proto: FunctionProto,
    pub(in crate::codegen) block_offsets: HashMap<BlockId, u64>,
    pub(in crate::codegen) patches: Vec<(usize, BlockId)>,
    pub(in crate::codegen) block_links: Vec<BytecodeBlockLink>,
    pub(in crate::codegen) instruction_links: Vec<BytecodeInstructionLink>,
    pub(in crate::codegen) failure_cleanup_map: Vec<BytecodeFailureCleanupId>,
    pub(in crate::codegen) failure_cleanups: BytecodeFailureCleanupInterner,
    pub(in crate::codegen) failure_index: FailureCodegenIndex<'a>,
}

impl Emitter<'_> {
    pub(in crate::codegen) fn record_failure_range(
        &mut self,
        start: u64,
        end: u64,
        plan: Option<SsaFailureCleanupRoots>,
        unentered_plan: Option<BytecodeFailureCleanupId>,
    ) -> Result<()> {
        if start >= end {
            return Err(Error::msg("failure cleanup has an empty bytecode range"));
        }
        let map_root = |root: Option<lkjscript_ir::FailureCleanupId>| {
            root.map(|root| {
                self.failure_cleanup_map
                    .get(root.index().unwrap_or(usize::MAX))
                    .copied()
                    .ok_or_else(|| Error::msg("failure-cleanup root is out of range"))
            })
            .transpose()
        };
        let plan = plan
            .map(|roots| {
                Ok(BytecodeFailureCleanupRoots {
                    loans: map_root(roots.loans)?,
                    unplaced: map_root(roots.unplaced)?,
                    places: map_root(roots.places)?,
                })
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
    ) -> Result<Option<BytecodeFailureCleanupId>> {
        let InstructionKind::Call { arguments, .. } = &instruction.kind else {
            return Ok(None);
        };
        let mut root = None;
        // Unentered cleanup runs moved arguments in reverse call order. Intern
        // from the opposite direction so links are always backward-only.
        for value in arguments {
            if !self.failure_index.moved(*value) {
                continue;
            }
            let action = compile_unentered_cleanup_action(
                *value,
                &self.slots,
                self.chunk,
                &self.failure_index,
            )?;
            root = Some(self.failure_cleanups.intern(action, root)?);
        }
        Ok(root)
    }
}
