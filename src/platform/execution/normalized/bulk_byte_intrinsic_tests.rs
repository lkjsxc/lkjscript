//! Public source and independent values establish the new intrinsic contracts.
use super::super::byte_buffer::StorageObservation;
use super::super::value::NormalizedRecord;
use super::*;
use crate::platform::execution::ExecutionFailureClass;
type Check = Result<(), String>;
fn checked<T, E: std::fmt::Debug>(result: Result<T, E>) -> Result<T, String> {
    result.map_err(|error| format!("{error:?}"))
}
const FIXTURE: &str = include_str!("../../../../tests/fixtures/bulk-byte-intrinsics.lkjc");
fn image(program: &NormalizedProgram, entry: DeclarationReference) -> Result<Vec<u8>, String> {
    let index = program.function(entry).ok_or("missing fixture function")?.0;
    Ok(
        checked(crate::platform::compiler::elf_x86_64::compile_function(
            program,
            index as usize,
        ))?
        .0,
    )
}
#[test]
fn bulk_intrinsics_public_words_and_range_traps_agree_across_host_engines() -> Check {
    let source = byte_buffer_tests::author_only(FIXTURE)?;
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let control = ExecutionControl::uncancelled();
    for (name, expected) in [
        ("minimum", Some(i64::MIN)),
        ("negative", Some(-1)),
        ("zero", Some(0)),
        ("positive", Some(0x0102030405060708)),
        ("maximum", Some(i64::MAX)),
        ("negative-start", None),
        ("reversed", None),
        ("beyond-end", None),
        ("maximum-start", None),
        ("word-negative", None),
        ("word-truncated", None),
        ("word-overflow", None),
    ] {
        let entry = declaration_named(&source, name);
        let storage = StorageObservation::start();
        let vm = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
            .invoke(entry, vec![], None, &control)
            .map(|pair| pair.0);
        let reference = NormalizedReferenceInterpreter::new(
            &source,
            &program,
            NormalizedRunPolicy::foreground(),
        )
        .invoke(entry, vec![], None, &control)
        .map(|pair| pair.0);
        for result in [vm, reference] {
            if let Some(value) = expected {
                assert_eq!(checked(result)?, NormalizedValue::I64(value), "{name}");
            } else {
                let error = result.err().ok_or("invalid range accepted")?;
                assert_eq!(error.class, ExecutionFailureClass::Trap, "{name}");
                assert_eq!(error.code, "normalized_bytes_range", "{name}");
            }
        }
        assert_eq!(storage.live(), (0, 0), "{name}");
        let output = native_filter_tests::execute_filter(&image(&program, entry)?, &[], &[], 127);
        if let Some(value) = expected {
            assert_eq!(output.status.code(), Some(0), "{name}: {output:?}");
            assert_eq!(output.stdout, value.to_string().as_bytes());
            assert!(output.stderr.is_empty());
        } else {
            assert_eq!(output.status.code(), Some(70), "{name}: {output:?}");
            assert!(output.stdout.is_empty());
            assert_eq!(output.stderr, b"normalized_bytes_range\n");
        }
    }
    Ok(())
}
#[test]
fn bulk_intrinsics_public_binary_payloads_and_ownership_match_host_native() -> Check {
    let source = byte_buffer_tests::author_only(FIXTURE)?;
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "binary");
    let executable = image(&program, entry)?;
    for input in [
        Vec::new(),
        vec![0, 255, 128, 42],
        (0..131_073).map(|i| (i % 251) as u8).collect(),
    ] {
        let mut expected = i64::MIN.to_le_bytes().to_vec();
        expected.extend_from_slice(&input);
        expected.extend_from_slice(&i64::MAX.to_le_bytes());
        let storage = StorageObservation::start();
        let control = ExecutionControl::uncancelled();
        for reference in [false, true] {
            let arguments = vec![NormalizedValue::bytes(input.as_slice())];
            let result = if reference {
                checked(
                    NormalizedReferenceInterpreter::new(
                        &source,
                        &program,
                        NormalizedRunPolicy::foreground(),
                    )
                    .invoke(entry, arguments, None, &control),
                )?
                .0
            } else {
                checked(
                    NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                        .invoke(entry, arguments, None, &control),
                )?
                .0
            };
            let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = result else {
                return Err("binary result not a structural record".to_owned());
            };
            let diagnostic = fields
                .iter()
                .find(|(name, _)| name.as_str() == "diagnostic")
                .ok_or("diagnostic absent")?;
            assert_eq!(diagnostic.1, NormalizedValue::I64(0));
            let image = fields
                .iter()
                .find(|(name, _)| name.as_str() == "image")
                .ok_or("image absent")?;
            assert_eq!(image.1, NormalizedValue::bytes(expected.as_slice()));
        }
        assert_eq!(storage.live(), (0, 0));
        let native = native_filter_tests::execute_filter(&executable, &input, &[], 4093);
        assert_eq!(native.status.code(), Some(0), "{native:?}");
        assert!(native.stderr.is_empty());
        assert_eq!(native.stdout, expected);
    }
    Ok(())
}
