//! Direct IR protects sparse ownership snapshots independently of elaboration.
use lkjscript::{check, ir};

fn program(locals: usize, branches: usize, consume_in_then: bool) -> ir::Program {
    let owner = locals - 1;
    let mut declarations = vec![
        ir::Local {
            name: String::new(),
            ty: ir::I64,
            mutable: false
        };
        locals
    ];
    declarations[0].ty = ir::BOOL;
    let mut ops = vec![ir::Op::Int {
        dst: owner,
        value: 42,
    }];
    for branch in 0..branches {
        ops.push(ir::Op::Int { dst: 0, value: 1 });
        ops.push(ir::Op::If { condition: 0 });
        ops.push(ir::Op::Int { dst: 1, value: 7 });
        ops.push(ir::Op::Drop {
            place: ir::Place::local(1),
        });
        if consume_in_then && branch + 1 == branches {
            ops.push(ir::Op::Drop {
                place: ir::Place::local(owner),
            });
        }
        ops.push(ir::Op::Else);
        ops.push(ir::Op::Int { dst: 2, value: 9 });
        ops.push(ir::Op::Drop {
            place: ir::Place::local(2),
        });
        ops.push(ir::Op::End);
    }
    ops.push(ir::Op::Return { src: owner });
    ir::Program {
        types: ir::primitives(),
        functions: vec![ir::Function {
            file: 0,
            name: "probe".to_string(),
            params: 0,
            result: ir::I64,
            return_source: None,
            locals: declarations,
            body: ops
                .into_iter()
                .enumerate()
                .map(|(index, op)| ir::Instruction {
                    column: 1,
                    line: index + 1,
                    op,
                })
                .collect(),
            test: false,
        }],
        destructors: Vec::new(),
    }
}

#[test]
fn sparse_local_snapshots_accept_balanced_branches() {
    // Far-apart owner bits and changing temporaries must survive Else restoration.
    for locals in [65, 4097, 65_537] {
        if let Err(diagnostic) = check::check(program(locals, 128, false)) {
            panic!("balanced branches rejected: {diagnostic:?}");
        }
    }
}

#[test]
fn sparse_local_snapshots_reject_reading_a_conditionally_consumed_owner() {
    for locals in [65, 4097, 65_537] {
        let diagnostic = match check::check(program(locals, 128, true)) {
            Ok(_) => panic!("read of a conditionally consumed owner accepted"),
            Err(diagnostic) => diagnostic,
        };
        assert_eq!(diagnostic.code, "E_MOVE");
        assert!(diagnostic.message.contains("moved on a reachable path"));
        assert_eq!(
            diagnostic.line,
            program(locals, 128, true).functions[0]
                .body
                .last()
                .unwrap()
                .line
        );
    }
}
