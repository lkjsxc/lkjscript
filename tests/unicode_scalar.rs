#[path = "../bridge/library.rs"]
mod bootstrap_library;
use lkjscript::{check, compiler, interpreter, ir, source};

fn program() -> ir::Program {
    let mut files = bootstrap_library::standard_library();
    files.push(source::SourceInput {
        path: "unicode.rs".to_string(),
        text: "pub fn probe()->i64 { match char::from_u32(97u32) { Some(c) => c as u32 as i64, None => 0 } }".to_string(),
    });
    compiler::check_bundle(&source::SourceBundle {
        root: "unicode.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    })
    .unwrap()
    .program()
    .clone()
}

#[test]
fn character_construction_checks_its_signature_on_both_ir_boundaries() {
    let original = program();
    let entry = original
        .functions
        .iter()
        .position(|f| f.name == "probe")
        .unwrap();
    let instruction = original.functions[entry]
        .body
        .iter()
        .position(|i| matches!(&i.op, ir::Op::Intrinsic {name,..} if *name == ir::IntrinsicOp::CharFromU32))
        .unwrap();
    let (destination, argument) = match &original.functions[entry].body[instruction].op {
        ir::Op::Intrinsic { dst, args, .. } => (*dst, args[0]),
        _ => unreachable!(),
    };
    let option = original.functions[entry].locals[destination].ty;
    for mutation in 0..8 {
        let mut forged = original.clone();
        match mutation {
            0 => forged.functions[entry].locals[argument].ty = ir::I32,
            1 => forged.functions[entry].locals[destination].ty = ir::U64,
            2..=5 => {
                let ir::TypeKind::Enum { variants, .. } = &mut forged.types[option].kind else {
                    panic!("Option shape");
                };
                match mutation {
                    2 => variants.swap(0, 1),
                    3 => variants[1].fields[0].ty = ir::U64,
                    4 => variants[0].fields.push(ir::Field {
                        name: "payload".to_string(),
                        ty: ir::U64,
                    }),
                    5 => variants.push(ir::Variant {
                        name: "Extra".to_string(),
                        fields: vec![],
                    }),
                    _ => unreachable!(),
                }
            }
            6 | 7 => {
                let ir::Op::Intrinsic { args, .. } =
                    &mut forged.functions[entry].body[instruction].op
                else {
                    unreachable!()
                };
                if mutation == 6 {
                    args.clear();
                } else {
                    args.push(argument);
                }
            }
            _ => unreachable!(),
        }
        assert!(
            check::check(forged.clone()).is_err(),
            "checker accepted signature mutation {mutation}"
        );
        let error = interpreter::run(&forged, entry, &[], &mut Vec::new())
            .err()
            .expect("model accepted malformed construction");
        assert!(
            matches!(error.kind, interpreter::ErrorKind::Interpreter(_)),
            "signature mutation {mutation}: {}",
            error.message
        );
    }
}
