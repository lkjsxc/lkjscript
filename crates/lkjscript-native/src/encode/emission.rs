use super::*;

impl FunctionEncoder<'_> {
    pub(super) fn patch_fixups(&mut self) -> Result<(), NativeError> {
        for fixup in &self.fixups {
            let target = match fixup.target {
                FixupTarget::Block(block) => self
                    .block_offsets
                    .get(block.host_index().unwrap_or(usize::MAX))
                    .copied()
                    .flatten(),
                FixupTarget::Trap(trap) => self.trap_offsets[trap_index(trap)],
                FixupTarget::StatusReturn => self.status_return_offset,
                FixupTarget::UnregisteredStatusReturn => self.unregistered_status_return_offset,
            }
            .ok_or(NativeError::Encode(EncodeError::InvalidLabel))?;
            patch_relative(self.bytes, fixup.displacement_offset, target)?;
        }
        Ok(())
    }

    pub(super) fn find_signature(&self, function: FunctionId) -> Option<&crate::Signature> {
        self.signatures
            .iter()
            .find(|(function_id, _)| *function_id == function)
            .map(|(_, signature)| signature)
    }

    pub(super) fn value_type(&self, value: ValueId) -> Result<ValueType, NativeError> {
        self.function
            .values
            .get(value.host_index().unwrap_or(usize::MAX))
            .filter(|fact| fact.id == value)
            .map(|fact| fact.value_type)
            .ok_or(NativeError::Encode(EncodeError::InvalidValue))
    }

    pub(super) fn context_offset(&self) -> i32 {
        -8
    }

    pub(super) fn local_offset(&self, local: crate::LocalId) -> Result<i32, NativeError> {
        local_home_offset(local.host_index().unwrap_or(usize::MAX))
    }

    pub(super) fn value_offset(&self, value: ValueId) -> Result<i32, NativeError> {
        value_home_offset(self.function, value.host_index().unwrap_or(usize::MAX))
    }

    pub(super) fn load_rax_immediate(&mut self, value: u64) -> Result<(), NativeError> {
        self.emit(&[0x48, 0xb8])?;
        self.emit(&value.to_le_bytes())
    }

    pub(super) fn load_integer_register_immediate(
        &mut self,
        register: u8,
        value: u64,
    ) -> Result<(), NativeError> {
        if register > 15 {
            return Err(NativeError::Encode(EncodeError::UnsupportedSignature));
        }
        let rex = if register > 7 { 0x49 } else { 0x48 };
        self.emit(&[rex, 0xb8 | (register & 7)])?;
        self.emit(&value.to_le_bytes())
    }

    pub(super) fn zero_rax(&mut self) -> Result<(), NativeError> {
        self.emit(&[0x31, 0xc0])
    }

    pub(super) fn load_rax(&mut self, offset: i32) -> Result<(), NativeError> {
        self.emit(&[0x48, 0x8b, 0x85])?;
        self.emit_displacement(offset)
    }

    pub(super) fn load_rax_from_context(&mut self, offset: u8) -> Result<(), NativeError> {
        self.load_integer_register(1, self.context_offset())?;
        self.emit(&[0x48, 0x8b, 0x41, offset])
    }

    pub(super) fn load_xmm0_from_context(&mut self, offset: u8) -> Result<(), NativeError> {
        self.load_integer_register(1, self.context_offset())?;
        self.emit(&[0xf2, 0x0f, 0x10, 0x41, offset])
    }

    pub(super) fn store_rax(&mut self, offset: i32) -> Result<(), NativeError> {
        self.emit(&[0x48, 0x89, 0x85])?;
        self.emit_displacement(offset)
    }

    pub(super) fn load_integer_register(
        &mut self,
        register: u8,
        offset: i32,
    ) -> Result<(), NativeError> {
        if register > 15 {
            return Err(NativeError::Encode(EncodeError::UnsupportedSignature));
        }
        let rex = if register > 7 { 0x4c } else { 0x48 };
        self.emit(&[rex, 0x8b, 0x85 | ((register & 7) << 3)])?;
        self.emit_displacement(offset)
    }

    pub(super) fn load_xmm0(&mut self, offset: i32) -> Result<(), NativeError> {
        self.load_xmm(0, offset)
    }

    pub(super) fn load_xmm(&mut self, register: usize, offset: i32) -> Result<(), NativeError> {
        let register = u8::try_from(register)
            .map_err(|_| NativeError::Encode(EncodeError::UnsupportedSignature))?;
        self.emit(&[0xf2, 0x0f, 0x10, 0x85 | (register << 3)])?;
        self.emit_displacement(offset)
    }

    pub(super) fn store_xmm0(&mut self, offset: i32) -> Result<(), NativeError> {
        self.store_xmm(0, offset)
    }

    pub(super) fn store_xmm(&mut self, register: usize, offset: i32) -> Result<(), NativeError> {
        let register = u8::try_from(register)
            .map_err(|_| NativeError::Encode(EncodeError::UnsupportedSignature))?;
        self.emit(&[0xf2, 0x0f, 0x11, 0x85 | (register << 3)])?;
        self.emit_displacement(offset)
    }

    pub(super) fn emit_displacement(&mut self, displacement: i32) -> Result<(), NativeError> {
        self.emit(&displacement.to_le_bytes())
    }

    pub(super) fn reserve_i32(&mut self) -> Result<usize, NativeError> {
        let offset = self.bytes.len();
        self.emit(&0_i32.to_le_bytes())?;
        Ok(offset)
    }

    pub(super) fn emit(&mut self, bytes: &[u8]) -> Result<(), NativeError> {
        let next_len = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(NativeError::Encode(EncodeError::LimitExceeded(
                "code bytes",
            )))?;
        if next_len > self.maximum_code_bytes {
            return Err(NativeError::Encode(EncodeError::LimitExceeded(
                "code bytes",
            )));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    pub(super) fn check_code_limit(&self) -> Result<(), NativeError> {
        if self.bytes.len() > self.maximum_code_bytes {
            return Err(NativeError::Encode(EncodeError::LimitExceeded(
                "code bytes",
            )));
        }
        Ok(())
    }
}
