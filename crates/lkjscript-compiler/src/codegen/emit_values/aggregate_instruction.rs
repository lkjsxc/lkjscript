impl Emitter<'_> {
    fn emit_aggregate_instruction(&mut self, instruction: &Instruction) -> Result<bool> {
        match &instruction.kind {
            InstructionKind::ProductValue { product, fields } => {
                for field in fields {
                    self.load(*field)?;
                }
                self.proto.try_emit_op_u16(Op::MakeProduct, product.raw())?;
            }
            InstructionKind::ProductField {
                product,
                field,
                value,
            } => {
                self.load(*value)?;
                let descriptor = intern_product_field(self.chunk, product.raw(), *field)?;
                self.proto
                    .try_emit_op_u16(Op::LoadProductField, descriptor)?;
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
                self.proto
                    .try_emit_op_u16(Op::WithProductField, descriptor)?;
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
                self.proto.try_emit_op_u16(Op::MakeEnum, descriptor)?;
            }
            InstructionKind::EnumIsVariant {
                enum_id,
                variant,
                layout,
                value,
            } => {
                self.load(*value)?;
                let descriptor = intern_enum_variant(self.chunk, *enum_id, *variant, *layout)?;
                self.proto
                    .try_emit_op_u16(Op::IsEnumVariant, descriptor)?;
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
                self.proto
                    .try_emit_op_u16(Op::LoadEnumField, descriptor)?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
