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
            InstructionKind::PlaceInit { place, value }
                if instruction.ty == SsaType::Unit
                    && self.value_type(*value)? == &SsaType::ByteVector =>
            {
                let operand = self.place_slot(*place, *value)?;
                self.proto.emit_op_u16(Op::ByteVectorPlaceInit, operand);
            }
            InstructionKind::PlaceInit { place, value }
                if instruction.ty == SsaType::Unit
                    && self.value_type(*value)? == &SsaType::Bytes
                    && !self.static_bytes_value(*value)? =>
            {
                let operand = self.place_slot(*place, *value)?;
                self.proto.emit_op_u16(Op::BytesPlaceInit, operand);
            }
            InstructionKind::PlaceEnd { place }
                if self.bytes_place(*place)? && !self.bytes_place_is_static(*place)? =>
            {
                let place = u8::try_from(place.raw())
                    .map_err(|_| Error::msg("bytes PlaceId exceeds bytecode u8"))?;
                self.proto.emit_op_u8(Op::BytesPlaceEnd, place);
            }
            InstructionKind::PlaceEnd { place } if self.byte_vector_place(*place)? => {
                let place = u8::try_from(place.raw())
                    .map_err(|_| Error::msg("byte-vector PlaceId exceeds bytecode u8"))?;
                self.proto.emit_op_u8(Op::ByteVectorPlaceEnd, place);
            }
            InstructionKind::EndBorrow { value, .. } => {
                let slot = self.slot(*value)?;
                self.proto.emit_op_u8(Op::EndBorrowLocal, slot);
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
                let operand = self.place_slot(*place, *value)?;
                self.proto.emit_op_u16(Op::BytesMove, operand);
            }
            InstructionKind::Move { place, value }
                if self.value_type(*value)? == &SsaType::ByteVector =>
            {
                let operand = self.place_slot(*place, *value)?;
                self.proto.emit_op_u16(Op::ByteVectorMove, operand);
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
                self.proto.emit_op_u8(opcode, slot);
            }
            InstructionKind::Move { value, .. } => self.load(*value)?,
            InstructionKind::PlaceInit { .. }
            | InstructionKind::PlaceEnd { .. }
            | InstructionKind::Drop { .. } => self.proto.emit(Op::Unit),
            InstructionKind::FunctionRef(function) => {
                let global = self.global(*function)?;
                self.proto.emit_op_u16(Op::LoadGlobal, global);
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
                    self.proto.emit_op_u16(opcode, representation);
                } else {
                    self.proto.emit(opcode);
                }
            }
            InstructionKind::F64FromI64Exact { value }
            | InstructionKind::F64FromI64Rounded { value }
            | InstructionKind::I64FromF64Exact { value }
            | InstructionKind::I64FromF64Trunc { value } => self.emit_numeric(instruction, *value)?,
            InstructionKind::Call {
                target,
                arguments,
                consuming,
                instantiation,
                ..
            } => {
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
                let arity = u8::try_from(arguments.len())
                    .map_err(|_| Error::msg("SSA call arity exceeds bytecode u8"))?;
                if let Some(instantiation) = instantiation
                    .as_ref()
                    .filter(|item| !item.memory_witnesses.is_empty())
                {
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
                        .map(|binding| {
                            let parameter = instantiation
                                .substitutions
                                .iter()
                                .position(|item| item.parameter == binding.parameter)
                                .and_then(|index| u16::try_from(index).ok())
                                .ok_or_else(|| Error::msg("call witness parameter is stale"))?;
                            let witness = self
                                .chunk
                                .memory_witnesses
                                .binary_search_by_key(&binding.witness.bytes(), |item| {
                                    item.id.bytes()
                                })
                                .ok()
                                .and_then(|index| u16::try_from(index).ok())
                                .ok_or_else(|| Error::msg("call witness is not installed"))?;
                            Ok(lkjscript_core::MemoryWitnessBinding { parameter, witness })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    self.proto.call_witnesses.push(lkjscript_core::CallWitnessSite {
                        offset: u32::from(self.code_base)
                            .checked_add(
                                u32::try_from(self.proto.len())
                                    .map_err(|_| Error::msg("call witness offset exceeds u32"))?,
                            )
                            .ok_or_else(|| Error::msg("call witness offset overflow"))?,
                        callee,
                        bindings,
                    });
                }
                self.proto.emit_op_u8(Op::Call, arity);
            }
            _ => return Err(Error::msg("structural instruction dispatch mismatch")),
        }
        if store_result {
            self.store_result(instruction.id)?;
        }
        Ok(())
    }
}
