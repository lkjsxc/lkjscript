//! The independent model rejects unchecked IR without calling the checker.
use crate::{frontend, interpreter, ir, source};
use interpreter::{ErrorKind, InterpreterError, Scalar};

fn compiled(text: &str) -> ir::CheckedProgram {
    let mut files = Vec::new();
    let names: Vec<String> = vec![
        "$std/lib.rs".to_string(),
        "$std/prelude.rs".to_string(),
        "$std/sequence.rs".to_string(),
        "$std/bytes.rs".to_string(),
        "$std/string.rs".to_string(),
        "$std/str.rs".to_string(),
        "$std/sha256.rs".to_string(),
        "$std/char.rs".to_string(),
        "$std/fmt.rs".to_string(),
        "$std/iter.rs".to_string(),
        "$std/integer.rs".to_string(),
        "$std/io.rs".to_string(),
        "$std/ordered_map.rs".to_string(),
        "$std/ordered_set.rs".to_string(),
    ];
    for name in names {
        files.push(super::source_file(&name, None));
    }
    files.push(source::SourceInput {
        path: "interpreter.rs".to_string(),
        text: text.to_string(),
    });
    frontend::check_bundle(&source::SourceBundle {
        root: "interpreter.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    })
    .unwrap()
}
fn function_index(program: &ir::Program, name: &str) -> usize {
    let mut index = 0usize;
    while index < program.functions.len() {
        if program.functions[index].name == name {
            return index;
        }
        index += 1;
    }
    panic!("missing function {name}")
}
fn execute(
    text: &str,
    name: &str,
    args: &[u64],
) -> Result<interpreter::RunResult, interpreter::RunError> {
    let checked = compiled(text);
    let program = checked.program();
    let entry = function_index(program, name);
    interpreter::run(program, entry, args, &mut Vec::<u8>::new())
}
fn must_run(
    result: Result<interpreter::RunResult, interpreter::RunError>,
) -> interpreter::RunResult {
    result.unwrap_or_else(|error| panic!("{}: {}", error.code(), error.message))
}
fn observed(value: Scalar) -> i64 {
    match value {
        Scalar::Int { value, .. } => value as i64,
        _ => panic!("expected an integer scalar"),
    }
}

fn model_runs_values_moves_borrows_collections_and_source_library() {
    let cases: Vec<(String, i64)> = vec![
        (include_str!("../../tests/conformance/pass/arithmetic.rs").to_string(), 42),
        (include_str!("../../tests/conformance/pass/fibonacci.rs").to_string(), 55),
        (include_str!("../../tests/conformance/pass/loop_control.rs").to_string(), 25),
        (
            include_str!("../../tests/conformance/pass/disjoint_mutable_fields.rs").to_string(),
            42,
        ),
        (
            include_str!("../../tests/conformance/pass/partial_move_reinitialize.rs").to_string(),
            42,
        ),
        (include_str!("../../tests/conformance/pass/recursive_box.rs").to_string(), 42),
        (
            "pub fn probe() -> i64 {let mut v:Vec<i64>=vec![10,20];v.push(12);let out=v.pop().unwrap();v[0]+v[1]+out}".to_string(),
            42,
        ),
        (
            "pub fn probe() -> i64 {let mut s=String::from_str(\"ab\");s.push_str(\"c\");let t=s.clone();t.len() as i64}".to_string(),
            3,
        ),
    ];
    for (text, expected) in cases {
        let result = execute(&text, "probe", &[0u64; 0])
            .unwrap_or_else(|error| panic!("{}: {}\n{text}", error.code(), error.message));
        assert_eq!(observed(result.value), expected);
    }
}

fn model_float_negation_preserves_payload_bits() {
    let result = must_run(execute(
        include_str!("../../tests/conformance/pass/float_bits.rs"),
        "probe",
        &[0u64; 0],
    ));
    assert_eq!(observed(result.value), 42);
    let program = crate::ir_text::read(include_str!(
        "../../tests/conformance/migrated/backend_nan_negation.lkjir"
    ))
    .unwrap();
    let result = must_run(interpreter::run(
        &program,
        0,
        &[0u64; 0],
        &mut Vec::<u8>::new(),
    ));
    assert!(
        result.value
            == Scalar::F64 {
                bits: 0xfff8_0000_0000_0000
            }
    );
}

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
    assert_eq!(
        observed(must_run(execute(text, "probe", &[0u64; 0])).value),
        118
    );
}

fn model_uses_an_explicit_stack_for_non_tail_calls_and_deep_cleanup() {
    let checked = compiled(
        "fn recurse(n:i64)->i64 { if n==0 {0} else { let value=recurse(n-1);value+1 } } pub fn probe()->i64 {recurse(20000)}",
    );
    let program = checked.program().clone();
    let entry = function_index(&program, "probe");
    let result = must_run(interpreter::run(
        &program,
        entry,
        &[0u64; 0],
        &mut Vec::<u8>::new(),
    ));
    assert_eq!(observed(result.value), 20000);
    assert_eq!(result.peak_frames, 20002);
    let cleanup = "enum List {End,Next(Box<List>)} pub fn probe()->i64 {let mut v=List::End;let mut i:i64=0;while i<20000 {v=List::Next(Box::new(v));i=i+1;}i}";
    assert_eq!(
        observed(must_run(execute(cleanup, "probe", &[0u64; 0])).value),
        20000
    );
}

fn model_reports_program_traps_separately_from_machine_errors() {
    let cases: Vec<(String, u64, u64)> = vec![
        ("fn probe(x:i8)->i8{x+1i8}".to_string(), 127, 1),
        ("fn probe(x:i64)->i64{42/x}".to_string(), 0, 2),
        (
            "fn probe(x:usize)->i64{let a:[i64;1]=[1];a[x]}".to_string(),
            1,
            3,
        ),
        ("fn probe(x:i64)->i64{assert!(x==0);x}".to_string(), 1, 5),
    ];
    for (text, argument, code) in cases {
        let error = execute(&text, "probe", &[argument])
            .err()
            .expect("expected a interpreter failure");
        assert!(error.kind == ErrorKind::Trap(code), "{}", error.message);
        assert_eq!(error.line, 1);
    }
}

fn raw(types: Vec<ir::Type>, locals: &[usize], result: usize, ops: Vec<ir::Op>) -> ir::Program {
    let mut primitives = ir::primitives();
    for ty in types {
        primitives.push(ty);
    }
    let mut slots = Vec::new();
    for ty in locals {
        slots.push(ir::Local {
            name: String::new(),
            ty: *ty,
            mutable: true,
        });
    }
    let mut body = Vec::new();
    let mut line = 1usize;
    for op in ops {
        body.push(ir::Instruction {
            column: 1,
            line,
            op,
        });
        line += 1;
    }
    ir::Program {
        types: primitives,
        functions: vec![ir::Function {
            file: 0,
            name: "probe".to_string(),
            params: 0,
            result,
            return_source: None,
            locals: slots,
            body,
            test: false,
        }],
        destructors: Vec::new(),
    }
}
fn read(dst: usize, local: usize, moving: bool) -> ir::Op {
    ir::Op::Read {
        dst,
        place: ir::Place::local(local),
        moving,
    }
}
fn drop_local(local: usize) -> ir::Op {
    ir::Op::Drop {
        place: ir::Place::local(local),
    }
}
fn fails(program: ir::Program, expected: InterpreterError) {
    let error = interpreter::run(&program, 0, &[0u64; 0], &mut Vec::<u8>::new())
        .err()
        .expect("expected a interpreter failure");
    assert!(
        error.kind == ErrorKind::Interpreter(expected),
        "{}: {}",
        error.code(),
        error.message
    );
}
fn raw_ir_detects_moves_double_drop_and_conflicting_access_without_checker() {
    fails(
        raw(
            Vec::new(),
            &[ir::I64],
            ir::I64,
            vec![
                ir::Op::Int { dst: 0, value: 1 },
                drop_local(0),
                drop_local(0),
            ],
        ),
        InterpreterError::DoubleDrop,
    );
    fails(
        raw(
            Vec::new(),
            &[ir::I64, ir::I64, ir::I64],
            ir::I64,
            vec![
                ir::Op::Int { dst: 0, value: 1 },
                read(1, 0, true),
                read(2, 0, false),
            ],
        ),
        InterpreterError::UseAfterMove,
    );
    let reference = ir::Type {
        name: "&mut i64".to_string(),
        kind: ir::TypeKind::Ref {
            inner: ir::I64,
            mutable: true,
        },
    };
    fails(
        raw(
            vec![reference],
            &[ir::I64, 7, ir::I64],
            ir::I64,
            vec![
                ir::Op::Int { dst: 0, value: 1 },
                ir::Op::Borrow {
                    dst: 1,
                    place: ir::Place::local(0),
                    mutable: true,
                },
                read(2, 0, false),
            ],
        ),
        InterpreterError::ConflictingAccess,
    );
}
fn raw_ir_detects_frame_escape_and_inactive_variant_without_checker() {
    let reference = ir::Type {
        name: "&i64".to_string(),
        kind: ir::TypeKind::Ref {
            inner: ir::I64,
            mutable: false,
        },
    };
    fails(
        raw(
            vec![reference],
            &[ir::I64, 7],
            7,
            vec![
                ir::Op::Int { dst: 0, value: 1 },
                ir::Op::Borrow {
                    dst: 1,
                    place: ir::Place::local(0),
                    mutable: false,
                },
                ir::Op::Return { src: 1 },
            ],
        ),
        InterpreterError::DanglingReference,
    );
    let enumeration = ir::Type {
        name: "E".to_string(),
        kind: ir::TypeKind::Enum {
            copy: false,
            variants: vec![
                ir::Variant {
                    name: "A".to_string(),
                    fields: vec![ir::Field {
                        name: "0".to_string(),
                        ty: ir::I64,
                    }],
                },
                ir::Variant {
                    name: "B".to_string(),
                    fields: vec![ir::Field {
                        name: "0".to_string(),
                        ty: ir::I64,
                    }],
                },
            ],
        },
    };
    fails(
        raw(
            vec![enumeration],
            &[ir::I64, 7, ir::I64],
            ir::I64,
            vec![
                ir::Op::Int { dst: 0, value: 1 },
                ir::Op::Variant {
                    dst: 1,
                    variant: 0,
                    fields: vec![0],
                },
                ir::Op::Read {
                    dst: 2,
                    place: ir::Place {
                        local: 1,
                        projections: vec![ir::Projection::VariantField(1, 0)],
                    },
                    moving: false,
                },
            ],
        ),
        InterpreterError::InactiveVariant,
    );
}
fn raw_ir_cannot_fabricate_host_authority_or_supply_it_as_integer_argument() {
    for kind in vec![
        ir::TypeKind::World,
        ir::TypeKind::Stdout,
        ir::TypeKind::Stdin,
        ir::TypeKind::Stderr,
        ir::TypeKind::Directory,
        ir::TypeKind::File,
        ir::TypeKind::Process,
        ir::TypeKind::Clock,
        ir::TypeKind::Child,
        ir::TypeKind::Listener,
    ] {
        let program = raw(
            vec![ir::Type {
                name: "authority".to_string(),
                kind,
            }],
            &[7],
            7,
            vec![ir::Op::Int { dst: 0, value: 0 }, ir::Op::Return { src: 0 }],
        );
        fails(program.clone(), InterpreterError::InvalidIr);
        let mut entry = program;
        entry.functions[0].params = 1;
        assert!(
            interpreter::run(&entry, 0, &[0], &mut Vec::<u8>::new(),)
                .err()
                .expect("expected a interpreter failure")
                .kind
                == ErrorKind::Interpreter(InterpreterError::InvalidIr)
        );
    }
}

fn run_output<H: interpreter::Host>(
    text: &str,
    host: &mut H,
) -> Result<interpreter::RunResult, interpreter::RunError> {
    let checked = compiled(text);
    let program = checked.program();
    let entry = function_index(program, "main");
    interpreter::run(program, entry, &[0u64; 0], host)
}

fn model_output_preserves_bytes_drop_order_and_trap_prefix() {
    let mut bytes: Vec<u8> = Vec::new();
    let result = must_run(run_output(
        include_str!("../../tests/conformance/output/bytes.rs"),
        &mut bytes,
    ));
    assert!(result.value == Scalar::Unit);
    let mut expected = "Hello, 世界!\n".as_bytes().to_vec();
    expected.extend_from_slice(&[0, 255, 13, 10]);
    for byte in 0u16..256u16 {
        expected.push(byte as u8);
    }
    assert_eq!(bytes, expected);
    bytes.clear();
    must_run(run_output(
        include_str!("../../tests/conformance/output/drop_order.rs"),
        &mut bytes,
    ));
    assert_eq!(bytes, "abc\n".as_bytes().to_vec());
    bytes.clear();
    must_run(run_output(
        include_str!("../../tests/conformance/output/derived_writer.rs"),
        &mut bytes,
    ));
    assert_eq!(bytes, "derived\n".as_bytes().to_vec());
    bytes.clear();
    let error = run_output(
        include_str!("../../tests/conformance/output/before_trap.rs"),
        &mut bytes,
    )
    .err()
    .expect("expected a interpreter failure");
    assert!(error.kind == ErrorKind::Trap(5), "{}", error.message);
    assert_eq!(bytes, "before trap\n".as_bytes().to_vec());
}

pub(super) struct FailingHost(pub(super) Vec<u8>);
impl interpreter::Host for FailingHost {
    fn argument_count(&self) -> usize {
        0
    }
    fn argument(&self, _index: usize) -> Option<Vec<u8>> {
        None
    }
    fn read_stdin(&mut self, _bytes: &mut [u8]) -> Result<usize, i64> {
        Err(38)
    }
    fn write_stderr(&mut self, _bytes: &[u8]) -> Result<(), String> {
        Err("stderr is unavailable".to_string())
    }
    fn open_directory(&mut self, _path: &[u8]) -> Result<u64, i64> {
        self.0.push(b'D');
        Ok(1)
    }
    fn open_file(&mut self, _directory: u64, _path: &[u8], _create: bool) -> Result<u64, i64> {
        Err(38)
    }
    fn read_file(&mut self, _handle: u64, _bytes: &mut [u8]) -> Result<usize, i64> {
        Err(38)
    }
    fn write_file(&mut self, _handle: u64, _bytes: &[u8]) -> Result<usize, i64> {
        Err(38)
    }
    fn close_handle(&mut self, handle: u64) {
        assert_eq!(handle, 1u64);
        self.0.push(b'C');
    }
    fn set_exit_status(&mut self, _status: u8) {}
    fn duplicate_file(&mut self, _handle: u64) -> Result<u64, i64> {
        Err(38)
    }
    fn readonly_file(&mut self, _handle: u64) -> Result<u64, i64> {
        Err(38)
    }
    fn world_file(&mut self, _kind: u8) -> Result<u64, i64> {
        Err(38)
    }
    fn open_listener(&mut self, _address: &[u8]) -> Result<u64, i64> {
        self.0.push(b'L');
        Err(38)
    }
    fn accept_listener(&mut self, _listener: u64) -> Result<u64, i64> {
        Err(38)
    }
    fn listener_address(&mut self, _listener: u64, _bytes: &mut [u8]) -> Result<(), i64> {
        Err(38)
    }
    fn open_subdirectory(&mut self, _directory: u64, _path: &[u8]) -> Result<u64, i64> {
        Err(38)
    }
    fn read_directory(&mut self, _directory: u64, _bytes: &mut [u8]) -> Result<usize, i64> {
        Err(38)
    }
    fn rewind_handle(&mut self, _handle: u64) -> Result<(), i64> {
        Err(38)
    }
    fn create_directory(&mut self, _directory: u64, _name: &[u8]) -> Result<(), i64> {
        Err(38)
    }
    fn remove_entry(
        &mut self,
        _directory: u64,
        _name: &[u8],
        _is_directory: bool,
    ) -> Result<(), i64> {
        Err(38)
    }
    fn rename_entry(
        &mut self,
        _from: u64,
        _old_name: &[u8],
        _to: u64,
        _new_name: &[u8],
    ) -> Result<(), i64> {
        Err(38)
    }
    fn file_executable(&mut self, _file: u64) -> Result<(), i64> {
        Err(38)
    }
    fn monotonic_time(&mut self, _bytes: &mut [u8]) -> Result<(), i64> {
        Err(38)
    }
    fn spawn_process(
        &mut self,
        _executable: u64,
        _arguments: &[u8],
        _stdin: u64,
        _stdout: u64,
        _stderr: u64,
        _result: u64,
    ) -> Result<u64, i64> {
        Err(38)
    }
    fn wait_process(&mut self, _child: u64, _nonblocking: bool) -> Result<Option<i64>, i64> {
        Err(38)
    }
    fn terminate_process(&mut self, _child: u64) -> Result<(), i64> {
        Err(38)
    }
    fn drop_process(&mut self, _child: u64) {}

    fn write_stdout(&mut self, bytes: &[u8]) -> Result<(), String> {
        if !self.0.is_empty() {
            return Err("injected write failure".to_string());
        }
        self.0.extend_from_slice(bytes);
        Ok(())
    }
}

fn model_host_failure_keeps_previous_output_and_has_its_own_trap_kind() {
    let mut host = FailingHost(Vec::<u8>::new());
    let error = run_output("pub fn main(mut world:World){let mut out=world.stdout();out.write(\"a\".as_bytes());out.write(\"b\".as_bytes());}", &mut host).err().expect("expected a interpreter failure");
    assert!(error.kind == ErrorKind::Trap(7), "{}", error.message);
    assert_eq!(host.0, "a".as_bytes().to_vec());
}

fn conditional_projected_drop_clears_moves_while_strict_drop_rejects_repeated_destruction() {
    let types = vec![
        ir::Type {
            name: "Box".to_string(),
            kind: ir::TypeKind::Box { inner: ir::I64 },
        },
        ir::Type {
            name: "Pair".to_string(),
            kind: ir::TypeKind::Struct {
                fields: vec![ir::Field {
                    name: "left".to_string(),
                    ty: 7,
                }],
                copy: false,
            },
        },
    ];
    let field = ir::Place {
        local: 2,
        projections: vec![ir::Projection::Field(0)],
    };
    let mut ops = vec![
        ir::Op::Int { dst: 0, value: 1 },
        ir::Op::Intrinsic {
            dst: 1,
            name: ir::IntrinsicOp::BoxNew,
            args: vec![0],
        },
        ir::Op::Struct {
            dst: 2,
            fields: vec![1],
        },
        ir::Op::Read {
            dst: 6,
            place: field.clone(),
            moving: true,
        },
        ir::Op::DropIf {
            place: field.clone(),
        },
        ir::Op::Int { dst: 3, value: 2 },
        ir::Op::Intrinsic {
            dst: 4,
            name: ir::IntrinsicOp::BoxNew,
            args: vec![3],
        },
        ir::Op::Assign {
            place: field.clone(),
            src: 4,
        },
        ir::Op::Drop {
            place: field.clone(),
        },
    ];
    let mut forged = ops.clone();
    forged.push(ir::Op::Drop { place: field });
    fails(
        raw(
            types.clone(),
            &[ir::I64, 7, 8, ir::I64, 7, ir::UNIT, 7],
            ir::UNIT,
            forged,
        ),
        InterpreterError::DoubleDrop,
    );
    for op in vec![
        drop_local(6),
        drop_local(2),
        ir::Op::Unit { dst: 5 },
        ir::Op::Return { src: 5 },
    ] {
        ops.push(op);
    }
    let valid = raw(
        types,
        &[ir::I64, 7, 8, ir::I64, 7, ir::UNIT, 7],
        ir::UNIT,
        ops,
    );
    assert!(
        must_run(interpreter::run(
            &valid,
            0,
            &[0u64; 0],
            &mut Vec::<u8>::new(),
        ))
        .value
            == Scalar::Unit
    );
}

fn model_checks_signed_minimum_overflow_and_invalid_raw_integer_widths() {
    let cases: Vec<(String, u64)> = vec![
        ("fn probe(x:i8)->i8{-x}".to_string(), (-128i64) as u64),
        (
            "fn probe(x:i16)->i16{x / -1i16}".to_string(),
            (-32768i64) as u64,
        ),
        (
            "fn probe(x:i32)->i32{x % -1i32}".to_string(),
            (-2147483648i64) as u64,
        ),
        (
            "fn probe(x:i64)->i64{x / -1i64}".to_string(),
            (-9223372036854775808i64) as u64,
        ),
    ];
    for (text, argument) in cases {
        assert!(
            execute(&text, "probe", &[argument])
                .err()
                .expect("expected a interpreter failure")
                .kind
                == ErrorKind::Trap(1)
        );
    }
    fails(
        raw(
            vec![ir::Type {
                name: "invalid".to_string(),
                kind: ir::TypeKind::Int {
                    bits: 0,
                    signed: true,
                },
            }],
            &[7],
            7,
            vec![ir::Op::Int { dst: 0, value: 0 }],
        ),
        InterpreterError::InvalidIr,
    );
}

fn read_program(text: &str) -> ir::Program {
    crate::ir_text::read(text)
        .unwrap_or_else(|problem| panic!("{}: {}", problem.code, problem.message))
}
fn strict_and_conditional_drops_have_independent_static_and_dynamic_checks() {
    let accepted = read_program(include_str!(
        "../../tests/conformance/ir/drop_if_owned.lkjir"
    ));
    crate::check::check(accepted.clone())
        .unwrap_or_else(|problem| panic!("{}: {}", problem.code, problem.message));
    assert!(
        must_run(interpreter::run(
            &accepted,
            0,
            &[0u64; 0],
            &mut Vec::<u8>::new(),
        ))
        .value
            == Scalar::Int {
                value: 42,
                bits: 64,
                signed: true,
            }
    );
    let cases: Vec<(String, String, InterpreterError, String)> = vec![
        (
            include_str!("../../tests/conformance/ir/double_projected_drop.lkjir").to_string(),
            "E_MOVE".to_string(),
            InterpreterError::DoubleDrop,
            "pair.left".to_string(),
        ),
        (
            include_str!("../../tests/conformance/ir/drop_after_projected_move.lkjir").to_string(),
            "E_MOVE".to_string(),
            InterpreterError::DoubleDrop,
            "pair.left".to_string(),
        ),
        (
            include_str!("../../tests/conformance/ir/drop_if_absent_ancestor.lkjir").to_string(),
            "E_MOVE".to_string(),
            InterpreterError::UseAfterMove,
            "outer.*".to_string(),
        ),
        (
            include_str!("../../tests/conformance/ir/drop_if_borrowed_owner.lkjir").to_string(),
            "E_BORROW".to_string(),
            InterpreterError::UseAfterMove,
            "slot.*".to_string(),
        ),
        (
            include_str!("../../tests/conformance/ir/drop_if_indexed_owner.lkjir").to_string(),
            "E_BORROW".to_string(),
            InterpreterError::UseAfterMove,
            "array[index]".to_string(),
        ),
    ];
    for (text, code, fault, place) in cases {
        let raw = read_program(&text);
        let problem = crate::check::check(raw.clone())
            .err()
            .expect("forged drop passed the checker");
        assert_eq!(problem.code, code);
        assert!(problem.message.contains(&place), "{}", problem.message);
        fails(raw, fault);
    }
}

fn shared_reference_copy_keeps_real_carrier_and_exclusive_parent_conflicts() {
    let cases: Vec<(String, usize)> = vec![
        (
            include_str!("../../tests/conformance/ir/borrowed_reference_carrier_drop.lkjir")
                .to_string(),
            6,
        ),
        (
            include_str!("../../tests/conformance/ir/shared_reborrow_parent_write.lkjir")
                .to_string(),
            9,
        ),
    ];
    for (text, line) in cases {
        let program = crate::ir_text::read(&text).unwrap();
        let diagnostic = crate::check::check(program.clone()).err().unwrap();
        assert_eq!(diagnostic.code, "E_BORROW");
        let error = interpreter::run(&program, 0, &[0u64; 0], &mut Vec::<u8>::new())
            .err()
            .unwrap();
        assert!(
            error.kind == ErrorKind::Interpreter(InterpreterError::ConflictingAccess),
            "{}",
            error.message
        );
        assert_eq!(error.line, line);
    }
}

fn range_views_validate_types_loans_and_utf8_without_the_checker() {
    let cases: Vec<(String, ErrorKind)> = vec![
        (
            include_str!("../../tests/conformance/ir/range_wrong_bound.lkjir").to_string(),
            ErrorKind::Interpreter(InterpreterError::InvalidIr),
        ),
        (
            include_str!("../../tests/conformance/ir/range_wrong_source.lkjir").to_string(),
            ErrorKind::Interpreter(InterpreterError::InvalidIr),
        ),
        (
            include_str!("../../tests/conformance/ir/range_wrong_element.lkjir").to_string(),
            ErrorKind::Interpreter(InterpreterError::InvalidIr),
        ),
        (
            include_str!("../../tests/conformance/ir/range_immutable_mut.lkjir").to_string(),
            ErrorKind::Interpreter(InterpreterError::ConflictingAccess),
        ),
        (
            include_str!("../../tests/conformance/ir/range_consumed_bound.lkjir").to_string(),
            ErrorKind::Interpreter(InterpreterError::DoubleDrop),
        ),
        (
            include_str!("../../tests/conformance/ir/range_live_owner_drop.lkjir").to_string(),
            ErrorKind::Interpreter(InterpreterError::ConflictingAccess),
        ),
        (
            include_str!("../../tests/conformance/ir/range_owner_write.lkjir").to_string(),
            ErrorKind::Interpreter(InterpreterError::ConflictingAccess),
        ),
        (
            include_str!("../../tests/conformance/ir/range_mutable_text.lkjir").to_string(),
            ErrorKind::Interpreter(InterpreterError::InvalidIr),
        ),
        (
            include_str!("../../tests/conformance/ir/range_end_past.lkjir").to_string(),
            ErrorKind::Trap(3),
        ),
        (
            include_str!("../../tests/conformance/ir/range_utf8_start.lkjir").to_string(),
            ErrorKind::Trap(3),
        ),
    ];
    for (text, expected) in cases {
        let program = crate::ir_text::read(&text).unwrap();
        let error = interpreter::run(&program, 0, &[0u64; 0], &mut Vec::<u8>::new())
            .err()
            .unwrap();
        assert!(
            error.kind == expected,
            "{}: {}",
            error.code(),
            error.message
        );
    }
}

fn listener_intrinsics_reject_wrong_signatures_without_host_effects() {
    let cases: Vec<(String, ir::IntrinsicOp, ir::IntrinsicOp, bool)> = vec![
        (
            "fn main(mut world: World) { let _directory = world.__lkjscript_directory(b\".\\0\"); }".to_string(),
            ir::IntrinsicOp::WorldDirectory,
            ir::IntrinsicOp::WorldListen,
            false,
        ),
        (
            "fn main(mut world: World) { let _directory = world.__lkjscript_directory(b\".\\0\"); }".to_string(),
            ir::IntrinsicOp::WorldDirectory,
            ir::IntrinsicOp::ListenerAccept,
            true,
        ),
        (
            "fn main(mut world: World) { let mut clock = world.clock(); let mut bytes = [0u8; 16]; let _status = clock.__lkjscript_read(&mut bytes); }".to_string(),
            ir::IntrinsicOp::ClockRead,
            ir::IntrinsicOp::ListenerAddress,
            false,
        ),
    ];
    for (source, before, after, trim) in cases {
        let checked = compiled(&source);
        let mut program = checked.program().clone();
        let entry = function_index(&program, "main");
        let mut changed = 0usize;
        for function_index in 0..program.functions.len() {
            let function = &mut program.functions[function_index];
            for instruction_index in 0..function.body.len() {
                if let ir::Op::Intrinsic { name, args, .. } = &mut function.body[instruction_index].op {
                    if *name == before {
                        *name = after;
                        if trim { args.pop(); }
                        changed += 1;
                    }
                }
            }
        }
        assert_eq!(changed, 1);
        let rejected = crate::check::check(program.clone()).err().unwrap();
        assert_eq!(rejected.code, "E_TYPE");
        let failure = interpreter::run(&program, entry, &[0u64; 0], &mut Vec::<u8>::new()).err().unwrap();
        assert!(failure.kind == ErrorKind::Interpreter(InterpreterError::InvalidIr), "{}", failure.message);
    }
}

pub fn run(world: &mut World, trace: bool) -> usize {
    if trace {
        let mut output = world.stdout();
        output.write(b"model_runs_values_moves_borrows_collections_and_source_library\n");
    }
    model_runs_values_moves_borrows_collections_and_source_library();
    if trace {
        let mut output = world.stdout();
        output.write(b"model_float_negation_preserves_payload_bits\n");
    }
    model_float_negation_preserves_payload_bits();
    if trace {
        let mut output = world.stdout();
        output.write(b"model_runs_custom_drop_in_source_order\n");
    }
    model_runs_custom_drop_in_source_order();
    if trace {
        let mut output = world.stdout();
        output.write(b"model_uses_an_explicit_stack_for_non_tail_calls_and_deep_cleanup\n");
    }
    model_uses_an_explicit_stack_for_non_tail_calls_and_deep_cleanup();
    if trace {
        let mut output = world.stdout();
        output.write(b"model_reports_program_traps_separately_from_machine_errors\n");
    }
    model_reports_program_traps_separately_from_machine_errors();
    if trace {
        let mut output = world.stdout();
        output.write(b"raw_ir_detects_moves_double_drop_and_conflicting_access_without_checker\n");
    }
    raw_ir_detects_moves_double_drop_and_conflicting_access_without_checker();
    if trace {
        let mut output = world.stdout();
        output.write(b"raw_ir_detects_frame_escape_and_inactive_variant_without_checker\n");
    }
    raw_ir_detects_frame_escape_and_inactive_variant_without_checker();
    if trace {
        let mut output = world.stdout();
        output.write(b"raw_ir_cannot_fabricate_host_authority_or_supply_it_as_integer_argument\n");
    }
    raw_ir_cannot_fabricate_host_authority_or_supply_it_as_integer_argument();
    if trace {
        let mut output = world.stdout();
        output.write(b"model_output_preserves_bytes_drop_order_and_trap_prefix\n");
    }
    model_output_preserves_bytes_drop_order_and_trap_prefix();
    if trace {
        let mut output = world.stdout();
        output.write(b"model_host_failure_keeps_previous_output_and_has_its_own_trap_kind\n");
    }
    model_host_failure_keeps_previous_output_and_has_its_own_trap_kind();
    if trace {
        let mut output = world.stdout();
        output.write(b"conditional_projected_drop_clears_moves_while_strict_drop_rejects_repeated_destruction\n");
    }
    conditional_projected_drop_clears_moves_while_strict_drop_rejects_repeated_destruction();
    if trace {
        let mut output = world.stdout();
        output.write(b"model_checks_signed_minimum_overflow_and_invalid_raw_integer_widths\n");
    }
    model_checks_signed_minimum_overflow_and_invalid_raw_integer_widths();
    if trace {
        let mut output = world.stdout();
        output.write(b"strict_and_conditional_drops_have_independent_static_and_dynamic_checks\n");
    }
    strict_and_conditional_drops_have_independent_static_and_dynamic_checks();
    if trace {
        let mut output = world.stdout();
        output.write(b"shared_reference_copy_keeps_real_carrier_and_exclusive_parent_conflicts\n");
    }
    shared_reference_copy_keeps_real_carrier_and_exclusive_parent_conflicts();
    if trace {
        let mut output = world.stdout();
        output.write(b"range_views_validate_types_loans_and_utf8_without_the_checker\n");
    }
    range_views_validate_types_loans_and_utf8_without_the_checker();
    if trace {
        let mut output = world.stdout();
        output.write(b"listener_intrinsics_reject_wrong_signatures_without_host_effects\n");
    }
    listener_intrinsics_reject_wrong_signatures_without_host_effects();
    16
}
