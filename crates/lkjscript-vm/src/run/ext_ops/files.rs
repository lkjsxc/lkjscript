use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    match op {
        x if x == Op::SysOpenRead as u8 => {
            vm.ensure_host_deadline_support("open-file-reader", false)?;
            let path = pop_filesystem_path(vm)?;
            let result = vm.resources.sys_open_read(&path);
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Io,
                crate::run::structural_ops::HostValueType::Resource(
                    lkjscript_core::ResourceKind::FileReader,
                ),
                result,
            );
            Ok(true)
        }
        x if x == Op::SysOpenWrite as u8 => {
            vm.ensure_host_deadline_support("open-file-writer", false)?;
            let path = pop_filesystem_path(vm)?;
            let result = vm.resources.sys_open_write(&path);
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Io,
                crate::run::structural_ops::HostValueType::Resource(
                    lkjscript_core::ResourceKind::FileWriter,
                ),
                result,
            );
            Ok(true)
        }
        x if x == Op::SysOpenAppend as u8 => {
            vm.ensure_host_deadline_support("open-file-appender", false)?;
            let path = pop_filesystem_path(vm)?;
            let result = vm.resources.sys_open_append(&path);
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Io,
                crate::run::structural_ops::HostValueType::Resource(
                    lkjscript_core::ResourceKind::FileAppender,
                ),
                result,
            );
            Ok(true)
        }
        x if x == Op::SysOpenCreateNew as u8 => {
            vm.ensure_host_deadline_support("create-file", false)?;
            let path = pop_filesystem_path(vm)?;
            let result = vm.resources.sys_open_create_new(&path);
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Io,
                crate::run::structural_ops::HostValueType::Resource(
                    lkjscript_core::ResourceKind::FileWriter,
                ),
                result,
            );
            Ok(true)
        }
        x if x == Op::SysOpenDir as u8 => {
            vm.ensure_host_deadline_support("open-directory", false)?;
            let path = pop_filesystem_path(vm)?;
            let result = vm.resources.sys_open_dir(&path);
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Io,
                crate::run::structural_ops::HostValueType::Resource(
                    lkjscript_core::ResourceKind::Directory,
                ),
                result,
            );
            Ok(true)
        }
        x if x == Op::SysFsync as u8 => {
            vm.ensure_host_deadline_support("sync-file", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.sys_fsync(handle);
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Io,
                crate::run::structural_ops::HostValueType::Unit,
                result,
            );
            Ok(true)
        }
        x if x == Op::SysTruncate as u8 => {
            vm.ensure_host_deadline_support("truncate-file", false)?;
            let length = vm.pop()?;
            let length = vm.as_i64(length)?;
            let handle = vm.pop()?;
            let result = vm.resources.sys_truncate(handle, length);
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Io,
                crate::run::structural_ops::HostValueType::Unit,
                result,
            );
            Ok(true)
        }
        x if x == Op::SysRename as u8 => {
            vm.ensure_host_deadline_support("rename-path", false)?;
            let to = vm.pop()?;
            let from = vm.pop()?;
            vm.require_capability(lkjscript_core::CapabilityKind::FileSystem)?;
            let from = crate::run::structural_ops::copy_path(vm, from)?;
            let to = crate::run::structural_ops::copy_path(vm, to)?;
            let result = crate::host_ext::ResourceTable::sys_rename(&from, &to);
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Io,
                crate::run::structural_ops::HostValueType::Unit,
                result,
            );
            Ok(true)
        }
        x if x == Op::ResourceDrop as u8 => {
            vm.ensure_host_deadline_support("drop", false)?;
            let handle = vm.pop()?;
            let kind = vm.resources.owned_kind(handle, "drop-resource")?;
            match kind {
                lkjscript_core::ResourceKind::SqliteConnection => {
                    let _ = vm.resources.sqlite_close(handle)?;
                }
                lkjscript_core::ResourceKind::SqliteStatement => {
                    let _ = vm.resources.sqlite_finalize(handle)?;
                }
                _ => {
                    let _ = vm.resources.close(handle)?;
                }
            }
            clear_resource_aliases(vm, handle);
            vm.push(Value::UNIT);
            Ok(true)
        }
        x if x == Op::SysClose as u8 => {
            vm.ensure_host_deadline_support("drop", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.close(handle);
            clear_resource_aliases(vm, handle);
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Io,
                crate::run::structural_ops::HostValueType::Unit,
                result,
            );
            Ok(true)
        }
        x if x == Op::SysReadByte as u8 => {
            let handle = vm.pop()?;
            if let Some(error) = wait_readable(vm, handle, "read-resource-byte")? {
                push_i64_result(vm, lkjscript_core::SystemErrorKind::Io, Err(error));
                return Ok(true);
            }
            let result = vm.resources.read_byte(handle);
            push_i64_result(vm, lkjscript_core::SystemErrorKind::Io, result);
            Ok(true)
        }
        x if x == Op::SysWriteByte as u8 => {
            vm.ensure_host_deadline_support("write-resource-byte", false)?;
            let byte = vm.pop()?;
            let handle = vm.pop()?;
            let byte = vm.as_i64(byte)?;
            let result = vm.resources.write_byte(handle, byte);
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Io,
                crate::run::structural_ops::HostValueType::Unit,
                result,
            );
            Ok(true)
        }
        x if x == Op::SysPathExists as u8 => {
            vm.ensure_host_deadline_support("does-path-exist", false)?;
            let path = pop_filesystem_path(vm)?;
            let result = crate::host_ext::ResourceTable::sys_path_exists(&path);
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Io,
                crate::run::structural_ops::HostValueType::Bool,
                result,
            );
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn pop_filesystem_path<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<Vec<u8>> {
    let path = vm.pop()?;
    vm.require_capability(lkjscript_core::CapabilityKind::FileSystem)?;
    crate::run::structural_ops::copy_path(vm, path)
}
