use super::*;

mod estimate;

use estimate::{checked_estimate, EstimateIndex};

const INITIAL_SEAL_NODES: u64 = 8;
const INITIAL_SEAL_BYTES: u64 = 256;
const MAX_SEAL_DEPENDENCIES: usize = 64;
const MAX_SEAL_RELEASE_WORK: u64 = 4_096;

pub(super) fn verify_value_placements(
    program: &hir::Program,
    plan: &HirMemoryPlan,
    facts: &Facts<'_>,
) -> Result<u64> {
    let estimate_index = EstimateIndex::new(program)?;
    let mut entries_by_expression = HashMap::new();
    let mut witnesses_by_id = HashMap::new();
    for entry in &plan.entries {
        if let MemorySubject::Expression { expression, .. } = entry.subject {
            if entries_by_expression.insert(expression, entry).is_some() {
                return Err(Error::msg(
                    "placement verifier expression entry is duplicated",
                ));
            }
        }
    }
    for witness in &plan.witnesses {
        if witnesses_by_id.insert(witness.id, witness).is_some() {
            return Err(Error::msg(
                "placement verifier witness identity is duplicated",
            ));
        }
    }
    let independent_owners = verified_index_independent_owners(facts)?;
    let branch_divergence = verified_index_branch_divergence(facts);
    let mut expected = Vec::new();
    let mut work = 0_u64;
    for fact in &facts.expressions {
        let entry = entries_by_expression
            .get(&fact.id)
            .copied()
            .ok_or_else(|| Error::msg("placement verifier lost expression entry"))?;
        if entry.root_projection != MemoryRootProjection::Structural {
            continue;
        }
        let type_fact = plan
            .type_fact(entry.type_fact)
            .ok_or_else(|| Error::msg("placement verifier lost exact type fact"))?;
        let witness = witnesses_by_id
            .get(&type_fact.witness)
            .copied()
            .ok_or_else(|| Error::msg("placement verifier lost exact witness"))?;
        let binding = verified_expression_binding(facts, fact);
        let uses = binding
            .map(|binding| facts.binding_use_indices(fact.function, binding.raw()))
            .unwrap_or_default();
        let use_count = u64::try_from(uses.len().max(1))
            .map_err(|_| Error::msg("verified placement use count exceeds u64"))?;
        let last_use = binding.is_some_and(|_| {
            uses.last()
                .and_then(|index| facts.expressions.get(*index))
                .is_some_and(|usage| usage.id == fact.id)
        });
        let independent = binding
            .and_then(|binding| independent_owners.get(&(fact.function, binding)).copied())
            .unwrap_or(1);
        let branch_divergence = binding.is_some_and(|binding| {
            branch_divergence
                .get(&(fact.function, binding))
                .copied()
                .unwrap_or(false)
        });
        let (nodes, bytes) = checked_estimate(&estimate_index, fact.expression)?;
        let dependencies = witness.facts.dependencies.len();
        let dependency_count = u64::try_from(dependencies)
            .map_err(|_| Error::msg("verified placement dependency count exceeds u64"))?;
        let dependency_cost = dependency_count;
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
            .checked_add(1)
            .ok_or_else(|| Error::msg("verified placement work overflow"))?;
    }
    let expected_count = u64::try_from(expected.len())
        .map_err(|_| Error::msg("verified value-placement count exceeds u64"))?;
    if expected != plan.value_placements
        || plan.work.value_placements != expected_count
        || plan.work.placement_work != work
    {
        return Err(Error::msg(
            "independent HIR verifier rejected per-value placement",
        ));
    }
    Ok(work)
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
            let parent = facts.expression(fact.parent?)?;
            let hir::ExprKind::Let { bindings, .. } = &parent.expression.kind else {
                return None;
            };
            bindings
                .get(usize::try_from(fact.child_index).ok()?)
                .map(|item| item.binding)
        }
    }
}

include!("policy.rs");
