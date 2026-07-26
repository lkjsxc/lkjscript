use super::*;
use crate::analyze::verify_match_plans;

fn source() -> String {
    concat!(
        "edition/\n2\n/edition\nmain/\nsig/\n->\nI64\n/sig\n",
        "match/\ntrue\narms/\n",
        "arm/\nbool-pattern/\nfalse\n/bool-pattern\n0\n/arm\n",
        "arm/\nbool-pattern/\ntrue\n/bool-pattern\n1\n/arm\n",
        "/arms\n/match\n/main\n",
    )
    .into()
}

#[test]
fn match_plan_verification_rejects_forged_and_stale_facts() {
    let program = analyze_one(&source()).expect("analyze exhaustive match");
    verify_match_plans(&program).expect("current plan verifies");

    let mut stale_type = program.clone();
    stale_type.match_plans[0].scrutinee.ty = Type::I64;
    assert!(verify_match_plans(&stale_type)
        .expect_err("stale scrutinee type")
        .to_string()
        .contains("stale"));

    let mut forged_order = program.clone();
    forged_order.match_plans[0].arms.swap(0, 1);
    assert!(verify_match_plans(&forged_order)
        .expect_err("forged source order")
        .to_string()
        .contains("identity/order"));

    let mut forged_edge = program;
    forged_edge.match_plans[0].edges.clear();
    assert!(verify_match_plans(&forged_edge)
        .expect_err("forged default edge")
        .to_string()
        .contains("edges"));
}
