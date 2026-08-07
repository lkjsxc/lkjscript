//! Checked byte-view host boundaries for bulk I/O, entropy, hashing, and terminal state.

use crate::host_ext::ResourceTable;
use crate::run::unique::UniqueRuntime;
use crate::run::{RuntimeTier, Vm};
use lkjscript_core::{Error, ResourceLimitKind, Result, Value};

// Private syscall and polling geometry. A larger language view remains valid;
// partial-count operations process one chunk and full operations iterate.
pub(crate) const IO_CHUNK_BYTES: usize = 256 * 1024;

pub(crate) fn read_into<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    resource: Value,
    view: Value,
) -> Result<i64> {
    let interruption = vm.interruption()?;
    let destination = vm.unique.exclusive_bytes(view)?;
    let chunk_len = destination.len().min(IO_CHUNK_BYTES);
    let chunk = &mut destination[..chunk_len];
    interruption.check()?;
    let count = vm.resources.read_into(resource, chunk)?;
    i64::try_from(count).map_err(|_| Error::msg("read-into count out of range"))
}

pub(crate) fn write_from<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    resource: Value,
    view: Value,
) -> Result<i64> {
    let interruption = vm.interruption()?;
    let remaining_output = vm.remaining_output_capacity()?;
    let source = vm.unique.shared_bytes(view)?;
    if !source.is_empty() && remaining_output == 0 {
        return Err(Error::resource(
            ResourceLimitKind::OutputBytes,
            "write-from output policy exhausted",
        ));
    }
    let chunk_len = source.len().min(IO_CHUNK_BYTES).min(remaining_output);
    interruption.check()?;
    let count = vm.resources.write_from(resource, &source[..chunk_len])?;
    vm.record_output(count)?;
    i64::try_from(count).map_err(|_| Error::msg("write-from count out of range"))
}

pub(crate) fn fill_random<J: RuntimeTier>(vm: &mut Vm<'_, J>, view: Value) -> Result<Value> {
    let interruption = vm.interruption()?;
    let destination = vm.unique.exclusive_bytes(view)?;
    for chunk in destination.chunks_mut(IO_CHUNK_BYTES) {
        interruption.check()?;
        lkjscript_sys::random_fill(chunk)
            .map_err(|error| Error::msg(format!("fill-random: {error}")))?;
    }
    Ok(Value::UNIT)
}

pub(crate) fn sha256(unique: &mut UniqueRuntime, view: Value) -> Result<Value> {
    let source = unique.shared_bytes(view)?;
    let digest = lkjscript_core::sha256(source);
    unique.allocate_bytes(digest.to_vec())
}

pub(crate) fn tty_get(
    unique: &mut UniqueRuntime,
    resources: &ResourceTable,
    resource: Value,
    view: Value,
) -> Result<Value> {
    let raw = resources.raw_fd(resource, "get-terminal-state")?;
    let state = unique.exclusive_bytes(view)?;
    lkjscript_sys::tty_get(raw, state)
        .map_err(|error| Error::msg(format!("get-terminal-state: {error}")))?;
    Ok(Value::UNIT)
}

pub(crate) fn tty_set(
    unique: &mut UniqueRuntime,
    resources: &ResourceTable,
    resource: Value,
    view: Value,
) -> Result<Value> {
    let raw = resources.raw_fd(resource, "set-terminal-state")?;
    let state = unique.shared_bytes(view)?;
    lkjscript_sys::tty_set(raw, state)
        .map_err(|error| Error::msg(format!("set-terminal-state: {error}")))?;
    Ok(Value::UNIT)
}

pub(crate) fn tty_guard_save(unique: &mut UniqueRuntime, view: Value) -> Result<Value> {
    let state = unique.shared_bytes(view)?;
    lkjscript_sys::tty_guard_save(state)
        .map_err(|error| Error::msg(format!("save-terminal-guard: {error}")))?;
    Ok(Value::UNIT)
}

pub(crate) fn tty_guard_clear() -> Result<Value> {
    lkjscript_sys::tty_guard_clear()
        .map_err(|error| Error::msg(format!("clear-terminal-guard: {error}")))?;
    Ok(Value::UNIT)
}

pub(crate) fn poll(resources: &ResourceTable, resource: Value, timeout: i64) -> Result<i64> {
    let raw = resources.raw_fd(resource, "poll-streams")?;
    let timeout = i32::try_from(timeout).map_err(|_| Error::msg("poll timeout out of range"))?;
    if timeout < 0 {
        return Err(Error::msg("poll timeout out of range"));
    }
    lkjscript_sys::poll_fd(raw, timeout)
        .map(i64::from)
        .map_err(|error| Error::msg(format!("poll-streams: {error}")))
}

pub(crate) fn standard_input() -> Value {
    ResourceTable::stdin_handle()
}

pub(crate) fn is_terminal(resources: &ResourceTable, resource: Value) -> Result<Value> {
    let raw = resources.raw_fd(resource, "is-terminal")?;
    Ok(Value::from_bool(lkjscript_sys::is_tty(raw)))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::run::NoTier;
    use std::os::unix::ffi::OsStrExt;

    fn chunk() -> lkjscript_core::ValidatedChunk {
        let mut chunk = lkjscript_core::Chunk::new();
        chunk.main.emit(lkjscript_core::Op::Unit);
        chunk.main.emit(lkjscript_core::Op::Return);
        lkjscript_core::validate_chunk(chunk, lkjscript_core::ValidationPolicy::Unrestricted)
            .expect("host-byte test chunk validates")
    }

    fn path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "lkjscript-{label}-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("host-bytes")
        ))
    }

    #[test]
    fn file_read_and_write_requests_cross_former_bulk_limit() -> Result<()> {
        let len = 64_usize * 1024 + 1;
        let len_i64 = i64::try_from(len).expect("test length fits i64");
        let len_u64 = u64::try_from(len).expect("test length fits u64");
        let path = path("bulk-io");
        let _ = std::fs::remove_file(&path);
        let chunk = chunk();
        let config = lkjscript_core::ExecutionConfig {
            max_heap_bytes: len * 4,
            max_output_bytes: len * 2,
            ..lkjscript_core::ExecutionConfig::default()
        };
        let mut vm = Vm::new(&chunk, NoTier, crate::ExecutionInputs::default(), config);

        let writer = vm.resources.sys_open_write(path.as_os_str().as_bytes())?;
        let owner = vm.unique.allocate(len_i64)?;
        let view = vm.unique.borrow(owner, false)?;
        assert_eq!(write_from(&mut vm, writer, view)?, len_i64);
        vm.unique.end_borrow(view)?;
        vm.unique.drop_owner(owner)?;
        vm.resources.close(writer)?;
        assert_eq!(
            std::fs::metadata(&path)
                .map_err(|error| Error::host(error.to_string()))?
                .len(),
            len_u64
        );

        let reader = vm.resources.sys_open_read(path.as_os_str().as_bytes())?;
        let owner = vm.unique.allocate(len_i64)?;
        let view = vm.unique.borrow(owner, true)?;
        assert_eq!(read_into(&mut vm, reader, view)?, len_i64);
        let read = vm.unique.exclusive_bytes(view)?;
        assert_eq!((read.first(), read.last()), (Some(&0), Some(&0)));
        vm.unique.end_borrow(view)?;
        vm.unique.drop_owner(owner)?;
        vm.resources.close(reader)?;
        vm.unique.verify_empty()?;
        std::fs::remove_file(path).map_err(|error| Error::host(error.to_string()))?;
        Ok(())
    }

    #[test]
    fn partial_write_obeys_output_policy_and_cancelled_write_has_no_effect() -> Result<()> {
        let len = 64_usize * 1024 + 1;
        let len_i64 = i64::try_from(len).expect("test length fits i64");
        let len_u64 = u64::try_from(len).expect("test length fits u64");
        let path = path("bulk-policy");
        let _ = std::fs::remove_file(&path);
        let chunk = chunk();
        let config = lkjscript_core::ExecutionConfig {
            max_heap_bytes: len * 2,
            max_output_bytes: len - 1,
            ..lkjscript_core::ExecutionConfig::default()
        };
        let mut vm = Vm::new(&chunk, NoTier, crate::ExecutionInputs::default(), config);
        let writer = vm.resources.sys_open_write(path.as_os_str().as_bytes())?;
        let owner = vm.unique.allocate(len_i64)?;
        let view = vm.unique.borrow(owner, false)?;
        assert_eq!(write_from(&mut vm, writer, view)?, len_i64 - 1);
        let error = write_from(&mut vm, writer, view).expect_err("output policy is exhausted");
        assert_eq!(
            error.class(),
            lkjscript_core::ErrorClass::Resource(ResourceLimitKind::OutputBytes)
        );
        vm.unique.end_borrow(view)?;
        vm.unique.drop_owner(owner)?;
        vm.resources.close(writer)?;
        assert_eq!(
            std::fs::metadata(&path)
                .map_err(|error| Error::host(error.to_string()))?
                .len(),
            len_u64 - 1
        );

        let token = lkjscript_host::CancellationToken::new();
        token.cancel();
        let inputs = crate::ExecutionInputs {
            host: lkjscript_host::HostEnvironment {
                cancellation: Some(std::sync::Arc::new(token)),
                ..lkjscript_host::HostEnvironment::default()
            },
            ..crate::ExecutionInputs::default()
        };
        let mut cancelled = Vm::new(
            &chunk,
            NoTier,
            inputs,
            lkjscript_core::ExecutionConfig {
                max_heap_bytes: len * 2,
                ..lkjscript_core::ExecutionConfig::default()
            },
        );
        let writer = cancelled
            .resources
            .sys_open_write(path.as_os_str().as_bytes())?;
        let owner = cancelled.unique.allocate(len_i64)?;
        let view = cancelled.unique.borrow(owner, false)?;
        let error = write_from(&mut cancelled, writer, view).expect_err("cancelled write");
        assert_eq!(error.class(), lkjscript_core::ErrorClass::Host);
        cancelled.unique.end_borrow(view)?;
        cancelled.unique.drop_owner(owner)?;
        cancelled.resources.close(writer)?;
        cancelled.unique.verify_empty()?;
        assert_eq!(
            std::fs::metadata(&path)
                .map_err(|error| Error::host(error.to_string()))?
                .len(),
            0
        );
        std::fs::remove_file(path).map_err(|error| Error::host(error.to_string()))?;
        Ok(())
    }
}
