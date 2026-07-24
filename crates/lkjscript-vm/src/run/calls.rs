//! Call and pair opcodes.

use lkjscript_core::{Error, HeapObj, Op, ResourceLimitKind, Result, Value};
use lkjscript_jit::{EntryDecision, NativeValue, ScalarInvocationOutcome, ValueType};

use crate::run::{Frame, RuntimeTier, Vm};

pub fn make_closure<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let _caps = vm.read_u16()?;
    let value = vm.pop()?;
    let proto_id = vm
        .as_i64(value)
        .map_err(|_| Error::msg("MakeClosure expects proto index"))?;
    let proto_id =
        u32::try_from(proto_id).map_err(|_| Error::msg("MakeClosure proto index out of range"))?;
    let v = vm.arena.alloc(HeapObj::Closure {
        proto: proto_id,
        captures: Vec::new(),
    });
    vm.push(v);
    Ok(())
}

pub fn car<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let p = vm.pop()?;
    match vm.arena.get(p)? {
        HeapObj::Pair { car, .. } => {
            let c = *car;
            vm.push(c);
            Ok(())
        }
        _ => Err(Error::msg("car expects pair")),
    }
}

pub fn cdr<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let p = vm.pop()?;
    match vm.arena.get(p)? {
        HeapObj::Pair { cdr, .. } => {
            let c = *cdr;
            vm.push(c);
            Ok(())
        }
        _ => Err(Error::msg("cdr expects pair")),
    }
}

pub fn call<J: RuntimeTier>(vm: &mut Vm<'_, J>, argc: u8) -> Result<()> {
    let callee = vm.pop()?;
    let obj = vm.arena.get(callee)?.clone();
    match obj {
        HeapObj::Closure { proto, .. } => {
            let p = vm
                .chunk
                .protos()
                .get(proto as usize)
                .ok_or_else(|| Error::msg("call proto index out of range"))?;
            if argc as usize != p.arity as usize {
                return Err(Error::msg(format!(
                    "arity mismatch for {}: got {argc}, want {}",
                    p.name, p.arity
                )));
            }
            let locals = p.locals;
            let argument_count = usize::from(argc);
            let args_start = vm
                .stack
                .len()
                .checked_sub(argument_count)
                .ok_or_else(|| Error::msg("call argument stack underflow"))?;
            if let EntryDecision::Native(function) = vm.jit.observe_function_entry(proto) {
                let signature = vm.jit.scalar_signature(function).ok_or_else(|| {
                    Error::msg("installed native function has no scalar signature")
                })?;
                if signature.parameters().len() != argument_count {
                    return Err(Error::msg("native scalar signature arity mismatch"));
                }
                let argument_values = vm.stack[args_start..].to_vec();
                let native_arguments = signature
                    .parameters()
                    .iter()
                    .copied()
                    .zip(argument_values)
                    .map(|(ty, value)| unbox_native(vm, ty, value))
                    .collect::<Result<Vec<_>>>()?;
                let mut execution = vm.config.clone();
                execution.instruction_fuel = vm.fuel_remaining;
                execution.wall_time = vm.remaining_wall_time()?;
                match vm
                    .jit
                    .invoke_scalar(function, &native_arguments, &execution)
                {
                    Ok(invocation) => {
                        vm.fuel_remaining = vm.fuel_remaining.saturating_sub(invocation.poll_count);
                        match invocation.outcome {
                            ScalarInvocationOutcome::Returned(value) => {
                                vm.stack.truncate(args_start);
                                let value = box_native(vm, value);
                                vm.push(value);
                                return Ok(());
                            }
                            ScalarInvocationOutcome::Trapped(trap) => {
                                let message = vm.jit.trap_message(function, trap);
                                return Err(Error::msg(message));
                            }
                            ScalarInvocationOutcome::Exited(code) => {
                                let code = i32::try_from(code)
                                    .map_err(|_| Error::msg("exit code out of range"))?;
                                vm.stack.truncate(args_start);
                                vm.exit_code = Some(code);
                                return Ok(());
                            }
                            ScalarInvocationOutcome::DeadlineExceeded => {
                                return Err(Error::deadline(
                                    "execution wall deadline exceeded in native PollV1",
                                ));
                            }
                            ScalarInvocationOutcome::ResourceLimitExceeded => {
                                return Err(Error::resource(
                                    ResourceLimitKind::InstructionFuel,
                                    "native PollV1 fuel exhausted",
                                ));
                            }
                            ScalarInvocationOutcome::HostFailure => {
                                return Err(Error::host("native PollV1 host clock failure"));
                            }
                        }
                    }
                    Err(_) => {
                        // Auto mode remains VM-correct. The session disables the
                        // failed object in this epoch before this untouched
                        // scalar function body is interpreted.
                        vm.jit.record_invocation_failure(function);
                    }
                }
            }
            if is_tail_position(vm) {
                let stack_base = vm.frames.last().map(|frame| frame.stack_base).unwrap_or(0);
                let args = vm.stack[args_start..].to_vec();
                vm.stack.truncate(stack_base);
                vm.stack.extend_from_slice(&args);
                while vm.stack.len() < stack_base + locals as usize {
                    vm.stack.push(Value::INVALID);
                }
                if let Some(frame) = vm.frames.last_mut() {
                    *frame = Frame {
                        proto,
                        ip: 0,
                        stack_base,
                        locals_base: stack_base,
                    };
                }
                return Ok(());
            }
            while vm.stack.len() < args_start + locals as usize {
                vm.stack.push(Value::INVALID);
            }
            vm.frames.push(Frame {
                proto,
                ip: 0,
                stack_base: args_start,
                locals_base: args_start,
            });
            Ok(())
        }
        _ => Err(Error::msg("call expects closure")),
    }
}

fn unbox_native<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    ty: ValueType,
    value: Value,
) -> Result<NativeValue> {
    match ty {
        ValueType::Unit if value.is_unit() => Ok(NativeValue::Unit),
        ValueType::Bool => value
            .as_bool()
            .map(NativeValue::Bool)
            .ok_or_else(|| Error::msg("native boundary expected Bool")),
        ValueType::I64 => vm
            .as_i64(value)
            .map(NativeValue::I64)
            .map_err(|_| Error::msg("native boundary expected I64")),
        ValueType::F64 => match vm.arena.get(value) {
            Ok(HeapObj::Float(number)) => Ok(NativeValue::F64Bits(number.to_bits())),
            _ => Err(Error::msg("native boundary expected F64")),
        },
        ValueType::Unit => Err(Error::msg("native boundary expected Unit")),
    }
}

fn box_native<J: RuntimeTier>(vm: &mut Vm<'_, J>, value: NativeValue) -> Value {
    match value {
        NativeValue::Unit => Value::UNIT,
        NativeValue::Bool(value) => Value::from_bool(value),
        NativeValue::I64(value) => vm.make_i64(value),
        NativeValue::F64Bits(bits) => vm.arena.alloc(HeapObj::Float(f64::from_bits(bits))),
    }
}

fn is_tail_position<J: RuntimeTier>(vm: &Vm<'_, J>) -> bool {
    let Some(frame) = vm.frames.last() else {
        return false;
    };
    if frame.proto == u32::MAX {
        return false;
    }
    vm.chunk
        .protos()
        .get(frame.proto as usize)
        .and_then(|proto| proto.code.get(frame.ip))
        .copied()
        == Some(Op::Return as u8)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;
    use lkjscript_core::{validate_chunk, Chunk, ExecutionConfig, FunctionProto, ValidationLimits};

    use crate::run::NoTier as NullJit;

    #[test]
    fn tail_call_reuses_the_current_frame() {
        let mut chunk = Chunk::new();
        chunk.main.emit(Op::Unit);
        chunk.main.emit(Op::Return);
        chunk.protos.push(FunctionProto {
            name: "callee".into(),
            arity: 1,
            locals: 1,
            code: vec![Op::LoadLocal as u8, 0, Op::Return as u8],
        });
        chunk.protos.push(FunctionProto {
            name: "caller".into(),
            arity: 0,
            locals: 0,
            code: vec![Op::Unit as u8, Op::Return as u8],
        });
        let chunk =
            validate_chunk(chunk, &ValidationLimits::default()).expect("call test chunk validates");
        let mut vm = Vm::new(&chunk, NullJit, Vec::new(), ExecutionConfig::default());
        vm.frames.push(Frame {
            proto: 1,
            ip: 1,
            stack_base: 0,
            locals_base: 0,
        });
        let argument = vm.make_i64(42);
        vm.push(argument);
        let callee = vm.arena.alloc(HeapObj::Closure {
            proto: 0,
            captures: Vec::new(),
        });
        vm.push(callee);

        call(&mut vm, 1).expect("tail call");

        assert_eq!(vm.frames.len(), 1);
        assert_eq!(vm.frames[0].proto, 0);
        assert_eq!(vm.stack, vec![argument]);
    }
}
