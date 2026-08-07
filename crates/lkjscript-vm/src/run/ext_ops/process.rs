use super::*;

pub(super) fn dispatch(vm: &mut Vm<'_>, op: u8) -> Result<bool> {
    match op {
        x if x == Op::Arg as u8 => {
            let value = vm.pop()?;
            vm.require_capability(lkjscript_core::CapabilityKind::Arguments)?;
            let index = vm.as_i64(value)?;
            let index = usize::try_from(index).ok();
            let argument = index
                .filter(|index| *index < vm.inputs.arguments.len())
                .map(|index| {
                    crate::run::structural_ops::HostValue::String(
                        vm.inputs.arguments[index].clone(),
                    )
                });
            let value = crate::run::structural_ops::publish_option(
                vm,
                crate::run::structural_ops::HostValueType::String,
                argument,
            )?;
            vm.push(value);
            Ok(true)
        }
        x if x == Op::Argc as u8 => {
            vm.require_capability(lkjscript_core::CapabilityKind::Arguments)?;
            let count = i64::try_from(vm.inputs.arguments.len())
                .map_err(|_| lkjscript_core::Error::msg("argc out of range"))?;
            let value = Value::from_i64(count);
            vm.push(value);
            Ok(true)
        }
        x if x == Op::SysNowMs as u8 => {
            vm.require_capability(lkjscript_core::CapabilityKind::Clock)?;
            let result = i64::try_from(clock(vm)?.monotonic_time().0 / 1_000_000)
                .map_err(|_| lkjscript_core::Error::msg("sys-now-ms: value out of range"));
            push_i64_result(vm, lkjscript_core::SystemErrorKind::Time, result);
            Ok(true)
        }
        x if x == Op::SysWaitMs as u8 => {
            let duration = vm.pop()?;
            vm.require_capability(lkjscript_core::CapabilityKind::Clock)?;
            let clock = clock(vm)?;
            let milliseconds = vm
                .as_i64(duration)
                .map_err(|_| lkjscript_core::Error::msg("sys-wait-ms: expected I64 duration"));
            let milliseconds = milliseconds.and_then(|milliseconds| {
                u64::try_from(milliseconds)
                    .map_err(|_| lkjscript_core::Error::msg("sys-wait-ms: duration out of range"))
            });
            let result = match milliseconds {
                Ok(milliseconds) => {
                    if let Some(remaining) = vm.remaining_wall_time()? {
                        let requested = std::time::Duration::from_millis(milliseconds);
                        if requested > remaining {
                            let sleep_ms = u64::try_from(remaining.as_millis()).unwrap_or(u64::MAX);
                            match clock.sleep(std::time::Duration::from_millis(sleep_ms)) {
                                Ok(()) => {
                                    return Err(lkjscript_core::Error::deadline(
                                        "execution wall deadline exceeded during sys-wait-ms",
                                    ));
                                }
                                Err(error) => {
                                    Err(lkjscript_core::Error::msg(format!("sys-wait-ms: {error}")))
                                }
                            }
                        } else {
                            sleep_result(clock.as_ref(), milliseconds)
                        }
                    } else {
                        sleep_result(clock.as_ref(), milliseconds)
                    }
                }
                Err(error) => Err(error),
            };
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Time,
                crate::run::structural_ops::HostValueType::Unit,
                result,
            );
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn clock(vm: &Vm<'_>) -> Result<std::sync::Arc<dyn lkjscript_host::Clock>> {
    vm.inputs
        .host
        .clock
        .clone()
        .ok_or_else(|| lkjscript_core::Error::host("clock capability has no granted provider"))
}
