use std::collections::{BTreeMap, BTreeSet};

use super::{SealedBuilder, SealedRegionStore};
use crate::structural::{DomainClass, DomainKey, StructuralError, StructuralLimit};

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub(super) fn validate_graph(
        &mut self,
        builders: &[SealedBuilder<T, D>],
    ) -> Result<(), StructuralError> {
        if builders.len() > self.limits.max_release_work as usize {
            return Err(StructuralError::LimitExceeded(StructuralLimit::ReleaseWork));
        }
        let nodes = builders
            .iter()
            .map(|builder| builder.key)
            .collect::<BTreeSet<_>>();
        if nodes.len() != builders.len() {
            return Err(StructuralError::DuplicateDependency);
        }
        let mut graph = BTreeMap::new();
        for &key in &nodes {
            let record = &self.records[self.record_index(key)?].1;
            let mut outgoing = Vec::new();
            for &target in record.dependencies.as_slice() {
                match target.class() {
                    DomainClass::RegionBuilding if nodes.contains(&target) => outgoing.push(target),
                    DomainClass::RegionBuilding => {
                        return Err(StructuralError::UnsupportedDependency);
                    }
                    DomainClass::RegionSealed => {
                        self.record_index(target)?;
                    }
                    _ => return Err(StructuralError::UnsupportedDependency),
                }
            }
            outgoing.sort_unstable();
            graph.insert(key, outgoing);
        }
        let mut colors = BTreeMap::new();
        let mut path = Vec::new();
        for &node in &nodes {
            if colors.get(&node).copied().unwrap_or(0) == 0 {
                if let Some(witness) = visit(node, &graph, &mut colors, &mut path)? {
                    self.metrics.rejected_cycles = self.metrics.rejected_cycles.saturating_add(1);
                    return Err(StructuralError::DependencyCycle(witness));
                }
            }
        }
        Ok(())
    }
}

fn visit(
    node: DomainKey,
    graph: &BTreeMap<DomainKey, Vec<DomainKey>>,
    colors: &mut BTreeMap<DomainKey, u8>,
    path: &mut Vec<DomainKey>,
) -> Result<Option<Vec<DomainKey>>, StructuralError> {
    if path.len() >= graph.len() {
        return Err(StructuralError::LimitExceeded(StructuralLimit::ReleaseWork));
    }
    colors.insert(node, 1);
    path.push(node);
    if let Some(targets) = graph.get(&node) {
        for &target in targets {
            match colors.get(&target).copied().unwrap_or(0) {
                0 => {
                    if let Some(witness) = visit(target, graph, colors, path)? {
                        return Ok(Some(witness));
                    }
                }
                1 => {
                    let start = path
                        .iter()
                        .position(|candidate| *candidate == target)
                        .unwrap_or(0);
                    let mut witness = path[start..].to_vec();
                    witness.push(target);
                    return Ok(Some(witness));
                }
                _ => {}
            }
        }
    }
    path.pop();
    colors.insert(node, 2);
    Ok(None)
}
