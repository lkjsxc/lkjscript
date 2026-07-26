#![allow(clippy::expect_used)]

use super::{check, OWNERSHIP_ANALYSIS_MAX_EXPRESSION_NODES};
use crate::hir::{EffectSet, Expr, ExprKind, Main, Program, SourceId, Type};

#[test]
fn aggregate_expression_budget_is_enforced_on_constructed_hir() {
    let origin = SourceId::new(0);
    let leaf = || Expr {
        ty: Type::Unit,
        effects: EffectSet::PURE,
        origin,
        kind: ExprKind::LitUnit,
    };
    let body = Expr {
        ty: Type::Unit,
        effects: EffectSet::PURE,
        origin,
        kind: ExprKind::Do(
            (0..OWNERSHIP_ANALYSIS_MAX_EXPRESSION_NODES)
                .map(|_| leaf())
                .collect(),
        ),
    };
    let program = Program {
        sources: Vec::new(),
        bindings: Vec::new(),
        products: Vec::new(),
        enums: Vec::new(),
        traits: Vec::new(),
        implementations: Vec::new(),
        match_plans: Vec::new(),
        functions: Vec::new(),
        main: Main {
            origin,
            return_type: Type::Unit,
            local_count: 0,
            body,
        },
        global_layout: Vec::new(),
    };
    let error = check(&program)
        .expect_err("constructed HIR over the aggregate budget must fail")
        .to_string();
    assert!(
        error.contains(&format!(
            "ownership analysis expression budget exceeded {OWNERSHIP_ANALYSIS_MAX_EXPRESSION_NODES}"
        )),
        "wrong aggregate budget diagnostic: {error}"
    );
}
