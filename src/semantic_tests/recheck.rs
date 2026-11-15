//! Existing checked IR proves only exact bodies under an unchanged shared graph.
use crate::{check, diagnostic, ir, optimizer, reuse};

fn local(name: &str, ty: usize) -> ir::Local {
    ir::Local { name: name.to_string(), ty, mutable: false }
}
fn instruction(op: ir::Op, line: usize) -> ir::Instruction {
    ir::Instruction { op, file: None, line, column: 3 }
}
fn function(
    name: &str, params: usize, result: usize, locals: Vec<ir::Local>, operations: Vec<ir::Op>,
) -> ir::Function {
    let mut body = Vec::new();
    for (index, op) in operations.into_iter().enumerate() {
        body.push(instruction(op, index + 1));
    }
    ir::Function {
        file: 0, name: name.to_string(), params, result, return_source: None,
        locals, body, test: false,
    }
}
fn literal(name: &str, value: i64) -> ir::Function {
    function(name, 0, ir::I64, vec![local("value", ir::I64)], vec![
        ir::Op::Int { dst: 0, value }, ir::Op::Return { src: 0 },
    ])
}
fn small() -> ir::Program {
    ir::Program {
        types: ir::primitives(), functions: vec![literal("first", 41), literal("second", 42)],
        destructors: Vec::new(),
    }
}
fn calls() -> ir::Program {
    ir::Program {
        types: ir::primitives(), destructors: Vec::new(),
        functions: vec![
            function("identity", 1, ir::I64, vec![local("input", ir::I64)], vec![
                ir::Op::Return { src: 0 },
            ]),
            function("probe", 0, ir::I64, vec![local("argument", ir::I64), local("result", ir::I64)], vec![
                ir::Op::Int { dst: 0, value: 42 },
                ir::Op::Call { dst: 1, function: 0, args: vec![0usize] },
                ir::Op::Return { src: 1 },
            ]),
        ],
    }
}
fn records() -> reuse::Records { reuse::Records::new(&[0x51u8; 32], &[0x62u8; 32]) }
fn replay(previous: &reuse::Records) -> reuse::Records {
    let mut result = records();
    for pending in previous.pending() {
        assert!(result.load(&pending.namespace, &pending.key, &pending.bytes));
    }
    result
}
fn rejects_same(
    original: &ir::CheckedProgram, candidate: ir::Program, structural: bool,
) -> Vec<diagnostic::Diagnostic> {
    let before = original.program().clone();
    let expected = check::check_all(candidate.clone()).err().expect("candidate must fail full checking");
    let actual = check::recheck_all(original, candidate.clone()).err().expect("local reuse admitted invalid IR");
    assert!(actual == expected);
    let mut known = records();
    check::check_all_cached(reuse::Prepared::new(before.clone()), &mut known).unwrap();
    let mut loaded = replay(&known);
    let actual = check::recheck_all_cached(original, reuse::Prepared::new(candidate), &mut loaded)
        .err().expect("local and persistent reuse admitted invalid IR");
    assert!(actual == expected);
    if structural {
        assert_eq!(loaded.hits, 0usize);
        assert_eq!(loaded.misses, 0usize);
        assert_eq!(loaded.writes, 0usize);
    }
    assert!(original.program() == &before);
    expected
}

fn exact_bodies_reuse_proof_and_keep_candidate_values() {
    let original = check::check_all(small()).unwrap();
    let (unchanged, count) = check::recheck_all(&original, original.program().clone()).unwrap();
    assert_eq!(count, 2usize);
    assert!(unchanged.program() == original.program());
    let mut changed = original.program().clone();
    changed.functions[1].body[0].op = ir::Op::Int { dst: 0, value: 7 };
    changed.functions[1].body[0].file = Some(9usize);
    changed.functions[1].body[0].line = 24;
    changed.functions[1].body[0].column = 11;
    let (checked, count) = check::recheck_all(&original, changed.clone()).unwrap();
    assert_eq!(count, 1usize);
    assert!(checked.program() == &changed);
    assert!(check::check_all(changed.clone()).unwrap().program() == checked.program());

    let mut empty = records();
    let (_, count) = check::recheck_all_cached(
        &original, reuse::Prepared::new(original.program().clone()), &mut empty,
    ).unwrap();
    assert_eq!(count, 2usize);
    assert_eq!(empty.hits, 0usize);
    assert_eq!(empty.misses, 0usize);
    assert_eq!(empty.writes, 0usize);
    let (checked, count) = check::recheck_all_cached(
        &original, reuse::Prepared::new(changed.clone()), &mut empty,
    ).unwrap();
    assert_eq!(count, 1usize);
    assert_eq!(empty.hits, 0usize);
    assert_eq!(empty.misses, 1usize);
    assert_eq!(empty.writes, 1usize);
    assert!(checked.program() == &changed);
    let mut warm = replay(&empty);
    let (_, count) = check::recheck_all_cached(
        &original, reuse::Prepared::new(changed), &mut warm,
    ).unwrap();
    assert_eq!(count, 1usize);
    assert_eq!(warm.hits, 1usize);
    assert_eq!(warm.misses, 0usize);
    assert_eq!(warm.writes, 0usize);

    // Whole-function equality includes nonparameter locals and source provenance.
    let mut renamed = original.program().clone();
    renamed.functions[0].locals[0].name = "different local".to_string();
    let (_, count) = check::recheck_all(&original, renamed).unwrap();
    assert_eq!(count, 1usize);
    // Float instructions compare their stored bits, including NaNs and signed zero.
    for bits in vec![9221120237041090560u64, 0u64] {
        let raw = ir::Program {
            types: ir::primitives(), destructors: Vec::new(),
            functions: vec![function("float", 0, ir::F64, vec![local("value", ir::F64)], vec![
                ir::Op::Float { dst: 0, bits }, ir::Op::Return { src: 0 },
            ])],
        };
        let floating = check::check_all(raw).unwrap();
        let (_, count) = check::recheck_all(&floating, floating.program().clone()).unwrap();
        assert_eq!(count, 1usize);
        let mut changed = floating.program().clone();
        if bits == 0 {
            changed.functions[0].body[0].op = ir::Op::Float { dst: 0, bits: 9223372036854775808u64 };
            let (checked, count) = check::recheck_all(&floating, changed.clone()).unwrap();
            assert_eq!(count, 0usize);
            assert!(checked.program() == &changed);
        } else {
            changed.functions[0].body[0].op = ir::Op::Float { dst: 0, bits: bits ^ 1u64 };
            let problems = rejects_same(&floating, changed, false);
            assert!(problems[0].message.contains("canonical representation"));
        }
    }
}

fn changed_unreachable_bodies_keep_all_diagnostics() {
    let original = check::check_all(small()).unwrap();
    let mut changed = original.program().clone();
    changed.functions[0].body.push(ir::Instruction {
        op: ir::Op::Return { src: !0usize }, file: Some(7usize), line: 31, column: 17,
    });
    changed.functions[1].body[0] = ir::Instruction {
        op: ir::Op::Int { dst: !0usize, value: 9 }, file: Some(8usize), line: 41, column: 19,
    };
    let problems = rejects_same(&original, changed, false);
    assert_eq!(problems.len(), 2usize);
    assert_eq!(problems[0].file, 7usize);
    assert_eq!(problems[0].line, 31usize);
    assert_eq!(problems[0].column, 17usize);
    assert_eq!(problems[1].file, 8usize);
    assert_eq!(problems[1].line, 41usize);
    assert_eq!(problems[1].column, 19usize);
}

fn shared_structure_is_always_validated_first() {
    let original = check::check_all(small()).unwrap();
    for variant in 0usize..4usize {
        let mut changed = original.program().clone();
        match variant {
            0 => changed.functions[0].params = 2,
            1 => changed.functions[1].name = "first".to_string(),
            2 => changed.functions[0].return_source = Some(ir::ReturnSource::Leaves(Vec::new())),
            _ => changed.destructors.push((!0usize, 0usize)),
        }
        rejects_same(&original, changed, true);
    }
}

fn callee_signatures_and_numeric_identity_invalidate_callers() {
    let original = check::check_all(calls()).unwrap();
    let mut changed = original.program().clone();
    changed.functions[0].locals[0].ty = ir::U64;
    changed.functions[0].result = ir::U64;
    let problems = rejects_same(&original, changed, false);
    assert_eq!(problems.len(), 1usize);
    assert_eq!(problems[0].code, "E_TYPE");
    assert_eq!(problems[0].line, 2usize);
    let mut reordered = original.program().clone();
    reordered.functions = vec![original.program().functions[1].clone(), original.program().functions[0].clone()];
    rejects_same(&original, reordered, false);
    let mut added = original.program().clone();
    added.functions.push(literal("extra", 9));
    let (_, count) = check::recheck_all(&original, added).unwrap();
    assert_eq!(count, 0usize);
    let mut renamed = original.program().clone();
    renamed.functions[0].locals[0].name = "renamed parameter".to_string();
    let (_, count) = check::recheck_all(&original, renamed).unwrap();
    assert_eq!(count, 0usize);
}

fn shared_types_destructors_and_return_contracts_invalidate_bodies() {
    let mut copying = ir::Program { types: ir::primitives(), functions: Vec::new(), destructors: Vec::new() };
    let owner = copying.types.len();
    copying.types.push(ir::Type {
        name: "CopyOwner".to_string(), kind: ir::TypeKind::Struct {
            fields: vec![ir::Field { name: "value".to_string(), ty: ir::I64 }], copy: true,
        },
    });
    copying.functions.push(function("copy", 1, owner, vec![local("input", owner), local("copy", owner)], vec![
        ir::Op::Read { dst: 1, place: ir::Place::local(0), moving: false },
        ir::Op::Drop { place: ir::Place::local(0) }, ir::Op::Return { src: 1 },
    ]));
    let original = check::check_all(copying).unwrap();
    let mut changed = original.program().clone();
    if let ir::TypeKind::Struct { copy, .. } = &mut changed.types[owner].kind { *copy = false; }
    rejects_same(&original, changed, false);

    let mut dropping = ir::Program { types: ir::primitives(), functions: Vec::new(), destructors: Vec::new() };
    let owner = dropping.types.len();
    dropping.types.push(ir::Type {
        name: "Owner".to_string(), kind: ir::TypeKind::Struct { fields: Vec::new(), copy: false },
    });
    let reference = dropping.types.len();
    dropping.types.push(ir::Type {
        name: "OwnerRef".to_string(), kind: ir::TypeKind::Ref { inner: owner, mutable: true },
    });
    dropping.functions.push(function("destroy", 1, ir::UNIT, vec![local("owner", reference), local("unit", ir::UNIT)], vec![
        ir::Op::Drop { place: ir::Place::local(0) }, ir::Op::Unit { dst: 1 }, ir::Op::Return { src: 1 },
    ]));
    let mut parameter = local("owner", owner);
    parameter.mutable = true;
    dropping.functions.push(function("call_destroy", 1, ir::UNIT,
        vec![parameter, local("view", reference), local("unit", ir::UNIT)], vec![
            ir::Op::Borrow { dst: 1, place: ir::Place::local(0), mutable: true },
            ir::Op::Call { dst: 2, function: 0, args: vec![1usize] },
            ir::Op::Drop { place: ir::Place::local(0) }, ir::Op::Return { src: 2 },
        ],
    ));
    let original = check::check_all(dropping).unwrap();
    let mut changed = original.program().clone();
    changed.destructors.push((owner, 0usize));
    let problems = rejects_same(&original, changed, false);
    assert!(problems[0].message.contains("destructor functions"));

    let mut returning = ir::Program { types: ir::primitives(), functions: Vec::new(), destructors: Vec::new() };
    let reference = returning.types.len();
    returning.types.push(ir::Type {
        name: "View".to_string(), kind: ir::TypeKind::Ref { inner: ir::I64, mutable: false },
    });
    let mut choose = function("choose", 2, reference, vec![local("first", reference), local("second", reference)], vec![
        ir::Op::Drop { place: ir::Place::local(1) }, ir::Op::Return { src: 0 },
    ]);
    choose.return_source = Some(ir::ReturnSource::Parameter { index: 0, projections: Vec::new() });
    returning.functions.push(choose);
    let mut caller = function("return_view", 1, reference,
        vec![local("input", reference), local("temporary", ir::I64), local("view", reference), local("result", reference)],
        vec![
            ir::Op::Int { dst: 1, value: 7 },
            ir::Op::Borrow { dst: 2, place: ir::Place::local(1), mutable: false },
            ir::Op::Call { dst: 3, function: 0, args: vec![0usize, 2usize] },
            ir::Op::Drop { place: ir::Place::local(1) }, ir::Op::Return { src: 3 },
        ],
    );
    caller.return_source = Some(ir::ReturnSource::Parameter { index: 0, projections: Vec::new() });
    returning.functions.push(caller);
    let original = check::check_all(returning).unwrap();
    let mut changed = original.program().clone();
    changed.functions[0].return_source = Some(ir::ReturnSource::Parameter { index: 1, projections: Vec::new() });
    changed.functions[0].body[0].op = ir::Op::Drop { place: ir::Place::local(0) };
    changed.functions[0].body[1].op = ir::Op::Return { src: 1 };
    let problems = rejects_same(&original, changed, false);
    assert_eq!(problems.len(), 1usize);
    assert_eq!(problems[0].code, "E_BORROW");
}

fn optimizer_recheck_and_fallback_match_the_independent_gate() {
    let raw = calls();
    let original = check::check_all(raw.clone()).unwrap();
    let rewrite = optimizer::inline(original.program(), 8usize);
    assert_eq!(rewrite.calls, 1usize);
    assert_eq!(rewrite.functions, 1usize);
    let candidate = rewrite.program.unwrap();
    let expected = check::check_all(candidate.clone()).unwrap();
    let (checked, count) = check::recheck_all(&original, candidate.clone()).unwrap();
    assert_eq!(count, 1usize);
    assert!(checked.program() == expected.program());
    let selected = reuse::Selection::after_check(
        reuse::Checked::uncached(original), Ok(reuse::Checked::uncached(checked)),
    );
    assert!(selected.optimized());
    assert!(!selected.rejected());
    assert!(selected.original().program() == &raw);
    assert!(selected.native().program() == &candidate);

    let original = check::check_all(raw.clone()).unwrap();
    let mut damaged = candidate;
    damaged.functions[1].body.push(instruction(ir::Op::Return { src: !0usize }, 99usize));
    let expected = check::check_all(damaged.clone()).err().unwrap();
    let result = check::recheck_all(&original, damaged);
    assert!(result.as_ref().err().unwrap() == &expected);
    let result = result.map(|(checked, _)| reuse::Checked::uncached(checked));
    let selected = reuse::Selection::after_check(reuse::Checked::uncached(original), result);
    assert!(selected.rejected());
    assert!(!selected.optimized());
    assert!(selected.original().program() == &raw);
    assert!(selected.native().program() == &raw);
}

pub fn run(world: &mut World, trace: bool) -> usize {
    if trace { world.stdout().write(b"recheck_exact_bodies_and_candidate_values\n"); }
    exact_bodies_reuse_proof_and_keep_candidate_values();
    if trace { world.stdout().write(b"recheck_changed_unreachable_diagnostics\n"); }
    changed_unreachable_bodies_keep_all_diagnostics();
    if trace { world.stdout().write(b"recheck_shared_structure_first\n"); }
    shared_structure_is_always_validated_first();
    if trace { world.stdout().write(b"recheck_callee_signatures_and_indices\n"); }
    callee_signatures_and_numeric_identity_invalidate_callers();
    if trace { world.stdout().write(b"recheck_types_destructors_and_return_contracts\n"); }
    shared_types_destructors_and_return_contracts_invalidate_bodies();
    if trace { world.stdout().write(b"recheck_optimizer_admission_and_fallback\n"); }
    optimizer_recheck_and_fallback_match_the_independent_gate();
    6usize
}
