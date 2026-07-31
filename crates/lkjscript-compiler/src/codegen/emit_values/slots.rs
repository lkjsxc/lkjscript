impl Emitter<'_> {
    pub(in crate::codegen) fn offset(&self) -> Result<u16> {
        let local = u16::try_from(self.proto.len())
            .map_err(|_| Error::msg("bytecode function offset exceeds u16"))?;
        self.code_base
            .checked_add(local)
            .ok_or_else(|| Error::msg("bytecode function offset exceeds u16"))
    }

    pub(in crate::codegen) fn slot(&self, value: ValueId) -> Result<u8> {
        self.slots.get(&value).copied().ok_or_else(|| {
            Error::msg(format!(
                "SSA value {} has no bytecode local slot",
                value.raw()
            ))
        })
    }
}
