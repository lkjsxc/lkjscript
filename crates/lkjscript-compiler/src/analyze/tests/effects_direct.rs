use super::*;

#[test]
fn direct_effect_categories_are_inferred_without_unrelated_bits() {
    let allocation = function_source(
        "allocation",
        &[],
        "->\nList\nI64",
        "",
        "cons/\n1\nempty-list/\nI64\n/empty-list\n/cons",
    );
    let trapper = function_source("trapper", &[], "->\nI64", "", "div/\n8\n2\n/div");
    let host = function_source(
        "host",
        &[],
        "->\nUnit",
        "",
        "print/\nstr/\nhello\n/str\n/print",
    );
    let outcome = function_source("outcome", &[], "->\nUnit", "", "exit/\n0\n/exit");
    let mutation = function_source(
        "mutation",
        &[],
        "->\nUnit",
        "",
        "var/\nname/\nx\n/name\ntype/\nI64\n/type\n0\nset/\nx\n1\n/set\n/var",
    );
    let read = function_source(
        "memory-read",
        &[],
        "Buf\n->\nI64",
        "value\nBuf",
        "buf-len/\nvalue\n/buf-len",
    );
    let write = function_source(
        "memory-write",
        &[],
        "Buf\n->\nUnit",
        "value\nBuf",
        "buf-set/\nvalue\n0\n1\n/buf-set",
    );
    let source = format!(
        "{allocation}{trapper}{host}{outcome}{mutation}{read}{write}{}",
        main_source("Unit", "unit")
    );
    let program = analyze_one(&source).expect("analyze direct effect categories");
    assert_eq!(
        summary(&program, "allocation"),
        EffectSet::ALLOCATES.union(EffectSet::MAY_TRAP)
    );
    assert_eq!(summary(&program, "trapper"), EffectSet::MAY_TRAP);
    assert_eq!(
        summary(&program, "host"),
        EffectSet::HOST_IO
            .union(EffectSet::ALLOCATES)
            .union(EffectSet::MAY_TRAP)
    );
    assert_eq!(
        summary(&program, "outcome"),
        EffectSet::HOST_IO.union(EffectSet::MAY_EXIT)
    );
    assert_eq!(summary(&program, "mutation"), EffectSet::MUTATES_LOCAL);
    assert_eq!(summary(&program, "memory-read"), EffectSet::READS_MEMORY);
    assert_eq!(
        summary(&program, "memory-write"),
        EffectSet::WRITES_MEMORY.union(EffectSet::MAY_TRAP)
    );
}

#[test]
fn generic_direct_call_uses_canonical_binding_and_keeps_argument_effects() {
    let identity = function_source("identity", &["T"], "T\n->\nT", "value\nT", "value");
    let source = format!(
        "{identity}{}",
        main_source("I64", "identity/\ndiv/\n8\n2\n/div\n/identity")
    );
    let program = analyze_one(&source).expect("analyze generic direct call");
    let identity = &program.functions[0];
    assert_eq!(identity.summary, EffectSet::PURE);
    let ExprKind::Call { callee, args, .. } = &program.main.body.kind else {
        panic!("expected generic direct call");
    };
    assert_eq!(callee.binding, identity.binding);
    assert_eq!(callee.storage, BindingStorage::Function);
    assert_eq!(args[0].effects, EffectSet::MAY_TRAP);
    assert_eq!(program.main.body.effects, EffectSet::MAY_TRAP);
}

#[test]
fn indirect_local_call_is_conservative_and_loses_no_effect_bit() {
    let leaf = function_source("leaf", &[], "->\nI64", "", "7");
    let indirect = function_source(
        "indirect",
        &[],
        "->\nI64",
        "",
        "let/\nbind/\nf\nleaf\n/bind\nf/\n/f\n/let",
    );
    let source = format!("{leaf}{indirect}{}", main_source("Unit", "unit"));
    let program = analyze_one(&source).expect("analyze indirect local call");
    assert_eq!(summary(&program, "leaf"), EffectSet::PURE);
    assert_eq!(summary(&program, "indirect"), EffectSet::CONSERVATIVE_CALL);
    for required in [
        EffectSet::ALLOCATES,
        EffectSet::READS_MEMORY,
        EffectSet::WRITES_MEMORY,
        EffectSet::MUTATES_LOCAL,
        EffectSet::HOST_IO,
        EffectSet::MAY_TRAP,
        EffectSet::MAY_EXIT,
        EffectSet::MAY_DIVERGE,
    ] {
        assert!(EffectSet::CONSERVATIVE_CALL.contains(required));
    }
}
