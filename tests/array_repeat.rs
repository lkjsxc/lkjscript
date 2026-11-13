//! Repeat is one atomic K construction with independent owned element storage.
#[path = "../bridge/library.rs"]
mod bootstrap_library;
#[path = "support/native.rs"]
mod native;
use lkjscript::{check, compiler, k, k_text, machine, source};

fn checked(text: &str) -> k::CheckedProgram {
    let mut files = bootstrap_library::standard_library();
    files.push(source::SourceInput {
        path: "repeat.rs".to_string(),
        text: text.to_string(),
    });
    compiler::check_bundle(&source::SourceBundle {
        root: "repeat.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    })
    .unwrap()
}
fn entry_index(program: &k::Program) -> usize {
    program
        .functions
        .iter()
        .position(|f| f.name == "probe")
        .unwrap()
}
fn value(program: &k::Program) -> i64 {
    match machine::run(
        program,
        entry_index(program),
        &[],
        &mut Vec::new(),
        machine::Resources::unlimited(),
    )
    .unwrap_or_else(|error| panic!("{}:{}: {}", error.file, error.line, error.message))
    .value
    {
        machine::Scalar::Int {
            value,
            bits: 64,
            signed: true,
        } => value as i64,
        _ => panic!("unexpected repeat result"),
    }
}
fn repeat(program: &k::Program) -> (usize, usize, usize) {
    let function = &program.functions[entry_index(program)];
    function
        .body
        .iter()
        .enumerate()
        .find_map(|(index, instruction)| match &instruction.op {
            k::Op::Intrinsic { dst, name, args } if name == "array_repeat" => {
                assert_eq!(args.len(), 1);
                Some((index, *dst, args[0]))
            }
            _ => None,
        })
        .unwrap()
}

#[test]
fn repeated_copy_aggregates_have_independent_storage_on_both_routes() {
    let program = checked(include_str!("conformance/pass/array_repeat_compact.rs"));
    assert_eq!(value(program.program()), 42);
    native::assert_values(&program, &[("probe", 42)]);
}

#[test]
fn repeat_extent_does_not_expand_k_locals_or_instructions() {
    let small = checked("pub fn probe()->i64 { let a=[7u8;16]; a.len() as i64 }");
    let large = checked("pub fn probe()->i64 { let a=[7u8;8192]; a.len() as i64 }");
    let small_body = &small.program().functions[entry_index(small.program())];
    let large_body = &large.program().functions[entry_index(large.program())];
    assert_eq!(small_body.locals.len(), large_body.locals.len());
    assert_eq!(small_body.body.len(), large_body.body.len());
    for program in [small.program(), large.program()] {
        assert_eq!(
            program.functions[entry_index(program)]
                .body
                .iter()
                .filter(|i| { matches!(&i.op,k::Op::Intrinsic{name,..} if name=="array_repeat") })
                .count(),
            1
        );
        let text = k_text::write(program);
        assert!(*program == k_text::read(&text).unwrap());
    }
    assert_eq!(value(large.program()), 8192);
}

#[test]
fn array_type_is_the_only_repeat_extent() {
    let original = checked("pub fn probe()->i64 { let a=[7u8;4]; a.len() as i64 }");
    let mut program = original.program().clone();
    let (_, dst, _) = repeat(&program);
    let array = program.functions[entry_index(&program)].locals[dst].ty;
    let k::TypeKind::Array { len, .. } = &mut program.types[array].kind else {
        panic!("array")
    };
    *len = 7;
    let checked = check::check(program).unwrap();
    assert_eq!(value(checked.program()), 7);
    native::assert_values(&checked, &[("probe", 7)]);
}

#[test]
fn repeat_rejects_malformed_k_signatures_and_uninitialized_seeds() {
    let original = checked("pub fn probe()->i64 {let a=[7u8;4];a.len() as i64}")
        .program()
        .clone();
    let entry = entry_index(&original);
    let (instruction, destination, seed) = repeat(&original);
    let array = original.functions[entry].locals[destination].ty;
    for mutation in 0..8 {
        let mut forged = original.clone();
        match mutation {
            0 => forged.functions[entry].locals[destination].ty = k::I64,
            1 => forged.functions[entry].locals[seed].ty = k::I32,
            2 | 3 => {
                let k::TypeKind::Array { len, .. } = &mut forged.types[array].kind else {
                    panic!("array")
                };
                *len = mutation - 2;
            }
            4 | 5 => {
                let k::Op::Intrinsic { args, .. } =
                    &mut forged.functions[entry].body[instruction].op
                else {
                    panic!("repeat")
                };
                if mutation == 4 {
                    args.clear()
                } else {
                    args.push(seed)
                }
            }
            6 => {
                let k::TypeKind::Array { inner, .. } = &mut forged.types[array].kind else {
                    panic!("array")
                };
                *inner = k::I64;
            }
            7 => {
                let local = forged.functions[entry].locals.len();
                forged.functions[entry].locals.push(k::Local {
                    name: "uninitialized".to_string(),
                    ty: k::U8,
                    mutable: false,
                });
                let k::Op::Intrinsic { args, .. } =
                    &mut forged.functions[entry].body[instruction].op
                else {
                    panic!("repeat")
                };
                args[0] = local;
            }
            _ => unreachable!(),
        }
        assert!(
            check::check(forged.clone()).is_err(),
            "checker accepted mutation {mutation}"
        );
        let error = machine::run(
            &forged,
            entry,
            &[],
            &mut Vec::new(),
            machine::Resources::unlimited(),
        )
        .err()
        .expect("model accepted malformed repeat");
        assert!(
            matches!(error.kind, machine::ErrorKind::Machine(_)),
            "mutation {mutation}: {}",
            error.message
        );
    }
    let mut noncopy=checked("#[derive(Clone,Copy)]struct Item {value:i64} pub fn probe()->i64 {let a=[Item {value:7};4];a.len() as i64}").program().clone();
    let (_, _, seed) = repeat(&noncopy);
    let noncopy_entry = entry_index(&noncopy);
    let item = noncopy.functions[noncopy_entry].locals[seed].ty;
    let k::TypeKind::Struct { copy, .. } = &mut noncopy.types[item].kind else {
        panic!("item")
    };
    *copy = false;
    assert!(check::check(noncopy.clone()).is_err());
    let error = machine::run(
        &noncopy,
        noncopy_entry,
        &[],
        &mut Vec::new(),
        machine::Resources::unlimited(),
    )
    .err()
    .expect("model accepted affine repeat");
    assert!(matches!(error.kind, machine::ErrorKind::Machine(_)));
}
