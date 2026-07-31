use super::*;

#[test]
fn direct_effect_categories_are_inferred_without_unrelated_bits() {
    let allocation = function_source(
        "allocation",
        &[],
        "inputs/\n/inputs\noutput/\nlist/\ni64\n/list\n/output",
        "",
        "list-prepend/\n1\nempty-list/\ni64\n/empty-list\n/list-prepend",
    );
    let trapper = function_source(
        "trapper",
        &[],
        "inputs/\n/inputs\noutput/\ni64\n/output",
        "",
        "divide/\n8\n2\n/divide",
    );
    let host = function_source(
        "host",
        &[],
        "inputs/\ncapability/\nstdio\n/capability\n/inputs\noutput/\nunit\n/output",
        "stdio\ncapability/\nstdio\n/capability",
        "print/\nstdio\nstring-literal/\nhello\n/string-literal\n/print",
    );
    let outcome = function_source(
        "outcome",
        &[],
        "inputs/\n/inputs\noutput/\nunit\n/output",
        "",
        "exit/\n0\n/exit",
    );
    let mutation = function_source(
        "mutation",
        &[],
        "inputs/\n/inputs\noutput/\nunit\n/output",
        "",
        "var/\nname/\nx\n/name\ntype/\ni64\n/type\n0\nset/\nx\n1\n/set\n/var",
    );
    let read = function_source(
        "memory-read",
        &[],
        "inputs/\nbyte-slice\n/inputs\noutput/\ni64\n/output",
        "value\nbyte-slice",
        "byte-slice-length/\nvalue\n/byte-slice-length",
    );
    let write = function_source(
        "memory-write",
        &[],
        "inputs/\nbyte-slice-mut\n/inputs\noutput/\nunit\n/output",
        "value\nbyte-slice-mut",
        "byte-slice-mut-set-byte/\nvalue\n0\n1\n/byte-slice-mut-set-byte",
    );
    let source = format!(
        "{allocation}{trapper}{host}{outcome}{mutation}{read}{write}{}",
        main_source("unit", "unit")
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
    let identity = function_source(
        "identity",
        &["t"],
        "inputs/\nt\n/inputs\noutput/\nt\n/output",
        "value\nt",
        "value",
    );
    let source = format!(
        "{identity}{}",
        main_source("i64", "identity/\ndivide/\n8\n2\n/divide\n/identity")
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
    let leaf = function_source(
        "leaf",
        &[],
        "inputs/\n/inputs\noutput/\ni64\n/output",
        "",
        "7",
    );
    let indirect = function_source(
        "indirect",
        &[],
        "inputs/\n/inputs\noutput/\ni64\n/output",
        "",
        "let/\nbind/\nf\nleaf\n/bind\nf/\n/f\n/let",
    );
    let source = format!("{leaf}{indirect}{}", main_source("unit", "unit"));
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
