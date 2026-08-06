impl Emitter<'_> {
    pub(in crate::codegen) fn offset(&self) -> Result<u16> {
        let local = u16::try_from(self.proto.len())
            .map_err(|_| Error::msg("bytecode function offset exceeds u16"))?;
        self.code_base
            .checked_add(local)
            .ok_or_else(|| Error::msg("bytecode function offset exceeds u16"))
    }

    pub(in crate::codegen) fn slot(&self, value: ValueId) -> Result<usize> {
        self.slots.get(&value).copied().ok_or_else(|| {
            Error::msg(format!(
                "SSA value {} has no bytecode local slot",
                value.raw()
            ))
        })
    }

    pub(in crate::codegen) fn emit_index(&mut self, op: Op, index: usize) -> Result<()> {
        let index = u64::try_from(index)
            .map_err(|_| Error::msg("bytecode index exceeds u64"))?;
        self.proto.emit_op_u64(op, index);
        Ok(())
    }

    pub(in crate::codegen) fn emit_place_local(
        &mut self,
        op: Op,
        place: lkjscript_ir::PlaceId,
        value: ValueId,
    ) -> Result<()> {
        let place = u64::from(place.raw());
        let local = u64::try_from(self.slot(value)?)
            .map_err(|_| Error::msg("bytecode local index exceeds u64"))?;
        self.proto.emit_op_u64_pair(op, place, local);
        Ok(())
    }
}
