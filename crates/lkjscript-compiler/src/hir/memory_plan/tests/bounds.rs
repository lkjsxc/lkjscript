use super::super::*;
use super::fixtures::{derive, product, program, unit};
use crate::hir;
use lkjscript_core::Result;

#[test]
fn aggregate_authority_budgets_accept_exact_and_reject_plus_one() -> Result<()> {
    for (name, maximum) in [
        ("destinations", MAX_MEMORY_PLAN_DESTINATIONS),
        ("borrow scopes", MAX_MEMORY_PLAN_BORROW_SCOPES),
        ("drop paths", MAX_MEMORY_PLAN_DROP_PATHS),
    ] {
        let mut exact = maximum.saturating_sub(1);
        producer::bounded_add(&mut exact, 1, maximum, name)?;
        assert_eq!(exact, maximum);
        assert!(producer::bounded_add(&mut exact, 1, maximum, name).is_err());
    }
    assert_eq!(MAX_MEMORY_PLAN_OBLIGATIONS, 32_768);
    Ok(())
}

#[test]
fn declaration_graph_crosses_former_type_node_and_scc_work_boundaries() -> Result<()> {
    const DECLARATIONS: u64 = 32_769;
    const FORMER_TYPE_NODES: u64 = 16_384;
    const FORMER_SCC_WORK: u64 = 65_536;
    const _: () = assert!(DECLARATIONS > FORMER_TYPE_NODES);

    let products = (0..DECLARATIONS)
        .map(|index| product(index, &format!("wide-{index}"), &[]))
        .collect();
    let plan = derive(&program(hir::Type::Unit, unit(), products, Vec::new()))?;
    assert!(plan.work.scc_work > FORMER_SCC_WORK);
    assert_eq!(plan.work.type_edges, 0);
    Ok(())
}
