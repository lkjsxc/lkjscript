use lkjscript::{check, compiler, ir, ir_text, source};
use std::path::Path;
use std::process::Command;

#[test]
fn capabilities_cannot_be_fabricated_by_source_or_raw_ir() {
    for text in [
        "fn probe() -> World { 42 }",
        "fn probe(mut world: World) { let out = world.stdout::<i64>(); }",
        "fn probe(mut world: World) { let mut out = world.stdout(); out.write::<i64>(&[42u8]); }",
        "fn probe() -> Stdout { Stdout {} }",
        "fn probe(mut world: World) { let other: World = world; let again: World = world; }",
        "fn probe(mut world: World) { let mut out: Stdout = world.stdout(); let copied: Stdout = out.clone(); }",
    ] {
        let bundle = source::SourceBundle {
            root: "forged.rs".to_string(),
            library: None,
            files: vec![source::SourceInput {
                path: "forged.rs".to_string(),
                text: text.to_string(),
            }],
        };
        assert!(compiler::compile(&bundle).is_err(), "accepted: {text}");
    }
    for kind in [ir::TypeKind::World, ir::TypeKind::Stdout] {
        for op in [
            ir::Op::Int { dst: 0, value: 1 },
            ir::Op::Unit { dst: 0 },
            ir::Op::Struct {
                dst: 0,
                fields: vec![],
            },
        ] {
            let mut types = ir::primitives();
            types.push(ir::Type {
                name: "capability".to_string(),
                kind: kind.clone(),
            });
            let program = ir::Program {
                types,
                destructors: vec![],
                functions: vec![ir::Function {
                    file: 0,
                    name: "probe".to_string(),
                    params: 0,
                    result: 7,
                    return_source: None,
                    test: false,
                    locals: vec![ir::Local {
                        name: "cap".to_string(),
                        ty: 7,
                        mutable: false,
                    }],
                    body: vec![
                        ir::Instruction {
                            column: 1,
                            op,
                            line: 1,
                        },
                        ir::Instruction {
                            column: 1,
                            op: ir::Op::Return { src: 0 },
                            line: 2,
                        },
                    ],
                }],
            };
            let text = ir_text::write(&program);
            assert!(check::check(ir_text::read(&text).unwrap()).is_err());
        }
    }
}

#[test]
fn host_output_error_is_reported_at_the_write() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/hello.rs");
    let sink = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_lkjscript"))
        .arg("run")
        .arg(source)
        .stdout(sink)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("R0007 host output failure"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
