//! Current Mode-0 consumer controls, run after public compiler graph adoption.
//! Mutate real checked exports, repair framing commitments, and keep each exact
//! failure at its documented descriptor/form/signature-or-transfer boundary.
use super::*;

fn exported(name: &str) -> Vec<u8> {
    let (source, program) = fixture();
    input::export(program, named_index(source, program, name)).unwrap()
}

fn matching_node(bytes: &[u8], function: usize, predicate: impl Fn(usize) -> bool) -> usize {
    (0..wide_at(bytes, function + 40) as usize)
        .map(|pc| node_row(bytes, function, pc))
        .find(|at| predicate(*at))
        .unwrap()
}

fn descriptor(bytes: &[u8], identity: u64) -> usize {
    assert!((1..=wide_at(bytes, 80)).contains(&identity));
    input::DESCRIPTORS + (identity as usize - 1) * input::DESCRIPTOR_BYTES
}

fn changed(bytes: &[u8], words: &[(usize, u64)]) -> Vec<u8> {
    let mut mutant = bytes.to_vec();
    for &(at, value) in words {
        assert_ne!(
            wide_at(bytes, at),
            value,
            "mutation at {at} must change its field"
        );
        write_wide(&mut mutant, at, value);
    }
    input::rechecksum(&mut mutant);
    assert_eq!(mutant.len(), bytes.len());
    assert_eq!(
        &mutant[8..16],
        &bytes[8..16],
        "profile and policy are unchanged"
    );
    assert_eq!(wide_at(&mutant, 16), mutant.len() as u64);
    assert_eq!(wide_at(&mutant, 112), 0);
    assert_ne!(
        &mutant[40..72],
        &bytes[40..72],
        "changed payload commitment"
    );
    mutant
}

fn check_descriptor_layer(bytes: &[u8], expected: i64) {
    let (source, program) = compiler();
    let module = source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Module(id), OwnerRecord::Module(record))
                if record.name.as_str() == "types" =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap();
    let declaration = source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if record.module == module && record.name.as_str() == "validate" =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap();
    let declaration = DeclarationReference {
        package: source.root.package_id,
        declaration,
    };
    let arguments = vec![
        NormalizedValue::bytes(bytes),
        NormalizedValue::I64(wide_at(bytes, 128) as i64),
    ];
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::foreground();
    let vm = NormalizedVm::for_test(program, policy)
        .invoke(declaration, arguments.clone(), None, &control)
        .unwrap();
    let reference = NormalizedReferenceInterpreter::new(source, program, policy)
        .invoke(declaration, arguments, None, &control)
        .unwrap();
    for (value, frames) in [
        (vm.0, vm.1.live_call_frames_after),
        (reference.0, reference.1.live_call_frames_after),
    ] {
        assert!(matches!(value, NormalizedValue::I64(actual) if actual == expected));
        assert_eq!(frames, 0);
    }
}

fn reject(label: &str, base: &[u8], mutant: &[u8], diagnostic: i64) {
    // flow::tables runs types::validate before function layout or execution
    // admission. A negative descriptor cursor becomes 5; accepted descriptor
    // bytes must end exactly where the first function's local metadata begins.
    let descriptor_end = if diagnostic == 5 {
        -1
    } else {
        wide_at(base, row(base, 0) + 56) as i64
    };
    check_descriptor_layer(mutant, descriptor_end);
    // The shared helper independently runs the maintained compiler in the VM,
    // canonical reference interpreter and native ELF, including native status,
    // diagnostic text and exact output agreement. No rejected image is emitted.
    assert_eq!(compile(mutant), (diagnostic, vec![]), "{label}");
}

fn accept(bytes: &[u8], expected: i64) {
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 9);
    check_descriptor_layer(bytes, wide_at(bytes, row(bytes, 0) + 56) as i64);
    let (diagnostic, image) = compile(bytes);
    assert_eq!(diagnostic, 0);
    let output = execute(&image, &[]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, expected.to_string().as_bytes());
    assert!(output.stderr.is_empty());
}

#[test]
fn selfhost_profile8_consumer_checks_callable_descriptors_and_signatures() {
    let base = exported("entry");
    accept(&base, 42);
    let root = root_row(&base);
    let value = matching_node(&base, root, |at| wide_at(&base, at) == 17);
    let target = row(&base, wide_at(&base, value + 8) as usize);
    let full = wide_at(&base, target + 64);
    let function = descriptor(&base, full);
    assert_eq!(signature(&base, full), (vec![1, 1], 1));
    let parameters = wide_at(&base, function + 16) as usize;
    let empty = descriptor(&base, wide_at(&base, root + 64));
    assert_eq!(wide_at(&base, empty + 8), 0);

    for (label, fields) in [
        (
            "Function parameter vector cannot escape the input",
            vec![(function + 16, base.len() as u64)],
        ),
        (
            "zero-arity Function still requires the exact metadata cursor",
            vec![(empty + 16, wide_at(&base, empty + 16) + 8)],
        ),
        (
            "Function parameter cannot refer to its own descriptor",
            vec![(parameters, full)],
        ),
        (
            "Function result cannot expose ByteBuffer",
            vec![(function + 24, 6)],
        ),
    ] {
        reject(label, &base, &changed(&base, &fields), 5);
    }
    // These retain structurally valid descriptors, callers and function layouts.
    // Change the row cells rather than the shared descriptor: only the target's
    // flow::functions -> signature check then disagrees, before its body runs.
    let target_locals = wide_at(&base, target + 56) as usize;
    assert_eq!(wide_at(&base, target_locals), 1);
    for (label, fields) in [
        (
            "eligible declaration cannot omit its full signature",
            vec![(root + 64, 0)],
        ),
        (
            "full signature must match the ordered parameter cells",
            vec![(target_locals, 2)],
        ),
        (
            "full signature must match the result cell",
            vec![(target + 8, 2)],
        ),
    ] {
        reject(label, &base, &changed(&base, &fields), 8);
    }

    let generic = exported("generic");
    accept(&generic, 42);
    let get = (0..wide_at(&generic, 96) as usize)
        .map(|i| row(&generic, i))
        .find(|at| wide_at(&generic, at + 48) == 27)
        .unwrap();
    assert_eq!(wide_at(&generic, get + 16), 2);
    // The row and its Function descriptor still agree. Registered list-length
    // (26) cannot accept the list-get (27) row's two-argument closed signature.
    reject(
        "registered external signature is independently checked",
        &generic,
        &changed(&generic, &[(get + 48, 26)]),
        8,
    );
}

#[test]
fn selfhost_profile8_consumer_checks_callable_transfers_and_opcode_forms() {
    let base = exported("entry");
    accept(&base, 42);
    let root = root_row(&base);
    let value = matching_node(&base, root, |at| wide_at(&base, at) == 17);
    let empty = matching_node(&base, root, |at| {
        wide_at(&base, at) == 18 && wide_at(&base, at + 16) == 0
    });
    let bind = matching_node(&base, root, |at| {
        wide_at(&base, at) == 18 && wide_at(&base, at + 16) == 1
    });
    let invoke = matching_node(&base, root, |at| {
        matches!(wide_at(&base, at), 19 | 20) && wide_at(&base, at + 16) == 1
    });
    let pass = matching_node(&base, root, |at| wide_at(&base, at) == 21);
    let full = wide_at(&base, bind + 8);
    let suffix = wide_at(&base, bind + 24);
    assert_ne!(full, suffix);

    for (label, fields) in [
        (
            "FunctionValue target must exist",
            vec![(value + 8, wide_at(&base, 96))],
        ),
        (
            "Bind input must be an actual Function descriptor",
            vec![(bind + 8, 1)],
        ),
        (
            "Bind cannot consume beyond its available prefix",
            vec![(bind + 16, 3)],
        ),
        (
            "Bind suffix must remove exactly its captured prefix",
            vec![(bind + 24, full)],
        ),
        (
            "empty Bind cannot change its Function identity",
            vec![(empty + 24, suffix)],
        ),
        (
            "Invoke requires its exact residual Function identity",
            vec![(invoke + 8, full)],
        ),
        (
            "Invoke requires the complete residual arity",
            vec![(invoke + 16, 0)],
        ),
    ] {
        reject(label, &base, &changed(&base, &fields), 8);
    }
    // Keep the capture producer and every preceding Pass certificate coherent,
    // so a Bool reaches Bind's I64 parameter check instead of an earlier edge.
    let first = node_row(&base, root, 0);
    let bind_pc = (bind - first) / input::NODE_BYTES;
    let capture_pc = (0..bind_pc)
        .rev()
        .find(|pc| {
            let at = node_row(&base, root, *pc);
            wide_at(&base, at) == 1 && wide_at(&base, at + 32) == 20
        })
        .unwrap();
    let capture = node_row(&base, root, capture_pc);
    let depth = wide_at(&base, capture + 56) as usize;
    let mut fields = vec![(capture, 2), (capture + 32, 1)];
    for pc in capture_pc + 1..=bind_pc {
        let at = node_row(&base, root, pc);
        assert!(pc == bind_pc || wide_at(&base, at) == 21);
        assert_eq!(wide_at(&base, at + 56) as usize, depth + 1);
        let cell = wide_at(&base, at + 48) as usize
            + wide_at(&base, root + 24) as usize
            + depth * input::VALUE_TYPE_BYTES;
        assert_eq!(wide_at(&base, cell), 1);
        fields.push((cell, 2));
    }
    reject(
        "coherently produced Bool cannot capture an I64 parameter",
        &base,
        &changed(&base, &fields),
        8,
    );
    // flow::nodes checks form before step: these unused operands are exactly 7.
    for (label, fields) in [
        (
            "FunctionValue cannot carry an argument count",
            vec![(value + 16, 1)],
        ),
        (
            "Invoke cannot carry Bind's result-identity operand",
            vec![(invoke + 24, 1)],
        ),
        ("Pass operands must be zero", vec![(pass + 8, 1)]),
    ] {
        reject(label, &base, &changed(&base, &fields), 7);
    }

    let held = exported("held");
    accept(&held, 42);
    let root = root_row(&held);
    let invoke = matching_node(&held, root, |at| wide_at(&held, at) == 19);
    let locals = wide_at(&held, root + 56) as usize;
    let state = wide_at(&held, invoke + 48) as usize;
    assert!((0..wide_at(&held, root + 24) as usize).any(|i| {
        let cell = locals + i * input::VALUE_TYPE_BYTES;
        wide_at(&held, cell) == 6 && wide_at(&held, cell + 8) == 0 && held[state + i] == 1
    }));
    reject(
        "live owned buffer forbids terminal Invoke transfer",
        &held,
        &changed(&held, &[(invoke, 20)]),
        8,
    );
    let direct_only = (0..wide_at(&held, 96) as usize)
        .find(|i| wide_at(&held, row(&held, *i) + 48) == 20)
        .unwrap();
    assert_eq!(wide_at(&held, row(&held, direct_only) + 64), 0);
    let value = matching_node(&held, root, |at| wide_at(&held, at) == 17);
    reject(
        "FunctionValue cannot name a direct-only owned result",
        &held,
        &changed(&held, &[(value + 8, direct_only as u64)]),
        8,
    );
}

#[test]
fn selfhost_profile8_consumer_checks_uncalled_functions_and_untaken_callable_nodes() {
    let source = byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create callable-inventory
  (external create add (visibility private) (implementation core.i64.add)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (function create dormant (visibility private) (effect pure) (returns I64)
    (body (invoke (bind (function-value add) (i64 10)) (i64 7))))
  (function create entry (visibility public) (effect pure) (returns I64)
    (body (if (bool true) (i64 42) (invoke (function-value dormant)))))))
declarations.end"#,
    )
    .unwrap();
    let program = prepare_snapshot(&source);
    let base = input::export(&program, named_index(&source, &program, "entry")).unwrap();
    accept(&base, 42);
    let root = root_row(&base);
    let branch = matching_node(&base, root, |at| wide_at(&base, at) == 8);
    let target_pc = wide_at(&base, branch + 8) as usize;
    let untaken = matching_node(&base, root, |at| wide_at(&base, at) == 17);
    assert!(untaken >= node_row(&base, root, target_pc));
    let condition = matching_node(&base, root, |at| wide_at(&base, at) == 2);
    assert!(condition < branch);
    assert_eq!(wide_at(&base, condition + 32), 1);
    // The same checked export executes its distinct dormant answer when only
    // the Bool literal changes; no new host projection supplies that branch.
    accept(&changed(&base, &[(condition + 32, 0)]), 17);
    let dormant = row(&base, wide_at(&base, untaken + 8) as usize);
    assert_ne!(dormant, root);
    let bind = matching_node(&base, dormant, |at| wide_at(&base, at) == 18);
    reject(
        "uncalled function body still needs a valid Bind prefix",
        &base,
        &changed(&base, &[(bind + 16, 3)]),
        8,
    );
    reject(
        "untaken FunctionValue target is still admitted",
        &base,
        &changed(&base, &[(untaken + 8, wide_at(&base, 96))]),
        8,
    );
}
