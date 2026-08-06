impl Emitter<'_> {
    pub(in crate::codegen) fn emit_instruction(
        &mut self,
        instruction: &Instruction,
        store_result: bool,
    ) -> Result<()> {
        if self.emit_structural_instruction(instruction)?
            || self.emit_aggregate_instruction(instruction)?
        {
            if store_result {
                self.store_result(instruction.id)?;
            }
            return Ok(());
        }
        match &instruction.kind {
            InstructionKind::Constant(constant) => self.emit_constant(constant)?,
            InstructionKind::Copy(value) => self.load(*value)?,
            InstructionKind::MemoryWitnessIndependentOwner { .. }
            | InstructionKind::MemoryWitnessCompare { .. }
            | InstructionKind::MemoryWitnessDispose { .. } => {
                self.emit_witness_instruction(&instruction.kind)?;
            }
            InstructionKind::PlaceInit { place, value }
                if instruction.ty == SsaType::Unit
                    && self.value_type(*value)? == &SsaType::ByteVector =>
            {
                self.emit_place_local(Op::ByteVectorPlaceInit, *place, *value)?;
            }
            InstructionKind::PlaceInit { place, value }
                if instruction.ty == SsaType::Unit
                    && self.value_type(*value)? == &SsaType::Bytes
                    && !self.static_bytes_value(*value)? =>
            {
                self.emit_place_local(Op::BytesPlaceInit, *place, *value)?;
            }
            InstructionKind::PlaceEnd { place }
                if self.bytes_place(*place)? && !self.bytes_place_is_static(*place)? =>
            {
                let place = usize::try_from(place.raw())
                    .map_err(|_| Error::msg("bytes PlaceId exceeds host usize"))?;
                self.emit_index(Op::BytesPlaceEnd, place)?;
            }
            InstructionKind::PlaceEnd { place } if self.byte_vector_place(*place)? => {
                let place = usize::try_from(place.raw())
                    .map_err(|_| Error::msg("byte-vector PlaceId exceeds host usize"))?;
                self.emit_index(Op::ByteVectorPlaceEnd, place)?;
            }
            InstructionKind::EndBorrow { value, .. } => {
                let slot = self.slot(*value)?;
                self.emit_index(Op::EndBorrowLocal, slot)?;
            }
            InstructionKind::Drop {
                place, value, glue, ..
            } if matches!(glue, DropGlueIdentity::ByteVector | DropGlueIdentity::Bytes) => {
                self.emit_unique_drop(*place, *value, *glue)?;
            }
            InstructionKind::Drop {
                value,
                glue: DropGlueIdentity::Resource(kind),
                kind: lkjscript_ir::DropEventKind::ImplicitCleanup,
                ..
            } => self.emit_implicit_resource_drop(*value, *kind)?,
            InstructionKind::Move { place, value }
                if self.value_type(*value)? == &SsaType::Bytes
                    && !self.static_bytes_value(*value)? =>
            {
                self.emit_place_local(Op::BytesMove, *place, *value)?;
            }
            InstructionKind::Move { place, value }
                if self.value_type(*value)? == &SsaType::ByteVector =>
            {
                self.emit_place_local(Op::ByteVectorMove, *place, *value)?;
            }
            InstructionKind::Borrow { kind, value, .. } => {
                let slot = self.slot(*value)?;
                let opcode = if self.value_type(*value)? == &SsaType::Bytes {
                    Op::BytesBorrow
                } else {
                    match kind {
                        lkjscript_ir::BorrowKind::Shared => Op::ByteVectorBorrow,
                        lkjscript_ir::BorrowKind::Mutable => Op::ByteVectorBorrowMut,
                    }
                };
                self.emit_index(opcode, slot)?;
            }
            InstructionKind::Move { value, .. } => self.load(*value)?,
            InstructionKind::PlaceInit { .. }
            | InstructionKind::PlaceEnd { .. }
            | InstructionKind::Drop { .. } => self.proto.try_emit(Op::Unit)?,
            InstructionKind::FunctionRef(function) => {
                let global = self.global(*function)?;
                self.proto.try_emit_op_u16(Op::LoadGlobal, global)?;
            }
            InstructionKind::Runtime {
                operation,
                arguments,
                ..
            } => {
                for argument in arguments {
                    self.load_observed_structural(*argument)?;
                }
                let opcode = runtime_opcode(*operation);
                if *operation == RuntimeOp::Car {
                    let representation =
                        structural_owner_representation(self.chunk, &instruction.ty)
                            .map_or(u16::MAX, |representation| representation.raw());
                    self.proto.try_emit_op_u16(opcode, representation)?;
                } else {
                    self.proto.try_emit(opcode)?;
                }
            }
            InstructionKind::F64FromI64Exact { value }
            | InstructionKind::F64FromI64Rounded { value }
            | InstructionKind::I64FromF64Exact { value }
            | InstructionKind::I64FromF64Trunc { value } => self.emit_numeric(instruction, *value)?,
            InstructionKind::Call { .. } => self.emit_call_instruction(instruction)?,
            _ => return Err(Error::msg("structural instruction dispatch mismatch")),
        }
        if store_result {
            self.store_result(instruction.id)?;
        }
        Ok(())
    }
}
