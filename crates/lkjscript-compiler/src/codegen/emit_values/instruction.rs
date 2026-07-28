impl Emitter<'_> {
    pub(in crate::codegen) fn emit_instruction(
        &mut self,
        instruction: &Instruction,
        store_result: bool,
    ) -> Result<()> {
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
                place,
                value,
                glue: DropGlueIdentity::ByteVector,
                ..
            } => {
                let operand = self.place_slot(*place, *value)?;
                self.proto.emit_op_u16(Op::ByteVectorDropPlace, operand);
            }
            InstructionKind::Move { place, value }
                if self.value_type(*value)? == &SsaType::ByteVector =>
            {
                let operand = self.place_slot(*place, *value)?;
                self.proto.emit_op_u16(Op::ByteVectorMove, operand);
            }
            InstructionKind::Borrow { kind, value, .. } => {
                let slot = self.slot(*value)?;
                self.proto.emit_op_u8(
                    match kind {
                        lkjscript_ir::BorrowKind::Shared => Op::ByteVectorBorrow,
                        lkjscript_ir::BorrowKind::Mutable => Op::ByteVectorBorrowMut,
                    },
                    slot,
                );
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
                    self.load(*argument)?;
                }
                self.proto.emit(runtime_opcode(*operation));
            }
            InstructionKind::F64FromI64Exact { value }
            | InstructionKind::F64FromI64Rounded { value }
            | InstructionKind::I64FromF64Exact { value }
            | InstructionKind::I64FromF64Trunc { value } => {
                self.load(*value)?;
                self.proto.emit(match &instruction.kind {
                    InstructionKind::F64FromI64Exact { .. } => Op::F64FromI64Exact,
                    InstructionKind::F64FromI64Rounded { .. } => Op::F64FromI64Rounded,
                    InstructionKind::I64FromF64Exact { .. } => Op::I64FromF64Exact,
                    InstructionKind::I64FromF64Trunc { .. } => Op::I64FromF64Trunc,
                    _ => return Err(Error::msg("numeric opcode lowering mismatch")),
                });
            }
            InstructionKind::Call {
                target, arguments, ..
            } => {
                for argument in arguments {
                    self.load(*argument)?;
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
                self.proto.emit_op_u8(Op::Call, arity);
            }
            InstructionKind::ProductValue { product, fields } => {
                for field in fields {
                    self.load(*field)?;
                }
                self.proto.emit_op_u16(Op::MakeProduct, product.raw());
            }
            InstructionKind::ProductField {
                product,
                field,
                value,
            } => {
                self.load(*value)?;
                let descriptor = intern_product_field(self.chunk, product.raw(), *field)?;
                self.proto.emit_op_u16(Op::LoadProductField, descriptor);
            }
            InstructionKind::WithProductField {
                product,
                field,
                value,
                replacement,
            } => {
                self.load(*value)?;
                self.load(*replacement)?;
                let descriptor = intern_product_field(self.chunk, product.raw(), *field)?;
                self.proto.emit_op_u16(Op::WithProductField, descriptor);
            }
            InstructionKind::EnumValue {
                enum_id,
                variant,
                layout,
                fields,
            } => {
                for field in fields {
                    self.load(*field)?;
                }
                let SsaType::Enum { arguments, .. } = &instruction.ty else {
                    return Err(Error::msg(
                        "verified enum construction lost enum result type",
                    ));
                };
                let descriptor = intern_enum_construction(
                    self.chunk,
                    *enum_id,
                    *variant,
                    *layout,
                    arguments.len(),
                )?;
                self.proto.emit_op_u16(Op::MakeEnum, descriptor);
            }
            InstructionKind::EnumIsVariant {
                enum_id,
                variant,
                layout,
                value,
            } => {
                self.load(*value)?;
                let descriptor = intern_enum_variant(self.chunk, *enum_id, *variant, *layout)?;
                self.proto.emit_op_u16(Op::IsEnumVariant, descriptor);
            }
            InstructionKind::EnumField {
                enum_id,
                variant,
                field,
                layout,
                value,
            } => {
                self.load(*value)?;
                let descriptor =
                    intern_enum_field(self.chunk, (*enum_id, *variant, *field), *layout)?;
                self.proto.emit_op_u16(Op::LoadEnumField, descriptor);
            }
        }
        if store_result {
            self.store_result(instruction.id)?;
        }
        Ok(())
    }
}
