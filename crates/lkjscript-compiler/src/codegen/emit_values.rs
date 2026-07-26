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
        self.proto.emit_op_u8(Op::LoadLocal, slot);
        Ok(())
    }

    pub(in crate::codegen) fn store_result(&mut self, value: ValueId) -> Result<()> {
        let slot = self.slot(value)?;
        self.proto.emit_op_u8(Op::StoreLocal, slot);
        self.proto.emit(Op::Pop);
        Ok(())
    }

    pub(in crate::codegen) fn emit_instruction(
        &mut self,
        instruction: &Instruction,
        store_result: bool,
    ) -> Result<()> {
        match &instruction.kind {
            InstructionKind::Constant(constant) => self.emit_constant(constant)?,
            InstructionKind::Copy(value)
            | InstructionKind::Move { value, .. }
            | InstructionKind::Borrow { value, .. } => self.load(*value)?,
            InstructionKind::PlaceInit { .. } | InstructionKind::PlaceEnd { .. } => {
                self.proto.emit(Op::Unit);
            }
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
