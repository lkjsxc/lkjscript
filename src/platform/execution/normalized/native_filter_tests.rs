//! Binary-filter I/O, bounded child lifetimes, and independent BLAKE3 vectors.
use super::*;
use crate::platform::compiler::elf_x86_64;
use std::io::{Read, Write};
use std::process::{Command, Output, Stdio};

pub(super) fn execute_filter(
    image: &[u8],
    input: &[u8],
    arguments: &[&str],
    chunk: usize,
) -> Output {
    execute_filter_controlled(image, input, arguments, chunk, FilterHostControl::Ordinary)
}

#[derive(Clone, Copy)]
enum FilterHostControl {
    Ordinary,
    AddressSpaceKib(u32),
    UnreadableInput,
}

fn execute_filter_controlled(
    image: &[u8],
    input: &[u8],
    arguments: &[&str],
    chunk: usize,
    control: FilterHostControl,
) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("compiler");
    crate::release_container::inspect_static_elf_bytes(image).unwrap();
    crate::platform::owned_output::publish_executable_create_new(
        &path,
        image,
        Some(32 * 1024 * 1024),
        "native compiler test",
    )
    .unwrap();
    let mut command = if let FilterHostControl::AddressSpaceKib(limit) = control {
        // Apply an explicit host limit only in the child, without pre_exec or unsafe.
        let mut command = Command::new("/bin/sh");
        command
            .args([
                "-c",
                "set -e; ulimit -v \"$1\"; shift; exec \"$@\"",
                "native-filter-limit",
            ])
            .arg(limit.to_string())
            .arg(&path);
        command
    } else {
        Command::new(&path)
    };
    let stdin = if matches!(control, FilterHostControl::UnreadableInput) {
        Stdio::from(std::fs::File::open(directory.path()).unwrap())
    } else {
        Stdio::piped()
    };
    let mut child = command
        .args(arguments)
        .env_clear()
        .current_dir(directory.path())
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::scope(|scope| {
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let writer = scope.spawn(move || {
            if let Some(mut stdin) = stdin {
                for bytes in input.chunks(chunk) {
                    if let Err(error) = stdin.write_all(bytes) {
                        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
                        break;
                    }
                }
            } else {
                assert!(input.is_empty());
            }
        });
        let out = scope.spawn(move || {
            let mut bytes = Vec::new();
            stdout
                .take(32 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .unwrap();
            bytes
        });
        let err = scope.spawn(move || {
            let mut bytes = Vec::new();
            stderr.take(65537).read_to_end(&mut bytes).unwrap();
            bytes
        });
        let start = std::time::Instant::now();
        let mut timed_out = false;
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if start.elapsed() > std::time::Duration::from_secs(10) {
                timed_out = true;
                child.kill().unwrap();
                break child.wait().unwrap();
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        };
        writer.join().unwrap();
        let output = Output {
            status,
            stdout: out.join().unwrap(),
            stderr: err.join().unwrap(),
        };
        assert!(!timed_out, "native filter timed out: {output:?}");
        assert!(output.stdout.len() <= 32 * 1024 * 1024);
        assert!(output.stderr.len() <= 65536);
        output
    })
}

pub(super) fn check_filter_host_failures(image: &[u8]) {
    let unreadable =
        execute_filter_controlled(image, b"", &[], 1, FilterHostControl::UnreadableInput);
    assert_eq!(unreadable.status.code(), Some(65));
    assert!(unreadable.stdout.is_empty());
    assert_eq!(unreadable.stderr, b"native_input\n");

    let limit = FilterHostControl::AddressSpaceKib(8 * 1024);
    let small = vec![0xa5; 65_537];
    let accepted = execute_filter_controlled(image, &small, &[], 4093, limit);
    assert_eq!(accepted.status.code(), Some(0), "{:?}", accepted.stderr);
    assert!(accepted.stderr.is_empty());
    assert_eq!(accepted.stdout, small);

    // EOF at full capacity must not require another mapping under this host limit.
    let exact = vec![0x3c; 4 * 1024 * 1024];
    let accepted = execute_filter_controlled(image, &exact, &[], 4093, limit);
    assert_eq!(accepted.status.code(), Some(0), "{:?}", accepted.stderr);
    assert!(accepted.stderr.is_empty());
    assert_eq!(accepted.stdout, exact);

    let large = vec![0x5a; 8 * 1024 * 1024 + 1];
    let refused = execute_filter_controlled(image, &large, &[], 4093, limit);
    assert_eq!(refused.status.code(), Some(70));
    assert!(refused.stdout.is_empty());
    assert_eq!(refused.stderr, b"native_allocation\n");
}

fn fixture() -> (crate::platform::kernel::KernelSnapshot, NormalizedProgram) {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/native-filter.lkjc"
    ))
    .unwrap();
    let program = prepare_snapshot(&source);
    (source, program)
}

#[test]
fn native_elf_binary_filter_grows_input_and_preserves_usage_and_traps() {
    let (source, program) = fixture();
    let build = |name| {
        let index = program
            .function(declaration_named(&source, name))
            .unwrap()
            .0 as usize;
        elf_x86_64::compile_function(&program, index).unwrap().0
    };
    let echo = build("echo");
    for length in [
        0,
        1,
        256,
        65_535,
        65_536,
        65_537,
        256 * 1024,
        16 * 1024 * 1024,
        16 * 1024 * 1024 + 1,
        20 * 1024 * 1024 + 17,
    ] {
        let bytes: Vec<_> = (0..length)
            .map(|i| (i ^ (i >> 8) ^ (i >> 16)) as u8)
            .collect();
        let output = execute_filter(&echo, &bytes, &[], if length < 1024 { 1 } else { 65521 });
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        assert!(output.stdout == bytes, "echo length {length}");
    }
    check_filter_host_failures(&echo);
    let output = execute_filter(&echo, b"", &["extra"], 1);
    assert_eq!(output.status.code(), Some(64));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"native_argument\n");
    let output = execute_filter(&build("reject"), b"secret output", &[], 1);
    assert_eq!(output.status.code(), Some(65));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"compiler_rejected:-123\n");
    let output = execute_filter(&build("trap"), b"", &[], 1);
    assert_eq!(output.status.code(), Some(70));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"normalized_integer_division\n");
}

#[test]
fn native_elf_blake3_matches_independent_block_chunk_and_tree_vectors() {
    let (source, program) = fixture();
    let declaration = declaration_named(&source, "hash");
    let index = program.function(declaration).unwrap().0 as usize;
    let image = elf_x86_64::compile_function(&program, index).unwrap().0;
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/blake3-unkeyed-vectors.json"
    ))
    .unwrap();
    for vector in vectors["cases"].as_array().unwrap() {
        let length = vector["input_len"].as_u64().unwrap() as usize;
        let input: Vec<_> = (0..length).map(|i| (i % 251) as u8).collect();
        let expected = vector["hash"].as_str().unwrap();
        let output = execute_filter(&image, &input, &[], 127);
        assert_eq!(output.status.code(), Some(0), "{length}: {output:?}");
        assert!(output.stderr.is_empty());
        assert_eq!(
            crate::platform::semantic_id::encode_hex(&output.stdout),
            expected,
            "length {length}"
        );
        let policy = NormalizedRunPolicy::default();
        let control = ExecutionControl::uncancelled();
        let values = vec![NormalizedValue::bytes(input.as_slice())];
        let vm = NormalizedVm::for_test(&program, policy)
            .invoke(declaration, values.clone(), None, &control)
            .unwrap()
            .0;
        let reference = NormalizedReferenceInterpreter::new(&source, &program, policy)
            .invoke(declaration, values, None, &control)
            .unwrap()
            .0;
        assert_eq!(vm, reference);
        let super::super::value::NormalizedRecord::Structural { fields } = (match vm {
            NormalizedValue::Record(record) => record,
            _ => panic!("record"),
        }) else {
            panic!("structural record")
        };
        let NormalizedValue::Bytes(bytes) = &fields
            .iter()
            .find(|(n, _)| n.as_str() == "image")
            .unwrap()
            .1
        else {
            panic!("bytes")
        };
        assert_eq!(output.stdout, bytes.as_ref());
    }
}
