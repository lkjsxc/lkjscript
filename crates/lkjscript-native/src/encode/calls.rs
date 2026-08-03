mod failure;
mod structural;
use super::*;

impl FunctionEncoder<'_> {
    pub(super) fn emit_call(
        &mut self,
        instruction: &Instruction,
        signature: &crate::Signature,
        arguments: &[ValueId],
        target: RelocationTarget,
    ) -> Result<(), NativeError> {
        self.load_integer_register(7, self.context_offset())?;
        let mut integer_index = 0_usize;
        let mut float_index = 0_usize;
        for (argument, argument_type) in arguments.iter().zip(signature.parameters()) {
            match argument_type {
                ValueType::I64
                | ValueType::Bool
                | ValueType::StaticBytes
                | ValueType::StaticString(_)
                | ValueType::Capability(_)
                | ValueType::Resource(_)
                | ValueType::Unique(_)
                | ValueType::Loan(_)
                | ValueType::StructuralKey
                | ValueType::MemoryWitnessLocator
                | ValueType::StructuralOwner(_)
                | ValueType::StructuralView(_)
                | ValueType::StructuralDestination(_)
                | ValueType::Reference(_) => {
                    let register = [6_u8, 2_u8, 1_u8, 8_u8, 9_u8]
                        .get(integer_index)
                        .copied()
                        .ok_or(NativeError::Encode(EncodeError::UnsupportedSignature))?;
                    self.load_integer_register(register, self.value_offset(*argument)?)?;
                    integer_index += 1;
                }
                ValueType::F64 => {
                    self.load_xmm(float_index, self.value_offset(*argument)?)?;
                    float_index += 1;
                }
                ValueType::Unit => {}
            }
        }
        self.emit_call_target(target)?;
        self.emit(&[0x41, 0xff, 0xd3])?;
        self.load_integer_register(1, self.context_offset())?;
        self.emit(&[0x83, 0x39, 0x00])?;
        self.emit_call_status_cleanup(instruction)?;
        match signature.result() {
            ValueType::I64
            | ValueType::Bool
            | ValueType::StaticBytes
            | ValueType::StaticString(_)
            | ValueType::Capability(_)
            | ValueType::Resource(_)
            | ValueType::Unique(_)
            | ValueType::Loan(_)
            | ValueType::StructuralKey
            | ValueType::MemoryWitnessLocator
            | ValueType::StructuralOwner(_)
            | ValueType::StructuralView(_)
            | ValueType::StructuralDestination(_)
            | ValueType::Reference(_) => {
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
            ValueType::F64 => self.store_xmm0(self.value_offset(instruction.output)?)?,
            ValueType::Unit => {
                self.zero_rax()?;
                self.store_rax(self.value_offset(instruction.output)?)?;
            }
        }
        Ok(())
    }

    pub(super) fn emit_heap_call(
        &mut self,
        instruction: &Instruction,
        descriptor: &crate::HeapCallDescriptor,
        arguments: &[ValueId],
    ) -> Result<(), NativeError> {
        let site_id = to_u32(self.heap_runtime_sites.len())?;
        self.runtime_calls.insert(RuntimeCallSlot::HeapDispatch);
        self.load_integer_register(7, self.context_offset())?;
        self.load_integer_register_immediate(6, u64::from(site_id))?;
        self.emit_call_target(RelocationTarget::Runtime(RuntimeCallSlot::HeapDispatch))?;
        self.emit(&[0x41, 0xff, 0xd3])?;
        let argument_homes = arguments
            .iter()
            .map(|argument| value_frame_home(self.function, *argument))
            .collect::<Result<Vec<_>, _>>()?;
        let result = value_frame_home(self.function, instruction.output)?;
        self.heap_runtime_sites.push(heap_runtime_site(
            site_id,
            self.function.id,
            descriptor.clone(),
            argument_homes,
            result,
            instruction.source,
        ));
        self.load_integer_register(1, self.context_offset())?;
        self.emit(&[0x83, 0x39, 0x00])?;
        self.emit_status_cleanup(instruction)
    }

    pub(super) fn emit_call_target(&mut self, target: RelocationTarget) -> Result<(), NativeError> {
        self.emit(&[0x49, 0xbb])?;
        let relocation_offset = self.bytes.len();
        self.emit(&0_u64.to_le_bytes())?;
        self.relocations.push(relocation(
            to_u32(relocation_offset)?,
            RelocationKind::Absolute64,
            target,
        ));
        Ok(())
    }

    pub(super) fn emit_runtime_call_target(
        &mut self,
        slot: RuntimeCallSlot,
    ) -> Result<(), NativeError> {
        self.emit_call_target(RelocationTarget::Runtime(slot))?;
        self.emit(&[0x41, 0xff, 0xd3])
    }
}
