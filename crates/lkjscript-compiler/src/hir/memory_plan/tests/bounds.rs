use super::super::*;
use lkjscript_core::Result;

#[test]
fn aggregate_authority_budgets_accept_exact_and_reject_plus_one() -> Result<()> {
    for (name, maximum) in [
        ("type nodes", MAX_MEMORY_PLAN_TYPE_NODES),
        ("type edges", MAX_MEMORY_PLAN_TYPE_EDGES),
        ("SCC work", MAX_MEMORY_PLAN_SCC_WORK),
        ("aggregate fields", MAX_MEMORY_PLAN_AGGREGATE_FIELDS),
        ("aggregate variants", MAX_MEMORY_PLAN_AGGREGATE_VARIANTS),
        ("destinations", MAX_MEMORY_PLAN_DESTINATIONS),
        ("borrow scopes", MAX_MEMORY_PLAN_BORROW_SCOPES),
        ("drop paths", MAX_MEMORY_PLAN_DROP_PATHS),
    ] {
        let mut exact = maximum.saturating_sub(1);
        producer::bounded_add(&mut exact, 1, maximum, name)?;
        assert_eq!(exact, maximum);
        assert!(producer::bounded_add(&mut exact, 1, maximum, name).is_err());
    }
    assert_eq!(MAX_MEMORY_PLAN_ENTRIES, 65_536);
    assert_eq!(MAX_MEMORY_PLAN_OBLIGATIONS, 32_768);
    assert_eq!(MAX_MEMORY_PLAN_VERIFIER_STEPS, 262_144);
    Ok(())
}
