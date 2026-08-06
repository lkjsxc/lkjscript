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
        self.emit_place_local(
            if glue == DropGlueIdentity::ByteVector {
                Op::ByteVectorDropPlace
            } else {
                Op::BytesDropPlace
            },
            place,
            value,
        )?;
        Ok(())
    }

    pub(in crate::codegen) fn emit_implicit_resource_drop(
        &mut self,
        value: ValueId,
        _kind: lkjscript_core::ResourceKind,
    ) -> Result<()> {
        self.load(value)?;
        self.proto.emit(Op::ResourceDrop);
        Ok(())
    }
}
