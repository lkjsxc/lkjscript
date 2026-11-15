// A cyclic IR reference type exercises the projection representation without
// introducing a source alias or a numeric count that expands during checking.
fn cyclic_reference_program(depth: usize) -> crate::ir::Program {
    use crate::ir;
    let mut types = ir::primitives();
    let reference = types.len();
    types.push(ir::Type {
        name: "CyclicReference".to_string(),
        kind: ir::TypeKind::Ref {
            inner: reference,
            mutable: false,
        },
    });
    let projections = vec![ir::Projection::Deref; depth];
    let function = ir::Function {
        file: 0,
        name: "cyclic".to_string(),
        params: 1,
        result: reference,
        return_source: Some(ir::ReturnSource::Parameter {
            index: 0,
            projections: projections.clone(),
        }),
        locals: vec![
            ir::Local {
                name: "parameter".to_string(),
                ty: reference,
                mutable: false,
            },
            ir::Local {
                name: "returned".to_string(),
                ty: reference,
                mutable: false,
            },
        ],
        body: vec![
            ir::Instruction {
                file: None,
                column: 1,
                line: 1,
                op: ir::Op::Read {
                    dst: 1,
                    place: ir::Place {
                        local: 0,
                        projections,
                    },
                    moving: false,
                },
            },
            ir::Instruction {
                file: None,
                column: 1,
                line: 2,
                op: ir::Op::Drop {
                    place: ir::Place::local(0),
                },
            },
            ir::Instruction {
                file: None,
                column: 1,
                line: 3,
                op: ir::Op::Return { src: 1 },
            },
        ],
        test: false,
    };
    ir::Program {
        types,
        functions: vec![function],
        destructors: Vec::new(),
    }
}

fn explicit_return_paths_check_and_round_trip_on_cyclic_reference_types() {
    for depth in vec![1usize, 2usize, 64usize] {
        let original = cyclic_reference_program(depth);
        crate::check::check(original.clone()).unwrap();
        let text = crate::ir_text::write(&original);
        let mut count = 0usize;
        for line in text.as_str().lines() {
            if line.starts_with("fn ") {
                count = occurrences(line, " deref");
            }
        }
        assert_eq!(count, depth);
        let parsed = crate::ir_text::read(&text).unwrap();
        assert!(
            parsed == original,
            "return projection path changed during text roundtrip"
        );
        crate::check::check(parsed).unwrap();
    }
}

fn return_path_text_rejects_encoded_numeric_repetition() {
    let text = crate::ir_text::write(&cyclic_reference_program(1));
    for count in vec![0usize, 1usize, 64usize, !0usize] {
        let old_form = text.replace(" from 0 deref", &format!(" from 0 deref {count}"));
        assert_ne!(old_form, text);
        let error = crate::ir_text::read(&old_form)
            .err()
            .expect("old numeric repetition must not expand a projection path");
        assert_eq!(error.code, "E_IR_TEXT", "{error:?}");
    }
}

fn return_contract_rejects_unsupported_projection_kinds_in_the_ir_checker() {
    use crate::ir::{Projection, ReturnSource};
    for projection in vec![
        Projection::Field(0),
        Projection::VariantField(0, 0),
        Projection::Index(!0usize),
    ] {
        let mut original = cyclic_reference_program(1);
        original.functions[0].return_source = Some(ReturnSource::Parameter {
            index: 0,
            projections: vec![projection],
        });
        // The generic IR place syntax preserves the requested projection. Its
        // return-contract legality is checked at the semantic IR boundary.
        let parsed = crate::ir_text::read(&crate::ir_text::write(&original)).unwrap();
        assert!(parsed == original);
        let error = crate::check::check(parsed)
            .err()
            .expect("unsupported return projection must reach and fail the IR checker");
        assert_eq!(error.code, "E_BORROW", "{error:?}");
        assert!(error.message.contains("source contract"), "{error:?}");
    }
}

fn occurrences(text: &str, needle: &str) -> usize {
    let bytes = text.as_bytes();
    let pattern = needle.as_bytes();
    let mut at = 0usize;
    let mut count = 0usize;
    while at + pattern.len() <= bytes.len() {
        if &bytes[at..at + pattern.len()] == pattern {
            count += 1;
        }
        at += 1;
    }
    count
}
pub fn run(world: &mut World, trace: bool) -> usize {
    if trace {
        world.stdout().write(b"IR return contracts\n");
    }
    explicit_return_paths_check_and_round_trip_on_cyclic_reference_types();
    return_path_text_rejects_encoded_numeric_repetition();
    return_contract_rejects_unsupported_projection_kinds_in_the_ir_checker();
    3usize
}
