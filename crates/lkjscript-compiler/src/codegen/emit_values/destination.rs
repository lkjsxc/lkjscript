impl Emitter<'_> {
    pub(in crate::codegen) fn destination_metadata_for_value(
        &self,
        value: ValueId,
    ) -> Result<StructuralDestinationId> {
        let instruction = self
            .function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find(|instruction| instruction.id == value)
            .ok_or_else(|| Error::msg("SSA destination value has no defining instruction"))?;
        match &instruction.kind {
            InstructionKind::DestinationCreate {
                representation,
                active_variant,
            } => structural_destination(self.chunk, *representation, *active_variant),
            InstructionKind::DestinationFieldInit { destination, .. } => {
                self.destination_metadata_for_value(*destination)
            }
            InstructionKind::Constant(_)
            | InstructionKind::Copy(_)
            | InstructionKind::PlaceInit { .. }
            | InstructionKind::PlaceEnd { .. }
            | InstructionKind::EndBorrow { .. }
            | InstructionKind::Drop { .. }
            | InstructionKind::Move { .. }
            | InstructionKind::Borrow { .. }
            | InstructionKind::StructuralPublish { .. }
            | InstructionKind::DestinationFinish { .. }
            | InstructionKind::DestinationAbort { .. }
            | InstructionKind::AggregateFieldBorrow { .. }
            | InstructionKind::AggregateTag { .. }
            | InstructionKind::AggregateConsumePayload { .. }
            | InstructionKind::StringUtf8View { .. }
            | InstructionKind::StructuralCopy { .. }
            | InstructionKind::MemoryWitnessIndependentOwner { .. }
            | InstructionKind::MemoryWitnessDispose { .. }
            | InstructionKind::FunctionRef(_)
            | InstructionKind::Runtime { .. }
            | InstructionKind::F64FromI64Exact { .. }
            | InstructionKind::F64FromI64Rounded { .. }
            | InstructionKind::I64FromF64Exact { .. }
            | InstructionKind::I64FromF64Trunc { .. }
            | InstructionKind::Call { .. }
            | InstructionKind::ProductValue { .. }
            | InstructionKind::ProductField { .. }
            | InstructionKind::WithProductField { .. }
            | InstructionKind::EnumValue { .. }
            | InstructionKind::EnumIsVariant { .. }
            | InstructionKind::EnumField { .. } => {
                Err(Error::msg("SSA destination metadata provenance is invalid"))
            }
        }
    }

}
