impl Emitter<'_> {
    fn emit_structural_instruction(&mut self, instruction: &Instruction) -> Result<bool> {
        match &instruction.kind {
            InstructionKind::PlaceInit { place, value }
                if self.structural_local_kind(*value)? == Some(StructuralLocalKind::Owner) =>
            {
                let operand = self.place_slot(*place, *value)?;
                self.proto.emit_op_u16(Op::StructuralPlaceInit, operand);
            }
            InstructionKind::PlaceEnd { place } if self.structural_place(*place)? => {
                let place = u8::try_from(place.raw())
                    .map_err(|_| Error::msg("structural PlaceId exceeds bytecode u8"))?;
                self.proto.emit_op_u8(Op::StructuralPlaceEnd, place);
            }
            InstructionKind::EndBorrow { value, .. }
                if self.structural_local_kind(*value)? == Some(StructuralLocalKind::View) =>
            {
                let slot = self.slot(*value)?;
                self.proto.emit_op_u8(Op::EndStructuralBorrowLocal, slot);
            }
            InstructionKind::Drop {
                place,
                value,
                glue: DropGlueIdentity::Structural(_),
                ..
            } => {
                let operand = self.place_slot(*place, *value)?;
                self.proto.emit_op_u16(Op::StructuralDropPlace, operand);
            }
            InstructionKind::Move { place, value }
                if self.structural_local_kind(*value)? == Some(StructuralLocalKind::Owner) =>
            {
                let operand = self.place_slot(*place, *value)?;
                self.proto.emit_op_u16(Op::StructuralMove, operand);
            }
            InstructionKind::Borrow {
                kind, value, ..
            } if structural_view_representation(self.chunk, self.value_type(*value)?).is_some() => {
                let representation = structural_view_representation(self.chunk, self.value_type(*value)?)
                    .ok_or_else(|| Error::msg("structural borrow representation is missing"))?;
                self.proto.emit_op_u8(Op::LoadStructuralOwnerLocal, self.slot(*value)?);
                self.proto.emit_op_u16(
                    match kind {
                        lkjscript_ir::BorrowKind::Shared => Op::StructuralBorrow,
                        lkjscript_ir::BorrowKind::Mutable => Op::StructuralBorrowMut,
                    },
                    representation.raw(),
                );
            }
            InstructionKind::StructuralPublish { value, .. } => {
                let representation = structural_owner_representation(self.chunk, &instruction.ty)
                    .ok_or_else(|| Error::msg("structural publish representation is missing"))?;
                self.load(*value)?;
                self.proto
                    .emit_op_u16(Op::StructuralPublish, representation.raw());
            }
            InstructionKind::DestinationCreate {
                representation,
                active_variant,
            } => {
                let destination = structural_destination(self.chunk, *representation, *active_variant)?;
                self.proto
                    .emit_op_u16(Op::StructuralDestinationCreate, destination.raw());
            }
            InstructionKind::DestinationFieldInit {
                destination,
                field,
                value,
            } => {
                let metadata = self.destination_metadata_for_value(*destination)?;
                self.load(*destination)?;
                self.load(*value)?;
                let reference = intern_destination_field(self.chunk, metadata, *field)?;
                self.proto
                    .emit_op_u16(Op::StructuralDestinationFieldInit, reference);
            }
            InstructionKind::DestinationFinish { destination } => {
                let metadata = self.destination_metadata_for_value(*destination)?;
                self.load(*destination)?;
                self.proto
                    .emit_op_u16(Op::StructuralDestinationFinish, metadata.raw());
            }
            InstructionKind::DestinationAbort { destination } => {
                let metadata = self.destination_metadata_for_value(*destination)?;
                self.load(*destination)?;
                self.proto
                    .emit_op_u16(Op::StructuralDestinationAbort, metadata.raw());
            }
            InstructionKind::AggregateFieldBorrow {
                representation,
                field,
                value,
                ..
            } => {
                self.proto.emit_op_u8(Op::LoadStructuralOwnerLocal, self.slot(*value)?);
                let reference =
                    intern_aggregate_field(self.chunk, *representation, *field, &instruction.ty)?;
                self.proto
                    .emit_op_u16(Op::StructuralAggregateFieldBorrow, reference);
            }
            InstructionKind::AggregateTag { value, .. } => {
                let representation =
                    structural_view_representation(self.chunk, self.value_type(*value)?)
                        .ok_or_else(|| Error::msg("structural tag representation is missing"))?;
                self.proto.emit_op_u8(Op::LoadStructuralOwnerLocal, self.slot(*value)?);
                self.proto
                    .emit_op_u16(Op::StructuralAggregateTag, representation.raw());
            }
            InstructionKind::AggregateConsumePayload { variant, value, .. } => {
                let representation =
                    structural_owner_representation(self.chunk, self.value_type(*value)?)
                        .ok_or_else(|| Error::msg("structural payload representation is missing"))?;
                self.load(*value)?;
                let reference =
                    intern_payload(self.chunk, representation, *variant, &instruction.ty)?;
                self.proto
                    .emit_op_u16(Op::StructuralAggregateConsumePayload, reference);
            }
            InstructionKind::StringUtf8View {
                representation,
                value,
                ..
            } => {
                self.proto.emit_op_u8(Op::LoadStructuralOwnerLocal, self.slot(*value)?);
                self.proto
                    .emit_op_u16(Op::StructuralStringUtf8View, representation.raw());
            }
            InstructionKind::StructuralCopy { value, .. } => {
                let representation = structural_owner_representation(self.chunk, &instruction.ty)
                    .ok_or_else(|| Error::msg("structural copy representation is missing"))?;
                self.load_observed_structural(*value)?;
                self.proto
                    .emit_op_u16(Op::StructuralCopy, representation.raw());
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
