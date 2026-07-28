impl Emitter<'_> {
    pub(in crate::codegen) fn emit_unique_drop(
        &mut self,
        place: lkjscript_ir::PlaceId,
        value: ValueId,
        glue: DropGlueIdentity,
    ) -> Result<()> {
        if glue == DropGlueIdentity::Bytes && self.static_bytes_value(value)? {
            self.proto.emit(Op::Unit);
            return Ok(());
        }
        let operand = self.place_slot(place, value)?;
        self.proto.emit_op_u16(
            if glue == DropGlueIdentity::ByteVector {
                Op::ByteVectorDropPlace
            } else {
                Op::BytesDropPlace
            },
            operand,
        );
        Ok(())
    }

    pub(in crate::codegen) fn emit_implicit_resource_drop(
        &mut self,
        value: ValueId,
        kind: lkjscript_core::ResourceKind,
    ) -> Result<()> {
        self.load(value)?;
        self.proto.emit(match kind {
            lkjscript_core::ResourceKind::SqliteConnection => Op::SysSqliteClose,
            lkjscript_core::ResourceKind::SqliteStatement => Op::SysSqliteFinalize,
            _ => Op::SysClose,
        });
        self.proto.emit(Op::Pop);
        self.proto.emit(Op::Unit);
        Ok(())
    }
}
