use super::*;

mod estimate;

use estimate::checked_estimate;

const INITIAL_SEAL_NODES: u64 = 8;
const INITIAL_SEAL_BYTES: u64 = 256;
const MAX_SEAL_DEPENDENCIES: usize = 64;
const MAX_SEAL_RELEASE_WORK: u64 = 4_096;

pub(super) fn verify_value_placements(
    program: &hir::Program,
    plan: &HirMemoryPlan,
    facts: &Facts<'_>,
) -> Result<u64> {
    let mut expected = Vec::new();
    let mut work = 0_u64;
    for fact in &facts.expressions {
        let entry = expression_entry(plan, fact.id)?;
        if entry.root_projection != MemoryRootProjection::Structural {
            continue;
        }
        let type_fact = plan
            .type_fact(entry.type_fact)
            .ok_or_else(|| Error::msg("placement verifier lost exact type fact"))?;
        let witness = plan
            .witness(type_fact.witness)
            .ok_or_else(|| Error::msg("placement verifier lost exact witness"))?;
        let binding = verified_expression_binding(facts, fact);
        let uses = verified_binding_uses(facts, fact.function, binding);
        let use_count = u32::try_from(uses.len().max(1))
            .map_err(|_| Error::msg("verified placement use count exceeds u32"))?;
        let last_use =
            binding.is_some_and(|_| uses.iter().map(|item| item.id).max() == Some(fact.id));
        let independent = binding
            .map(|binding| verified_independent_owners(facts, fact.function, binding))
            .unwrap_or(1);
        let branch_divergence = binding
            .is_some_and(|binding| verified_branch_divergence(facts, fact.function, binding));
        let (nodes, bytes) = checked_estimate(program, fact.expression)?;
        let dependencies = witness.facts.dependencies.len();
        let dependency_count = u16::try_from(dependencies)
            .map_err(|_| Error::msg("verified placement dependency count exceeds u16"))?;
        let dependency_cost = u64::from(dependency_count);
        let clone_cost = nodes
            .checked_add(bytes)
            .and_then(|value| value.checked_add(dependency_cost))
            .ok_or_else(|| Error::msg("verified placement clone cost overflow"))?;
        let release_cost = nodes
            .checked_add(dependency_cost)
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| Error::msg("verified placement release cost overflow"))?;
        let sealed = verified_sealed_selected(
            witness,
            independent,
            nodes,
            bytes,
            dependencies,
            release_cost,
        );
        let route = verified_route(fact, last_use, independent, nodes, bytes, sealed);
        let category = if route == MemoryValueRoute::Borrow {
            MemoryValueCategory::View
        } else {
            MemoryValueCategory::Owner
        };
        let storage = verified_storage(route, sealed);
        expected.push(MemoryValuePlacement {
            expression: fact.id,
            type_fact: entry.type_fact,
            witness: type_fact.witness,
            use_count,
            last_use,
            escape: entry.mode.escape,
            returned: entry.mode.escape == MemoryEscape::Returned,
            captured: entry.mode.escape == MemoryEscape::Captured,
            process_boundary: entry.mode.escape == MemoryEscape::Runtime,
            branch_divergence,
            independently_live_owners: independent,
            independent_owner_demand: independent >= 2,
            structural_nodes: nodes,
            payload_bytes: bytes,
            clone_cost,
            dependency_count,
            dependency_cost,
            release_cost,
            representation: verified_representation_id(type_fact, category, storage, route)?,
            storage,
            category,
            route,
            failure_cleanup: verified_cleanup(route, storage),
        });
        work = work
            .checked_add(1 + u64::try_from(uses.len()).unwrap_or(u64::MAX))
            .ok_or_else(|| Error::msg("verified placement work overflow"))?;
    }
    if expected != plan.value_placements
        || plan.work.value_placements != u64::try_from(expected.len()).unwrap_or(u64::MAX)
        || plan.work.placement_work != work
    {
        return Err(Error::msg(
            "independent HIR verifier rejected per-value placement",
        ));
    }
    Ok(work)
}

fn expression_entry(plan: &HirMemoryPlan, id: MemoryExpressionId) -> Result<&MemoryPlanEntry> {
    plan.entries.iter()
        .find(|entry| matches!(entry.subject, MemorySubject::Expression { expression, .. } if expression == id))
        .ok_or_else(|| Error::msg("placement verifier lost expression entry"))
}

fn verified_expression_binding(facts: &Facts<'_>, fact: &ExprFact<'_>) -> Option<BindingId> {
    match &fact.expression.kind {
        hir::ExprKind::Load(reference)
        | hir::ExprKind::Move {
            binding: reference, ..
        }
        | hir::ExprKind::Borrow {
            binding: reference, ..
        }
        | hir::ExprKind::BorrowBytes {
            binding: reference, ..
        } => Some(reference.binding),
        _ => {
            let parent = facts
                .expressions
                .iter()
                .find(|item| Some(item.id) == fact.parent)?;
            let hir::ExprKind::Let { bindings, .. } = &parent.expression.kind else {
                return None;
            };
            bindings
                .get(usize::try_from(fact.child_index).ok()?)
                .map(|item| item.binding)
        }
    }
}

fn verified_binding_uses<'a>(
    facts: &'a Facts<'_>,
    function: MemoryFunctionId,
    binding: Option<BindingId>,
) -> Vec<&'a ExprFact<'a>> {
    let Some(binding) = binding else {
        return Vec::new();
    };
    facts
        .expressions
        .iter()
        .filter(|fact| fact.function == function)
        .filter(|fact| {
            matches!(&fact.expression.kind,
            hir::ExprKind::Load(reference)
            | hir::ExprKind::Move { binding: reference, .. }
            | hir::ExprKind::Borrow { binding: reference, .. }
            | hir::ExprKind::BorrowBytes { binding: reference, .. } if reference.binding == binding)
        })
        .collect()
}

include!("policy.rs");
