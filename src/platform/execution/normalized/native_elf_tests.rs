//! Direct process execution agrees with separately checked bytecode and canonical syntax.
use super::*;
use crate::platform::compiler::{egraph_opt, elf_x86_64};
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Output, Stdio};

fn execute(bytes: &[u8], arguments: &[i64]) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("program");
    crate::platform::owned_output::publish_executable_create_new(
        &path,
        bytes,
        Some(32 * 1024 * 1024),
        "test ELF",
    )
    .unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o755
    );
    crate::release_container::inspect_static_elf_bytes(bytes).unwrap();
    let mut child = Command::new(&path)
        .args(arguments.iter().map(ToString::to_string))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let started = std::time::Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if started.elapsed() > std::time::Duration::from_secs(5) {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!("native process exceeded its bound: {output:?}");
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    child.wait_with_output().unwrap()
}

fn fixture() -> (crate::platform::kernel::KernelSnapshot, NormalizedProgram) {
    let input = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native-elf.lkjc"),
    )
    .unwrap();
    let source = byte_buffer_tests::author_only(&input).unwrap();
    let program = prepare_snapshot(&source);
    (source, program)
}

#[test]
fn native_elf_compiler_byte_operations_preserve_octets_text_ranges_and_traps() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/native-bytes.lkjc"
    ))
    .unwrap();
    let program = prepare_snapshot(&source);
    for (name, arguments, expected, trap) in [
        ("octet", vec![0], "0", ""),
        ("octet", vec![128], "128", ""),
        ("octet", vec![255], "255", ""),
        ("index", vec![0], "0", ""),
        ("index", vec![2], "255", ""),
        ("empty-length", vec![], "0", ""),
        ("same-octets", vec![], "true", ""),
        ("different-octets", vec![], "false", ""),
        ("different-length", vec![], "false", ""),
        ("empty-equal", vec![], "true", ""),
        ("text-octets", vec![0], "0", ""),
        ("text-octets", vec![1], "10", ""),
        ("text-octets", vec![2], "34", ""),
        ("text-octets", vec![3], "92", ""),
        ("text-octets", vec![4], "231", ""),
        ("text-octets", vec![5], "140", ""),
        ("text-octets", vec![6], "171", ""),
        ("decimal-octets", vec![i64::MIN, 0], "45", ""),
        ("decimal-octets", vec![i64::MIN, 19], "56", ""),
        ("decimal-octets", vec![i64::MAX, 18], "55", ""),
        ("decimal-length", vec![i64::MIN], "20", ""),
        ("decimal-length", vec![0], "1", ""),
        ("range-length", vec![0, 3], "3", ""),
        ("range-length", vec![3, 3], "0", ""),
        ("range-length", vec![0, 0], "0", ""),
        ("copied-range", vec![], "true", ""),
        ("concatenated", vec![], "true", ""),
        ("empty-concat", vec![], "true", ""),
        ("copied-empty", vec![], "0", ""),
        ("buffer-roundtrip", vec![0], "0", ""),
        ("buffer-roundtrip", vec![128], "128", ""),
        ("buffer-roundtrip", vec![255], "255", ""),
        ("buffer-size", vec![0], "0", ""),
        ("buffer-size", vec![8], "8", ""),
        ("buffer-size", vec![9], "9", ""),
        ("buffer-size", vec![16384], "16384", ""),
        ("buffer-growth", vec![16385, 16384], "255", ""),
        ("buffer-growth", vec![9, 0], "255", ""),
        ("buffer-discard", vec![], "null", ""),
        ("hash-length", vec![], "32", ""),
        ("buffer-roundtrip", vec![-1], "", "buffer_octet"),
        ("buffer-roundtrip", vec![256], "", "buffer_octet"),
        ("buffer-growth", vec![0, 0], "", "buffer_index"),
        ("buffer-growth", vec![8, 8], "", "buffer_index"),
        ("buffer-growth", vec![8, -1], "", "buffer_index"),
        ("octet", vec![-1], "", "bytes_octet"),
        ("octet", vec![256], "", "bytes_octet"),
        ("octet", vec![i64::MIN], "", "bytes_octet"),
        ("octet", vec![i64::MAX], "", "bytes_octet"),
        ("index", vec![-1], "", "bytes_index"),
        ("index", vec![3], "", "bytes_index"),
        ("index", vec![i64::MIN], "", "bytes_index"),
        ("index", vec![i64::MAX], "", "bytes_index"),
        ("empty-index", vec![], "", "bytes_index"),
        ("range-length", vec![-1, 0], "", "bytes_range"),
        ("range-length", vec![0, -1], "", "bytes_range"),
        ("range-length", vec![-2, -1], "", "bytes_range"),
        ("range-length", vec![2, 1], "", "bytes_range"),
        ("range-length", vec![0, 4], "", "bytes_range"),
        ("range-length", vec![i64::MIN, i64::MAX], "", "bytes_range"),
    ] {
        let declaration = declaration_named(&source, name);
        let index = program.function(declaration).unwrap().0 as usize;
        let bytes = elf_x86_64::compile_function(&program, index).unwrap().0;
        let output = execute(&bytes, &arguments);
        let values = arguments
            .iter()
            .map(|v| NormalizedValue::I64(*v))
            .collect::<Vec<_>>();
        let control = ExecutionControl::uncancelled();
        let policy = NormalizedRunPolicy::default();
        let vm = NormalizedVm::for_test(&program, policy).invoke(
            declaration,
            values.clone(),
            None,
            &control,
        );
        let reference = NormalizedReferenceInterpreter::new(&source, &program, policy).invoke(
            declaration,
            values,
            None,
            &control,
        );
        if trap.is_empty() {
            assert!(output.status.success(), "{name} {arguments:?}: {output:?}");
            assert!(output.stderr.is_empty());
            assert_eq!(output.stdout, expected.as_bytes(), "{name} {arguments:?}");
            let vm = vm.unwrap().0;
            assert_eq!(vm, reference.unwrap().0);
            assert_eq!(
                encode_typed(
                    &program,
                    &vm,
                    program.functions[index].result.ty,
                    JsonLimits::default()
                )
                .unwrap(),
                expected.as_bytes()
            );
        } else {
            assert_eq!(
                output.status.code(),
                Some(70),
                "{name} {arguments:?}: {output:?}"
            );
            assert!(output.stdout.is_empty());
            assert_eq!(output.stderr, format!("normalized_{trap}\n").as_bytes());
            assert_eq!(vm.unwrap_err().code, format!("normalized_{trap}"));
            let reference_prefix = if trap.starts_with("buffer_") {
                "normalized"
            } else {
                "reference"
            };
            assert_eq!(
                reference.unwrap_err().code,
                format!("{reference_prefix}_{trap}")
            );
        }
    }
}

#[test]
fn native_elf_executes_runtime_arguments_recursion_closures_and_structured_json() {
    let (source, program) = fixture();
    for (name, arguments, expected) in [
        ("arithmetic", vec![4, -2], "-12"),
        ("arithmetic", vec![7, 3], "14"),
        ("constant", vec![], "42"),
        ("branch", vec![-9], "9"),
        ("branch", vec![9], "12"),
        ("sum", vec![2000, 0], "2001000"),
        ("captured", vec![], "42"),
        ("collection", vec![], "42"),
        (
            "nominal",
            vec![],
            "{\"active\":true,\"empty\":null,\"value\":42}",
        ),
        ("division", vec![1], "-9223372036854775808"),
        ("untaken", vec![], "3"),
        (
            "aggregate",
            vec![],
            "{\"answer\":42,\"text\":\"a\\n\\\"b\",\"values\":[-9223372036854775808,0,9223372036854775807]}",
        ),
    ] {
        let declaration = declaration_named(&source, name);
        let index = program.function(declaration).unwrap().0 as usize;
        let (bytes, _, _) = elf_x86_64::compile_function(&program, index).unwrap();
        let second = elf_x86_64::compile_function(&program, index).unwrap().0;
        assert_eq!(bytes, second);
        let output = execute(&bytes, &arguments);
        assert!(output.status.success(), "{name}: {output:?}");
        assert!(output.stderr.is_empty());
        let values: Vec<_> = arguments.iter().map(|v| NormalizedValue::I64(*v)).collect();
        let policy = NormalizedRunPolicy::foreground();
        let control = ExecutionControl::uncancelled();
        let (vm, _) = NormalizedVm::for_test(&program, policy)
            .invoke(declaration, values.clone(), None, &control)
            .unwrap();
        let (reference, _) = NormalizedReferenceInterpreter::new(&source, &program, policy)
            .invoke(declaration, values, None, &control)
            .unwrap();
        assert_eq!(vm, reference, "{name}");
        let encoded = encode_typed(
            &program,
            &vm,
            program.functions[index].result.ty,
            JsonLimits::default(),
        )
        .unwrap();
        assert_eq!(encoded, expected.as_bytes(), "independent {name}");
        assert_eq!(output.stdout, encoded, "native {name}");
    }
}

#[test]
fn native_elf_checked_traps_and_unsupported_code_never_become_build_time_results() {
    let (source, program) = fixture();
    for (name, arguments, code) in [
        ("overflow", vec![], "normalized_integer_overflow"),
        ("annihilator", vec![], "normalized_integer_overflow"),
        ("division", vec![0], "normalized_integer_division"),
        ("division", vec![-1], "normalized_integer_division"),
        ("branch", vec![i64::MIN], "normalized_integer_overflow"),
    ] {
        let declaration = declaration_named(&source, name);
        let index = program.function(declaration).unwrap().0 as usize;
        let bytes = elf_x86_64::compile_function(&program, index).unwrap().0;
        let output = execute(&bytes, &arguments);
        assert_eq!(output.status.code(), Some(70));
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, format!("{code}\n").as_bytes());
        let values: Vec<_> = arguments.iter().map(|v| NormalizedValue::I64(*v)).collect();
        let policy = NormalizedRunPolicy::foreground();
        let control = ExecutionControl::uncancelled();
        assert_eq!(
            NormalizedVm::for_test(&program, policy)
                .invoke(declaration, values.clone(), None, &control)
                .unwrap_err()
                .code,
            code
        );
        assert_eq!(
            NormalizedReferenceInterpreter::new(&source, &program, policy)
                .invoke(declaration, values, None, &control)
                .unwrap_err()
                .code,
            code.replace("normalized_", "reference_")
        );
    }
    let index = program
        .function(declaration_named(&source, "unsupported"))
        .unwrap()
        .0 as usize;
    assert_eq!(
        elf_x86_64::compile_function(&program, index)
            .unwrap_err()
            .code,
        "native_elf_unsupported"
    );
}

#[test]
fn native_egraph_plan_is_bounded_and_preserves_exact_vm_quota_boundaries() {
    let (source, program) = fixture();
    let declaration = declaration_named(&source, "constant");
    let index = program.function(declaration).unwrap().0 as usize;
    let NormalizedFunctionBody::Code(code) = &program.functions[index].body else {
        panic!("code");
    };
    let original = code.clone();
    let plan = egraph_opt::optimize(&program, code);
    assert!(!plan.rewrites.is_empty());
    assert!(plan.iterations <= egraph_opt::MAX_ITERATIONS);
    assert!(plan.nodes <= egraph_opt::MAX_NODES);
    assert_eq!(code, &original);
    let mut many = original.clone();
    many.instructions = vec![NormalizedInstruction::I64(1); egraph_opt::MAX_NODES * 2].into();
    let bounded = egraph_opt::optimize(&program, &many);
    assert_eq!(bounded.nodes, egraph_opt::MAX_NODES);
    many.instructions =
        vec![NormalizedInstruction::I64(1); egraph_opt::MAX_INSTRUCTIONS + 1].into();
    assert_eq!(egraph_opt::optimize(&program, &many).nodes, 0);
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::foreground();
    let before = NormalizedVm::for_test(&program, policy)
        .invoke(declaration, vec![], None, &control)
        .unwrap();
    elf_x86_64::compile_function(&program, index).unwrap();
    let after = NormalizedVm::for_test(&program, policy)
        .invoke(declaration, vec![], None, &control)
        .unwrap();
    assert_eq!(before, after);
    for (steps, allocation, succeeds) in [
        (before.1.instructions, before.1.allocated_bytes, true),
        (before.1.instructions - 1, before.1.allocated_bytes, false),
        (before.1.instructions, before.1.allocated_bytes - 1, false),
    ] {
        let policy = NormalizedRunPolicy {
            instruction_steps: Some(steps),
            maximum_allocated_bytes: Some(allocation),
            ..policy
        };
        assert_eq!(
            NormalizedVm::for_test(&program, policy)
                .invoke(declaration, vec![], None, &control)
                .is_ok(),
            succeeds
        );
    }
}

#[test]
fn native_egraph_fuses_exact_standard_map_fold_only_after_trap_free_proof() {
    let memory = crate::platform::MemProjectStore::new(None).unwrap();
    crate::platform::project_creation::create_memory_project(&memory, "fusion", "command").unwrap();
    let repository = GraphRepository::open_memory(&memory).unwrap();
    let input = format!(
        "request base={}\n{}",
        repository.view_current().unwrap().revision(),
        include_str!("../../../../tests/fixtures/native-fusion.lkjc")
    );
    let decoded =
        crate::platform::control::decode_compact_change("native-fusion", input.as_bytes()).unwrap();
    let authored = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    repository.publish(&authored.publication).unwrap();
    let source = repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    let prepared = crate::platform::normalized_lifecycle::prepare_repository(repository).unwrap();
    let program = &prepared.program;
    for (name, arguments, fused) in [
        ("total", vec![], true),
        ("identity-total", vec![3], true),
        ("runtime-total", vec![3], false),
        ("trapped", vec![], false),
    ] {
        let declaration = declaration_named(&source, name);
        let index = program.function(declaration).unwrap().0 as usize;
        let NormalizedFunctionBody::Code(code) = &program.functions[index].body else {
            panic!("code");
        };
        assert_eq!(
            egraph_opt::optimize(program, code).fusions > 0,
            fused,
            "{name}"
        );
        let bytes = elf_x86_64::compile_function(program, index).unwrap().0;
        let output = execute(&bytes, &arguments);
        let values: Vec<_> = arguments.iter().map(|v| NormalizedValue::I64(*v)).collect();
        let policy = NormalizedRunPolicy::foreground();
        let control = ExecutionControl::uncancelled();
        let vm = NormalizedVm::for_test(program, policy).invoke(
            declaration,
            values.clone(),
            None,
            &control,
        );
        let reference =
            NormalizedReferenceInterpreter::from_reader(&prepared.reference, program, policy)
                .invoke(declaration, values, None, &control);
        if name == "trapped" {
            assert_eq!(output.status.code(), Some(70));
            assert!(output.stdout.is_empty());
            assert_eq!(vm.unwrap_err().code, "normalized_integer_overflow");
            assert_eq!(reference.unwrap_err().code, "reference_integer_overflow");
            assert_eq!(output.stderr, b"normalized_integer_overflow\n");
        } else {
            assert!(output.status.success(), "{name}: {output:?}");
            assert_eq!(vm.unwrap().0, NormalizedValue::I64(42));
            assert_eq!(reference.unwrap().0, NormalizedValue::I64(42));
            assert_eq!(output.stdout, b"42");
        }
    }
}

#[test]
fn native_elf_compiles_beyond_former_function_and_instruction_inventories() {
    use std::fmt::Write as _;
    let count = 4097;
    let mut input = String::from(
        "declarations.begin\n(units (module create native-capacity\n         (external create length (visibility private) (implementation core.list.length)          (type-parameter create T) (parameter create values (type (list T))) (returns I64))\n",
    );
    let items = " (i64 0)".repeat(16);
    for index in 0..count {
        writeln!(input,
            "(function create f{index} (visibility private) (effect pure) (returns I64)              (body (call length (types I64) (list I64{items}))))"
        ).unwrap();
    }
    input.push_str(
        "(function create entry (visibility public) (effect pure) (returns I64)          (body (call length (types I64) (list I64"
    );
    for index in 0..count {
        write!(input, " (call f{index})").unwrap();
    }
    input.push_str("))))\n))\ndeclarations.end\n");
    // Exercise ordinary unmetered authoring and project storage; finite-policy
    // boundaries are covered independently by their exact admission controls.
    let empty = empty_normalized_snapshot(b"native-capacity");
    let memory = crate::platform::MemProjectStore::new(None).unwrap();
    let repository = GraphRepository::create_memory(&memory, &empty, None)
        .unwrap()
        .repository;
    let base = repository.view_current().unwrap().revision();
    let request = format!("request base={base}\n{input}");
    let decoded =
        crate::platform::control::decode_compact_change("native-capacity", request.as_bytes())
            .unwrap();
    let prepared = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    repository.publish(&prepared.publication).unwrap();
    let source = repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    let program = prepare_snapshot(&source);
    let instructions: usize = program
        .functions
        .iter()
        .map(|function| match &function.body {
            NormalizedFunctionBody::Code(code) => code.instructions.len(),
            _ => 0,
        })
        .sum();
    assert!(instructions > 65_536);
    let declaration = declaration_named(&source, "entry");
    let index = program.function(declaration).unwrap().0 as usize;
    let (image, functions, _) = elf_x86_64::compile_function(&program, index).unwrap();
    assert!(functions > 4096);
    let output = execute(&image, &[]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"4097");
    assert!(output.stderr.is_empty());
    let policy = NormalizedRunPolicy::default();
    let control = ExecutionControl::uncancelled();
    let vm = NormalizedVm::for_test(&program, policy)
        .invoke(declaration, vec![], None, &control)
        .unwrap()
        .0;
    let reference = NormalizedReferenceInterpreter::new(&source, &program, policy)
        .invoke(declaration, vec![], None, &control)
        .unwrap()
        .0;
    assert_eq!(vm, NormalizedValue::I64(4097));
    assert_eq!(reference, vm);
}
