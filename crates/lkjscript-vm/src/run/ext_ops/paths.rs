use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    match op {
        x if x == Op::PathFromStr as u8 => {
            let value = vm.pop()?;
            let bytes = crate::host_ext::as_str(&vm.arena, value)?.as_bytes();
            let object = crate::host_ext::path_object(bytes);
            let result = match object {
                Ok(object) => vm.arena.alloc(object),
                Err(error) => Err(error),
            };
            push_language_result(vm, lkjscript_core::SystemErrorKind::Io, result);
            Ok(true)
        }
        x if x == Op::PathFromBuf as u8 => {
            let value = vm.pop()?;
            let bytes = crate::host_buf::as_buf(&vm.arena, value)?;
            let object = crate::host_ext::path_object(bytes);
            let result = match object {
                Ok(object) => vm.arena.alloc(object),
                Err(error) => Err(error),
            };
            push_language_result(vm, lkjscript_core::SystemErrorKind::Io, result);
            Ok(true)
        }
        x if x == Op::PathToBuf as u8 => {
            let value = vm.pop()?;
            let bytes = crate::host_ext::as_path(&vm.arena, value)?;
            let object = crate::host_ext::path_buffer_object(bytes)?;
            let buffer = vm.arena.alloc(object)?;
            vm.push(buffer);
            Ok(true)
        }
        x if x == Op::PathToStr as u8 => {
            let value = vm.pop()?;
            let bytes = crate::host_ext::as_path(&vm.arena, value)?;
            let validated = lkjscript_core::validate_utf8(bytes).map(str::to_owned);
            let result = match validated {
                Ok(text) => Ok(vm.arena.alloc(HeapObj::Str(text))?),
                Err(error) => Err(error),
            };
            let result = crate::host_ext::utf8_result(&mut vm.arena, result)?;
            vm.push(result);
            Ok(true)
        }
        _ => Ok(false),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;
    use lkjscript_core::{validate_chunk, Chunk, ExecutionConfig, GcHeap, ValidationLimits};

    use crate::run::NoTier as NullJit;

    fn validated_unit() -> lkjscript_core::ValidatedChunk {
        let mut chunk = Chunk::new();
        chunk.main.emit(Op::Unit);
        chunk.main.emit(Op::Return);
        validate_chunk(chunk, &ValidationLimits::default()).expect("unit chunk")
    }

    fn result_payload(vm: &Vm<'_, NullJit>, result: Value, ok: bool) -> Value {
        let HeapObj::Enum {
            physical_tag,
            active_payload,
            ..
        } = vm.arena.get(result).expect("Result object")
        else {
            panic!("expected Result");
        };
        assert_eq!(*physical_tag, u16::from(!ok));
        assert_eq!(active_payload.len(), 1);
        active_payload[0]
    }

    #[test]
    fn path_buf_roundtrip_preserves_non_utf8_bytes_and_observation_fails() {
        let chunk = validated_unit();
        let mut vm = Vm::new(
            &chunk,
            NullJit,
            crate::ExecutionInputs::default(),
            ExecutionConfig::default(),
        );
        let bytes = vec![b'/', 0xff, b'x'];
        let buffer = vm.arena.alloc(HeapObj::Buf(bytes.clone())).expect("buffer");
        vm.push(buffer);
        assert!(dispatch(&mut vm, Op::PathFromBuf as u8).expect("from buffer"));
        let result = vm.pop().expect("path result");
        let path = result_payload(&vm, result, true);
        assert_eq!(crate::host_ext::as_path(&vm.arena, path).unwrap(), bytes);

        vm.push(path);
        assert!(dispatch(&mut vm, Op::PathToBuf as u8).expect("to buffer"));
        let observed = vm.pop().expect("observed buffer");
        assert_eq!(crate::host_buf::as_buf(&vm.arena, observed).unwrap(), bytes);

        vm.push(path);
        assert!(dispatch(&mut vm, Op::PathToStr as u8).expect("to string"));
        let result = vm.pop().expect("UTF-8 result");
        let _error = result_payload(&vm, result, false);
    }

    #[test]
    fn path_construction_rejects_empty_relative_nul_and_oversized_bytes() {
        let chunk = validated_unit();
        let mut vm = Vm::new(
            &chunk,
            NullJit,
            crate::ExecutionInputs::default(),
            ExecutionConfig::default(),
        );
        for text in ["", "relative", "/nul\0byte"] {
            let string = vm.arena.alloc(HeapObj::Str(text.into())).expect("string");
            vm.push(string);
            dispatch(&mut vm, Op::PathFromStr as u8).expect("invalid constructor result");
            let result = vm.pop().expect("invalid Path result");
            let _error = result_payload(&vm, result, false);
        }
        for bytes in [b"".as_slice(), b"relative", b"/nul\0byte"] {
            assert!(crate::host_ext::allocate_path(&mut GcHeap::default(), bytes).is_err());
        }
        let mut maximum = vec![b'x'; crate::host_ext::MAX_PATH_BYTES];
        maximum[0] = b'/';
        assert!(crate::host_ext::allocate_path(&mut GcHeap::default(), &maximum).is_ok());
        let oversized = vec![b'x'; crate::host_ext::MAX_PATH_BYTES + 1];
        let mut oversized = [vec![b'/'], oversized].concat();
        oversized.truncate(crate::host_ext::MAX_PATH_BYTES + 1);
        assert!(crate::host_ext::allocate_path(&mut GcHeap::default(), &oversized).is_err());
    }

    #[test]
    fn path_string_roundtrip_is_explicit_and_exact() {
        let chunk = validated_unit();
        let mut vm = Vm::new(
            &chunk,
            NullJit,
            crate::ExecutionInputs::default(),
            ExecutionConfig::default(),
        );
        let text = vm
            .arena
            .alloc(HeapObj::Str("/tmp/exact-path".into()))
            .expect("string");
        vm.push(text);
        dispatch(&mut vm, Op::PathFromStr as u8).expect("from string");
        let result = vm.pop().expect("path result");
        let path = result_payload(&vm, result, true);
        vm.push(path);
        dispatch(&mut vm, Op::PathToStr as u8).expect("to string");
        let result = vm.pop().expect("string result");
        let string = result_payload(&vm, result, true);
        assert_eq!(
            crate::host_ext::as_str(&vm.arena, string).unwrap(),
            "/tmp/exact-path"
        );
    }
}
