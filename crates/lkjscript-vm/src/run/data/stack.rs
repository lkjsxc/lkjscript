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

    pub(crate) fn make_i64(&mut self, number: i64) -> Result<Value> {
        match Value::from_small_i64(number) {
            Some(value) => Ok(value),
            None => self.arena.alloc(HeapObj::Int(number)),
        }
    }

    pub(crate) fn as_i64(&self, value: Value) -> Result<i64> {
        if let Some(number) = value.as_small_i64() {
            return Ok(number);
        }
        match value.as_heap().and_then(|_| self.arena.get(value).ok()) {
            Some(HeapObj::Int(number)) => Ok(*number),
            _ => Err(Error::msg("expected I64")),
        }
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
            Constant::I64(number) => self.make_i64(*number),
            Constant::F64(number) => self.arena.alloc(HeapObj::Float(*number)),
            Constant::Str(text) => self.arena.alloc(HeapObj::Str(text.clone())),
            Constant::Symbol(symbol) => self.arena.alloc(HeapObj::Symbol(symbol.clone())),
            Constant::Proto(proto) => self.make_i64(i64::from(*proto)),
        }
    }
}
