use super::super::*;

impl Evaluator<'_> {
    pub(crate) fn runtime_buffers(
        &mut self,
        operation: RuntimeOp,
        arguments: Vec<EvalValue>,
    ) -> std::result::Result<EvalValue, Flow> {
        use RuntimeOp as Op;
        match operation {
            Op::BufNew | Op::OwnedBufNew => unary(&arguments, |size| {
                let size = usize::try_from(as_i64(size)?)
                    .map_err(|_| Flow::Trap("buf-new size out of range".into()))?;
                if size > self.config.max_buffer_bytes || size > 1_000_000 {
                    return Err(Flow::Trap("buf-new size out of range".into()));
                }
                self.allocate_buffer(vec![0; size])
            }),
            Op::BufLen | Op::OwnedBufLen => unary(&arguments, |buffer| {
                let buffer = as_buffer(buffer)?;
                let length = i64::try_from(buffer.bytes.borrow().len())
                    .map_err(|_| Flow::Trap("buf-len out of range".into()))?;
                Ok(EvalValue::I64(length))
            }),
            Op::BufRef | Op::OwnedBufRef => binary(&arguments, |buffer, index| {
                let buffer = as_buffer(buffer)?;
                let index = index_value(index, "buf-ref")?;
                let byte = buffer
                    .bytes
                    .borrow()
                    .get(index)
                    .copied()
                    .ok_or_else(|| Flow::Trap("buf-ref out of bounds".into()))?;
                Ok(EvalValue::I64(i64::from(byte)))
            }),
            Op::BufSet | Op::OwnedBufSet => ternary(&arguments, |buffer, index, byte| {
                let buffer = as_buffer(buffer)?;
                let index = index_value(index, "buf-set")?;
                let byte = u8::try_from(as_i64(byte)?)
                    .map_err(|_| Flow::Trap("buf-set byte out of range".into()))?;
                let mut bytes = buffer.bytes.borrow_mut();
                let Some(slot) = bytes.get_mut(index) else {
                    return Err(Flow::Trap("buf-set out of bounds".into()));
                };
                *slot = byte;
                Ok(EvalValue::Unit)
            }),
            Op::BufClone => unary(&arguments, |buffer| {
                let buffer = as_buffer(buffer)?;
                self.allocate_buffer(buffer.bytes.borrow().clone())
            }),
            Op::BufFromStr => unary(&arguments, |text| {
                self.allocate_buffer(as_str(text)?.as_bytes().to_vec())
            }),
            Op::BufToStr => unary(&arguments, |buffer| {
                let buffer = as_buffer(buffer)?;
                match String::from_utf8(buffer.bytes.borrow().clone()) {
                    Ok(text) => {
                        let payload = self.allocate_string(text)?;
                        self.allocate_result(payload, true)
                    }
                    Err(_) => self.allocate_result_error("buf-to-str: invalid UTF-8"),
                }
            }),
            Op::BufSlice => ternary(&arguments, |buffer, offset, length| {
                let buffer = as_buffer(buffer)?;
                let offset = match usize::try_from(as_i64(offset)?) {
                    Ok(offset) => offset,
                    Err(_) => return self.allocate_result_error("buf-slice offset out of range"),
                };
                let length = match usize::try_from(as_i64(length)?) {
                    Ok(length) => length,
                    Err(_) => return self.allocate_result_error("buf-slice length out of range"),
                };
                let Some(end) = offset.checked_add(length) else {
                    return self.allocate_result_error("buf-slice range overflow");
                };
                let bytes = {
                    let bytes = buffer.bytes.borrow();
                    let Some(bytes) = bytes.get(offset..end) else {
                        return self.allocate_result_error("buf-slice range out of bounds");
                    };
                    bytes.to_vec()
                };
                let payload = self.allocate_buffer(bytes)?;
                self.allocate_result(payload, true)
            }),
            Op::BufGetU32 => binary(&arguments, |buffer, index| {
                let buffer = as_buffer(buffer)?;
                let index = index_value(index, "buf-get-u32")?;
                let end = index
                    .checked_add(4)
                    .ok_or_else(|| Flow::Trap("buf-get-u32 index overflow".into()))?;
                let bytes = buffer.bytes.borrow();
                let slice = bytes
                    .get(index..end)
                    .ok_or_else(|| Flow::Trap("buf-get-u32 out of bounds".into()))?;
                let mut word = [0; 4];
                word.copy_from_slice(slice);
                Ok(EvalValue::I64(i64::from(u32::from_le_bytes(word))))
            }),
            Op::BufSetU32 => ternary(&arguments, |buffer, index, number| {
                let buffer = as_buffer(buffer)?;
                let index = index_value(index, "buf-set-u32")?;
                let end = index
                    .checked_add(4)
                    .ok_or_else(|| Flow::Trap("buf-set-u32 index overflow".into()))?;
                let number = u32::try_from(as_i64(number)?)
                    .map_err(|_| Flow::Trap("buf-set-u32 value out of range".into()))?;
                let mut bytes = buffer.bytes.borrow_mut();
                let destination = bytes
                    .get_mut(index..end)
                    .ok_or_else(|| Flow::Trap("buf-set-u32 out of bounds".into()))?;
                destination.copy_from_slice(&number.to_le_bytes());
                Ok(EvalValue::Unit)
            }),
            _ => unreachable!("runtime operation dispatched to the wrong family"),
        }
    }
}
