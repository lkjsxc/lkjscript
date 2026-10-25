//! Independent programs and hostile control-flow certificates for the closure backend.
use super::*;

#[test]
fn selfhost_closure_executes_calls_branches_recursion_and_lexical_locals() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-closure.lkjc"
    ))
    .unwrap();
    let program = prepare_snapshot(&source);
    for (name, expected) in [
        ("branch-true", Some(42)),
        ("branch-false", Some(17)),
        ("tail-loop", Some(5_000_050_000)),
        ("recursive", Some(3_628_800)),
        ("mutually-recursive", Some(42)),
        ("locals", Some(42)),
        ("bools", Some(42)),
        ("unit-local", Some(42)),
        ("untaken-trap", Some(42)),
        ("called-overflow", None),
    ] {
        let declaration = declaration_named(&source, name);
        let index = program.function(declaration).unwrap().0 as usize;
        let payload = input::export(&program, index).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let (code, bytes) = compile(&payload);
        assert_eq!(code, 0, "{name}");
        let output = execute(&bytes, &[]);
        let host_bytes = crate::platform::compiler::elf_x86_64::compile_function(&program, index)
            .unwrap()
            .0;
        let host = execute(&host_bytes, &[]);
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
            assert_eq!(output.stdout, expected.to_string().as_bytes(), "{name}");
            assert!(output.stderr.is_empty());
            assert_eq!(vm.unwrap().0, NormalizedValue::I64(expected));
            assert_eq!(reference.unwrap().0, NormalizedValue::I64(expected));
        } else {
            assert_eq!(output.status.code(), Some(70));
            assert!(output.stdout.is_empty());
            assert_eq!(output.stderr, b"normalized_integer_overflow\n");
            assert_eq!(vm.unwrap_err().code, "normalized_integer_overflow");
            assert_eq!(reference.unwrap_err().code, "reference_integer_overflow");
        }
    }
}

fn local_function() -> input::Function {
    input::Function {
        result: 1,
        parameters: 0,
        locals: vec![input::ValueType::owned(1), input::ValueType::owned(1)],
        external: 0,
        nodes: vec![
            input::Node {
                tag: 1,
                immediate: 42,
                initialized: vec![0, 0],
                ..input::Node::default()
            },
            input::Node {
                tag: 5,
                initialized: vec![0, 0],
                stack: vec![input::ValueType::owned(1)],
                ..input::Node::default()
            },
            input::Node {
                tag: 4,
                initialized: vec![1, 0],
                ..input::Node::default()
            },
            input::Node {
                tag: 11,
                initialized: vec![1, 0],
                stack: vec![input::ValueType::owned(1)],
                ..input::Node::default()
            },
        ],
    }
}

#[test]
fn selfhost_closure_rejects_uninitialized_reads_types_and_inconsistent_edges() {
    let valid = local_function();
    let (code, bytes) = compile(&input::encode(std::slice::from_ref(&valid), 0).unwrap());
    assert_eq!(code, 0);
    assert_eq!(execute(&bytes, &[]).stdout, b"42");
    let mut cases = vec![];
    let mut changed = valid.clone();
    changed.nodes[0].initialized[0] = 1;
    cases.push(changed);
    let mut changed = valid.clone();
    changed.nodes[2].initialized[0] = 0;
    cases.push(changed);
    let mut changed = valid.clone();
    changed.nodes[2].a = 1;
    changed.nodes[2].initialized = vec![0, 1];
    cases.push(changed);
    let mut changed = valid.clone();
    changed.nodes[1].a = 2;
    cases.push(changed);
    let mut changed = valid.clone();
    changed.nodes[1].stack = vec![input::ValueType::owned(2)];
    cases.push(changed);
    let mut changed = valid.clone();
    changed.nodes[3].stack = vec![input::ValueType::owned(2)];
    cases.push(changed);
    let mut changed = valid.clone();
    changed.nodes[2].tag = 7;
    changed.nodes[2].a = 0;
    cases.push(changed);
    let mut changed = valid.clone();
    changed.nodes[2].tag = 7;
    changed.nodes[2].a = 4;
    cases.push(changed);
    let mut changed = valid.clone();
    changed.nodes[2].tag = 9;
    changed.nodes[2].a = 1;
    cases.push(changed);
    let mut changed = valid.clone();
    changed.nodes[2].tag = 9;
    changed.nodes[2].b = 1;
    cases.push(changed);
    let mut branch = input::Function {
        result: 1,
        locals: vec![input::ValueType::owned(1)],
        ..input::Function::default()
    };
    for (tag, a, immediate, initialized, stack) in [
        (2, 0, 1, vec![0], vec![]),
        (8, 4, 0, vec![0], vec![2]),
        (1, 0, 42, vec![0], vec![]),
        (5, 0, 0, vec![0], vec![1]),
        (4, 0, 0, vec![1], vec![]),
        (11, 0, 0, vec![1], vec![1]),
    ] {
        branch.nodes.push(input::Node {
            tag,
            a,
            immediate,
            initialized,
            stack: stack.into_iter().map(input::ValueType::owned).collect(),
            ..input::Node::default()
        });
    }
    // The untaken edge still cannot promise an initialized local.
    cases.push(branch);
    // Clearing a scalar local with Unit may not leave an initialized certificate.
    let mut changed = valid.clone();
    changed.nodes[0].tag = 3;
    changed.nodes[0].immediate = 0;
    changed.nodes[1].stack = vec![input::ValueType::owned(3)];
    cases.push(changed);
    for (i, function) in cases.into_iter().enumerate() {
        assert_eq!(
            compile(&input::encode(&[function], 0).unwrap()),
            (8, vec![]),
            "case {i}"
        );
    }
}

#[test]
fn selfhost_closure_calls_align_the_machine_stack_at_every_operand_depth() {
    // Replace only the selected callee with a tiny independently specified ABI
    // probe: return RSP & 15. It must see 8 at entry after the call's return address.
    for depth in 0..16 {
        let mut body = "(call probe)".to_owned();
        for _ in 0..depth {
            body = format!("(call add (i64 0) {body})");
        }
        let source = byte_buffer_tests::author_only(&format!(r#"declarations.begin
(units (module create alignment
 (external create add (visibility private) (implementation core.i64.add) (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
 (function create probe (visibility private) (effect pure) (returns I64) (body (i64 0)))
 (function create entry (visibility public) (effect pure) (returns I64) (body {body}))))
declarations.end"#)).unwrap();
        let program = prepare_snapshot(&source);
        let entry = program
            .function(declaration_named(&source, "entry"))
            .unwrap()
            .0 as usize;
        let payload = input::export(&program, entry).unwrap();
        let (code, mut bytes) = compile(&payload);
        assert_eq!(code, 0);
        let count = wide_at(&payload, 96) as usize;
        let declarations = wide_at(&payload, 88) as usize;
        let nodes = wide_at(&payload, 104) as usize;
        let probe = (0..count).find(|&f| {
            let at = declarations + f * input::FUNCTION_BYTES;
            let first = wide_at(&payload, at + 32) as usize;
            wide_at(&payload, at + 40) == 2 && wide_at(&payload, nodes + first * input::NODE_BYTES) == 1
        }).unwrap();
        // Read the source compiler's measured layout through an independently
        // admitted probe, rather than reconstructing its code-selection sizes.
        let layout = measured_layout(&payload);
        let first = wide_at(&payload, declarations + probe * input::FUNCTION_BYTES + 32) as usize;
        let address = wide_at(&layout, (probe + first) * 8) as usize;
        let probe_code = [0x48, 0x89, 0xe0, 0x48, 0x83, 0xe0, 0x0f, 0xc3];
        bytes[address..address + probe_code.len()].copy_from_slice(&probe_code);
        let result = execute(&bytes, &[]);
        assert_eq!(result.status.code(), Some(0), "depth {depth}: {result:?}");
        assert_eq!(result.stdout, b"8", "depth {depth}");
    }
}


#[test]
fn selfhost_closure_rejects_unsupported_untaken_syntax_and_unregistered_intrinsics() {
    let error = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-unregistered-intrinsic.lkjc"
    ))
    .unwrap_err();
    assert!(error.contains("intrinsic_unknown"), "{error}");
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-untaken-unsupported.lkjc"
    ))
    .unwrap();
    let program = prepare_snapshot(&source);
    let entry = program
        .function(declaration_named(&source, "entry"))
        .unwrap()
        .0 as usize;
    assert_eq!(
        input::export(&program, entry).unwrap_err().code,
        "compiler_input_unsupported"
    );
}

pub(super) fn wide_parameter_source() -> String {
    use std::fmt::Write;
    let mut text = String::from("declarations.begin\n(units (module create wide-frame ");
    text.push_str("(function create select (visibility public) (effect pure) ");
    for parameter in 0..256 { write!(text, "(parameter create p{parameter} (type I64)) ").unwrap(); }
    text.push_str("(returns I64) (body (let (binding held (type I64) (local p255)) (in (local held))))) ");
    text.push_str("(function create entry (visibility public) (effect pure) (returns I64) (body (call select ");
    for argument in 0..256 { write!(text, "(i64 {argument}) ").unwrap(); }
    text.push_str("))) (component create app (visibility public) (port create run (type (function () I64)) (function entry)))) ");
    text.push_str("(target create wide-main (component wide-frame::app) (runner command) (port wide-frame::app::run)))\ndeclarations.end\n");
    text
}

#[test]
fn selfhost_closure_executes_more_than_128_parameters_locals_and_stack_cells() {
    let source = byte_buffer_tests::author_only(&wide_parameter_source()).unwrap();
    let program = prepare_snapshot(&source);
    let declaration = declaration_named(&source, "entry");
    let index = program.function(declaration).unwrap().0 as usize;
    let bytes = input::export(&program, index).unwrap();
    let functions = wide_at(&bytes, 96) as usize;
    let declarations = wide_at(&bytes, 88) as usize;
    assert!((0..functions).any(|f| wide_at(&bytes, declarations + f * input::FUNCTION_BYTES + 16) == 256));
    assert!((0..functions).any(|f| wide_at(&bytes, declarations + f * input::FUNCTION_BYTES + 24) > 256));
    let nodes = wide_at(&bytes, 104) as usize;
    assert!((0..wide_at(&bytes, 120) as usize).any(|i| wide_at(&bytes, nodes + i * input::NODE_BYTES + 56) >= 256));
    let (code, image) = compile(&bytes);
    assert_eq!(code, 0);
    let output = execute(&image, &[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(output.stdout, b"255");
    assert!(output.stderr.is_empty());
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::foreground();
    let vm = NormalizedVm::for_test(&program, policy).invoke(declaration, vec![], None, &control).unwrap();
    let reference = NormalizedReferenceInterpreter::new(&source, &program, policy)
        .invoke(declaration, vec![], None, &control).unwrap();
    assert_eq!(vm.0, NormalizedValue::I64(255));
    assert_eq!(reference.0, vm.0);
}

#[test]
fn selfhost_wide_address_encodings_preserve_signed_boundaries_and_flags() {
    let source = apply_proposals(&compiler().0, proposals::replay(include_str!(
        "../../../../tests/fixtures/selfhost-wide-encoding-probes.lkjc")));
    let program = prepare_snapshot(&source);
    let declaration = declaration_named(&source, "assemble");
    let index = program.function(declaration).unwrap().0 as usize;
    let payload = input::export(&program, index).unwrap();
    let (code, image) = compile(&payload);
    assert_eq!(code, 0);
    let produced = native_filter_tests::execute_filter(&image, b"", &[], 1);
    assert_eq!(produced.status.code(), Some(0), "{:?}", produced.stderr);
    assert!(produced.stderr.is_empty());

    // Independent x86-64 instruction bytes: movabs R11, LEA with an R11 index,
    // then load/store. LEA preserves arithmetic flags; no ADD synthesizes addresses.
    let mut expected = vec![0x48, 0x8b, 0x85, 0xff, 0xff, 0xff, 0x7f,
        0x48, 0x8b, 0x85, 0x00, 0x00, 0x00, 0x80];
    for (displacement, opcode) in [(2_147_483_648i64, 0x8b), (-2_147_483_649, 0x89)] {
        expected.extend_from_slice(&[0x49, 0xbb]);
        expected.extend_from_slice(&displacement.to_le_bytes());
        expected.extend_from_slice(&[0x4e, 0x8d, 0x9c, 0x1d, 0, 0, 0, 0]);
        expected.extend_from_slice(&[0x49, opcode, 0x83, 0, 0, 0, 0]);
    }
    expected.extend_from_slice(&[0x49, 0xbb]);
    expected.extend_from_slice(&4_294_967_296u64.to_le_bytes());
    expected.extend_from_slice(&[0x4c, 0x01, 0xdc]); // ADD RSP,R11
    let absolute = 4_194_304u64 + 4_294_967_296;
    for conditional in [false, true] {
        if conditional { expected.extend_from_slice(&[0x75, 13]); } // inverted JE
        expected.extend_from_slice(&[0x49, 0xbb]);
        expected.extend_from_slice(&absolute.to_le_bytes());
        expected.extend_from_slice(&[0x41, 0xff, 0xe3]);
    }
    assert_eq!(produced.stdout, expected);
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::foreground();
    let arguments = vec![NormalizedValue::bytes(&b""[..])];
    let vm = NormalizedVm::for_test(&program, policy).invoke(declaration, arguments.clone(), None, &control).unwrap();
    let reference = NormalizedReferenceInterpreter::new(&source, &program, policy)
        .invoke(declaration, arguments, None, &control).unwrap();
    assert_eq!(vm.0, reference.0);
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = vm.0 else { panic!("assembly output"); };
    assert!(fields.iter().any(|(name, value)| name.as_str() == "image" && value == &NormalizedValue::bytes(expected.as_slice())));
}
