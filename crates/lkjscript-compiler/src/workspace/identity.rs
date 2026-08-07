use std::collections::{HashMap, HashSet};

use lkjscript_core::{Error, Result};

use super::model::NodeKey;
use super::{EntityId, NodeId, SemanticChild, SemanticOwner, SnapshotIndexes, WorkspaceNamespace};

#[derive(Clone)]
pub(super) struct IdentityAllocator {
    namespace: WorkspaceNamespace,
    entity_generations: Vec<u64>,
    entity_live: Vec<bool>,
    free_entities: Vec<u64>,
    node_generations: Vec<u64>,
    node_live: Vec<bool>,
    free_nodes: Vec<u64>,
}

impl IdentityAllocator {
    pub(super) fn from_indexes(
        namespace: WorkspaceNamespace,
        indexes: &SnapshotIndexes,
    ) -> Result<Self> {
        let mut allocator = Self {
            namespace,
            entity_generations: Vec::new(),
            entity_live: Vec::new(),
            free_entities: Vec::new(),
            node_generations: Vec::new(),
            node_live: Vec::new(),
            free_nodes: Vec::new(),
        };
        for header in &indexes.entities {
            allocator.install_entity(header.id)?;
        }
        for header in &indexes.nodes {
            allocator.install_node(header.id)?;
        }
        Ok(allocator)
    }

    fn install_entity(&mut self, id: EntityId) -> Result<()> {
        let slot = host_index(id.slot(), "entity")?;
        grow_slots(
            &mut self.entity_generations,
            &mut self.entity_live,
            slot,
            "entity allocator",
        )?;
        self.entity_generations[slot] = id.generation();
        self.entity_live[slot] = true;
        Ok(())
    }

    fn install_node(&mut self, id: NodeId) -> Result<()> {
        let slot = host_index(id.slot(), "node")?;
        grow_slots(
            &mut self.node_generations,
            &mut self.node_live,
            slot,
            "node allocator",
        )?;
        self.node_generations[slot] = id.generation();
        self.node_live[slot] = true;
        Ok(())
    }

    fn allocate_entity(&mut self) -> Result<EntityId> {
        let (slot, generation) = allocate_slot(
            &mut self.entity_generations,
            &mut self.entity_live,
            &mut self.free_entities,
            "entity",
        )?;
        Ok(EntityId::new(self.namespace, slot, generation))
    }

    fn allocate_node(&mut self) -> Result<NodeId> {
        let (slot, generation) = allocate_slot(
            &mut self.node_generations,
            &mut self.node_live,
            &mut self.free_nodes,
            "node",
        )?;
        Ok(NodeId::new(self.namespace, slot, generation))
    }

    fn tombstone_entity(&mut self, id: EntityId) -> Result<()> {
        tombstone_slot(
            id.slot(),
            id.generation(),
            &mut self.entity_generations,
            &mut self.entity_live,
            &mut self.free_entities,
            "entity",
        )
    }

    pub(super) fn tombstone_node(&mut self, id: NodeId) -> Result<()> {
        tombstone_slot(
            id.slot(),
            id.generation(),
            &mut self.node_generations,
            &mut self.node_live,
            &mut self.free_nodes,
            "node",
        )
    }
}

pub(super) fn reconcile(
    mut next: SnapshotIndexes,
    previous: &SnapshotIndexes,
    allocator: &mut IdentityAllocator,
    forced_nodes: &HashMap<NodeKey, NodeId>,
) -> Result<SnapshotIndexes> {
    let mut entity_map = HashMap::new();
    entity_map
        .try_reserve(next.entities.len())
        .map_err(|_| Error::host("workspace entity reconciliation allocation failed"))?;
    let mut used_entities = HashSet::new();
    used_entities
        .try_reserve(next.entities.len())
        .map_err(|_| Error::host("workspace entity reconciliation set allocation failed"))?;

    for index in 0..next.entities.len() {
        let temporary = next.entities[index].id;
        let address = next.entity_addresses[index];
        let stable = previous
            .address_entities
            .get(&address)
            .copied()
            .map_or_else(|| allocator.allocate_entity(), Ok)?;
        entity_map.insert(temporary, stable);
        used_entities.insert(stable);
        next.entities[index].id = stable;
    }
    for header in &previous.entities {
        if !used_entities.contains(&header.id) {
            allocator.tombstone_entity(header.id)?;
        }
    }
    for header in &mut next.entities {
        header.owner = header
            .owner
            .map(|owner| remap_entity(&entity_map, owner))
            .transpose()?;
    }

    let mut exact_previous = HashMap::new();
    exact_previous
        .try_reserve(previous.nodes.len())
        .map_err(|_| Error::host("workspace exact node reconciliation allocation failed"))?;
    let mut sibling_previous = HashMap::new();
    sibling_previous
        .try_reserve(previous.nodes.len())
        .map_err(|_| Error::host("workspace moved node reconciliation allocation failed"))?;
    for index in 0..previous.nodes.len() {
        let id = previous.nodes[index].id;
        let key = previous.node_keys[index];
        let fingerprint = previous.node_fingerprints[index];
        exact_previous.insert(key, (id, fingerprint));
        sibling_previous
            .entry((key.owner, fingerprint))
            .and_modify(|candidate| *candidate = None)
            .or_insert(Some(id));
    }

    let mut node_map = HashMap::new();
    node_map
        .try_reserve(next.nodes.len())
        .map_err(|_| Error::host("workspace node reconciliation allocation failed"))?;
    let mut used_nodes = HashSet::new();
    used_nodes
        .try_reserve(next.nodes.len())
        .map_err(|_| Error::host("workspace node reconciliation set allocation failed"))?;

    for index in 0..next.nodes.len() {
        let temporary = next.nodes[index].id;
        let owner = remap_owner(&entity_map, &node_map, next.node_keys[index].owner)?;
        let key = NodeKey {
            owner,
            ordinal: next.node_keys[index].ordinal,
        };
        let fingerprint = next.node_fingerprints[index];
        let stable = if let Some(id) = forced_nodes.get(&key).copied() {
            if previous.node_lookup.contains_key(&id) && !used_nodes.contains(&id) {
                id
            } else {
                return Err(Error::msg("forced workspace node identity is stale"));
            }
        } else {
            choose_previous_node(
                &exact_previous,
                &sibling_previous,
                key,
                fingerprint,
                &used_nodes,
            )
            .map_or_else(|| allocator.allocate_node(), Ok)?
        };
        used_nodes.insert(stable);
        node_map.insert(temporary, stable);
        next.nodes[index].id = stable;
        next.nodes[index].owner = owner;
        next.node_keys[index] = key;
    }
    for header in &previous.nodes {
        if !used_nodes.contains(&header.id) {
            allocator.tombstone_node(header.id)?;
        }
    }

    for edge in &mut next.containment {
        edge.owner = remap_owner(&entity_map, &node_map, edge.owner)?;
        edge.child = match edge.child {
            SemanticChild::Entity(entity) => {
                SemanticChild::Entity(remap_entity(&entity_map, entity)?)
            }
            SemanticChild::Node(node) => SemanticChild::Node(remap_node(&node_map, node)?),
        };
    }
    for edge in &mut next.references {
        edge.site = remap_node(&node_map, edge.site)?;
        edge.target = remap_entity(&entity_map, edge.target)?;
    }
    for edge in &mut next.calls {
        edge.caller = remap_entity(&entity_map, edge.caller)?;
        edge.callee = remap_entity(&entity_map, edge.callee)?;
        edge.site = remap_node(&node_map, edge.site)?;
    }
    for edge in &mut next.dependencies {
        edge.dependent = remap_entity(&entity_map, edge.dependent)?;
        edge.dependency = remap_entity(&entity_map, edge.dependency)?;
    }
    for edge in &mut next.declaration_dependencies {
        edge.dependent = remap_entity(&entity_map, edge.dependent)?;
        edge.dependency = remap_entity(&entity_map, edge.dependency)?;
    }
    next.rebuild_maps()?;
    Ok(next)
}

fn choose_previous_node(
    exact: &HashMap<NodeKey, (NodeId, [u8; 32])>,
    siblings: &HashMap<(SemanticOwner, [u8; 32]), Option<NodeId>>,
    key: NodeKey,
    fingerprint: [u8; 32],
    used: &HashSet<NodeId>,
) -> Option<NodeId> {
    if let Some((id, old_fingerprint)) = exact.get(&key) {
        if *old_fingerprint == fingerprint && !used.contains(id) {
            return Some(*id);
        }
    }
    siblings
        .get(&(key.owner, fingerprint))
        .copied()
        .flatten()
        .filter(|id| !used.contains(id))
}

fn remap_owner(
    entities: &HashMap<EntityId, EntityId>,
    nodes: &HashMap<NodeId, NodeId>,
    owner: SemanticOwner,
) -> Result<SemanticOwner> {
    match owner {
        SemanticOwner::Entity(entity) => Ok(SemanticOwner::Entity(remap_entity(entities, entity)?)),
        SemanticOwner::Node(node) => Ok(SemanticOwner::Node(remap_node(nodes, node)?)),
    }
}

fn remap_entity(map: &HashMap<EntityId, EntityId>, id: EntityId) -> Result<EntityId> {
    map.get(&id)
        .copied()
        .ok_or_else(|| Error::msg("workspace entity reconciliation is incomplete"))
}

fn remap_node(map: &HashMap<NodeId, NodeId>, id: NodeId) -> Result<NodeId> {
    map.get(&id)
        .copied()
        .ok_or_else(|| Error::msg("workspace node reconciliation is incomplete"))
}

fn allocate_slot(
    generations: &mut Vec<u64>,
    live: &mut Vec<bool>,
    free: &mut Vec<u64>,
    kind: &str,
) -> Result<(u64, u64)> {
    if let Some(slot) = free.pop() {
        let index = host_index(slot, kind)?;
        if live.get(index).copied().unwrap_or(true) {
            return Err(Error::msg(format!(
                "workspace {kind} free list is inconsistent"
            )));
        }
        live[index] = true;
        return Ok((slot, generations[index]));
    }
    let slot = u64::try_from(generations.len())
        .map_err(|_| Error::host(format!("workspace {kind} identity exceeds u64")))?;
    generations
        .try_reserve(1)
        .map_err(|_| Error::host(format!("workspace {kind} allocator allocation failed")))?;
    live.try_reserve(1)
        .map_err(|_| Error::host(format!("workspace {kind} liveness allocation failed")))?;
    generations.push(1);
    live.push(true);
    Ok((slot, 1))
}

fn tombstone_slot(
    slot: u64,
    generation: u64,
    generations: &mut [u64],
    live: &mut [bool],
    free: &mut Vec<u64>,
    kind: &str,
) -> Result<()> {
    let index = host_index(slot, kind)?;
    let current = generations
        .get_mut(index)
        .ok_or_else(|| Error::msg(format!("workspace {kind} tombstone is stale")))?;
    let is_live = live
        .get_mut(index)
        .ok_or_else(|| Error::msg(format!("workspace {kind} liveness is stale")))?;
    if !*is_live || *current != generation {
        return Err(Error::msg(format!(
            "workspace {kind} tombstone generation is stale"
        )));
    }
    *current = current
        .checked_add(1)
        .ok_or_else(|| Error::host(format!("workspace {kind} generation exhausted")))?;
    *is_live = false;
    free.try_reserve(1)
        .map_err(|_| Error::host(format!("workspace {kind} free-list allocation failed")))?;
    free.push(slot);
    Ok(())
}

fn grow_slots(
    generations: &mut Vec<u64>,
    live: &mut Vec<bool>,
    index: usize,
    kind: &str,
) -> Result<()> {
    let required = index
        .checked_add(1)
        .ok_or_else(|| Error::host(format!("workspace {kind} size overflow")))?;
    if required > generations.len() {
        let additional = required - generations.len();
        generations
            .try_reserve(additional)
            .map_err(|_| Error::host(format!("workspace {kind} generation allocation failed")))?;
        live.try_reserve(additional)
            .map_err(|_| Error::host(format!("workspace {kind} liveness allocation failed")))?;
        generations.resize(required, 1);
        live.resize(required, false);
    }
    Ok(())
}

fn host_index(slot: u64, kind: &str) -> Result<usize> {
    usize::try_from(slot)
        .map_err(|_| Error::host(format!("workspace {kind} identity is not host-addressable")))
}
