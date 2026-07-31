use super::*;

fn push_unit_result<J: RuntimeTier>(vm: &mut Vm<'_, J>, result: Result<Value>) {
    super::push_runtime_result(
        vm,
        lkjscript_core::SystemErrorKind::Terminal,
        crate::run::structural_ops::HostValueType::Unit,
        result,
    );
}

fn push_bool_result<J: RuntimeTier>(vm: &mut Vm<'_, J>, result: Result<Value>) {
    super::push_runtime_result(
        vm,
        lkjscript_core::SystemErrorKind::Terminal,
        crate::run::structural_ops::HostValueType::Bool,
        result,
    );
}

fn push_i64_result<J: RuntimeTier>(vm: &mut Vm<'_, J>, result: Result<i64>) {
    super::push_i64_result(vm, lkjscript_core::SystemErrorKind::Terminal, result);
}

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    match op {
        x if x == Op::SysTtyGet as u8 => {
            vm.ensure_host_deadline_support("get-terminal-state", false)?;
            let buffer = vm.pop()?;
            let handle = vm.pop()?;
            let result = crate::host_bytes::tty_get(&mut vm.unique, &vm.resources, handle, buffer);
            push_unit_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysTtySet as u8 => {
            vm.ensure_host_deadline_support("set-terminal-state", false)?;
            let buffer = vm.pop()?;
            let handle = vm.pop()?;
            let result = crate::host_bytes::tty_set(&mut vm.unique, &vm.resources, handle, buffer);
            push_unit_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysPoll as u8 => {
            let timeout = vm.pop()?;
            let handle = vm.pop()?;
            let requested = vm.as_i64(timeout)?;
            let mut timeout = requested;
            let mut deadline_limited = false;
            if let Some(remaining) = vm.remaining_wall_time()? {
                let remaining_ms = remaining.as_millis().max(1);
                let remaining_ms = i64::try_from(remaining_ms).unwrap_or(i64::MAX);
                if timeout > remaining_ms {
                    timeout = remaining_ms;
                    deadline_limited = true;
                }
            }
            let result = crate::host_bytes::poll(&vm.resources, handle, timeout);
            if deadline_limited && matches!(result, Ok(0)) {
                return Err(lkjscript_core::Error::deadline(
                    "execution wall deadline exceeded during sys-poll",
                ));
            }
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::StdinHandle as u8 => {
            vm.require_capability(lkjscript_core::CapabilityKind::Stdio)?;
            vm.push(crate::host_bytes::standard_input());
            Ok(true)
        }
        x if x == Op::SysIsatty as u8 => {
            vm.ensure_host_deadline_support("is-terminal", false)?;
            let handle = vm.pop()?;
            let result = crate::host_bytes::is_terminal(&vm.resources, handle);
            push_bool_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysTtyGuardSave as u8 => {
            vm.ensure_host_deadline_support("save-terminal-guard", false)?;
            let buffer = vm.pop()?;
            vm.require_capability(lkjscript_core::CapabilityKind::Terminal)?;
            let result = crate::host_bytes::tty_guard_save(&mut vm.unique, buffer);
            push_unit_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysTtyGuardClear as u8 => {
            vm.ensure_host_deadline_support("clear-terminal-guard", false)?;
            vm.require_capability(lkjscript_core::CapabilityKind::Terminal)?;
            let result = crate::host_bytes::tty_guard_clear();
            push_unit_result(vm, result);
            Ok(true)
        }
        _ => Ok(false),
    }
}
