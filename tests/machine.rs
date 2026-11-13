#[path = "../bridge/library.rs"]
mod bootstrap_library;
use lkjscript::{compiler, k, machine, source};
use machine::{ErrorKind, MachineError, Scalar};

fn compiled(text: &str) -> k::CheckedProgram {
    let mut files = bootstrap_library::standard_library();
    files.push(source::SourceInput {
        path: "machine.rs".to_string(),
        text: text.to_string(),
    });
    compiler::check_bundle(&source::SourceBundle {
        root: "machine.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    })
    .unwrap()
}
fn execute(text: &str, name: &str, args: &[u64]) -> Result<machine::RunResult, machine::RunError> {
    let checked = compiled(text);
    let program = checked.program();
    let entry = program
        .functions
        .iter()
        .position(|function| function.name == name)
        .unwrap();
    machine::run(program, entry, args, &mut Vec::new())
}
fn must_run(result: Result<machine::RunResult, machine::RunError>) -> machine::RunResult {
    result.unwrap_or_else(|error| panic!("{}: {}", error.code(), error.message))
}
fn observed(value: Scalar) -> i64 {
    match value {
        Scalar::Int { value, .. } => value as i64,
        _ => panic!("expected an integer scalar"),
    }
}

#[test]
fn model_runs_values_moves_borrows_collections_and_source_library() {
    let cases = [
        (include_str!("conformance/pass/arithmetic.rs"), 42),
        (include_str!("conformance/pass/fibonacci.rs"), 55),
        (include_str!("conformance/pass/loop_control.rs"), 25),
        (
            include_str!("conformance/pass/disjoint_mutable_fields.rs"),
            42,
        ),
        (
            include_str!("conformance/pass/partial_move_reinitialize.rs"),
            42,
        ),
        (include_str!("conformance/pass/recursive_box.rs"), 42),
        (
            "pub fn probe() -> i64 {let mut v:Vec<i64>=vec![10,20];v.push(12);let out=v.pop().unwrap();v[0]+v[1]+out}",
            42,
        ),
        (
            "pub fn probe() -> i64 {let mut s=String::from_str(\"ab\");s.push_str(\"c\");let t=s.clone();t.len() as i64}",
            3,
        ),
    ];
    for (text, expected) in cases {
        let result = execute(text, "probe", &[])
            .unwrap_or_else(|error| panic!("{}: {}\n{text}", error.code(), error.message));
        assert_eq!(observed(result.value), expected);
    }
}

#[test]
fn model_runs_custom_drop_in_source_order() {
    let text = r#"
        struct Counter<'a> { count: &'a mut i64 }
        impl<'a> Drop for Counter<'a> {
            fn drop(&mut self) { *self.count = *self.count * 10 + 1; }
        }
        struct Parent<'a> { child: Counter<'a> }
        fn returning(count: &mut i64) -> i64 { let counter = Counter { count }; return 7; }
        fn breaking(count: &mut i64) { loop { let counter = Counter { count }; break; } }
        fn probe() -> i64 {
            let mut count: i64 = 0;
            let value = returning(&mut count);
            breaking(&mut count);
            { let parent = Parent { child: Counter { count: &mut count } }; }
            count + value
        }
    "#;
    assert_eq!(observed(must_run(execute(text, "probe", &[])).value), 118);
}

#[test]
fn model_uses_an_explicit_stack_for_non_tail_calls_and_deep_cleanup() {
    let checked = compiled(
        "fn recurse(n:i64)->i64 { if n==0 {0} else { let value=recurse(n-1);value+1 } } pub fn probe()->i64 {recurse(20000)}",
    );
    let program = checked.program().clone();
    let entry = program
        .functions
        .iter()
        .position(|function| function.name == "probe")
        .unwrap();
    let result = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || must_run(machine::run(&program, entry, &[], &mut Vec::new())))
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(observed(result.value), 20000);
    assert_eq!(result.peak_frames, 20002);
    let cleanup = "enum List {End,Next(Box<List>)} pub fn probe()->i64 {let mut v=List::End;let mut i:i64=0;while i<20000 {v=List::Next(Box::new(v));i=i+1;}i}";
    assert_eq!(
        observed(must_run(execute(cleanup, "probe", &[])).value),
        20000
    );
}

#[test]
fn model_reports_program_traps_separately_from_machine_errors() {
    for (text, argument, code) in [
        ("fn probe(x:i8)->i8{x+1i8}", 127, 1),
        ("fn probe(x:i64)->i64{42/x}", 0, 2),
        ("fn probe(x:usize)->i64{let a:[i64;1]=[1];a[x]}", 1, 3),
        ("fn probe(x:i64)->i64{assert!(x==0);x}", 1, 5),
    ] {
        let error = execute(text, "probe", &[argument])
            .err()
            .expect("expected a machine failure");
        assert!(error.kind == ErrorKind::Trap(code), "{}", error.message);
        assert_eq!(error.line, 1);
    }
}

fn raw(mut types: Vec<k::Type>, locals: &[usize], result: usize, ops: Vec<k::Op>) -> k::Program {
    let mut primitives = k::primitives();
    primitives.append(&mut types);
    k::Program {
        types: primitives,
        functions: vec![k::Function {
            file: 0,
            name: "probe".to_string(),
            params: 0,
            result,
            return_source: None,
            locals: locals
                .iter()
                .map(|ty| k::Local {
                    name: String::new(),
                    ty: *ty,
                    mutable: true,
                })
                .collect(),
            body: ops
                .into_iter()
                .enumerate()
                .map(|(line, op)| k::Instruction {
                    column: 1,
                    line: line + 1,
                    op,
                })
                .collect(),
            test: false,
        }],
        destructors: Vec::new(),
    }
}
fn read(dst: usize, local: usize, moving: bool) -> k::Op {
    k::Op::Read {
        dst,
        place: k::Place::local(local),
        moving,
    }
}
fn drop_local(local: usize) -> k::Op {
    k::Op::Drop {
        place: k::Place::local(local),
    }
}
fn fails(program: k::Program, expected: MachineError) {
    let error = machine::run(&program, 0, &[], &mut Vec::new())
        .err()
        .expect("expected a machine failure");
    assert!(
        error.kind == ErrorKind::Machine(expected),
        "{}: {}",
        error.code(),
        error.message
    );
}
#[test]
fn raw_k_detects_moves_double_drop_and_conflicting_access_without_checker() {
    fails(
        raw(
            Vec::new(),
            &[k::I64],
            k::I64,
            vec![
                k::Op::Int { dst: 0, value: 1 },
                drop_local(0),
                drop_local(0),
            ],
        ),
        MachineError::DoubleDrop,
    );
    fails(
        raw(
            Vec::new(),
            &[k::I64, k::I64, k::I64],
            k::I64,
            vec![
                k::Op::Int { dst: 0, value: 1 },
                read(1, 0, true),
                read(2, 0, false),
            ],
        ),
        MachineError::UseAfterMove,
    );
    let reference = k::Type {
        name: "&mut i64".to_string(),
        kind: k::TypeKind::Ref {
            inner: k::I64,
            mutable: true,
        },
    };
    fails(
        raw(
            vec![reference],
            &[k::I64, 7, k::I64],
            k::I64,
            vec![
                k::Op::Int { dst: 0, value: 1 },
                k::Op::Borrow {
                    dst: 1,
                    place: k::Place::local(0),
                    mutable: true,
                },
                read(2, 0, false),
            ],
        ),
        MachineError::ConflictingAccess,
    );
}
#[test]
fn raw_k_detects_frame_escape_and_inactive_variant_without_checker() {
    let reference = k::Type {
        name: "&i64".to_string(),
        kind: k::TypeKind::Ref {
            inner: k::I64,
            mutable: false,
        },
    };
    fails(
        raw(
            vec![reference],
            &[k::I64, 7],
            7,
            vec![
                k::Op::Int { dst: 0, value: 1 },
                k::Op::Borrow {
                    dst: 1,
                    place: k::Place::local(0),
                    mutable: false,
                },
                k::Op::Return { src: 1 },
            ],
        ),
        MachineError::DanglingReference,
    );
    let enumeration = k::Type {
        name: "E".to_string(),
        kind: k::TypeKind::Enum {
            copy: false,
            variants: vec![
                k::Variant {
                    name: "A".to_string(),
                    fields: vec![k::Field {
                        name: "0".to_string(),
                        ty: k::I64,
                    }],
                },
                k::Variant {
                    name: "B".to_string(),
                    fields: vec![k::Field {
                        name: "0".to_string(),
                        ty: k::I64,
                    }],
                },
            ],
        },
    };
    fails(
        raw(
            vec![enumeration],
            &[k::I64, 7, k::I64],
            k::I64,
            vec![
                k::Op::Int { dst: 0, value: 1 },
                k::Op::Variant {
                    dst: 1,
                    variant: 0,
                    fields: vec![0],
                },
                k::Op::Read {
                    dst: 2,
                    place: k::Place {
                        local: 1,
                        projections: vec![k::Projection::VariantField(1, 0)],
                    },
                    moving: false,
                },
            ],
        ),
        MachineError::InactiveVariant,
    );
}
#[test]
fn raw_k_cannot_fabricate_host_authority_or_supply_it_as_integer_argument() {
    for kind in [k::TypeKind::World, k::TypeKind::Stdout] {
        let program = raw(
            vec![k::Type {
                name: "authority".to_string(),
                kind,
            }],
            &[7],
            7,
            vec![k::Op::Int { dst: 0, value: 0 }, k::Op::Return { src: 0 }],
        );
        fails(program.clone(), MachineError::InvalidK);
        let mut entry = program;
        entry.functions[0].params = 1;
        assert!(
            machine::run(&entry, 0, &[0], &mut Vec::new())
                .err()
                .expect("expected a machine failure")
                .kind
                == ErrorKind::Machine(MachineError::InvalidK)
        );
    }
}

fn run_output<H: machine::Host>(
    text: &str,
    host: &mut H,
) -> Result<machine::RunResult, machine::RunError> {
    let checked = compiled(text);
    let program = checked.program();
    let entry = program
        .functions
        .iter()
        .position(|function| function.name == "main")
        .unwrap();
    machine::run(program, entry, &[], host)
}

#[test]
fn model_output_preserves_bytes_drop_order_and_trap_prefix() {
    let mut bytes = Vec::new();
    let result = must_run(run_output(
        include_str!("conformance/output/bytes.rs"),
        &mut bytes,
    ));
    assert!(result.value == Scalar::Unit);
    let mut expected = "Hello, 世界!\n".as_bytes().to_vec();
    expected.extend_from_slice(&[0, 255, 13, 10]);
    expected.extend(0u8..=255);
    assert_eq!(bytes, expected);
    bytes.clear();
    must_run(run_output(
        include_str!("conformance/output/drop_order.rs"),
        &mut bytes,
    ));
    assert_eq!(bytes, b"abc\n");
    bytes.clear();
    must_run(run_output(
        include_str!("conformance/output/derived_writer.rs"),
        &mut bytes,
    ));
    assert_eq!(bytes, b"derived\n");
    bytes.clear();
    let error = run_output(
        include_str!("conformance/output/before_trap.rs"),
        &mut bytes,
    )
    .err()
    .expect("expected a machine failure");
    assert!(error.kind == ErrorKind::Trap(5), "{}", error.message);
    assert_eq!(bytes, b"before trap\n");
}

#[test]
fn model_host_failure_keeps_previous_output_and_has_its_own_trap_kind() {
    struct FailingHost(Vec<u8>);
    impl machine::Host for FailingHost {
        fn write_stdout(&mut self, bytes: &[u8]) -> Result<(), String> {
            if !self.0.is_empty() {
                return Err("injected write failure".to_string());
            }
            self.0.extend_from_slice(bytes);
            Ok(())
        }
    }
    let mut host = FailingHost(Vec::new());
    let error = run_output("pub fn main(mut world:World){let mut out=world.stdout();out.write(\"a\".as_bytes());out.write(\"b\".as_bytes());}", &mut host).err().expect("expected a machine failure");
    assert!(error.kind == ErrorKind::Trap(7), "{}", error.message);
    assert_eq!(host.0, b"a");
}

#[test]
fn conditional_projected_drop_clears_moves_while_strict_drop_rejects_repeated_destruction() {
    let types = vec![
        k::Type {
            name: "Box".to_string(),
            kind: k::TypeKind::Box { inner: k::I64 },
        },
        k::Type {
            name: "Pair".to_string(),
            kind: k::TypeKind::Struct {
                fields: vec![k::Field {
                    name: "left".to_string(),
                    ty: 7,
                }],
                copy: false,
            },
        },
    ];
    let field = k::Place {
        local: 2,
        projections: vec![k::Projection::Field(0)],
    };
    let mut ops = vec![
        k::Op::Int { dst: 0, value: 1 },
        k::Op::Intrinsic {
            dst: 1,
            name: "box_new".to_string(),
            args: vec![0],
        },
        k::Op::Struct {
            dst: 2,
            fields: vec![1],
        },
        k::Op::Read {
            dst: 6,
            place: field.clone(),
            moving: true,
        },
        k::Op::DropIf {
            place: field.clone(),
        },
        k::Op::Int { dst: 3, value: 2 },
        k::Op::Intrinsic {
            dst: 4,
            name: "box_new".to_string(),
            args: vec![3],
        },
        k::Op::Assign {
            place: field.clone(),
            src: 4,
        },
        k::Op::Drop {
            place: field.clone(),
        },
    ];
    let mut forged = ops.clone();
    forged.push(k::Op::Drop { place: field });
    fails(
        raw(
            types.clone(),
            &[k::I64, 7, 8, k::I64, 7, k::UNIT, 7],
            k::UNIT,
            forged,
        ),
        MachineError::DoubleDrop,
    );
    ops.extend([
        drop_local(6),
        drop_local(2),
        k::Op::Unit { dst: 5 },
        k::Op::Return { src: 5 },
    ]);
    let valid = raw(types, &[k::I64, 7, 8, k::I64, 7, k::UNIT, 7], k::UNIT, ops);
    assert!(must_run(machine::run(&valid, 0, &[], &mut Vec::new())).value == Scalar::Unit);
}

#[test]
fn model_checks_signed_minimum_overflow_and_invalid_raw_integer_widths() {
    for (text, argument) in [
        ("fn probe(x:i8)->i8{-x}", (-128i64) as u64),
        ("fn probe(x:i16)->i16{x / -1i16}", (-32768i64) as u64),
        ("fn probe(x:i32)->i32{x % -1i32}", (i32::MIN as i64) as u64),
        ("fn probe(x:i64)->i64{x / -1i64}", i64::MIN as u64),
    ] {
        assert!(
            execute(text, "probe", &[argument])
                .err()
                .expect("expected a machine failure")
                .kind
                == ErrorKind::Trap(1)
        );
    }
    fails(
        raw(
            vec![k::Type {
                name: "invalid".to_string(),
                kind: k::TypeKind::Int {
                    bits: 0,
                    signed: true,
                },
            }],
            &[7],
            7,
            vec![k::Op::Int { dst: 0, value: 0 }],
        ),
        MachineError::InvalidK,
    );
}

#[test]
fn strict_and_conditional_drops_have_independent_static_and_dynamic_checks() {
    fn program(text: &str) -> k::Program {
        lkjscript::k_text::read(text)
            .unwrap_or_else(|problem| panic!("{}: {}", problem.code, problem.message))
    }
    let accepted = program(include_str!("conformance/k/drop_if_owned.k"));
    lkjscript::check::check(accepted.clone())
        .unwrap_or_else(|problem| panic!("{}: {}", problem.code, problem.message));
    assert!(
        must_run(machine::run(&accepted, 0, &[], &mut Vec::new())).value
            == Scalar::Int {
                value: 42,
                bits: 64,
                signed: true,
            }
    );
    for (text, code, fault, place) in [
        (
            include_str!("conformance/k/double_projected_drop.k"),
            "E_MOVE",
            MachineError::DoubleDrop,
            "pair.left",
        ),
        (
            include_str!("conformance/k/drop_after_projected_move.k"),
            "E_MOVE",
            MachineError::DoubleDrop,
            "pair.left",
        ),
        (
            include_str!("conformance/k/drop_if_absent_ancestor.k"),
            "E_MOVE",
            MachineError::UseAfterMove,
            "outer.*",
        ),
        (
            include_str!("conformance/k/drop_if_borrowed_owner.k"),
            "E_BORROW",
            MachineError::UseAfterMove,
            "slot.*",
        ),
        (
            include_str!("conformance/k/drop_if_indexed_owner.k"),
            "E_BORROW",
            MachineError::UseAfterMove,
            "array[index]",
        ),
    ] {
        let raw = program(text);
        let problem = lkjscript::check::check(raw.clone())
            .err()
            .expect("forged drop passed the checker");
        assert_eq!(problem.code, code);
        assert!(problem.message.contains(place), "{}", problem.message);
        fails(raw, fault);
    }
}

#[test]
fn shared_reference_copy_keeps_real_carrier_and_exclusive_parent_conflicts() {
    for (text, line) in [
        (
            include_str!("conformance/k/borrowed_reference_carrier_drop.k"),
            6,
        ),
        (
            include_str!("conformance/k/shared_reborrow_parent_write.k"),
            9,
        ),
    ] {
        let program = lkjscript::k_text::read(text).unwrap();
        let diagnostic = lkjscript::check::check(program.clone()).err().unwrap();
        assert_eq!(diagnostic.code, "E_BORROW");
        let error = machine::run(&program, 0, &[], &mut Vec::new())
            .err()
            .unwrap();
        assert!(
            error.kind == ErrorKind::Machine(MachineError::ConflictingAccess),
            "{}",
            error.message
        );
        assert_eq!(error.line, line);
    }
}
