use super::*;

pub fn call<J: RuntimeTier>(vm: &mut Vm<'_, J>, argc: u8) -> Result<()> {
    let callee = vm.pop()?;
    match callee.as_function() {
        Some(proto) => {
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
            #[cfg(feature = "jit")]
            if let EntryDecision::Native(function) = vm.jit.observe_function_entry(proto) {
                if !vm.chunk.proto_has_structural_execution(proto as usize) {
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
                        .map(|(ty, value)| native_from_value(ty, value))
                        .collect::<Result<Vec<_>>>()?;
                    let mut execution = vm.config.clone();
                    execution.instruction_fuel = vm.fuel_remaining;
                    execution.wall_time = vm.remaining_wall_time()?;
                    execution.max_stack_values =
                        execution.max_stack_values.saturating_sub(vm.stack.len());
                    match vm
                        .jit
                        .invoke_scalar(function, &native_arguments, &execution)
                    {
                        Ok(invocation) => {
                            vm.fuel_remaining =
                                vm.fuel_remaining.saturating_sub(invocation.poll_count);
                            vm.cleanup_failures.append(invocation.cleanup_failures);
                            match invocation.outcome {
                                ScalarInvocationOutcome::Returned(value) => {
                                    vm.stack.truncate(args_start);
                                    let value = value_from_native(value)?;
                                    vm.push(value);
                                    return Ok(());
                                }
                                ScalarInvocationOutcome::Trapped(trap, site) => {
                                    let message = vm.jit.trap_message(function, trap, site);
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
                                        "execution wall deadline exceeded in native Poll",
                                    ));
                                }
                                ScalarInvocationOutcome::ResourceLimitExceeded(kind) => {
                                    return Err(Error::resource(
                                        kind,
                                        "native execution resource limit exceeded",
                                    ));
                                }
                                ScalarInvocationOutcome::HostFailure => {
                                    return Err(Error::host("native Poll host clock failure"));
                                }
                            }
                        }
                        Err(error) => {
                            return Err(Error::msg(format!(
                                "native invocation failed without VM fallback: {error}"
                            )))
                        }
                    }
                }
            }
            let arguments = vm
                .stack
                .get(args_start..args_start.saturating_add(argument_count))
                .ok_or_else(|| Error::msg("call argument range is out of bounds"))?
                .to_vec();
            let return_type_variable_representation =
                super::super::structural_ops::call_return_type_variable_representation(
                    vm, p, &arguments,
                )?;
            let borrowed_resources = p
                .parameter_resources
                .iter()
                .enumerate()
                .filter_map(|(index, kind)| {
                    (kind.is_some()
                        && p.parameter_resource_places
                            .get(index)
                            .is_none_or(Option::is_none))
                    .then(|| arguments.get(index).copied())
                    .flatten()
                })
                .collect::<Vec<_>>();
            for (index, place) in p.parameter_resource_places.iter().enumerate() {
                let Some(value) = place.and_then(|_| arguments.get(index)).copied() else {
                    continue;
                };
                if value.as_resource().is_none() || vm.resources.is_borrowed_handle(value) {
                    continue;
                }
                let argument_slot = args_start.saturating_add(index);
                for (slot, candidate) in vm.stack.iter_mut().enumerate() {
                    if slot != argument_slot && *candidate == value {
                        *candidate = Value::INVALID;
                    }
                }
            }
            let mut unique_places = initial_unique_places(p, &arguments)?;
            super::super::structural_ops::initialize_call_places(
                vm.chunk,
                vm.structural.as_ref(),
                p,
                &arguments,
                &mut unique_places,
            )?;
            super::super::structural_ops::commit_call_arguments(vm, &arguments, p)?;
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
                        unique_places,
                        borrowed_resources,
                        return_type_variable_representation,
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
                unique_places,
                borrowed_resources,
                return_type_variable_representation,
            });
            Ok(())
        }
        None => Err(Error::msg("call expects closure")),
    }
}

include!("execution/setup.rs");

#[cfg(feature = "jit")]
include!("execution/native.rs");

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
