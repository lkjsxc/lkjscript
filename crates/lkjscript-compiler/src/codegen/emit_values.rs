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
                SsaType::Bytes if self.borrowed_bytes_value(value)? => Op::LoadViewLocal,
                SsaType::Bytes if !self.static_bytes_value(value)? => Op::TakeUniqueLocal,
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
            SsaType::Bytes if self.borrowed_bytes_value(value)? => {
                self.proto.emit_op_u8(Op::StoreViewLocal, slot);
            }
            SsaType::Bytes if !self.static_bytes_value(value)? => {
                self.proto.emit_op_u8(Op::StoreUniqueLocal, slot);
            }
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

    fn borrowed_bytes_value(&self, value: ValueId) -> Result<bool> {
        Ok(self.function.blocks.iter().any(|block| {
            block.instructions.iter().any(|instruction| {
                instruction.id == value
                    && matches!(instruction.kind, InstructionKind::Borrow { .. })
                    && instruction.ty == SsaType::Bytes
            })
        }))
    }

    fn static_bytes_value(&self, value: ValueId) -> Result<bool> {
        let instruction = self.function.blocks.iter().find_map(|block| {
            block
                .instructions
                .iter()
                .find(|instruction| instruction.id == value)
        });
        Ok(matches!(
            instruction.map(|instruction| &instruction.kind),
            Some(InstructionKind::Constant(Constant::StaticBytes(_)))
        ))
    }

    fn bytes_place_is_static(&self, place: lkjscript_ir::PlaceId) -> Result<bool> {
        for instruction in self
            .function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
        {
            if let InstructionKind::PlaceInit {
                place: candidate,
                value,
            } = instruction.kind
            {
                if candidate == place {
                    return self.static_bytes_value(value);
                }
            }
        }
        Ok(false)
    }

    fn bytes_place(&self, place: lkjscript_ir::PlaceId) -> Result<bool> {
        self.function
            .places
            .iter()
            .find(|metadata| metadata.id == place)
            .map(|metadata| metadata.ty == SsaType::Bytes)
            .ok_or_else(|| Error::msg("SSA bytecode lowering lost a PlaceId"))
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

impl Emitter<'_> {
    fn emit_numeric(&mut self, instruction: &Instruction, value: ValueId) -> Result<()> {
        self.load(value)?;
        self.proto.emit(match &instruction.kind {
            InstructionKind::F64FromI64Exact { .. } => Op::F64FromI64Exact,
            InstructionKind::F64FromI64Rounded { .. } => Op::F64FromI64Rounded,
            InstructionKind::I64FromF64Exact { .. } => Op::I64FromF64Exact,
            InstructionKind::I64FromF64Trunc { .. } => Op::I64FromF64Trunc,
            _ => return Err(Error::msg("numeric opcode lowering mismatch")),
        });
        Ok(())
    }
}

include!("emit_values/instruction.rs");
