use super::*;

impl<'a, J: RuntimeTier> Vm<'a, J> {
    pub(crate) fn code_len(&self) -> Result<usize> {
        Ok(self.code()?.len())
    }

    pub(crate) fn code(&self) -> Result<&[u8]> {
        let frame = self.frames.last().ok_or_else(|| Error::msg("no frame"))?;
        if let Some(prototype) = frame.proto {
            self.chunk
                .protos()
                .get(prototype)
                .map(|proto| proto.code.as_slice())
                .ok_or_else(|| Error::msg("frame proto index out of range"))
        } else {
            Ok(&self.chunk.main().code)
        }
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8> {
        let (proto, ip) = {
            let frame = self.frames.last().ok_or_else(|| Error::msg("no frame"))?;
            (frame.proto, frame.ip)
        };
        let code = if let Some(prototype) = proto {
            &self
                .chunk
                .protos()
                .get(prototype)
                .ok_or_else(|| Error::msg("frame proto index out of range"))?
                .code
        } else {
            &self.chunk.main().code
        };
        let byte = *code.get(ip).ok_or_else(|| Error::msg("ip out of range"))?;
        if let Some(frame) = self.frames.last_mut() {
            frame.ip = frame
                .ip
                .checked_add(1)
                .ok_or_else(|| Error::msg("instruction pointer overflow"))?;
        }
        Ok(byte)
    }

    pub(crate) fn read_u16(&mut self) -> Result<u16> {
        let low = u16::from(self.read_u8()?);
        let high = u16::from(self.read_u8()?);
        Ok(low | (high << 8))
    }

    pub(crate) fn read_u64(&mut self) -> Result<u64> {
        let mut bytes = [0_u8; 8];
        for byte in &mut bytes {
            *byte = self.read_u8()?;
        }
        Ok(u64::from_le_bytes(bytes))
    }

    pub(crate) fn read_index(&mut self) -> Result<usize> {
        usize::try_from(self.read_u64()?)
            .map_err(|_| Error::msg("bytecode index exceeds host usize"))
    }

    pub(crate) fn read_place_local(&mut self) -> Result<(usize, usize)> {
        let place = self.read_index()?;
        let local = self.read_index()?;
        Ok((place, local))
    }

    pub(crate) fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    pub(crate) fn as_i64(&self, value: Value) -> Result<i64> {
        value.as_i64().ok_or_else(|| Error::msg("expected I64"))
    }

    pub(crate) fn as_f64(&self, value: Value) -> Result<f64> {
        value.as_f64().ok_or_else(|| Error::msg("expected F64"))
    }

    pub(crate) fn pop(&mut self) -> Result<Value> {
        let value = self
            .stack
            .pop()
            .ok_or_else(|| Error::msg("VM stack underflow"))?;
        if value.is_invalid() {
            return Err(Error::msg("uninitialized VM value"));
        }
        Ok(value)
    }

    pub(crate) fn require_capability(
        &mut self,
        expected: lkjscript_core::CapabilityKind,
    ) -> Result<()> {
        let value = self.pop()?;
        match value.as_capability() {
            Some(actual) if actual == expected => Ok(()),
            Some(actual) => Err(Error::msg(format!(
                "expected {} capability, received {}",
                expected.as_str(),
                actual.as_str()
            ))),
            None => Err(Error::msg(format!(
                "expected {} capability",
                expected.as_str()
            ))),
        }
    }

    pub(crate) fn peek(&self) -> Result<Value> {
        let value = self
            .stack
            .last()
            .copied()
            .ok_or_else(|| Error::msg("VM stack underflow"))?;
        if value.is_invalid() {
            return Err(Error::msg("uninitialized VM value"));
        }
        Ok(value)
    }

    pub(crate) fn load_const(&mut self, id: usize) -> Result<Value> {
        match self
            .chunk
            .constants()
            .get(id)
            .ok_or_else(|| Error::msg("bad const"))?
        {
            Constant::I64(number) => Ok(Value::from_i64(*number)),
            Constant::F64(number) => Ok(Value::from_f64_bits(number.to_bits())),
            Constant::Str(_) => Ok(Value::from_static_string(
                u64::try_from(id)
                    .map_err(|_| Error::msg("static string constant index exceeds u64"))?,
            )),
            Constant::StaticBytes(_) => Ok(Value::from_static_bytes(
                u64::try_from(id)
                    .map_err(|_| Error::msg("static bytes constant index exceeds u64"))?,
            )),
            Constant::Symbol(_) => self.chunk.symbol_value(
                u64::try_from(id).map_err(|_| Error::msg("symbol constant index exceeds u64"))?,
            ),
            Constant::Proto(proto) => Ok(Value::from_function_prototype(*proto)),
        }
    }
}
