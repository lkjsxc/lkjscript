use super::*;

use lkjscript_core::Op;

pub(super) fn handles(op: u8) -> bool {
    op == Op::Jump as u8
        || op == Op::JumpIfFalse as u8
        || op == Op::MakeClosure as u8
        || op == Op::Call as u8
        || op == Op::Return as u8
        || op == Op::Exit as u8
        || op == Op::Trap as u8
}

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<()> {
    match op {
        x if x == Op::Jump as u8 => {
            let at = vm.read_u16()? as usize;
            if let Some(fr) = vm.frames.last_mut() {
                fr.ip = at;
            }
            Ok(())
        }
        x if x == Op::JumpIfFalse as u8 => {
            let at = vm.read_u16()? as usize;
            let condition = vm
                .pop()?
                .as_bool()
                .ok_or_else(|| Error::msg("JumpIfFalse expects Bool"))?;
            if !condition {
                let frame = vm
                    .frames
                    .last_mut()
                    .ok_or_else(|| Error::msg("JumpIfFalse without frame"))?;
                frame.ip = at;
            }
            Ok(())
        }
        x if x == Op::MakeClosure as u8 => make_closure(vm),
        x if x == Op::Call as u8 => {
            let argc = vm.read_u8()?;
            call(vm, argc)
        }
        x if x == Op::Return as u8 => {
            let ret = vm.pop()?;
            let frame = vm.frames.pop().ok_or_else(|| Error::msg("return"))?;
            vm.stack.truncate(frame.stack_base);
            vm.push(ret);
            Ok(())
        }
        x if x == Op::Exit as u8 => {
            let value = vm.pop()?;
            let code = vm.as_i64(value)?;
            let code = i32::try_from(code).map_err(|_| Error::msg("exit code out of range"))?;
            vm.exit_code = Some(code);
            Ok(())
        }
        x if x == Op::Trap as u8 => {
            let diagnostic = usize::from(vm.read_u16()?);
            let message = match vm.chunk.constants().get(diagnostic) {
                Some(lkjscript_core::Constant::Str(message)) => message.clone(),
                _ => return Err(Error::msg("Trap diagnostic is not a Str constant")),
            };
            Err(Error::msg(message))
        }
        _ => unreachable!("opcode family checked"),
    }
}

impl<'a, J: RuntimeTier> Vm<'a, J> {
    pub(crate) fn step(&mut self) -> Result<()> {
        let code_len = self.code_len()?;
        let ip = self
            .frames
            .last()
            .map(|frame| frame.ip)
            .ok_or_else(|| Error::msg("no frame"))?;
        if ip >= code_len {
            return Err(Error::msg("function ended without Return"));
        }
        let op = self.read_u8()?;
        dispatch::dispatch(self, op)
    }
}

impl<'a> Vm<'a, JitSession> {}
