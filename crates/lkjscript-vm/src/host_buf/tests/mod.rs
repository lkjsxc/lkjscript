#![allow(clippy::panic)]

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
        let path = std::env::temp_dir().join(format!("lkjscript-bulk-{}-{id}", std::process::id()));
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
    let round_trip = buf_to_str(&mut arena, buffer)
        .expect("decode operation")
        .expect("decode exact UTF-8");
    assert_eq!(
        crate::host_ext::as_str(&arena, round_trip).ok(),
        Some("nul\0é")
    );

    let invalid = buf_new(&mut arena, 2).expect("invalid buffer");
    buf_set(&mut arena, invalid, 0, 0xc3).expect("set invalid prefix");
    buf_set(&mut arena, invalid, 1, 0x28).expect("set invalid suffix");
    let conversion = buf_to_str(&mut arena, invalid).expect("decode operation");
    let result = crate::host_ext::utf8_result(&mut arena, conversion)
        .expect("typed UTF-8 Result allocation");
    let error = match arena.get(result).expect("Result value") {
        lkjscript_core::HeapObj::Enum {
            layout,
            physical_tag: 1,
            active_payload,
        } if layout.bytes() == lkjscript_core::RESULT_LAYOUT => active_payload[0],
        other => panic!("malformed UTF-8 Result: {other:?}"),
    };
    match arena.get(error).expect("Utf8Error value") {
        lkjscript_core::HeapObj::Enum {
            layout,
            physical_tag,
            active_payload,
        } => {
            assert_eq!(layout.bytes(), lkjscript_core::UTF8_ERROR_LAYOUT);
            assert_eq!(
                *physical_tag,
                lkjscript_core::Utf8ErrorKind::MissingContinuation.physical_tag()
            );
            assert_eq!(active_payload[0].as_small_i64(), Some(0));
        }
        other => panic!("malformed Utf8Error: {other:?}"),
    }
}

mod ranges;
mod terminal;
