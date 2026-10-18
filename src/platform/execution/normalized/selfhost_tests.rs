//! Level 1 runs the lkjscript compiler on VM, source evaluation and native ELF.
//! Level 2 inspects and directly executes its emitted ELF against independent cases.
use super::super::value::NormalizedRecord;
use super::*;
use crate::platform::compiler::selfhost_input as input;
use sha2::{Digest, Sha256};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

#[path = "selfhost_closure_tests.rs"]
mod closure;

const SOURCE: &str = concat!(
    include_str!("../../../../tools/selfhost/input.lkjc"),
    "\n",
    include_str!("../../../../tools/selfhost/elf.lkjc"),
    "\n",
    include_str!("../../../../tools/selfhost/flow.lkjc"),
    "\n",
    include_str!("../../../../tools/selfhost/machine.lkjc"),
);

fn compiler() -> &'static (crate::platform::kernel::KernelSnapshot, NormalizedProgram) {
    static COMPILER: OnceLock<(crate::platform::kernel::KernelSnapshot, NormalizedProgram)> =
        OnceLock::new();
    COMPILER.get_or_init(|| {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/selfhost/project");
        let repository = GraphRepository::open(&root).unwrap();
        let source = repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        let program = prepare_snapshot(&source);
        (source, program)
    })
}

fn compile_with(
    source: &crate::platform::kernel::KernelSnapshot,
    program: &NormalizedProgram,
    bytes: &[u8],
) -> (i64, Vec<u8>) {
    compile_with_policy(source, program, bytes, NormalizedRunPolicy::default())
}

fn compile_with_policy(
    source: &crate::platform::kernel::KernelSnapshot,
    program: &NormalizedProgram,
    bytes: &[u8],
    policy: NormalizedRunPolicy,
) -> (i64, Vec<u8>) {
    let declaration = declaration_named(source, "compile-elf");
    let values = vec![NormalizedValue::bytes(bytes)];
    let control = ExecutionControl::uncancelled();
    let (vm, vm_work) = NormalizedVm::for_test(program, policy)
        .invoke(declaration, values.clone(), None, &control)
        .unwrap();
    let (reference, reference_work) = NormalizedReferenceInterpreter::new(source, program, policy)
        .invoke(declaration, values, None, &control)
        .unwrap();
    assert_eq!(vm, reference, "compiler engines must emit identical bytes");
    assert_eq!(vm_work.live_call_frames_after, 0);
    assert_eq!(reference_work.live_call_frames_after, 0);
    if policy.instruction_steps.is_none() {
        eprintln!(
            "selfhost foreground input_bytes={} vm_steps={} vm_cumulative_allocation={} ref_expressions={} ref_cumulative_allocation={}",
            bytes.len(),
            vm_work.instructions,
            vm_work.allocated_bytes,
            reference_work.expressions,
            reference_work.allocated_bytes
        );
    }
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = vm else {
        panic!("compiler result");
    };
    let NormalizedValue::I64(code) = &fields
        .iter()
        .find(|(name, _)| name.as_str() == "diagnostic")
        .unwrap()
        .1
    else {
        panic!("diagnostic");
    };
    let NormalizedValue::Bytes(image) = &fields
        .iter()
        .find(|(name, _)| name.as_str() == "image")
        .unwrap()
        .1
    else {
        panic!("image");
    };
    let index = program.function(declaration).unwrap().0 as usize;
    let compiler_image = crate::platform::compiler::elf_x86_64::compile_function(program, index)
        .unwrap()
        .0;
    eprintln!(
        "selfhost stage=C1 tier=backend compiler_sha256={} compiler_bytes={} input_sha256={}",
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&compiler_image)),
        compiler_image.len(),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(bytes)),
    );
    let native = native_filter_tests::execute_filter(&compiler_image, bytes, &[], 127);
    assert_eq!(
        native.stdout,
        image.as_ref(),
        "native compiler output differs"
    );
    if *code == 0 {
        assert_eq!(native.status.code(), Some(0), "{native:?}");
        assert!(native.stderr.is_empty());
    } else {
        assert_eq!(native.status.code(), Some(65), "{native:?}");
        assert_eq!(
            native.stderr,
            format!("compiler_rejected:{code}\n").as_bytes()
        );
    }
    (*code, image.to_vec())
}

fn compile(bytes: &[u8]) -> (i64, Vec<u8>) {
    let (source, program) = compiler();
    compile_with(source, program, bytes)
}

fn execute(bytes: &[u8], arguments: &[&str]) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("emitted");
    crate::release_container::inspect_static_elf_bytes(bytes).unwrap();
    crate::platform::owned_output::publish_executable_create_new(
        &path,
        bytes,
        2 * 1024 * 1024,
        "selfhost test ELF",
    )
    .unwrap();
    let mut child = Command::new(&path)
        .args(arguments)
        .env_clear()
        .current_dir(directory.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = std::time::Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if start.elapsed() > std::time::Duration::from_secs(5) {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!("emitted program timed out: {output:?}");
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    child.wait_with_output().unwrap()
}

#[test]
fn selfhost_compiler_emits_runtime_arithmetic_and_exact_traps_on_both_engines() {
    let cases = [
        ("zero", "(i64 0)", Some("0"), ""),
        ("negative", "(i64 -123456789)", Some("-123456789"), ""),
        (
            "minimum",
            "(i64 -9223372036854775808)",
            Some("-9223372036854775808"),
            "",
        ),
        (
            "maximum",
            "(i64 9223372036854775807)",
            Some("9223372036854775807"),
            "",
        ),
        (
            "arithmetic",
            "(call add (call multiply (i64 6) (i64 7)) (call subtract (i64 9) (i64 9)))",
            Some("42"),
            "",
        ),
        (
            "division",
            "(call divide (i64 -91) (i64 7))",
            Some("-13"),
            "",
        ),
        (
            "min-div",
            "(call divide (i64 -9223372036854775808) (i64 1))",
            Some("-9223372036854775808"),
            "",
        ),
        (
            "add-trap",
            "(call add (i64 9223372036854775807) (i64 1))",
            None,
            "integer_overflow",
        ),
        (
            "sub-trap",
            "(call subtract (i64 -9223372036854775808) (i64 1))",
            None,
            "integer_overflow",
        ),
        (
            "mul-trap",
            "(call multiply (i64 9223372036854775807) (i64 2))",
            None,
            "integer_overflow",
        ),
        (
            "zero-div",
            "(call divide (i64 1) (i64 0))",
            None,
            "integer_division",
        ),
        (
            "overflow-div",
            "(call divide (i64 -9223372036854775808) (i64 -1))",
            None,
            "integer_division",
        ),
        (
            "trap-order",
            "(call divide (call add (i64 9223372036854775807) (i64 1)) (i64 0))",
            None,
            "integer_overflow",
        ),
        (
            "annihilator",
            "(call multiply (i64 0) (call add (i64 9223372036854775807) (i64 1)))",
            None,
            "integer_overflow",
        ),
    ];
    let mut proposal = String::from("declarations.begin\n(units (module create programs\n");
    for (name, primitive) in [
        ("add", "add"),
        ("subtract", "subtract"),
        ("multiply", "multiply"),
        ("divide", "divide"),
    ] {
        proposal.push_str(&format!("(external create {name} (visibility private) (implementation core.i64.{primitive}) (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))\n"));
    }
    for (name, body, _, _) in cases {
        proposal.push_str(&format!("(function create {name} (visibility public) (effect pure) (returns I64) (body {body}))\n"));
    }
    proposal.push_str("))\ndeclarations.end\n");
    let source = byte_buffer_tests::author_only(&proposal).unwrap();
    let program = prepare_snapshot(&source);
    for (name, _, expected, trap) in cases {
        let declaration = declaration_named(&source, name);
        let index = program.function(declaration).unwrap().0 as usize;
        let bytes = input::export(&program, index).unwrap();
        assert_eq!(bytes, input::export(&program, index).unwrap());
        let (code, image) = compile(&bytes);
        assert_eq!(code, 0, "{name}");
        let output = execute(&image, &[]);
        eprintln!(
            "selfhost profile=2 tier=backend case={name} input_sha256={} elf_sha256={} elf_bytes={} exit={:?}",
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&bytes)),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&image)),
            image.len(),
            output.status.code()
        );
        let host_image = crate::platform::compiler::elf_x86_64::compile_function(&program, index)
            .unwrap()
            .0;
        let host = execute(&host_image, &[]);
        assert_eq!(output.status.code(), host.status.code(), "{name}");
        assert_eq!(output.stdout, host.stdout, "{name}");
        assert_eq!(output.stderr, host.stderr, "{name}");
        let control = ExecutionControl::uncancelled();
        let policy = NormalizedRunPolicy::foreground();
        let vm =
            NormalizedVm::for_test(&program, policy).invoke(declaration, vec![], None, &control);
        let reference = NormalizedReferenceInterpreter::new(&source, &program, policy).invoke(
            declaration,
            vec![],
            None,
            &control,
        );
        if let Some(expected) = expected {
            assert_eq!(output.status.code(), Some(0), "{name}");
            assert_eq!(output.stdout, expected.as_bytes(), "{name}");
            assert!(output.stderr.is_empty(), "{name}");
            let value = NormalizedValue::I64(expected.parse().unwrap());
            assert_eq!(vm.unwrap().0, value, "{name}");
            assert_eq!(reference.unwrap().0, value, "{name}");
        } else {
            assert_eq!(output.status.code(), Some(70), "{name}");
            assert!(output.stdout.is_empty(), "{name}");
            assert_eq!(
                output.stderr,
                format!("normalized_{trap}\n").as_bytes(),
                "{name}"
            );
            assert_eq!(vm.unwrap_err().code, format!("normalized_{trap}"));
            assert_eq!(reference.unwrap_err().code, format!("reference_{trap}"));
        }
    }
}

#[test]
fn selfhost_compiler_rejects_malformed_inputs_before_emission() {
    let valid = input::arithmetic(&[(1, 42), (6, 0)]).unwrap();
    for length in [0, 7, 39, 40, 71, 72, 175, 247, valid.len() - 1] {
        assert_eq!(compile(&valid[..length]), (3, vec![]));
    }
    for (offset, value, expected) in [
        (0, 0, 1),
        (8, 1, 2),
        (12, 5, 5),
        (16, 0, 3),
        (20, 255, 3),
        (24, 0, 3),
        (28, 255, 3),
        (32, 0, 6),
        (36, 1, 6),
        (72, 113, 5),
        (76, 0, 5),
        (80, 129, 5),
        (84, 0, 5),
        (88, 0, 5),
        (92, 0, 5),
        (96, 0, 5),
        (100, 0, 3),
        (104, 0, 5),
        (108, 0, 5),
        (112, 1, 5),
        (116, 0, 5),
        (120, 1, 5),
        (128, 99, 5),
        (132, 1, 5),
        (144, 99, 5),
        (160, 99, 5),
        (176, 1, 5),
        (180, 0, 5),
        (180, 2, 9),
        (184, 1, 5),
        (188, 129, 5),
        (192, 1, 5),
        (196, 1, 5),
        (200, 12, 5),
        (204, 0, 5),
        (208, 1, 5),
        (212, 1, 5),
        (376, 99, 7),
        (380, 1, 7),
        (384, 1, 7),
        (388, 1, 7),
        (400, 0, 5),
        (404, 129, 5),
        (424, 1, 7),
        (440, 2, 8),
        (440, 0, 5),
        (440, 255, 5),
    ] {
        let mut changed = valid.clone();
        changed[offset] = value;
        input::rechecksum(&mut changed);
        assert_eq!(compile(&changed), (expected, vec![]), "byte {offset}");
    }
    for nodes in [
        vec![(6, 0)],
        vec![(2, 0), (6, 0)],
        vec![(1, 1)],
        vec![(1, 1), (1, 2), (6, 0)],
        vec![(1, 1), (6, 0), (1, 2)],
    ] {
        assert_eq!(compile(&input::arithmetic(&nodes).unwrap()), (8, vec![]));
    }
    let mut corrupt = valid.clone();
    corrupt[392] ^= 1;
    assert_eq!(compile(&corrupt), (4, vec![]));
    let mut excessive = valid.clone();
    excessive[32..40].copy_from_slice(&16_777_217_u64.to_le_bytes());
    input::rechecksum(&mut excessive);
    assert_eq!(compile(&excessive), (6, vec![]));
    assert_eq!(compile(&vec![0; input::MAX_INPUT + 1]), (3, vec![]));
}

#[test]
fn selfhost_compiler_is_deterministic_and_source_mutation_changes_emitted_behavior() {
    let payload = input::arithmetic(&[(1, 42), (6, 0)]).unwrap();
    let (code, image) = compile(&payload);
    assert_eq!(code, 0);
    assert_eq!(compile(&payload), (0, image.clone()));
    let proposed = byte_buffer_tests::author_only(SOURCE).unwrap();
    let proposed_program = prepare_snapshot(&proposed);
    assert_eq!(
        compile_with(&proposed, &proposed_program, &payload),
        (0, image.clone()),
        "maintained proposals and accepted graph must compile identically"
    );
    let bad_argument = execute(&image, &["unexpected"]);
    assert_eq!(bad_argument.status.code(), Some(64));
    assert!(bad_argument.stdout.is_empty());
    assert_eq!(bad_argument.stderr, b"native_argument\n");
    // Change an observable runtime diagnostic in the authored compiler, then
    // re-admit through the ordinary graph API. No host-generated output template.
    let mutant = SOURCE.replace("native_argument", "mutant_argument");
    let source = byte_buffer_tests::author_only(&mutant).unwrap();
    let program = prepare_snapshot(&source);
    let (code, changed) = compile_with(&source, &program, &payload);
    assert_eq!(code, 0);
    assert_ne!(image, changed);
    let output = execute(&changed, &["unexpected"]);
    assert_eq!(output.status.code(), Some(64));
    assert_eq!(output.stderr, b"mutant_argument\n");
    assert_eq!(execute(&changed, &[]).stdout, b"42");
}

#[test]
fn selfhost_compiler_accepts_exact_input_and_fuel_limits() {
    let mut nodes = vec![(1, 0)];
    for _ in 0..(input::MAX_NODES - 2) / 2 {
        nodes.extend_from_slice(&[(1, 1), (2, 0)]);
    }
    nodes.push((6, 0));
    assert_eq!(nodes.len(), input::MAX_NODES);
    let mut bytes = input::arithmetic(&nodes).unwrap();
    assert_eq!(bytes.len(), 137_598);
    let (source, program) = compiler();
    let (code, image) =
        compile_with_policy(source, program, &bytes, NormalizedRunPolicy::foreground());
    assert_eq!(code, 0);
    assert_eq!(image.len(), 5120 + (input::MAX_NODES + 5) * 128);
    let output = execute(&image, &[]);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"2047");
    assert!(output.stderr.is_empty());
    let fuel = u64::from_le_bytes(bytes[32..40].try_into().unwrap());
    bytes[32..40].copy_from_slice(&(fuel - 1).to_le_bytes());
    input::rechecksum(&mut bytes);
    assert_eq!(compile(&bytes), (6, vec![]));
}

#[test]
fn selfhost_exporter_rejects_unsupported_complete_function_bodies() {
    let source =
        byte_buffer_tests::author_only(include_str!("../../../../tests/fixtures/native-elf.lkjc"))
            .unwrap();
    let program = prepare_snapshot(&source);
    for name in [
        "arithmetic",
        "branch",
        "sum",
        "captured",
        "collection",
        "nominal",
        "unsupported",
    ] {
        let index = program
            .function(declaration_named(&source, name))
            .unwrap()
            .0 as usize;
        assert_eq!(
            input::export(&program, index).unwrap_err().code,
            "compiler_input_unsupported",
            "{name}"
        );
    }
}
