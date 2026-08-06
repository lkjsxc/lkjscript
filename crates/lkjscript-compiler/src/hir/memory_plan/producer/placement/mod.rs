use super::*;

mod estimate;
mod facts;

use estimate::{checked_estimate, EstimateIndex};
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
    let estimate_index = EstimateIndex::new(program)?;
    let mut entries_by_expression = HashMap::new();
    let mut witnesses_by_id = HashMap::new();
    let mut uses_by_binding: HashMap<(MemoryFunctionId, u64), Vec<&MemoryUse>> = HashMap::new();
    for entry in entries {
        if let MemorySubject::Expression { expression, .. } = entry.subject {
            if entries_by_expression.insert(expression, entry).is_some() {
                return Err(Error::msg("value placement expression entry is duplicated"));
            }
        }
    }
    for witness in witnesses {
        if witnesses_by_id.insert(witness.id, witness).is_some() {
            return Err(Error::msg("value placement witness identity is duplicated"));
        }
    }
    for usage in uses.iter().filter(|usage| {
        !matches!(
            usage.kind,
            MemoryUseKind::DirectCallTarget | MemoryUseKind::IndirectCallTarget
        )
    }) {
        let indexed = uses_by_binding
            .entry((usage.function, usage.binding))
            .or_default();
        indexed
            .try_reserve(1)
            .map_err(|_| Error::host("value placement use index allocation failed"))?;
        indexed.push(usage);
    }
    let independent_owners = index_independent_owners(&facts)?;
    let branch_divergence = index_branch_divergence(&facts);
    let mut output = Vec::new();
    let mut work = 0_u64;
    for fact in &facts {
        let entry = entries_by_expression
            .get(&fact.id)
            .copied()
            .ok_or_else(|| Error::msg("value placement lost expression entry"))?;
        if entry.root_projection != MemoryRootProjection::Structural {
            continue;
        }
        let type_fact = entry
            .type_fact
            .index()
            .and_then(|index| type_facts.get(index))
            .filter(|item| item.id == entry.type_fact)
            .ok_or_else(|| Error::msg("value placement lost exact type fact"))?;
        let witness = witnesses_by_id
            .get(&type_fact.witness)
            .copied()
            .ok_or_else(|| Error::msg("value placement lost exact witness"))?;
        let binding = expression_binding(&facts, fact);
        let binding_uses = relevant_uses(&uses_by_binding, fact.function, binding);
        let use_count = u64::try_from(binding_uses.len().max(1))
            .map_err(|_| Error::msg("value placement use count exceeds u64"))?;
        let last_use = binding.is_some_and(|_| {
            binding_uses
                .last()
                .is_some_and(|usage| usage.expression == fact.id)
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
            .map_err(|_| Error::msg("value placement dependency count exceeds u64"))?;
        let dependency_cost = dependency_count;
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
            .checked_add(1)
            .ok_or_else(|| Error::msg("value placement work overflow"))?;
    }
    Ok((output, work))
}

fn relevant_uses<'a>(
    uses: &'a HashMap<(MemoryFunctionId, u64), Vec<&'a MemoryUse>>,
    function: MemoryFunctionId,
    binding: Option<BindingId>,
) -> &'a [&'a MemoryUse] {
    binding
        .and_then(|binding| uses.get(&(function, binding.raw())))
        .map(Vec::as_slice)
        .unwrap_or_default()
}

include!("policy.rs");
