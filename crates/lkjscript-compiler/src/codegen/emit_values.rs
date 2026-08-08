use crate::codegen::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::codegen) enum StructuralLocalKind {
    Owner,
    OwnerRef,
    View,
    Destination,
}

impl Emitter<'_> {
    pub(in crate::codegen) fn load_observed_structural(&mut self, value: ValueId) -> Result<()> {
        if matches!(
            self.structural_local_kind(value)?,
            Some(StructuralLocalKind::Owner | StructuralLocalKind::OwnerRef)
        ) {
            let slot = self.slot(value)?;
            self.emit_index(Op::LoadStructuralOwnerLocal, slot)?;
            return Ok(());
        }
        self.load(value)
    }

    pub(in crate::codegen) fn load(&mut self, value: ValueId) -> Result<()> {
        let slot = self.slot(value)?;
        if let Some(kind) = self.structural_local_kind(value)? {
            self.emit_index(
                match kind {
                    StructuralLocalKind::Owner | StructuralLocalKind::Destination => {
                        Op::TakeStructuralLocal
                    }
                    StructuralLocalKind::OwnerRef => Op::LoadStructuralOwnerLocal,
                    StructuralLocalKind::View => Op::LoadStructuralViewLocal,
                },
                slot,
            )?;
            return Ok(());
        }
        self.emit_index(
            match self.value_type(value)? {
                SsaType::Bytes if self.borrowed_bytes_value(value)? => Op::LoadViewLocal,
                SsaType::Bytes if !self.static_bytes_value(value)? => Op::TakeUniqueLocal,
                SsaType::ByteVector => Op::TakeUniqueLocal,
                SsaType::ByteSlice | SsaType::ByteSliceMut => Op::LoadViewLocal,
                _ => Op::LoadLocal,
            },
            slot,
        )?;
        Ok(())
    }

    pub(in crate::codegen) fn store_result(&mut self, value: ValueId) -> Result<()> {
        let slot = self.slot(value)?;
        if self.structural_local_kind(value)?.is_some() {
            self.emit_index(Op::StoreStructuralLocal, slot)?;
            return Ok(());
        }
        match self.value_type(value)? {
            SsaType::Bytes if self.borrowed_bytes_value(value)? => {
                self.emit_index(Op::StoreViewLocal, slot)?;
            }
            SsaType::Bytes if !self.static_bytes_value(value)? => {
                self.emit_index(Op::StoreUniqueLocal, slot)?;
            }
            SsaType::ByteVector => self.emit_index(Op::StoreUniqueLocal, slot)?,
            SsaType::ByteSlice | SsaType::ByteSliceMut => {
                self.emit_index(Op::StoreViewLocal, slot)?;
            }
            _ => {
                self.emit_index(Op::StoreLocal, slot)?;
                self.proto.try_emit(Op::Pop)?;
            }
        }
        Ok(())
    }

    pub(in crate::codegen) fn witness_parameter_ordinal(&self, parameter: &str) -> Result<usize> {
        self.function
            .signature
            .type_parameters
            .iter()
            .position(|candidate| candidate == parameter)
            .ok_or_else(|| Error::msg("memory witness parameter is not declared"))
    }

    pub(in crate::codegen) fn value_type(&self, value: ValueId) -> Result<&SsaType> {
        self.local_metadata
            .get(&value)
            .map(|metadata| &metadata.ty)
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

    fn structural_place(&self, place: lkjscript_ir::PlaceId) -> Result<bool> {
        self.function
            .places
            .iter()
            .find(|metadata| metadata.id == place)
            .map(|metadata| structural_owner_representation(self.chunk, &metadata.ty).is_some())
            .ok_or_else(|| Error::msg("SSA bytecode lowering lost a structural PlaceId"))
    }

    fn byte_vector_place(&self, place: lkjscript_ir::PlaceId) -> Result<bool> {
        self.function
            .places
            .iter()
            .find(|metadata| metadata.id == place)
            .map(|metadata| metadata.ty == SsaType::ByteVector)
            .ok_or_else(|| Error::msg("SSA bytecode lowering lost a PlaceId"))
    }
}

include!("emit_values/call.rs");
include!("emit_values/slots.rs");
include!("emit_values/witness.rs");
include!("emit_values/destination.rs");
include!("emit_values/instruction.rs");
include!("emit_values/aggregate_instruction.rs");
include!("emit_values/numeric.rs");
include!("emit_values/structural_local.rs");
include!("emit_values/resource_drop.rs");
include!("emit_values/structural_instruction.rs");
