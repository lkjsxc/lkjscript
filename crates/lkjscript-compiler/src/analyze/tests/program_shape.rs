use super::*;

#[test]
fn explicit_main_is_unique_root_only_and_exactly_typed() {
    assert!(analysis_error("").contains("exactly one main"));
    let duplicate =
        "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
    assert!(analysis_error(duplicate).contains("duplicate main"));
    let mismatch = main_source("I64", "unit");
    assert!(analysis_error(&mismatch).contains("does not exactly equal"));
    let parameter = "main/\nsig/\nI64\n->\nI64\n/sig\n1\n/main\n";
    assert!(analysis_error(parameter).contains("requires params"));

    let ast = parsed_program(&[
        (
            "lib.lkjscript",
            "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n",
        ),
        (
            "root.lkjscript",
            "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n",
        ),
    ])
    .expect("parse imported main");
    assert!(analyze_program(&ast)
        .expect_err("imported main must fail")
        .to_string()
        .contains("imported file"));
}

#[test]
fn top_level_do_and_runtime_value_definitions_are_removed() {
    assert!(analysis_error("do/\nunit\n/do\n").contains("top-level do"));
    let value = "def/\nname/\nx\n/name\ntype/\nI64\n/type\n1\n/def\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
    assert!(analysis_error(value).contains("malformed top-level def declaration shape"));
}

#[test]
fn main_and_function_are_explicit_hir_nodes() {
    let source = "def/\nname/\nanswer\n/name\nfn/\nsig/\n->\nI64\n/sig\nparams/\n/params\n42\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nanswer/\n/answer\n/main\n";
    let program = analyze_one(source).expect("analyze explicit callable program");
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.main.return_type, Type::I64);
    assert_eq!(program.main.body.ty, Type::I64);
    assert_eq!(program.global_layout, vec![program.functions[0].binding]);
    assert_eq!(program.global_layout.len(), 1);
    let ExprKind::Call { callee, .. } = program.main.body.kind else {
        panic!("expected resolved function call");
    };
    assert_eq!(callee.storage, BindingStorage::Function);

    let chunk = compile_hir(&program).expect("lower explicit main through SSA");
    assert_eq!(chunk.global_names, vec!["answer"]);
    assert_eq!(chunk.main.name, "main");
    assert!(chunk.main.code.ends_with(&[Op::Return as u8]));
}

#[test]
fn mutable_local_has_stable_binding_and_slot_and_set_resolves_nearest() {
    let source = main_source(
        "I64",
        "var/\nname/\nx\n/name\ntype/\nI64\n/type\n1\nvar/\nname/\nx\n/name\ntype/\nI64\n/type\nx\ndo/\nset/\nx\n2\n/set\nx\n/do\n/var\n/var",
    );
    let program = analyze_one(&source).expect("analyze nested vars");
    let ExprKind::MutableLocal {
        binding: outer,
        slot: outer_slot,
        body: outer_body,
        ..
    } = &program.main.body.kind
    else {
        panic!("expected outer mutable local");
    };
    let ExprKind::MutableLocal {
        binding: inner,
        slot: inner_slot,
        initial,
        body: inner_body,
        ..
    } = &outer_body.kind
    else {
        panic!("expected inner mutable local");
    };
    assert_ne!(outer, inner);
    assert_eq!((*outer_slot, *inner_slot), (0, 1));
    assert_eq!(program.main.local_count, 2);
    assert_eq!(
        program.binding(*outer).map(|binding| &binding.kind),
        Some(&BindingKind::MutableLocal)
    );
    let ExprKind::Load(initial_ref) = initial.kind else {
        panic!("inner initializer must resolve outer binding");
    };
    assert_eq!(initial_ref.binding, *outer);
    assert_eq!(initial_ref.storage, BindingStorage::Local(0));
    let ExprKind::Do(expressions) = &inner_body.kind else {
        panic!("expected inner body sequence");
    };
    let ExprKind::SetLocal { target, slot, .. } = expressions[0].kind else {
        panic!("expected local set");
    };
    assert_eq!(target, *inner);
    assert_eq!(slot, 1);
    assert_eq!(expressions[0].effects, EffectSet::MUTATES_LOCAL);
}

#[test]
fn var_initializer_set_type_and_binding_kinds_are_checked() {
    let self_ref = main_source(
        "I64",
        "var/\nname/\nx\n/name\ntype/\nI64\n/type\nx\nx\n/var",
    );
    assert!(analysis_error(&self_ref).contains("unknown symbol x"));
    let wrong_initial = main_source(
        "Unit",
        "var/\nname/\nx\n/name\ntype/\nI64\n/type\n1.0\nunit\n/var",
    );
    assert!(analysis_error(&wrong_initial).contains("exactly equal"));
    let wrong_set = main_source(
        "Unit",
        "var/\nname/\nx\n/name\ntype/\nI64\n/type\n1\nset/\nx\n1.0\n/set\n/var",
    );
    assert!(analysis_error(&wrong_set).contains("exactly equal"));
    let immutable = main_source("Unit", "let/\nbind/\nx\n1\n/bind\nset/\nx\n2\n/set\n/let");
    assert!(analysis_error(&immutable).contains("not a function-local mutable var"));
}

#[test]
fn resolution_never_crosses_a_function_boundary() {
    let source = "def/\nname/\nmutate\n/name\nfn/\nsig/\n->\nUnit\n/sig\nparams/\n/params\nset/\nstate\n1\n/set\n/fn\n/def\nmain/\nsig/\n->\nUnit\n/sig\nvar/\nname/\nstate\n/name\ntype/\nI64\n/type\n0\nmutate/\n/mutate\n/var\n/main\n";
    assert!(analysis_error(source).contains("unknown set target state"));
}

#[test]
fn operation_identity_and_local_mutation_reach_bytecode() {
    let source = concat!(
        "main/\nsig/\nCapability/\nArguments\n/Capability\n->\nI64\n/sig\n",
        "params/\narguments\nCapability/\nArguments\n/Capability\n/params\n",
        "var/\nname/\nx\n/name\ntype/\nI64\n/type\n",
        "argc/\narguments\n/argc\ndo/\nset/\nx\n+/\nx\n2\n/+\n/set\nx\n/do\n/var\n/main\n"
    );
    let program = analyze_one(source).expect("analyze operation and set");
    let chunk = compile_hir(&program).expect("lower operation and set through SSA");
    assert!(chunk.main.code.contains(&(Op::Add as u8)));
    assert!(chunk.main.code.contains(&(Op::StoreLocal as u8)));
    assert!(chunk.global_names.is_empty());
}
