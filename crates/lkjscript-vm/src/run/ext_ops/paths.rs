use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    match op {
        x if x == Op::PathFromStr as u8 => {
            let value = vm.pop()?;
            let result = crate::run::structural_ops::copy_string(vm, value)
                .and_then(|text| crate::host_ext::copy_validated_path(text.as_bytes()))
                .map(crate::run::structural_ops::HostValue::Path);
            push_language_result(
                vm,
                lkjscript_core::SystemErrorKind::Io,
                crate::run::structural_ops::HostValueType::Path,
                result,
            );
            Ok(true)
        }
        x if x == Op::PathFromBytes as u8 => {
            let value = vm.pop()?;
            let result = exact_bytes(vm, value)
                .and_then(|bytes| crate::host_ext::copy_validated_path(&bytes))
                .map(crate::run::structural_ops::HostValue::Path);
            push_language_result(
                vm,
                lkjscript_core::SystemErrorKind::Io,
                crate::run::structural_ops::HostValueType::Path,
                result,
            );
            Ok(true)
        }
        x if x == Op::PathToBytes as u8 => {
            let value = vm.pop()?;
            let bytes = crate::run::structural_ops::copy_path(vm, value)?;
            let bytes = vm.unique.allocate_bytes(bytes)?;
            vm.push(bytes);
            Ok(true)
        }
        x if x == Op::PathToStr as u8 => {
            let value = vm.pop()?;
            let bytes = crate::run::structural_ops::copy_path(vm, value)?;
            let result = lkjscript_core::validate_utf8(&bytes)
                .map(|text| crate::run::structural_ops::HostValue::String(text.to_owned()));
            let result = crate::run::structural_ops::publish_utf8_result(vm, result)?;
            vm.push(result);
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn exact_bytes<J: RuntimeTier>(vm: &mut Vm<'_, J>, value: Value) -> Result<Vec<u8>> {
    if let Some(index) = value.as_static_bytes() {
        return match vm.chunk.constants().get(usize::from(index)) {
            Some(lkjscript_core::Constant::StaticBytes(bytes)) => Ok(bytes.to_vec()),
            _ => Err(Error::msg("stale static bytes constant")),
        };
    }
    vm.unique.copy_bytes(value)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use lkjscript_core::{validate_chunk, Chunk, ExecutionConfig, ValidationLimits};

    use crate::run::NoTier;

    fn validated_unit() -> lkjscript_core::ValidatedChunk {
        let mut chunk = Chunk::new();
        chunk.main.emit(Op::Unit);
        chunk.main.emit(Op::Return);
        validate_chunk(chunk, &ValidationLimits::default()).expect("unit chunk")
    }

    #[test]
    fn path_result_requires_exact_structural_metadata_without_traced_fallback() {
        let chunk = validated_unit();
        let mut vm = Vm::new(
            &chunk,
            NoTier,
            crate::ExecutionInputs::default(),
            ExecutionConfig::default(),
        );
        let input = vm
            .unique
            .allocate_bytes(vec![b'/', 0xff, b'x'])
            .expect("bytes");
        vm.push(input);
        assert!(dispatch(&mut vm, Op::PathFromBytes as u8).expect("from bytes"));
        assert!(vm.stack.is_empty());
        assert!(vm.allocation_error.as_ref().is_some_and(|error| error
            .to_string()
            .contains("lacks exact structural type metadata")));
    }

    #[test]
    fn path_validation_rejects_invalid_inputs_and_accepts_the_limit() {
        for bytes in [b"".as_slice(), b"relative", b"/nul\0byte"] {
            assert!(crate::host_ext::copy_validated_path(bytes).is_err());
        }
        let mut maximum = vec![b'x'; crate::host_ext::MAX_PATH_BYTES];
        maximum[0] = b'/';
        assert!(crate::host_ext::copy_validated_path(&maximum).is_ok());
        let oversized = vec![b'x'; crate::host_ext::MAX_PATH_BYTES + 1];
        let mut oversized = [vec![b'/'], oversized].concat();
        oversized.truncate(crate::host_ext::MAX_PATH_BYTES + 1);
        assert!(crate::host_ext::copy_validated_path(&oversized).is_err());
    }
}
