#[derive(Debug, Clone)]
pub struct ValidatedChunk {
    chunk: Chunk,
    main_instructions: Vec<DecodedInstruction>,
    proto_instructions: Vec<Vec<DecodedInstruction>>,
}

impl ValidatedChunk {
    pub fn constants(&self) -> &[Constant] {
        &self.chunk.constants
    }

    pub fn constant(&self, index: u64) -> Option<&Constant> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.chunk.constants.get(index))
    }

    pub fn protos(&self) -> &[FunctionProto] {
        &self.chunk.protos
    }

    pub fn function_value(&self, prototype: u64) -> Result<Value> {
        let index = usize::try_from(prototype)
            .map_err(|_| Error::msg("function prototype index exceeds host usize"))?;
        self.chunk
            .protos
            .get(index)
            .ok_or_else(|| Error::msg("function prototype index is out of range"))?;
        Ok(Value::from_function(prototype))
    }

    pub fn symbol_value(&self, constant: u64) -> Result<Value> {
        let index = usize::try_from(constant)
            .map_err(|_| Error::msg("symbol constant index exceeds host usize"))?;
        match self.chunk.constants.get(index) {
            Some(Constant::Symbol(_)) => Ok(Value::from_symbol(constant)),
            _ => Err(Error::msg("symbol constant index is invalid")),
        }
    }

    pub fn main(&self) -> &FunctionProto {
        &self.chunk.main
    }

    pub fn required_capabilities(&self) -> &[crate::CapabilityKind] {
        &self.chunk.required_capabilities
    }

    pub fn global_names(&self) -> &[String] {
        &self.chunk.global_names
    }

    pub fn products(&self) -> &[crate::ProductMetadata] {
        &self.chunk.products
    }

    pub fn product_fields(&self) -> &[crate::ProductFieldRef] {
        &self.chunk.product_fields
    }

    pub fn enums(&self) -> &[crate::EnumMetadata] {
        &self.chunk.enums
    }

    pub fn enum_constructions(&self) -> &[crate::EnumConstructionRef] {
        &self.chunk.enum_constructions
    }

    pub fn enum_variants(&self) -> &[crate::EnumVariantRef] {
        &self.chunk.enum_variants
    }

    pub fn enum_fields(&self) -> &[crate::EnumFieldRef] {
        &self.chunk.enum_fields
    }

    pub fn memory_witnesses(&self) -> &[crate::InstalledMemoryWitness] {
        &self.chunk.memory_witnesses
    }

    pub fn structural_types(&self) -> &[crate::StructuralTypeMetadata] {
        &self.chunk.structural_types
    }

    pub fn structural_layouts(&self) -> &[crate::StructuralLayoutMetadata] {
        &self.chunk.structural_layouts
    }

    pub fn structural_representations(&self) -> &[crate::StructuralRepresentationMetadata] {
        &self.chunk.structural_representations
    }

    pub fn structural_destinations(&self) -> &[crate::StructuralDestinationMetadata] {
        &self.chunk.structural_destinations
    }

    pub fn structural_destination_fields(&self) -> &[crate::StructuralDestinationFieldRef] {
        &self.chunk.structural_destination_fields
    }

    pub fn structural_aggregate_fields(&self) -> &[crate::StructuralAggregateFieldRef] {
        &self.chunk.structural_aggregate_fields
    }

    pub fn structural_payloads(&self) -> &[crate::StructuralPayloadRef] {
        &self.chunk.structural_payloads
    }

    pub fn main_instructions(&self) -> &[DecodedInstruction] {
        &self.main_instructions
    }

    pub fn proto_instructions(&self, index: usize) -> Option<&[DecodedInstruction]> {
        self.proto_instructions.get(index).map(Vec::as_slice)
    }

    pub fn has_structural_execution(&self) -> bool {
        !self.chunk.structural_types.is_empty()
            || self
                .main_instructions
                .iter()
                .chain(self.proto_instructions.iter().flatten())
                .any(|instruction| {
                    matches!(
                        instruction.op(),
                        crate::Op::StoreStructuralLocal
                            | crate::Op::TakeStructuralLocal
                            | crate::Op::LoadStructuralViewLocal
                            | crate::Op::EndStructuralBorrowLocal
                            | crate::Op::LoadStructuralOwnerLocal
                            | crate::Op::StructuralPlaceInit
                            | crate::Op::StructuralMove
                            | crate::Op::StructuralDropPlace
                            | crate::Op::StructuralPlaceEnd
                            | crate::Op::StructuralBorrow
                            | crate::Op::StructuralBorrowMut
                            | crate::Op::StructuralPublish
                            | crate::Op::StructuralDestinationCreate
                            | crate::Op::StructuralDestinationFieldInit
                            | crate::Op::StructuralDestinationFinish
                            | crate::Op::StructuralDestinationAbort
                            | crate::Op::StructuralAggregateFieldBorrow
                            | crate::Op::StructuralAggregateFieldCopy
                            | crate::Op::StructuralAggregateTag
                            | crate::Op::StructuralAggregateConsumePayload
                            | crate::Op::StructuralStringUtf8View
                            | crate::Op::StructuralCopy
                    )
                })
    }

    pub fn proto_has_structural_execution(&self, index: usize) -> bool {
        self.proto_instructions
            .get(index)
            .is_some_and(|instructions| {
                instructions
                    .iter()
                    .any(|instruction| is_structural_opcode(instruction.op()))
            })
    }
}

fn is_structural_opcode(op: crate::Op) -> bool {
    matches!(
        op,
        crate::Op::StoreStructuralLocal
            | crate::Op::TakeStructuralLocal
            | crate::Op::LoadStructuralViewLocal
            | crate::Op::EndStructuralBorrowLocal
            | crate::Op::LoadStructuralOwnerLocal
            | crate::Op::StructuralPlaceInit
            | crate::Op::StructuralMove
            | crate::Op::StructuralDropPlace
            | crate::Op::StructuralPlaceEnd
            | crate::Op::StructuralBorrow
            | crate::Op::StructuralBorrowMut
            | crate::Op::StructuralPublish
            | crate::Op::StructuralDestinationCreate
            | crate::Op::StructuralDestinationFieldInit
            | crate::Op::StructuralDestinationFinish
            | crate::Op::StructuralDestinationAbort
            | crate::Op::StructuralAggregateFieldBorrow
            | crate::Op::StructuralAggregateFieldCopy
            | crate::Op::StructuralAggregateTag
            | crate::Op::StructuralAggregateConsumePayload
            | crate::Op::StructuralStringUtf8View
            | crate::Op::StructuralCopy
    )
}
