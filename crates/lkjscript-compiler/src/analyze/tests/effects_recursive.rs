use super::*;

#[test]
fn pure_leaf_and_direct_transitive_calls_remain_exact() {
    let pure_leaf = function_source(
        "pure-leaf",
        &[],
        "inputs/\n/inputs\noutput/\ni64\n/output",
        "",
        "7",
    );
    let pure_middle = function_source(
        "pure-middle",
        &[],
        "inputs/\n/inputs\noutput/\ni64\n/output",
        "",
        "pure-leaf/\n/pure-leaf",
    );
    let source = format!(
        "{pure_leaf}{pure_middle}{}",
        main_source("i64", "pure-middle/\n/pure-middle")
    );
    let program = analyze_one(&source).expect("analyze pure direct calls");
    assert_eq!(summary(&program, "pure-leaf"), EffectSet::PURE);
    assert_eq!(summary(&program, "pure-middle"), EffectSet::PURE);
    assert_eq!(program.main.body.effects, EffectSet::PURE);

    let trap_leaf = function_source(
        "trap-leaf",
        &[],
        "inputs/\n/inputs\noutput/\ni64\n/output",
        "",
        "divide/\n8\n2\n/divide",
    );
    let middle = function_source(
        "middle",
        &[],
        "inputs/\n/inputs\noutput/\ni64\n/output",
        "",
        "trap-leaf/\n/trap-leaf",
    );
    let outer = function_source(
        "outer",
        &[],
        "inputs/\n/inputs\noutput/\ni64\n/output",
        "",
        "middle/\n/middle",
    );
    let source = format!(
        "{trap_leaf}{middle}{outer}{}",
        main_source("i64", "outer/\n/outer")
    );
    let program = analyze_one(&source).expect("analyze transitive direct effects");
    assert_eq!(summary(&program, "trap-leaf"), EffectSet::MAY_TRAP);
    assert_eq!(summary(&program, "middle"), EffectSet::MAY_TRAP);
    assert_eq!(summary(&program, "outer"), EffectSet::MAY_TRAP);
    assert_eq!(program.main.body.effects, EffectSet::MAY_TRAP);
}

#[test]
fn pure_direct_recursion_adds_only_divergence() {
    let recurse = function_source(
        "recurse",
        &[],
        "inputs/\n/inputs\noutput/\nunit\n/output",
        "",
        "recurse/\n/recurse",
    );
    let source = format!("{recurse}{}", main_source("unit", "recurse/\n/recurse"));
    let program = analyze_one(&source).expect("analyze pure recursion");
    assert_eq!(summary(&program, "recurse"), EffectSet::MAY_DIVERGE);
    assert_eq!(program.functions[0].body.effects, EffectSet::MAY_DIVERGE);
    assert_eq!(program.main.body.effects, EffectSet::MAY_DIVERGE);
}

#[test]
fn effectful_recursion_retains_only_its_real_effects_and_divergence() {
    let recurse = function_source(
        "recurse",
        &[],
        "inputs/\n/inputs\noutput/\nunit\n/output",
        "",
        "do/\ndivide/\n8\n2\n/divide\nrecurse/\n/recurse\n/do",
    );
    let source = format!("{recurse}{}", main_source("unit", "unit"));
    let program = analyze_one(&source).expect("analyze effectful recursion");
    assert_eq!(
        summary(&program, "recurse"),
        EffectSet::MAY_TRAP.union(EffectSet::MAY_DIVERGE)
    );
}

#[test]
fn mutual_recursion_and_declaration_order_have_identical_summaries() {
    let a = function_source(
        "a",
        &[],
        "inputs/\n/inputs\noutput/\nunit\n/output",
        "",
        "b/\n/b",
    );
    let b = function_source(
        "b",
        &[],
        "inputs/\n/inputs\noutput/\nunit\n/output",
        "",
        "a/\n/a",
    );
    let caller = function_source(
        "caller",
        &[],
        "inputs/\n/inputs\noutput/\nunit\n/output",
        "",
        "a/\n/a",
    );
    let main = main_source("unit", "caller/\n/caller");
    let first = analyze_one(&format!("{a}{b}{caller}{main}"))
        .expect("analyze mutual recursion in first declaration order");
    let second = analyze_one(&format!("{b}{caller}{a}{main}"))
        .expect("analyze mutual recursion in second declaration order");
    for name in ["a", "b", "caller"] {
        assert_eq!(summary(&first, name), EffectSet::MAY_DIVERGE);
        assert_eq!(summary(&first, name), summary(&second, name));
    }
    assert_eq!(first.main.body.effects, EffectSet::MAY_DIVERGE);
    assert_eq!(first.main.body.effects, second.main.body.effects);
}
