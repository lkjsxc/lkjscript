use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    match op {
        x if x == Op::SysOpenRead as u8 => {
            vm.ensure_host_deadline_support("sys-open-read", false)?;
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = vm.resources.sys_open_read(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysOpenWrite as u8 => {
            vm.ensure_host_deadline_support("sys-open-write", false)?;
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = vm.resources.sys_open_write(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysOpenAppend as u8 => {
            vm.ensure_host_deadline_support("sys-open-append", false)?;
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = vm.resources.sys_open_append(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysOpenCreateNew as u8 => {
            vm.ensure_host_deadline_support("sys-open-create-new", false)?;
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = vm.resources.sys_open_create_new(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysOpenDir as u8 => {
            vm.ensure_host_deadline_support("sys-open-dir", false)?;
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = vm.resources.sys_open_dir(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysFsync as u8 => {
            vm.ensure_host_deadline_support("sys-fsync", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.sys_fsync(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysTruncate as u8 => {
            vm.ensure_host_deadline_support("sys-truncate", false)?;
            let length = vm.pop()?;
            let length = vm.as_i64(length)?;
            let handle = vm.pop()?;
            let result = vm.resources.sys_truncate(handle, length);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysRename as u8 => {
            vm.ensure_host_deadline_support("sys-rename", false)?;
            let to = vm.pop()?;
            let from = vm.pop()?;
            let from = crate::host_ext::as_str(&vm.arena, from)?.to_string();
            let to = crate::host_ext::as_str(&vm.arena, to)?.to_string();
            let result = crate::host_ext::ResourceTable::sys_rename(&from, &to);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysClose as u8 => {
            vm.ensure_host_deadline_support("sys-close", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.close(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysReadByte as u8 => {
            let handle = vm.pop()?;
            if let Some(error) = wait_readable(vm, handle, "sys-read-byte")? {
                push_i64_result(vm, Err(error));
                return Ok(true);
            }
            let result = vm.resources.read_byte(handle);
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysWriteByte as u8 => {
            vm.ensure_host_deadline_support("sys-write-byte", false)?;
            let byte = vm.pop()?;
            let handle = vm.pop()?;
            let byte = vm.as_i64(byte)?;
            let result = vm.resources.write_byte(handle, byte);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysPathExists as u8 => {
            vm.ensure_host_deadline_support("sys-path-exists", false)?;
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = crate::host_ext::ResourceTable::sys_path_exists(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        _ => Ok(false),
    }
}
