use super::*;

mod estimate;
mod facts;

use estimate::checked_estimate;
use facts::{collect_placement_facts, expression_binding, PlacementFact};

const INITIAL_SEAL_NODES: u64 = 8;
const INITIAL_SEAL_BYTES: u64 = 256;
const MAX_SEAL_DEPENDENCIES: usize = 64;
const MAX_SEAL_RELEASE_WORK: u64 = 4_096;

pub(super) fn derive_value_placements(
    program: &hir::Program,
    entries: &[MemoryPlanEntry],
    type_facts: &[MemoryTypeFact],
    witnesses: &[MemoryWitness],
    uses: &[MemoryUse],
) -> Result<(Vec<MemoryValuePlacement>, u64)> {
    let facts = collect_placement_facts(program)?;
    let mut output = Vec::new();
    let mut work = 0_u64;
    for fact in &facts {
        let entry = expression_entry(entries, fact.id)?;
        if entry.root_projection != MemoryRootProjection::Structural {
            continue;
        }
        let type_fact = type_facts
            .get(entry.type_fact.index().unwrap_or(usize::MAX))
            .filter(|item| item.id == entry.type_fact)
            .ok_or_else(|| Error::msg("value placement lost exact type fact"))?;
        let witness = witnesses
            .iter()
            .find(|item| item.id == type_fact.witness)
            .ok_or_else(|| Error::msg("value placement lost exact witness"))?;
        let binding = expression_binding(&facts, fact);
        let binding_uses = relevant_uses(uses, fact.function, binding);
        let use_count = u32::try_from(binding_uses.len().max(1))
            .map_err(|_| Error::msg("value placement use count exceeds u32"))?;
        let last_use = binding.is_some_and(|_| {
            binding_uses.iter().map(|item| item.expression).max() == Some(fact.id)
        });
        let independent = binding
            .map(|binding| independent_owners(&facts, fact.function, binding))
            .unwrap_or(1);
        let branch_divergence =
            binding.is_some_and(|binding| diverges_across_branch(&facts, fact.function, binding));
        let (nodes, bytes) = checked_estimate(program, fact.expression)?;
        let dependencies = witness.facts.dependencies.len();
        let dependency_count = u16::try_from(dependencies)
            .map_err(|_| Error::msg("value placement dependency count exceeds u16"))?;
        let dependency_cost = u64::from(dependency_count);
        let clone_cost = nodes
            .checked_add(bytes)
            .and_then(|value| value.checked_add(dependency_cost))
            .ok_or_else(|| Error::msg("value placement clone cost overflow"))?;
        let release_cost = nodes
            .checked_add(dependency_cost)
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| Error::msg("value placement release cost overflow"))?;
        let sealed = sealed_selected(
            witness,
            independent,
            nodes,
            bytes,
            dependencies,
            release_cost,
        );
        let route = select_route(fact, last_use, independent, nodes, bytes, sealed);
        let category = if route == MemoryValueRoute::Borrow {
            MemoryValueCategory::View
        } else {
            MemoryValueCategory::Owner
        };
        let storage = selected_storage(route, sealed);
        let representation = representation_id(type_fact, category, storage, route)?;
        output.push(MemoryValuePlacement {
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
            representation,
            storage,
            category,
            route,
            failure_cleanup: cleanup(route, storage),
        });
        work = work
            .checked_add(1 + u64::try_from(binding_uses.len()).unwrap_or(u64::MAX))
            .ok_or_else(|| Error::msg("value placement work overflow"))?;
    }
    Ok((output, work))
}

fn expression_entry(
    entries: &[MemoryPlanEntry],
    id: MemoryExpressionId,
) -> Result<&MemoryPlanEntry> {
    entries
        .iter()
        .find(|entry| matches!(entry.subject, MemorySubject::Expression { expression, .. } if expression == id))
        .ok_or_else(|| Error::msg("value placement lost expression entry"))
}

fn relevant_uses(
    uses: &[MemoryUse],
    function: MemoryFunctionId,
    binding: Option<BindingId>,
) -> Vec<&MemoryUse> {
    let Some(binding) = binding else {
        return Vec::new();
    };
    uses.iter()
        .filter(|item| item.function == function && item.binding == binding.raw())
        .filter(|item| {
            !matches!(
                item.kind,
                MemoryUseKind::DirectCallTarget | MemoryUseKind::IndirectCallTarget
            )
        })
        .collect()
}

include!("policy.rs");
