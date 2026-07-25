//! Byte-buffer and bounded terminal/poll host helpers.

use crate::host_ext::ResourceTable;
use lkjscript_core::{
    Error, GcHeap as Arena, HeapObj, Result, Value, MAX_BUFFER_BYTES, MAX_BULK_IO_BYTES,
};

pub fn as_buf(arena: &Arena, value: Value) -> Result<&[u8]> {
    match arena.get(value)? {
        HeapObj::Buf(buffer) => Ok(buffer.as_slice()),
        _ => Err(Error::msg("expected buf")),
    }
}

pub fn buf_new(arena: &mut Arena, size: i64) -> Result<Value> {
    if !(0..=MAX_BUFFER_BYTES as i64).contains(&size) {
        return Err(Error::msg("buf-new size out of range"));
    }
    let size = usize::try_from(size).map_err(|_| Error::msg("buf-new size out of range"))?;
    arena.alloc(HeapObj::Buf(vec![0_u8; size]))
}

pub fn buf_len(arena: &Arena, value: Value) -> Result<i64> {
    i64::try_from(as_buf(arena, value)?.len()).map_err(|_| Error::msg("buf-len out of range"))
}

pub fn buf_ref(arena: &Arena, value: Value, index: i64) -> Result<i64> {
    let index = buffer_index(index, "buf-ref")?;
    let byte = *as_buf(arena, value)?
        .get(index)
        .ok_or_else(|| Error::msg("buf-ref out of bounds"))?;
    Ok(i64::from(byte))
}

pub fn buf_set(arena: &mut Arena, value: Value, index: i64, byte: i64) -> Result<Value> {
    let index = buffer_index(index, "buf-set")?;
    let byte = u8::try_from(byte).map_err(|_| Error::msg("buf-set byte out of range"))?;
    arena.mutate(value, |object| {
        let HeapObj::Buf(buffer) = object else {
            return Err(Error::msg("expected buf"));
        };
        let slot = buffer
            .get_mut(index)
            .ok_or_else(|| Error::msg("buf-set out of bounds"))?;
        *slot = byte;
        Ok(Value::UNIT)
    })
}

pub fn buf_get_u32(arena: &Arena, value: Value, index: i64) -> Result<i64> {
    let index = buffer_index(index, "buf-get-u32")?;
    let end = index
        .checked_add(4)
        .ok_or_else(|| Error::msg("buf-get-u32 index overflow"))?;
    let bytes = as_buf(arena, value)?
        .get(index..end)
        .ok_or_else(|| Error::msg("buf-get-u32 out of bounds"))?;
    let mut word = [0_u8; 4];
    word.copy_from_slice(bytes);
    Ok(i64::from(u32::from_le_bytes(word)))
}

pub fn buf_set_u32(arena: &mut Arena, value: Value, index: i64, number: i64) -> Result<Value> {
    let index = buffer_index(index, "buf-set-u32")?;
    let end = index
        .checked_add(4)
        .ok_or_else(|| Error::msg("buf-set-u32 index overflow"))?;
    let number = u32::try_from(number).map_err(|_| Error::msg("buf-set-u32 value out of range"))?;
    arena.mutate(value, |object| {
        let HeapObj::Buf(buffer) = object else {
            return Err(Error::msg("expected buf"));
        };
        let destination = buffer
            .get_mut(index..end)
            .ok_or_else(|| Error::msg("buf-set-u32 out of bounds"))?;
        destination.copy_from_slice(&number.to_le_bytes());
        Ok(Value::UNIT)
    })
}

pub fn buf_clone(arena: &mut Arena, value: Value) -> Result<Value> {
    let bytes = as_buf(arena, value)?.to_vec();
    arena.alloc(HeapObj::Buf(bytes))
}

pub fn buf_from_str(arena: &mut Arena, value: Value) -> Result<Value> {
    let bytes = crate::host_ext::as_str(arena, value)?.as_bytes();
    if bytes.len() > MAX_BUFFER_BYTES {
        return Err(Error::msg("buf-from-str string exceeds buffer limit"));
    }
    arena.alloc(HeapObj::Buf(bytes.to_vec()))
}

pub fn buf_to_str(arena: &mut Arena, value: Value) -> Result<Value> {
    let text = std::str::from_utf8(as_buf(arena, value)?)
        .map_err(|_| Error::msg("buf-to-str: invalid UTF-8"))?;
    arena.alloc(HeapObj::Str(text.to_owned()))
}

pub fn buf_slice(arena: &mut Arena, value: Value, offset: i64, length: i64) -> Result<Value> {
    let range = buffer_range(arena, value, offset, length, "buf-slice")?;
    let bytes = as_buf(arena, value)?
        .get(range)
        .ok_or_else(|| Error::msg("buf-slice range is invalid"))?
        .to_vec();
    arena.alloc(HeapObj::Buf(bytes))
}

pub fn sys_read_into(
    arena: &mut Arena,
    handles: &ResourceTable,
    handle: Value,
    buffer: Value,
    offset: i64,
    requested: i64,
) -> Result<i64> {
    let range = bulk_range(arena, buffer, offset, requested, "sys-read-into")?;
    let count = arena.mutate(buffer, |object| {
        let HeapObj::Buf(bytes) = object else {
            return Err(Error::msg("expected buf"));
        };
        let destination = bytes
            .get_mut(range)
            .ok_or_else(|| Error::msg("sys-read-into range is invalid"))?;
        handles.read_into(handle, destination)
    })?;
    i64::try_from(count).map_err(|_| Error::msg("sys-read-into count out of range"))
}

pub fn sys_write_from(
    arena: &Arena,
    handles: &ResourceTable,
    handle: Value,
    buffer: Value,
    offset: i64,
    requested: i64,
) -> Result<i64> {
    let range = bulk_range(arena, buffer, offset, requested, "sys-write-from")?;
    let source = as_buf(arena, buffer)?
        .get(range)
        .ok_or_else(|| Error::msg("sys-write-from range is invalid"))?;
    let count = handles.write_from(handle, source)?;
    i64::try_from(count).map_err(|_| Error::msg("sys-write-from count out of range"))
}

pub fn sys_random_fill(
    arena: &mut Arena,
    buffer: Value,
    offset: i64,
    requested: i64,
) -> Result<Value> {
    let range = bulk_range(arena, buffer, offset, requested, "sys-random-fill")?;
    arena.mutate(buffer, |object| {
        let HeapObj::Buf(bytes) = object else {
            return Err(Error::msg("expected buf"));
        };
        let destination = bytes
            .get_mut(range)
            .ok_or_else(|| Error::msg("sys-random-fill range is invalid"))?;
        lkjscript_sys::random_fill(destination)
            .map_err(|error| Error::msg(format!("sys-random-fill: {error}")))?;
        Ok(Value::UNIT)
    })
}

pub fn sys_sha256(arena: &mut Arena, buffer: Value, offset: i64, requested: i64) -> Result<Value> {
    let range = bulk_range(arena, buffer, offset, requested, "sys-sha256")?;
    let source = as_buf(arena, buffer)?
        .get(range)
        .ok_or_else(|| Error::msg("sys-sha256 range is invalid"))?;
    let digest = lkjscript_core::sha256(source);
    arena.alloc(HeapObj::Buf(digest.to_vec()))
}

pub fn sys_tty_get(
    arena: &mut Arena,
    handles: &ResourceTable,
    handle: Value,
    buffer: Value,
) -> Result<Value> {
    let raw = handles.raw_fd(handle, "sys-tty-get")?;
    arena.mutate(buffer, |object| {
        let HeapObj::Buf(state) = object else {
            return Err(Error::msg("expected buf"));
        };
        lkjscript_sys::tty_get(raw, state)
            .map_err(|error| Error::msg(format!("sys-tty-get: {error}")))?;
        Ok(Value::UNIT)
    })
}

pub fn sys_tty_set(
    arena: &Arena,
    handles: &ResourceTable,
    handle: Value,
    buffer: Value,
) -> Result<Value> {
    let raw = handles.raw_fd(handle, "sys-tty-set")?;
    let state = as_buf(arena, buffer)?;
    lkjscript_sys::tty_set(raw, state)
        .map_err(|error| Error::msg(format!("sys-tty-set: {error}")))?;
    Ok(Value::UNIT)
}

pub fn sys_poll(handles: &ResourceTable, handle: Value, timeout: i64) -> Result<i64> {
    let raw = handles.raw_fd(handle, "sys-poll")?;
    let timeout =
        i32::try_from(timeout).map_err(|_| Error::msg("sys-poll timeout out of range"))?;
    if timeout < 0 {
        return Err(Error::msg("sys-poll timeout out of range"));
    }
    let ready = lkjscript_sys::poll_fd(raw, timeout)
        .map_err(|error| Error::msg(format!("sys-poll: {error}")))?;
    Ok(i64::from(ready))
}

pub fn stdin_handle() -> Value {
    ResourceTable::stdin_handle()
}

pub fn sys_isatty(handles: &ResourceTable, handle: Value) -> Result<Value> {
    let raw = handles.raw_fd(handle, "sys-isatty")?;
    Ok(Value::from_bool(lkjscript_sys::is_tty(raw)))
}

pub fn sys_tty_guard_save(arena: &Arena, buffer: Value) -> Result<Value> {
    let state = as_buf(arena, buffer)?;
    lkjscript_sys::tty_guard_save(state)
        .map_err(|error| Error::msg(format!("sys-tty-guard-save: {error}")))?;
    Ok(Value::UNIT)
}

pub fn sys_tty_guard_clear() -> Result<Value> {
    lkjscript_sys::tty_guard_clear()
        .map_err(|error| Error::msg(format!("sys-tty-guard-clear: {error}")))?;
    Ok(Value::UNIT)
}

fn buffer_range(
    arena: &Arena,
    buffer: Value,
    offset: i64,
    requested: i64,
    operation: &str,
) -> Result<std::ops::Range<usize>> {
    let offset = usize::try_from(offset)
        .map_err(|_| Error::msg(format!("{operation} offset out of range")))?;
    let requested = usize::try_from(requested)
        .map_err(|_| Error::msg(format!("{operation} length out of range")))?;
    let end = offset
        .checked_add(requested)
        .ok_or_else(|| Error::msg(format!("{operation} range overflow")))?;
    if end > as_buf(arena, buffer)?.len() {
        return Err(Error::msg(format!("{operation} range out of bounds")));
    }
    Ok(offset..end)
}

fn bulk_range(
    arena: &Arena,
    buffer: Value,
    offset: i64,
    requested: i64,
    operation: &str,
) -> Result<std::ops::Range<usize>> {
    let range = buffer_range(arena, buffer, offset, requested, operation)?;
    if range.len() > MAX_BULK_IO_BYTES {
        return Err(Error::msg(format!(
            "{operation} length exceeds bulk I/O limit"
        )));
    }
    Ok(range)
}

fn buffer_index(index: i64, operation: &str) -> Result<usize> {
    usize::try_from(index).map_err(|_| Error::msg(format!("{operation} index out of range")))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use lkjscript_core::{Value, MAX_BULK_IO_BYTES};

    use crate::host_ext::ResourceTable;
    use lkjscript_core::GcHeap as Arena;

    use super::{
        as_buf, buf_from_str, buf_new, buf_set, buf_set_u32, buf_slice, buf_to_str, sys_poll,
        sys_random_fill, sys_read_into, sys_sha256, sys_write_from,
    };

    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

    struct TempFile(PathBuf);

    impl TempFile {
        fn new(bytes: &[u8]) -> std::io::Result<Self> {
            let id = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("lkjscript-bulk-{}-{id}", std::process::id()));
            fs::write(&path, bytes)?;
            Ok(Self(path))
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn polling_rejects_invalid_handles_and_timeouts() {
        let handles = ResourceTable::default();
        let integer = Value::from_small_i64(1).expect("small integer");
        assert!(sys_poll(&handles, integer, 0).is_err());
        assert!(sys_poll(&handles, ResourceTable::stdin_handle(), -1).is_err());
    }

    #[test]
    fn buffers_convert_exact_utf8_without_replacement() {
        let mut arena = Arena::default();
        let text = arena
            .alloc(lkjscript_core::HeapObj::Str("nul\0é".into()))
            .expect("test text allocation");
        let buffer = buf_from_str(&mut arena, text).expect("encode exact UTF-8");
        assert_eq!(as_buf(&arena, buffer).ok(), Some("nul\0é".as_bytes()));
        let round_trip = buf_to_str(&mut arena, buffer).expect("decode exact UTF-8");
        assert_eq!(
            crate::host_ext::as_str(&arena, round_trip).ok(),
            Some("nul\0é")
        );

        let invalid = buf_new(&mut arena, 2).expect("invalid buffer");
        buf_set(&mut arena, invalid, 0, 0xc3).expect("set invalid prefix");
        buf_set(&mut arena, invalid, 1, 0x28).expect("set invalid suffix");
        let conversion = buf_to_str(&mut arena, invalid);
        let result = crate::host_ext::language_result(&mut arena, conversion)
            .expect("language Result allocation");
        assert_eq!(
            crate::host_ext::is_ok(&arena, result).ok(),
            Some(Value::FALSE)
        );
        let error = crate::host_ext::unwrap_err(&arena, result).expect("ResultErr message");
        assert!(crate::host_ext::as_str(&arena, error)
            .expect("ResultErr string")
            .contains("invalid UTF-8"));
    }

    #[test]
    fn buf_slice_copies_exact_bounded_ranges() {
        let mut arena = Arena::default();
        let source = arena
            .alloc(lkjscript_core::HeapObj::Buf(vec![0, 1, 2, 3]))
            .expect("test source buffer allocation");
        let slice = buf_slice(&mut arena, source, 1, 2).expect("slice range");
        assert_eq!(as_buf(&arena, slice).expect("slice bytes"), &[1, 2]);
        assert!(buf_slice(&mut arena, source, -1, 1).is_err());
        assert!(buf_slice(&mut arena, source, 3, 2).is_err());
    }

    #[test]
    fn bulk_file_io_is_bounded_exact_and_reports_progress() -> std::io::Result<()> {
        let input = TempFile::new(&[0, 0xc3, 0xa9, 0xff, b'x'])?;
        let output = TempFile::new(&[])?;
        let mut arena = Arena::default();
        let mut handles = ResourceTable::default();
        let buffer = buf_new(&mut arena, 8).expect("bulk buffer");
        let input_handle = handles
            .sys_open_read(&input.0.to_string_lossy())
            .expect("open input");
        assert_eq!(
            sys_read_into(&mut arena, &handles, input_handle, buffer, 1, 7).ok(),
            Some(5)
        );
        assert_eq!(
            &as_buf(&arena, buffer).expect("buffer")[1..6],
            &[0, 0xc3, 0xa9, 0xff, b'x']
        );
        assert_eq!(
            sys_read_into(&mut arena, &handles, input_handle, buffer, 0, 1).ok(),
            Some(0)
        );
        assert!(sys_read_into(&mut arena, &handles, input_handle, buffer, -1, 1).is_err());
        assert!(sys_read_into(&mut arena, &handles, input_handle, buffer, 7, 2).is_err());
        assert!(sys_read_into(
            &mut arena,
            &handles,
            input_handle,
            buffer,
            0,
            MAX_BULK_IO_BYTES as i64 + 1,
        )
        .is_err());
        assert!(sys_read_into(
            &mut arena,
            &handles,
            Value::from_small_i64(1).expect("integer"),
            buffer,
            0,
            0,
        )
        .is_err());
        handles.close(input_handle).expect("close input");
        assert!(sys_read_into(&mut arena, &handles, input_handle, buffer, 0, 0).is_err());

        let output_handle = handles
            .sys_open_write(&output.0.to_string_lossy())
            .expect("open output");
        assert_eq!(
            sys_write_from(&arena, &handles, output_handle, buffer, 1, 5).ok(),
            Some(5)
        );
        assert_eq!(
            sys_write_from(&arena, &handles, output_handle, buffer, 0, 0).ok(),
            Some(0)
        );
        handles.close(output_handle).expect("close output");
        assert_eq!(fs::read(&output.0)?, vec![0, 0xc3, 0xa9, 0xff, b'x']);
        Ok(())
    }

    #[test]
    fn random_fill_obeys_exact_bounded_ranges() {
        let mut arena = Arena::default();
        let buffer = buf_new(&mut arena, 8).expect("buffer");
        for index in 0..8 {
            buf_set(&mut arena, buffer, index, 0xaa).expect("initialize buffer");
        }
        assert_eq!(
            sys_random_fill(&mut arena, buffer, 2, 4).ok(),
            Some(Value::UNIT)
        );
        let bytes = as_buf(&arena, buffer).expect("filled buffer");
        assert_eq!(&bytes[..2], &[0xaa, 0xaa]);
        assert_eq!(&bytes[6..], &[0xaa, 0xaa]);
        assert_ne!(&bytes[2..6], &[0, 0, 0, 0]);
        assert!(sys_random_fill(&mut arena, buffer, -1, 1).is_err());
        assert!(sys_random_fill(&mut arena, buffer, 7, 2).is_err());
        assert!(sys_random_fill(&mut arena, buffer, 0, MAX_BULK_IO_BYTES as i64 + 1,).is_err());
    }

    #[test]
    fn sha256_returns_new_exact_digests_for_bounded_binary_ranges() {
        let mut arena = Arena::default();
        let input = buf_new(&mut arena, 6).expect("input buffer");
        for (index, byte) in [b'!', b'a', b'b', b'c', 0, b'?'].into_iter().enumerate() {
            buf_set(&mut arena, input, index as i64, i64::from(byte)).expect("set input byte");
        }
        let digest = sys_sha256(&mut arena, input, 1, 3).expect("hash abc range");
        assert_ne!(digest, input);
        assert_eq!(as_buf(&arena, digest).expect("digest buffer").len(), 32);
        assert_eq!(
            as_buf(&arena, digest).expect("digest buffer"),
            &[
                0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
                0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
                0xf2, 0x00, 0x15, 0xad,
            ]
        );
        let empty = sys_sha256(&mut arena, input, 0, 0).expect("hash empty range");
        assert_eq!(
            as_buf(&arena, empty).expect("empty digest"),
            &[
                0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
                0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
                0x78, 0x52, 0xb8, 0x55,
            ]
        );
        let binary = buf_new(&mut arena, 1).expect("binary buffer");
        let binary_digest = sys_sha256(&mut arena, binary, 0, 1).expect("hash NUL byte");
        assert_eq!(
            as_buf(&arena, binary_digest).expect("binary digest"),
            &[
                0x6e, 0x34, 0x0b, 0x9c, 0xff, 0xb3, 0x7a, 0x98, 0x9c, 0xa5, 0x44, 0xe6, 0xbb, 0x78,
                0x0a, 0x2c, 0x78, 0x90, 0x1d, 0x3f, 0xb3, 0x37, 0x38, 0x76, 0x85, 0x11, 0xa3, 0x06,
                0x17, 0xaf, 0xa0, 0x1d,
            ]
        );
        assert!(sys_sha256(&mut arena, input, -1, 1).is_err());
        assert!(sys_sha256(&mut arena, input, 5, 2).is_err());
        assert!(sys_sha256(&mut arena, input, 0, MAX_BULK_IO_BYTES as i64 + 1).is_err());
        assert!(sys_sha256(&mut arena, Value::UNIT, 0, 0).is_err());
    }

    #[test]
    fn buffer_narrowing_rejects_truncation_and_wrapping() {
        let mut arena = Arena::default();
        let buffer = buf_new(&mut arena, 4).expect("buffer");
        assert!(buf_set(&mut arena, buffer, 0, -1).is_err());
        assert!(buf_set(&mut arena, buffer, 0, 256).is_err());
        assert!(buf_set_u32(&mut arena, buffer, 0, -1).is_err());
        assert!(buf_set_u32(&mut arena, buffer, 0, i64::from(u32::MAX) + 1).is_err());
        assert!(buf_set(&mut arena, buffer, 0, 255).is_ok());
        assert!(buf_set_u32(&mut arena, buffer, 0, i64::from(u32::MAX)).is_ok());
    }
}
