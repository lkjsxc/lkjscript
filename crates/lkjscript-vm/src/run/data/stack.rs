use super::*;

impl<'a, J: RuntimeTier> Vm<'a, J> {
    pub(crate) fn code_len(&self) -> Result<usize> {
        Ok(self.code()?.len())
    }

    pub(crate) fn code(&self) -> Result<&[u8]> {
        let frame = self.frames.last().ok_or_else(|| Error::msg("no frame"))?;
        if frame.proto == u32::MAX {
            Ok(&self.chunk.main().code)
        } else {
            self.chunk
                .protos()
                .get(frame.proto as usize)
                .map(|proto| proto.code.as_slice())
                .ok_or_else(|| Error::msg("frame proto index out of range"))
        }
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8> {
        let (proto, ip) = {
            let frame = self.frames.last().ok_or_else(|| Error::msg("no frame"))?;
            (frame.proto, frame.ip)
        };
        let code = if proto == u32::MAX {
            &self.chunk.main().code
        } else {
            &self
                .chunk
                .protos()
                .get(proto as usize)
                .ok_or_else(|| Error::msg("frame proto index out of range"))?
                .code
        };
        let byte = *code.get(ip).ok_or_else(|| Error::msg("ip out of range"))?;
        if let Some(frame) = self.frames.last_mut() {
            frame.ip += 1;
        }
        Ok(byte)
    }

    pub(crate) fn read_u16(&mut self) -> Result<u16> {
        let low = self.read_u8()? as u16;
        let high = self.read_u8()? as u16;
        Ok(low | (high << 8))
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
            Constant::Str(text) => self.arena.alloc(HeapObj::Str(text.clone())),
            Constant::StaticBytes(_) => Ok(Value::from_static_bytes(
                u16::try_from(id)
                    .map_err(|_| Error::msg("static bytes constant index exceeds u16"))?,
            )),
            Constant::Symbol(_) => self.chunk.symbol_value(
                u32::try_from(id).map_err(|_| Error::msg("symbol constant index exceeds u32"))?,
            ),
            Constant::Proto(proto) => Ok(Value::from_i64(i64::from(*proto))),
        }
    }
}
