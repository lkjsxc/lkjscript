//! First evaluator-hosted compiler component: typed current type inventory.
//! The reader checks byte ranges; semantic type admission remains the compiler's job.
use super::*;
use std::time::Instant;

const TYPED_SOURCE: &str = include_str!("../../../../tools/selfhost/typed-input.lkjc");

fn prototype() -> &'static (KernelSnapshot, NormalizedProgram) {
    static PROTOTYPE: OnceLock<(KernelSnapshot, NormalizedProgram)> = OnceLock::new();
    PROTOTYPE.get_or_init(|| {
        let source = byte_buffer_tests::author_only(TYPED_SOURCE).unwrap();
        let program = prepare_snapshot(&source);
        (source, program)
    })
}

fn invoke_typed(name: &str, values: Vec<NormalizedValue>) -> NormalizedValue {
    let (source, program) = prototype();
    let (result, work) = NormalizedVm::for_test(program, NormalizedRunPolicy::foreground())
        .invoke(
            declaration_named(source, name),
            values,
            None,
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    assert_eq!(work.live_call_frames_after, 0);
    assert_eq!(work.live_operands_after, 0);
    result
}

fn branch(value: &NormalizedValue) -> (&str, Option<&NormalizedValue>) {
    let NormalizedValue::Variant {
        layout,
        case,
        payload,
    } = value
    else {
        panic!("expected typed reader variant");
    };
    let program = &prototype().1;
    (
        program.variants[layout.0 as usize].cases[*case as usize]
            .name
            .as_str(),
        payload.as_deref(),
    )
}

fn member<'a>(value: &'a NormalizedValue, name: &str) -> &'a NormalizedValue {
    let NormalizedValue::Record(NormalizedRecord::Nominal { layout, fields }) = value else {
        panic!("expected typed reader record");
    };
    let index = prototype().1.records[layout.0 as usize]
        .fields
        .iter()
        .position(|field| field.name.as_str() == name)
        .unwrap();
    &fields[index]
}

fn integer(value: &NormalizedValue) -> i64 {
    let NormalizedValue::I64(value) = value else {
        panic!("expected I64");
    };
    *value
}

fn octets(value: &NormalizedValue) -> &[u8] {
    let NormalizedValue::Bytes(value) = value else {
        panic!("expected Bytes");
    };
    value
}

fn table(value: &NormalizedValue) -> &crate::platform::execution::normalized::list::List {
    let (case, payload) = branch(value);
    assert_eq!(case, "ready");
    let NormalizedValue::List(entries) = payload.unwrap() else {
        panic!("expected type table");
    };
    entries
}

fn assert_error(bytes: &[u8], at: i64) {
    let result = invoke_typed("read", vec![NormalizedValue::bytes(bytes)]);
    let (case, payload) = branch(&result);
    assert_eq!(case, "error");
    let error = payload.unwrap();
    assert_eq!(integer(member(error, "at")), at);
    assert!(
        matches!(member(error, "message"), NormalizedValue::Text(message) if !message.is_empty())
    );
}

fn u32_at(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}

#[test]
fn typed_compiler_input_structured_errors_release_partial_tables() {
    let (source, program) = prototype();
    let carrier = source_fixture::raw_object_packet(source, None, None);
    let result = invoke_typed("read", vec![NormalizedValue::bytes(carrier.as_slice())]);
    assert_eq!(table(&result).len(), source.types.len());
    let reference =
        NormalizedReferenceInterpreter::new(source, program, NormalizedRunPolicy::foreground())
            .invoke(
                declaration_named(source, "read"),
                vec![NormalizedValue::bytes(carrier.as_slice())],
                None,
                &ExecutionControl::uncancelled(),
            )
            .unwrap();
    assert_eq!(raw_value_equal(&result, &reference.0), Ok(true));
    assert_eq!(reference.1.live_call_frames_after, 0);
    for length in [0, 1, 2, 4, 8, 16, 32, 64, 127] {
        assert_error(&carrier[..length], 0);
    }
    for (at, replacement, expected) in [
        (0, 0, 0),
        (8, 0, 8),
        (112, 9, 112),
        (84, u32::MAX, 84),
        (92, u32::MAX, 92),
        (96, u32::MAX, 96),
    ] {
        let mut broken = carrier.clone();
        broken[at..at + 4].copy_from_slice(&replacement.to_le_bytes());
        assert_error(&broken, expected);
    }
    // The final row fails after every earlier typed record has been accumulated.
    let span = u32_at(&carrier, 92) + (u32_at(&carrier, 12) + source.types.len() - 1) * 8;
    let mut broken = carrier.clone();
    broken[span + 4..span + 8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_error(&broken, span as i64);
    drop(result);
    drop(reference);
    assert_eq!(
        table(&invoke_typed("read", vec![NormalizedValue::bytes(carrier)])).len(),
        source.types.len()
    );
}

#[test]
#[ignore = "slow: maintained compiler type table and cursor comparison"]
fn typed_compiler_input_matches_maintained_cursor_and_type_objects() {
    let (source, old_program) = compiler();
    let carrier = source_fixture::packet(source);
    let argument = NormalizedValue::bytes(carrier.as_slice());
    prototype();
    let start = Instant::now();
    let typed = invoke_typed("read", vec![argument.clone()]);
    let typed_time = start.elapsed();
    let entries = table(&typed);
    assert_eq!(entries.len(), source.types.len());
    let control = ExecutionControl::uncancelled();
    let run_old = |name: &str, values| {
        NormalizedVm::for_test(old_program, NormalizedRunPolicy::foreground())
            .invoke(function("source-type-table", name), values, None, &control)
            .unwrap()
            .0
    };
    let start = Instant::now();
    assert_eq!(
        integer(&run_old("count", vec![argument.clone()])),
        entries.len() as i64
    );
    let old_images: Vec<_> = (0..entries.len())
        .map(|row| {
            run_old(
                "data",
                vec![argument.clone(), NormalizedValue::I64(row as i64)],
            )
        })
        .collect();
    let cursor_time = start.elapsed();
    for (row, ((digest, object), entry)) in source.types.iter().zip(entries.iter()).enumerate() {
        let (encoded_digest, encoded) = kernel::encode_type_object(object).unwrap();
        assert_eq!(*digest, encoded_digest);
        assert_eq!(integer(member(entry, "index")), row as i64);
        assert_eq!(octets(member(entry, "digest")), digest.bytes());
        assert_eq!(octets(member(entry, "image")), encoded);
        assert_eq!(octets(member(entry, "image")), octets(&old_images[row]));
    }
    let list_value = branch(&typed).1.unwrap().clone();
    for (row, (digest, _)) in source.types.iter().enumerate() {
        let result = invoke_typed(
            "lookup",
            vec![
                list_value.clone(),
                NormalizedValue::bytes(digest.bytes().as_slice()),
            ],
        );
        assert_eq!(branch(&result).0, "found");
        assert_eq!(
            integer(member(branch(&result).1.unwrap(), "index")),
            row as i64
        );
    }
    let absent = invoke_typed(
        "lookup",
        vec![list_value, NormalizedValue::bytes(&[0_u8; 32][..])],
    );
    assert_eq!(branch(&absent).0, "missing");
    eprintln!(
        "typed compiler input carrier_bytes={} types={} typed_read_ms={} cursor_read_ms={} typed_source_bytes={} typed_source_words={}",
        carrier.len(),
        entries.len(),
        typed_time.as_millis(),
        cursor_time.as_millis(),
        TYPED_SOURCE.len(),
        TYPED_SOURCE.split_whitespace().count()
    );
}
