//! Level 1 runs the lkjscript compiler on VM, source evaluation and native ELF.
//! Level 2 inspects and directly executes its emitted ELF against independent cases.
use super::super::value::NormalizedRecord;
use super::*;
use crate::platform::compiler::selfhost_input as input;
use sha2::{Digest, Sha256};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

#[path = "selfhost_admission_tests.rs"]
mod admission;
#[path = "selfhost_artifact_tests.rs"]
mod artifact;
#[path = "selfhost_bytes_tests.rs"]
mod bytes;
#[path = "selfhost_closure_tests.rs"]
mod closure;
#[path = "selfhost_source_tests.rs"]
mod source;

#[test]
fn selfhost_normalized_byte_inventory_exports_current_compiler() {
    let u32_at =
        |bytes: &[u8], at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let (source, program) = compiler();
    let entry = program
        .function(declaration_named(source, "compile-elf"))
        .unwrap()
        .0 as usize;
    let payload = input::export(program, entry).unwrap();
    assert_eq!(u32_at(&payload, 8), 6);
    eprintln!(
        "selfhost byte inventory functions={} nodes={} input_bytes={} types={}",
        wide_at(&payload, 96),
        wide_at(&payload, 120),
        payload.len(),
        wide_at(&payload, 80)
    );
    let crate::platform::execution::normalized::prepare::NormalizedFunctionBody::Code(code) =
        &program.functions[entry].body
    else {
        panic!("compiler entry");
    };
    let validator = code
        .instructions
        .iter()
        .find_map(|instruction| match instruction {
            crate::platform::execution::normalized::prepare::NormalizedInstruction::Call {
                function,
                ..
            } => Some(program.functions[function.0 as usize].declaration),
            _ => None,
        })
        .unwrap();
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::foreground();
    let values = vec![NormalizedValue::bytes(payload.as_slice())];
    let vm = NormalizedVm::for_test(program, policy)
        .invoke(validator, values.clone(), None, &control)
        .unwrap();
    let reference = NormalizedReferenceInterpreter::new(source, program, policy)
        .invoke(validator, values, None, &control)
        .unwrap();
    assert_eq!(vm.0, NormalizedValue::I64(0));
    assert_eq!(reference.0, vm.0);
    eprintln!(
        "selfhost full normalized input admitted vm_steps={} ref_expressions={}",
        vm.1.instructions, reference.1.expressions
    );
    let independent = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-generic-list.lkjc"
    ))
    .unwrap();
    let projected = prepare_snapshot(&independent);
    input::export(
        &projected,
        projected
            .function(declaration_named(&independent, "entry"))
            .unwrap()
            .0 as usize,
    )
    .unwrap();
}

#[path = "../../../../tests/support/selfhost_proposals.rs"]
mod proposals;
const SOURCE: &str = proposals::SOURCE;

#[test]
fn selfhost_maintained_proposals_match_authoritative_function_bodies() {
    let (source, program) = compiler();
    // Replaying a complete compiler retains both old and new accepted bodies.
    // Use ordinary disk history instead of exhausting a finite memory arena.
    let temporary = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&temporary.path().join("replay"), source, None)
        .unwrap()
        .repository;
    let replayed = publish_proposals(&repository, proposals::replay(SOURCE));
    let replayed = prepare_snapshot(&replayed);
    // Canonical units include helpers with arbitrary parameter/result types;
    // backend filter exports deliberately restrict their root entry signature.
    let units = |program: &NormalizedProgram| {
        program
            .artifact()
            .objects
            .iter()
            .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
            .map(|(key, bytes)| {
                let unit = crate::platform::compiler::CompilationUnit::decode(bytes, *key).unwrap();
                (
                    (unit.source.package, unit.source.owner),
                    (unit.tables, unit.payload),
                )
            })
            .collect::<BTreeMap<_, _>>()
    };
    let before_units = units(program);
    let after_units = units(&replayed);
    assert_eq!(before_units.len(), after_units.len());
    for (owner, before) in &before_units {
        assert!(
            after_units.get(owner) == Some(before),
            "proposal replay changed maintained compiler tables or payload for {owner:?}"
        );
    }
    let declaration = declaration_named(source, "compile-elf");
    let before = input::export(program, program.function(declaration).unwrap().0 as usize).unwrap();
    let after = input::export(
        &replayed,
        replayed.function(declaration).unwrap().0 as usize,
    )
    .unwrap();
    assert!(
        before == after,
        "proposal replay changed a maintained compiler closure {declaration:?}: before={} after={}",
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&before)),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&after))
    );
    eprintln!(
        "selfhost maintained proposal replay matched {} canonical units and the native backend closure",
        before_units.len()
    );
}

fn apply_proposals(
    initial: &crate::platform::kernel::KernelSnapshot,
    changes: Vec<String>,
) -> crate::platform::kernel::KernelSnapshot {
    let memory = crate::platform::MemProjectStore::new(128 * 1_048_576).unwrap();
    let repo = GraphRepository::create_memory(&memory, initial, None)
        .unwrap()
        .repository;
    publish_proposals(&repo, changes)
}

fn publish_proposals(
    repo: &GraphRepository,
    changes: Vec<String>,
) -> crate::platform::kernel::KernelSnapshot {
    let control = ExecutionControl::uncancelled();
    for (i, change) in changes.iter().enumerate() {
        let view = repo.view_current().unwrap();
        let decoded = crate::platform::control::decode_transaction(
            "compiler proposal",
            change.as_bytes(),
            &view,
            &control,
        )
        .unwrap();
        match view.prepare_authored_change(&decoded.semantic, decoded.options) {
            Ok(plan) => {
                repo.publish(&plan.publication).unwrap();
            }
            Err(errors)
                if errors.len() == 1 && errors[0].code == "publication_semantic_no_change" =>
            {
                view.require_current_validation().unwrap();
            }
            Err(errors) => panic!("compiler proposal {i}: {errors:?}"),
        }
    }
    repo.view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value
}

fn proposed_compiler(
    mutated: bool,
) -> &'static (crate::platform::kernel::KernelSnapshot, NormalizedProgram) {
    static BASE: OnceLock<(crate::platform::kernel::KernelSnapshot, NormalizedProgram)> =
        OnceLock::new();
    static MUTANT: OnceLock<(crate::platform::kernel::KernelSnapshot, NormalizedProgram)> =
        OnceLock::new();
    let slot = if mutated { &MUTANT } else { &BASE };
    slot.get_or_init(|| {
        let proposal = if mutated {
            SOURCE.replace("native_argument", "mutant_argument")
        } else {
            SOURCE.to_owned()
        };
        let (initial, changes) = proposals::steps(&proposal).unwrap();
        let initial = byte_buffer_tests::author_only(&initial).unwrap();
        // Full reconstruction retains its accepted authoring history. Use the
        // ordinary disk repository instead of exhausting a finite memory arena.
        let directory = tempfile::tempdir().unwrap();
        let repository = GraphRepository::create(
            &directory.path().join("compiler-reconstruction"),
            &initial,
            None,
        )
        .unwrap()
        .repository;
        let source = publish_proposals(&repository, changes);
        let program = prepare_snapshot(&source);
        (source, program)
    })
}

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
    assert!(
        vm == reference,
        "compiler engines must emit identical bytes"
    );
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
    assert!(
        native.stdout == image.as_ref(),
        "native compiler output differs: exit={:?} stderr={} expected_bytes={} actual_bytes={} expected_sha256={} actual_sha256={}",
        native.status.code(),
        String::from_utf8_lossy(&native.stderr),
        image.len(),
        native.stdout.len(),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(image.as_ref())),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&native.stdout)),
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

fn wide_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}

fn write_wide(bytes: &mut [u8], at: usize, value: u64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

fn measured_layout(bytes: &[u8]) -> Vec<u8> {
    let (source, program) = compiler();
    let module = source.owners.iter().find_map(|(key, owner)| match (key, owner) {
        (OwnerKey::Module(id), OwnerRecord::Module(m)) if m.name.as_str() == "machine" => Some(*id),
        _ => None,
    }).unwrap();
    let declaration = source.owners.iter().find_map(|(key, owner)| match (key, owner) {
        (OwnerKey::Declaration(id), OwnerRecord::Declaration(d))
            if d.module == module && d.name.as_str() == "layout" => Some(*id),
        _ => None,
    }).unwrap();
    let reference = DeclarationReference { package: source.root.package_id, declaration };
    let values = vec![NormalizedValue::bytes(bytes)];
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::foreground();
    let vm = NormalizedVm::for_test(program, policy).invoke(reference, values.clone(), None, &control).unwrap();
    let oracle = NormalizedReferenceInterpreter::new(source, program, policy)
        .invoke(reference, values, None, &control).unwrap();
    assert_eq!(vm.0, oracle.0);
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = vm.0 else { panic!("layout result") };
    assert!(fields.iter().any(|(name, value)| name.as_str() == "code" && value == &NormalizedValue::I64(0)));
    fields.iter().find_map(|(name, value)| match value {
        NormalizedValue::Bytes(bytes) if name.as_str() == "bytes" => Some(bytes.to_vec()),
        _ => None,
    }).unwrap()
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
        Some(32 * 1024 * 1024),
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
    let artifact_input =
        crate::platform::compiler::selfhost_artifact_input::bundle(program.artifact()).unwrap();
    let emitted_artifact = artifact::image("compile-elf", &artifact_input);
    let artifact_program = NormalizedProgram::prepare(
        crate::platform::compiler::load_artifact(&emitted_artifact).unwrap(),
    )
    .unwrap();
    for (name, _, expected, trap) in cases {
        let declaration = declaration_named(&source, name);
        let index = program.function(declaration).unwrap().0 as usize;
        let bytes = input::export(&program, index).unwrap();
        assert_eq!(bytes, input::export(&program, index).unwrap());
        let (code, image) = compile(&bytes);
        assert_eq!(code, 0, "{name}");
        let output = execute(&image, &[]);
        eprintln!(
            "selfhost profile=6 tier=backend case={name} input_sha256={} elf_sha256={} elf_bytes={} exit={:?}",
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
        let artifact_vm = NormalizedVm::for_test(&artifact_program, policy).invoke(
            declaration,
            vec![],
            None,
            &control,
        );
        let artifact_reference = NormalizedReferenceInterpreter::new(
            &source,
            &artifact_program,
            policy,
        )
        .invoke(declaration, vec![], None, &control);
        if let Some(expected) = expected {
            assert_eq!(output.status.code(), Some(0), "{name}");
            assert_eq!(output.stdout, expected.as_bytes(), "{name}");
            assert!(output.stderr.is_empty(), "{name}");
            let value = NormalizedValue::I64(expected.parse().unwrap());
            assert_eq!(vm.unwrap().0, value, "{name}");
            assert_eq!(reference.unwrap().0, value, "{name}");
            assert_eq!(artifact_vm.unwrap().0, value, "{name}");
            assert_eq!(artifact_reference.unwrap().0, value, "{name}");
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
            assert_eq!(artifact_vm.unwrap_err().code, format!("normalized_{trap}"));
            assert_eq!(
                artifact_reference.unwrap_err().code,
                format!("reference_{trap}")
            );
        }
    }
}

#[test]
fn selfhost_compiler_rejects_malformed_inputs_before_emission() {
    let valid = input::arithmetic(&[(1, 42), (6, 0)]).unwrap();
    for length in [0, 7, 39, 40, 71, 72, 159, 160, valid.len() - 1] {
        assert_eq!(compile(&valid[..length]), (3, vec![]));
    }
    let declarations = wide_at(&valid, 88) as usize;
    let nodes = wide_at(&valid, 104) as usize;
    let state = wide_at(&valid, nodes + input::NODE_BYTES + 48) as usize;
    for (offset, value, expected) in [
        (16, 0, 3), (24, 0, 3), (72, 168, 5), (80, 6, 3),
        (88, declarations as u64 + 8, 5), (96, 4, 5), (104, nodes as u64 + 8, 5),
        (120, u64::MAX, 3), (128, wide_at(&valid, 128) + 8, 5),
        (136, wide_at(&valid, 136) + 1, 5), (144, 5, 5), (152, 1, 5),
        (160, 99, 5), (360, 2, 5),
        (declarations, 1, 5), (declarations + 8, 2, 9), (declarations + 16, 1, 5),
        (declarations + 24, i64::MAX as u64, 5), (declarations + 32, 1, 5),
        (declarations + 40, 1, 5), (declarations + 48, 29, 5),
        (declarations + 56, 0, 5), (declarations + 64, 1, 5), (declarations + 72, 1, 5),
        (nodes, 99, 7), (nodes + 8, 1, 7), (nodes + 16, 1, 7), (nodes + 24, 1, 7),
        (nodes + 40, 1, 5), (nodes + 48, u64::MAX, 5), (nodes + 56, i64::MAX as u64, 5),
        (nodes + input::NODE_BYTES + 32, 1, 7),
        (state, 2, 8), (state, 0, 5), (state, u64::MAX, 5), (state + 8, 1, 5),
    ] {
        let mut changed = valid.clone();
        write_wide(&mut changed, offset, value);
        input::rechecksum(&mut changed);
        assert_eq!(compile(&changed), (expected, vec![]), "wide field {offset}={value}");
    }
    for (offset, value, expected) in [(0, 0, 1), (8, 4, 2), (12, 2, 5), (112, 1, 2), (112, 2, 2)] {
        let mut changed = valid.clone();
        changed[offset] = value;
        input::rechecksum(&mut changed);
        assert_eq!(compile(&changed), (expected, vec![]), "discriminator byte {offset}");
    }
    let mut artifact_mode = valid.clone();
    artifact_mode[8..12].copy_from_slice(&5_u32.to_le_bytes());
    artifact_mode[112..116].copy_from_slice(&1_u32.to_le_bytes());
    input::rechecksum(&mut artifact_mode);
    assert_eq!(compile(&artifact_mode), (10, vec![]));
    for nodes in [vec![(6, 0)], vec![(2, 0), (6, 0)], vec![(1, 1)],
        vec![(1, 1), (1, 2), (6, 0)], vec![(1, 1), (6, 0), (1, 2)]] {
        assert_eq!(compile(&input::arithmetic(&nodes).unwrap()), (8, vec![]));
    }
    let mut corrupt = valid;
    corrupt[nodes] ^= 1;
    assert_eq!(compile(&corrupt), (4, vec![]));
}

#[test]
fn selfhost_compiler_is_deterministic_and_source_mutation_changes_emitted_behavior() {
    let payload = input::arithmetic(&[(1, 42), (6, 0)]).unwrap();
    let (code, image) = compile(&payload);
    assert_eq!(code, 0);
    assert_eq!(compile(&payload), (0, image.clone()));
    let (proposed, proposed_program) = proposed_compiler(false);
    assert_eq!(
        compile_with(proposed, proposed_program, &payload),
        (0, image.clone()),
        "maintained proposals and accepted graph must compile identically"
    );
    let malformed = admission::forgotten_buffer_input();
    let (diagnostic, emitted) = compile_with(proposed, proposed_program, &malformed);
    assert_eq!(diagnostic, 8);
    assert!(emitted.is_empty());
    let bad_argument = execute(&image, &["unexpected"]);
    assert_eq!(bad_argument.status.code(), Some(64));
    assert!(bad_argument.stdout.is_empty());
    assert_eq!(bad_argument.stderr, b"native_argument\n");
    // Change an observable runtime diagnostic in the authored compiler, then
    // re-admit through the ordinary graph API. No host-generated output template.
    let (source, program) = proposed_compiler(true);
    let (code, changed) = compile_with(source, program, &payload);
    assert_eq!(code, 0);
    assert_ne!(image, changed);
    let output = execute(&changed, &["unexpected"]);
    assert_eq!(output.status.code(), Some(64));
    assert_eq!(output.stderr, b"mutant_argument\n");
    assert_eq!(execute(&changed, &[]).stdout, b"42");
}

#[test]
fn selfhost_compiler_accepts_more_than_historical_node_limit_without_default_allowance() {
    const HISTORICAL_MAX_NODES: usize = 32_768;
    let node_count = HISTORICAL_MAX_NODES + 2;
    let mut nodes = vec![(1, 0)];
    for _ in 0..(node_count - 2) / 2 { nodes.extend_from_slice(&[(1, 1), (2, 0)]); }
    nodes.push((6, 0));
    assert_eq!(nodes.len(), node_count);
    let bytes = input::arithmetic(&nodes).unwrap();
    assert_eq!(wide_at(&bytes, 120) as usize, node_count);
    assert_eq!(&bytes[12..16], &[0; 4]);
    assert_eq!(wide_at(&bytes, 32), 0);
    let (source, program) = compiler();
    let (code, image) = compile_with_policy(source, program, &bytes, NormalizedRunPolicy::foreground());
    assert_eq!(code, 0);
    let output = execute(&image, &[]);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"16384");
    assert!(output.stderr.is_empty());
    eprintln!("selfhost current profile=6 nodes={node_count} input_bytes={} elf_bytes={} compiler_padding=bounded-runtime-only", bytes.len(), image.len());

}

#[test]
fn selfhost_compiler_rejects_hash_consistent_wide_extents_outside_input() {
    let valid = input::arithmetic(&[(1, 42), (6, 0)]).unwrap();
    for value in [u32::MAX as u64, u64::from(u32::MAX) + 1, i64::MAX as u64, u64::MAX] {
        for offset in [24, 80, 120] {
            let mut bytes = valid.clone();
            write_wide(&mut bytes, offset, value);
            input::rechecksum(&mut bytes);
            assert_eq!(compile(&bytes), (3, vec![]), "extent {offset}={value}");
        }
    }
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

#[test]
fn selfhost_mode0_policy_preserves_unbounded_zero_exact_and_maximum() {
    let mut bytes = input::arithmetic(&[(1, 42), (6, 0)]).unwrap();
    let (code, expected) = compile(&bytes);
    assert_eq!(code, 0);
    let exact = 4096 + wide_at(&bytes, 96) + 64 * wide_at(&bytes, 120) + 256 * wide_at(&bytes, 136);
    for (allowance, accepted) in [(None, true), (Some(0), false), (Some(exact - 1), false),
        (Some(exact), true), (Some(u64::MAX), true)] {
        input::set_allowance(&mut bytes, allowance);
        let (code, image) = compile(&bytes);
        assert_eq!(code, if accepted { 0 } else { 6 }, "policy {allowance:?}");
        assert_eq!(image, if accepted { expected.clone() } else { vec![] });
    }
    // The unlimited tag has one canonical payload, and no reserved tag is accepted.
    input::set_allowance(&mut bytes, None);
    write_wide(&mut bytes, 32, 1);
    input::rechecksum(&mut bytes);
    assert_eq!(compile(&bytes), (5, vec![]));
}
