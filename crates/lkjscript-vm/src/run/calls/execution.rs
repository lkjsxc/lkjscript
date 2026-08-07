use super::*;

pub fn call(vm: &mut Vm<'_>, argc: usize, call_offset: usize) -> Result<()> {
    let callee = vm.pop()?;
    match callee.as_function() {
        Some(prototype) => {
            let proto_index = usize::try_from(prototype)
                .map_err(|_| Error::msg("call proto index exceeds host usize"))?;
            let p = vm
                .chunk
                .protos()
                .get(proto_index)
                .ok_or_else(|| Error::msg("call proto index out of range"))?;
            if argc != p.arity {
                return Err(Error::msg(format!(
                    "arity mismatch for {}: got {argc}, want {}",
                    p.name, p.arity
                )));
            }
            let locals = p.locals;
            let argument_count = argc;
            let args_start = vm
                .stack
                .len()
                .checked_sub(argument_count)
                .ok_or_else(|| Error::msg("call argument stack underflow"))?;
            let tail_position = is_tail_position(vm);
            let stack_base = if tail_position {
                vm.frames.last().map_or(0, |frame| frame.stack_base)
            } else {
                args_start
            };
            let frame_end = stack_base
                .checked_add(locals)
                .ok_or_else(|| Error::msg("VM call frame size overflow"))?;
            if vm
                .config
                .max_stack_values()
                .is_some_and(|maximum| frame_end > maximum)
            {
                return Err(Error::resource(
                    lkjscript_core::ResourceLimitKind::StackValues,
                    "VM call frame exceeds the stack value limit",
                ));
            }
            let arguments = vm
                .stack
                .get(
                    args_start
                        ..args_start
                            .checked_add(argument_count)
                            .ok_or_else(|| Error::msg("call argument range overflow"))?,
                )
                .ok_or_else(|| Error::msg("call argument range is out of bounds"))?
                .to_vec();
            let memory_witnesses = super::super::structural_ops::call_memory_witnesses(
                vm,
                prototype,
                p,
                &arguments,
                call_offset,
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
                let argument_slot = args_start
                    .checked_add(index)
                    .ok_or_else(|| Error::msg("call argument slot overflow"))?;
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
            if tail_position {
                let args = vm.stack[args_start..].to_vec();
                super::super::structural_ops::cleanup_tail_copy_roots(vm, args_start, &args)?;
                vm.stack.truncate(stack_base);
                vm.stack
                    .try_reserve(args.len().max(locals))
                    .map_err(|_| Error::host("VM tail-call frame reservation failed"))?;
                vm.stack.extend_from_slice(&args);
                while vm.stack.len() < frame_end {
                    vm.stack.push(Value::INVALID);
                }
                if let Some(frame) = vm.frames.last_mut() {
                    *frame = Frame {
                        proto: Some(proto_index),
                        ip: 0,
                        instruction_offset: 0,
                        stack_base,
                        locals_base: stack_base,
                        unique_places,
                        borrowed_resources,
                        memory_witnesses,
                    };
                }
                return Ok(());
            }
            vm.stack
                .try_reserve(frame_end.saturating_sub(vm.stack.len()))
                .map_err(|_| Error::host("VM call frame reservation failed"))?;
            while vm.stack.len() < frame_end {
                vm.stack.push(Value::INVALID);
            }
            vm.frames
                .try_reserve(1)
                .map_err(|_| Error::host("VM frame-stack reservation failed"))?;
            vm.frames.push(Frame {
                proto: Some(proto_index),
                ip: 0,
                instruction_offset: 0,
                stack_base: args_start,
                locals_base: args_start,
                unique_places,
                borrowed_resources,
                memory_witnesses,
            });
            Ok(())
        }
        None => Err(Error::msg("call expects closure")),
    }
}

include!("execution/setup.rs");

#[cfg(test)]
mod forwarding_tests {
    include!("execution/tests.rs");
}
