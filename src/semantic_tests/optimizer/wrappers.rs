//! Small acyclic wrappers may retain ordinary calls to larger checked bodies.
use super::*;

fn local(name: &str, ty: usize, mutable: bool) -> ir::Local {
    ir::Local { name: name.to_string(), ty, mutable }
}
fn instruction(op: ir::Op) -> ir::Instruction {
    ir::Instruction { op, file: None, line: 3, column: 5 }
}
fn function(name: &str, params: usize, result: usize, locals: Vec<ir::Local>, body: Vec<ir::Instruction>) -> ir::Function {
    ir::Function {
        file: 0, name: name.to_string(), params, result, return_source: None,
        locals, body, test: false,
    }
}
fn calls(function: &ir::Function) -> Vec<usize> {
    let mut result = Vec::new();
    for instruction in &function.body {
        if let ir::Op::Call { function, .. } = instruction.op { result.push(function); }
    }
    result
}
fn selected(program: ir::Program, limit: usize) -> reuse::Selection {
    let original = reuse::Checked::uncached(check::check_all(program).unwrap());
    let before = original.program().clone();
    let rewrite = optimizer::inline(original.program(), limit);
    assert!(rewrite.calls > 0);
    assert!(original.program() == &before);
    let result = admitted(rewrite.program.unwrap());
    assert!(result.is_ok());
    reuse::Selection::after_check(original, result)
}
fn observed(world: &mut World, selected: &reuse::Selection, runtime: &runtime::Runtime, expected: i64) {
    assert!(selected.optimized());
    assert_eq!(value(selected.original().program()), expected);
    assert_eq!(native_value(world, selected.native(), runtime), expected);
}
fn wrapper_is_the_only_eligible_opportunity(world: &mut World, runtime: &runtime::Runtime) {
    let mut body = Vec::new();
    for _ in 0usize..16usize {
        body.push(instruction(ir::Op::Int { dst: 1, value: 1 }));
        body.push(instruction(ir::Op::Binary { dst: 0, op: ir::BinaryOp::Add, left: 0, right: 1 }));
    }
    body.push(instruction(ir::Op::Return { src: 0 }));
    let program = ir::Program {
        types: ir::primitives(), destructors: Vec::new(), functions: vec![
            function("large", 1, ir::I64, vec![local("value", ir::I64, false), local("one", ir::I64, false)], body),
            function("wrapper", 1, ir::I64, vec![local("value", ir::I64, false), local("result", ir::I64, false)], vec![
                instruction(ir::Op::Call { dst: 1, function: 0, args: vec![0usize] }),
                instruction(ir::Op::Return { src: 1 }),
            ]),
            function("probe", 0, ir::I64, vec![local("value", ir::I64, false), local("result", ir::I64, false)], vec![
                instruction(ir::Op::Int { dst: 0, value: 26 }),
                instruction(ir::Op::Call { dst: 1, function: 1, args: vec![0usize] }),
                instruction(ir::Op::Return { src: 1 }),
            ]),
        ],
    };
    assert!(program.functions[0].body.len() > 8usize);
    let selected = selected(program, 8usize);
    observed(world, &selected, runtime, 42);
    assert_eq!(calls(&selected.original().program().functions[2]), vec![1usize]);
    assert_eq!(calls(&selected.native().program().functions[2]), vec![0usize]);
    assert!(selected.native().program().functions[0] == selected.original().program().functions[0]);
    assert_eq!(calls(&selected.native().program().functions[1]), vec![0usize]);
    // The dynamic probe -> wrapper -> large chain is now probe -> large.
    assert_eq!(selected.native().program().functions[2].body.len(), 3usize);
}
fn recursive_bodies_and_blocked_wrappers_never_become_eligible(world: &mut World, runtime: &runtime::Runtime) {
    let original = compiled(r#"
        fn recursive(value: i64) -> i64 {
            if value == 0 { return 42; }
            recursive(value - 1)
        }
        pub fn probe() -> i64 { recursive(3) }
    "#);
    let rewrite = optimizer::inline(original.program(), 128usize);
    assert_eq!(rewrite.calls, 0usize);
    assert!(rewrite.program.is_none());
    let unchanged = reuse::Selection::unchanged(original);
    assert!(equivalent(world, &unchanged, runtime));
    assert_eq!(value(unchanged.original().program()), 42i64);

    let selected = transformed(r#"
        fn leaf(value: i64) -> i64 { value + 1 }
        fn recursive(value: i64) -> i64 {
            if value == 0 { return 41; }
            let next = recursive(value - 1);
            leaf(next) - 1
        }
        fn blocked_wrapper(value: i64) -> i64 { recursive(value) }
        pub fn probe() -> i64 { leaf(blocked_wrapper(3)) }
    "#, 128usize);
    observed(world, &selected, runtime, 42);
    let program = selected.native().program();
    let recursive = index(program, "recursive");
    let wrapper = index(program, "blocked_wrapper");
    let leaf = index(program, "leaf");
    let recursive_calls = calls(&program.functions[recursive]);
    assert!(recursive_calls.contains(&recursive));
    assert!(!recursive_calls.contains(&leaf));
    assert_eq!(calls(&program.functions[wrapper]), vec![recursive]);
    assert_eq!(calls(&program.functions[index(program, "probe")]), vec![wrapper]);
}
fn retained_large_calls_keep_observable_effects(world: &mut World, runtime: &runtime::Runtime) {
    let mut types = ir::primitives();
    let reference = types.len();
    types.push(ir::Type {
        name: "Counter".to_string(), kind: ir::TypeKind::Ref { inner: ir::I64, mutable: true },
    });
    let counter = ir::Place { local: 0, projections: vec![ir::Projection::Deref] };
    let mut body = Vec::new();
    for _ in 0usize..12usize {
        body.push(instruction(ir::Op::Read { dst: 3, place: counter.clone(), moving: false }));
        body.push(instruction(ir::Op::Int { dst: 2, value: 1 }));
        body.push(instruction(ir::Op::Binary { dst: 4, op: ir::BinaryOp::Add, left: 3, right: 2 }));
        body.push(instruction(ir::Op::Assign { place: counter.clone(), src: 4 }));
    }
    body.push(instruction(ir::Op::Drop { place: ir::Place::local(0) }));
    body.push(instruction(ir::Op::Return { src: 1 }));
    let program = ir::Program {
        types, destructors: Vec::new(), functions: vec![
            function("large_effect", 2, ir::I64, vec![local("counter", reference, false), local("value", ir::I64, false),
                local("one", ir::I64, false), local("loaded", ir::I64, false), local("sum", ir::I64, false)], body),
            function("wrapper", 2, ir::I64, vec![local("counter", reference, false), local("value", ir::I64, false),
                local("result", ir::I64, false)], vec![
                    instruction(ir::Op::Call { dst: 2, function: 0, args: vec![0usize, 1usize] }),
                    instruction(ir::Op::Return { src: 2 }),
                ],
            ),
            function("probe", 0, ir::I64, vec![local("counter", ir::I64, true), local("view", reference, false),
                local("argument", ir::I64, false), local("result", ir::I64, false),
                local("counter_result", ir::I64, false), local("answer", ir::I64, false)], vec![
                    instruction(ir::Op::Int { dst: 0, value: 30 }),
                    instruction(ir::Op::Borrow { dst: 1, place: ir::Place::local(0), mutable: true }),
                    instruction(ir::Op::Int { dst: 2, value: 100 }),
                    instruction(ir::Op::Call { dst: 3, function: 1, args: vec![1usize, 2usize] }),
                    instruction(ir::Op::Read { dst: 4, place: ir::Place::local(0), moving: true }),
                    instruction(ir::Op::Binary { dst: 5, op: ir::BinaryOp::Add, left: 3, right: 4 }),
                    instruction(ir::Op::Return { src: 5 }),
                ],
            ),
        ],
    };
    assert!(program.functions[0].body.len() > 8usize);
    let selected = selected(program, 8usize);
    observed(world, &selected, runtime, 142);
    assert_eq!(calls(&selected.native().program().functions[2]), vec![0usize]);
}
pub(super) fn run(world: &mut World, runtime: &runtime::Runtime, trace: bool) -> usize {
    if trace { world.stdout().write(b"inline_small_wrapper_around_large_body\n"); }
    wrapper_is_the_only_eligible_opportunity(world, runtime);
    if trace { world.stdout().write(b"inline_recursive_and_dependent_exclusion\n"); }
    recursive_bodies_and_blocked_wrappers_never_become_eligible(world, runtime);
    if trace { world.stdout().write(b"inline_retained_large_callee_effects\n"); }
    retained_large_calls_keep_observable_effects(world, runtime);
    3usize
}
