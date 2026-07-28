use crate::codegen::*;

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

    pub(in crate::codegen) fn load(&mut self, value: ValueId) -> Result<()> {
        let slot = self.slot(value)?;
        self.proto.emit_op_u8(
            match self.value_type(value)? {
                SsaType::ByteVector => Op::TakeUniqueLocal,
                SsaType::ByteSlice | SsaType::ByteSliceMut => Op::LoadViewLocal,
                _ => Op::LoadLocal,
            },
            slot,
        );
        Ok(())
    }

    pub(in crate::codegen) fn store_result(&mut self, value: ValueId) -> Result<()> {
        let slot = self.slot(value)?;
        match self.value_type(value)? {
            SsaType::ByteVector => self.proto.emit_op_u8(Op::StoreUniqueLocal, slot),
            SsaType::ByteSlice | SsaType::ByteSliceMut => {
                self.proto.emit_op_u8(Op::StoreViewLocal, slot);
            }
            _ => {
                self.proto.emit_op_u8(Op::StoreLocal, slot);
                self.proto.emit(Op::Pop);
            }
        }
        Ok(())
    }

    fn value_type(&self, value: ValueId) -> Result<&SsaType> {
        self.function
            .blocks
            .iter()
            .find_map(|block| {
                block
                    .parameters
                    .iter()
                    .find(|parameter| parameter.id == value)
                    .map(|parameter| &parameter.ty)
                    .or_else(|| {
                        block
                            .instructions
                            .iter()
                            .find(|instruction| instruction.id == value)
                            .map(|instruction| &instruction.ty)
                    })
            })
            .ok_or_else(|| Error::msg("SSA bytecode lowering lost a value type"))
    }

    fn byte_vector_place(&self, place: lkjscript_ir::PlaceId) -> Result<bool> {
        self.function
            .places
            .iter()
            .find(|metadata| metadata.id == place)
            .map(|metadata| metadata.ty == SsaType::ByteVector)
            .ok_or_else(|| Error::msg("SSA bytecode lowering lost a PlaceId"))
    }

    fn place_slot(&self, place: lkjscript_ir::PlaceId, value: ValueId) -> Result<u16> {
        let place = u8::try_from(place.raw())
            .map_err(|_| Error::msg("byte-vector PlaceId exceeds bytecode u8"))?;
        Ok((u16::from(place) << 8) | u16::from(self.slot(value)?))
    }
}

include!("emit_values/instruction.rs");
