//! Opaque calls preserve possible source bounds through forwarding and reborrows.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::source::{SourceBundle, SourceInput};
#[path = "support/native.rs"]
mod native;
use lkjscript::{check, compiler, ir, ir_text};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const REBORROW: &str = r#"struct Cell{value:i64}
struct Pair{left:Cell,right:Cell}
fn choose<'a>(owner:&'a Pair)->&'a Cell{&owner.right}
fn probe()->i64{
    let mut owner=Pair{left:Cell{value:1},right:Cell{value:2}};
    let selected=choose(&owner);
    let child=&selected.value;
    drop(selected);
    owner.right.value=9;
    *child
}
"#;

fn bundle(source: &str) -> SourceBundle {
    let mut files = bootstrap_library::standard_library();
    files.push(SourceInput {
        path: "source_bounds.rs".to_string(),
        text: source.to_string(),
    });
    SourceBundle {
        root: "source_bounds.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}

fn rust_metadata(source: &str) -> std::process::Output {
    let directory = std::env::temp_dir().join(format!(
        "lkjscript-source-bounds-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("case.rs"), source).unwrap();
    let output = Command::new("rustc")
        .args([
            "--edition=2024",
            "--crate-type=lib",
            "--emit=metadata",
            "--cap-lints=allow",
        ])
        .arg(directory.join("case.rs"))
        .arg("-o")
        .arg(directory.join("case.rmeta"))
        .output()
        .unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    output
}

fn run(checked: &ir::CheckedProgram, expected: i64) {
    native::assert_values(checked, &[("probe", expected)]);
}

fn reference(inner: usize) -> ir::TypeKind {
    ir::TypeKind::Ref {
        inner,
        mutable: false,
    }
}

fn record(fields: &[usize], copy: bool) -> ir::TypeKind {
    ir::TypeKind::Struct {
        copy,
        fields: fields
            .iter()
            .enumerate()
            .map(|(index, ty)| ir::Field {
                name: index.to_string(),
                ty: *ty,
            })
            .collect(),
    }
}

fn add_type(types: &mut Vec<ir::Type>, name: &str, kind: ir::TypeKind) -> usize {
    let index = types.len();
    types.push(ir::Type {
        name: name.to_string(),
        kind,
    });
    index
}

fn place(local: usize, projections: Vec<ir::Projection>) -> ir::Place {
    ir::Place { local, projections }
}

fn drop_local(local: usize) -> ir::Op {
    ir::Op::Drop {
        place: ir::Place::local(local),
    }
}

fn source(projections: Vec<ir::Projection>) -> Option<ir::ReturnSource> {
    Some(ir::ReturnSource::Parameter {
        index: 0,
        projections,
    })
}

fn function(
    name: &str,
    params: usize,
    locals: &[(usize, bool)],
    result: usize,
    return_source: Option<ir::ReturnSource>,
    body: Vec<ir::Op>,
) -> ir::Function {
    ir::Function {
        file: 0,
        name: name.to_string(),
        params,
        locals: locals
            .iter()
            .enumerate()
            .map(|(index, (ty, mutable))| ir::Local {
                name: format!("v{index}"),
                ty: *ty,
                mutable: *mutable,
            })
            .collect(),
        result,
        return_source,
        body: body
            .into_iter()
            .enumerate()
            .map(|(index, op)| ir::Instruction {
                column: 1,
                line: index + 1,
                op,
            })
            .collect(),
        test: false,
    }
}

fn call_reborrow(mutate: bool) -> ir::Program {
    use ir::Projection::{Deref, Field};
    let mut types = ir::primitives();
    let cell = add_type(&mut types, "Cell", record(&[ir::I64], false));
    let pair = add_type(&mut types, "Pair", record(&[cell, cell], false));
    let pair_ref = add_type(&mut types, "PairRef", reference(pair));
    let cell_ref = add_type(&mut types, "CellRef", reference(cell));
    let scalar_ref = add_type(&mut types, "ScalarRef", reference(ir::I64));
    let choose = function(
        "choose",
        1,
        &[(pair_ref, false), (cell_ref, false)],
        cell_ref,
        source(Vec::new()),
        vec![
            ir::Op::Borrow {
                dst: 1,
                place: place(0, vec![Deref, Field(1)]),
                mutable: false,
            },
            drop_local(0),
            ir::Op::Return { src: 1 },
        ],
    );
    let mut body = vec![
        ir::Op::Int { dst: 0, value: 1 },
        ir::Op::Int { dst: 1, value: 2 },
        ir::Op::Struct {
            dst: 2,
            fields: vec![0],
        },
        ir::Op::Struct {
            dst: 3,
            fields: vec![1],
        },
        ir::Op::Struct {
            dst: 4,
            fields: vec![2, 3],
        },
        ir::Op::Borrow {
            dst: 5,
            place: ir::Place::local(4),
            mutable: false,
        },
        ir::Op::Call {
            dst: 6,
            function: 0,
            args: vec![5],
        },
        ir::Op::Borrow {
            dst: 7,
            place: place(6, vec![Deref, Field(0)]),
            mutable: false,
        },
        drop_local(6),
    ];
    if mutate {
        body.extend([
            ir::Op::Int { dst: 8, value: 9 },
            ir::Op::Assign {
                place: place(4, vec![Field(1), Field(0)]),
                src: 8,
            },
        ]);
    }
    body.extend([
        ir::Op::Read {
            dst: 9,
            place: place(7, vec![Deref]),
            moving: false,
        },
        drop_local(7),
        drop_local(4),
        ir::Op::Return { src: 9 },
    ]);
    let probe = function(
        "probe",
        0,
        &[
            (ir::I64, false),
            (ir::I64, false),
            (cell, false),
            (cell, false),
            (pair, true),
            (pair_ref, false),
            (cell_ref, false),
            (scalar_ref, false),
            (ir::I64, false),
            (ir::I64, false),
        ],
        ir::I64,
        None,
        body,
    );
    ir::Program {
        types,
        functions: vec![choose, probe],
        destructors: Vec::new(),
    }
}

fn forwarded_sources(owned_field: Option<usize>) -> ir::Program {
    use ir::Projection::{Deref, Field};
    let mut types = ir::primitives();
    let scalar_ref = add_type(&mut types, "ScalarRef", reference(ir::I64));
    let input = add_type(
        &mut types,
        "Input",
        record(&[scalar_ref, scalar_ref, ir::I64], true),
    );
    let input_ref = add_type(&mut types, "InputRef", reference(input));
    let candidates = add_type(
        &mut types,
        "Candidates",
        record(&[scalar_ref, scalar_ref], true),
    );
    let choose = function(
        "choose",
        1,
        &[(candidates, false), (scalar_ref, false)],
        scalar_ref,
        source(Vec::new()),
        vec![
            ir::Op::Read {
                dst: 1,
                place: place(0, vec![Field(owned_field.unwrap_or(0))]),
                moving: false,
            },
            drop_local(0),
            ir::Op::Return { src: 1 },
        ],
    );
    let retain = function(
        "retain",
        1,
        &[(scalar_ref, false), (scalar_ref, false)],
        scalar_ref,
        source(Vec::new()),
        vec![
            ir::Op::Read {
                dst: 1,
                place: ir::Place::local(0),
                moving: false,
            },
            drop_local(0),
            ir::Op::Return { src: 1 },
        ],
    );
    let mut body = Vec::new();
    for field in 0..2 {
        if owned_field == Some(field) {
            body.push(ir::Op::Borrow {
                dst: field + 1,
                place: place(0, vec![Deref, Field(2)]),
                mutable: false,
            });
        } else {
            body.push(ir::Op::Read {
                dst: field + 1,
                place: place(0, vec![Deref, Field(field)]),
                moving: false,
            });
        }
    }
    body.extend([
        ir::Op::Struct {
            dst: 3,
            fields: vec![1, 2],
        },
        ir::Op::Call {
            dst: 4,
            function: 0,
            args: vec![3],
        },
        ir::Op::Borrow {
            dst: 5,
            place: place(4, vec![Deref]),
            mutable: false,
        },
        drop_local(4),
        ir::Op::Call {
            dst: 6,
            function: 1,
            args: vec![5],
        },
        drop_local(0),
        ir::Op::Return { src: 6 },
    ]);
    let wrapper = function(
        "forward",
        1,
        &[
            (input_ref, false),
            (scalar_ref, false),
            (scalar_ref, false),
            (candidates, false),
            (scalar_ref, false),
            (scalar_ref, false),
            (scalar_ref, false),
        ],
        scalar_ref,
        source(vec![Deref]),
        body,
    );
    ir::Program {
        types,
        functions: vec![choose, retain, wrapper],
        destructors: Vec::new(),
    }
}

#[test]
fn reborrowing_a_call_result_does_not_narrow_its_owner_footprint() {
    let error = compiler::check_bundle(&bundle(REBORROW))
        .err()
        .expect("a live reborrow must prevent mutation of its possible referent");
    assert_eq!(error.code, "E_BORROW", "{error:?}");
    assert_eq!(error.line, 9, "{error:?}");
    let rust = rust_metadata(REBORROW);
    assert!(!rust.status.success());
    assert!(
        String::from_utf8_lossy(&rust.stderr).contains("E0506"),
        "{}",
        String::from_utf8_lossy(&rust.stderr)
    );
}

#[test]
fn direct_ir_reborrow_preserves_an_opaque_call_footprint_after_carrier_drop() {
    let valid = check::check(call_reborrow(false)).unwrap();
    run(&valid, 2);
    let error = check::check(call_reborrow(true))
        .err()
        .expect("IR must retain the borrowed whole-owner footprint");
    assert_eq!(error.code, "E_BORROW", "{error:?}");
    assert_eq!(error.line, 11, "{error:?}");
}

#[test]
fn direct_ir_forwarding_requires_every_possible_source_after_reborrow() {
    let valid = forwarded_sources(None);
    check::check(valid.clone()).unwrap();
    let encoded = ir_text::write(&valid);
    let decoded = ir_text::read(&encoded).unwrap();
    assert!(decoded == valid);
    check::check(decoded).unwrap();
    for field in [0, 1] {
        let error = check::check(forwarded_sources(Some(field)))
            .err()
            .expect("one valid candidate cannot excuse an invalid source candidate");
        assert_eq!(error.code, "E_BORROW", "{error:?}");
        assert!(
            error
                .message
                .contains("returned view differs from its declared source"),
            "{error:?}"
        );
    }
}

#[test]
fn absent_input_reference_leaves_can_still_produce_static_text() {
    let source = r#"
fn selected<'a>(value:&Option<&'a str>)->&'a str {
    match value {Option::Some(text)=>{return *text;},Option::None=>{}}
    "static"
}
pub fn probe()->usize {
    let value:Option<&str>=Option::None;
    let returned=selected(&value);
    drop(value);
    returned.len()
}
"#;
    let rust = rust_metadata(source);
    assert!(
        rust.status.success(),
        "{}",
        String::from_utf8_lossy(&rust.stderr)
    );
    run(
        &compiler::check_bundle(&bundle(source)).unwrap_or_else(|error| panic!("{error:?}")),
        6,
    );
}

fn bounded_write(replace: bool, reference_value: bool, mutate_referent: bool) -> ir::Program {
    let mut program =
        ir_text::read(include_str!("fixtures/bounded_reference_replacement.lkjir")).unwrap();
    if !reference_value {
        let ir::TypeKind::Struct { fields, .. } = &mut program.types[9].kind else {
            panic!("fixture Holder must be a struct");
        };
        fields.push(ir::Field {
            name: "cache".to_string(),
            ty: ir::I64,
        });
        program.types[11].kind = ir::TypeKind::Ref {
            inner: ir::I64,
            mutable: true,
        };
        program.functions[0].body[0].op = ir::Op::Borrow {
            dst: 1,
            place: place(0, vec![ir::Projection::Deref, ir::Projection::Field(2)]),
            mutable: true,
        };
        let probe = &mut program.functions[2];
        probe.locals[8].ty = ir::I64;
        probe.locals[9].ty = ir::I64;
        probe.locals[11].ty = ir::I64;
        probe.result = ir::I64;
        probe.locals.push(ir::Local {
            name: "cache".to_string(),
            ty: ir::I64,
            mutable: false,
        });
        for instruction in &mut probe.body {
            match &mut instruction.op {
                ir::Op::Struct { dst: 5, fields } => fields.push(12),
                ir::Op::Borrow { dst: 8, .. } => {
                    instruction.op = ir::Op::Int { dst: 8, value: 8 };
                }
                ir::Op::Unit { dst: 11 } => {
                    instruction.op = ir::Op::Int { dst: 11, value: 0 };
                }
                _ => {}
            }
        }
        let at = probe
            .body
            .iter()
            .position(|i| matches!(i.op, ir::Op::Struct { dst: 5, .. }))
            .unwrap();
        probe.body.insert(
            at,
            ir::Instruction {
                column: 1,
                line: 6,
                op: ir::Op::Int { dst: 12, value: 7 },
            },
        );
    }
    if !replace {
        for instruction in &mut program.functions[2].body {
            if matches!(&instruction.op, ir::Op::Intrinsic { name, .. } if name == "replace") {
                instruction.op = ir::Op::Assign {
                    place: place(7, vec![ir::Projection::Deref]),
                    src: 8,
                };
            } else if matches!(&instruction.op, ir::Op::Drop { place } if *place == ir::Place::local(9))
            {
                instruction.op = drop_local(7);
            }
        }
    }
    if !mutate_referent {
        program.functions[2]
            .body
            .retain(|i| i.line != 12 && i.line != 13);
    }
    program
}

#[test]
fn bounded_reference_writes_cannot_erase_sibling_provenance_before_drop() {
    for replace in [true, false] {
        let error = check::check(bounded_write(replace, true, true))
            .err()
            .expect("a conservative storage bound cannot identify one replaceable reference slot");
        assert_eq!(error.code, "E_BORROW", "{error:?}");
    }
}

#[test]
fn bounded_scalar_writes_preserve_sibling_loans_and_destructor_inputs() {
    for replace in [true, false] {
        let valid = check::check(bounded_write(replace, false, false)).unwrap();
        run(&valid, 0);
        let error = check::check(bounded_write(replace, false, true))
            .err()
            .expect("a scalar write must retain its sibling reference loans");
        assert_eq!(error.code, "E_BORROW", "{error:?}");
        assert_eq!(error.line, 13, "{error:?}");
    }
}
