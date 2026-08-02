use lkjscript_core::Result;

use super::{HirMemoryPlan, MemoryPlanId, MemoryType};
use canonical::Encoder;

mod canonical;

const PLAN_DOMAIN: &[u8] = b"lkjscript.hir-memory-plan\0canonical-platform-contract";
const TYPE_DOMAIN: &[u8] = b"lkjscript.memory-type\0canonical-platform-contract";

pub(super) fn compute_plan_id(plan: &HirMemoryPlan) -> Result<MemoryPlanId> {
    let mut output = Encoder::new(PLAN_DOMAIN)?;
    output.value(plan.schema)?;
    output.value(&plan.functions)?;
    output.value(&plan.entries)?;
    output.value(&plan.uses)?;
    output.value(&plan.loans)?;
    output.value(&plan.constants)?;
    output.value(&plan.calls)?;
    output.value(&plan.obligations)?;
    output.value(&plan.type_facts)?;
    output.value(&plan.witnesses)?;
    output.value(&plan.destinations)?;
    output.value(&plan.borrow_scopes)?;
    output.value(&plan.drop_paths)?;
    output.value(&plan.drop_glues)?;
    output.value(&plan.work)?;
    Ok(MemoryPlanId::from_bytes(lkjscript_core::sha256(
        &output.finish(),
    )))
}

pub(super) fn compute_type_id(ty: &MemoryType) -> Result<[u8; 32]> {
    let mut output = Encoder::new(TYPE_DOMAIN)?;
    output.value(ty)?;
    Ok(lkjscript_core::sha256(&output.finish()))
}
