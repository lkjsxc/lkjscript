//! Scheduling metadata for early reference cleanup. These storage dependencies
//! keep a reference descriptor alive while another live value points into it.
//! They do not prove lifetime validity: every emitted operation still passes K.
use super::{Builder, Snapshot, k, place_type};
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Dependencies = BTreeMap<usize, BTreeSet<usize>>;

impl Builder<'_, '_> {
    fn storage_dependencies(&self, place: &k::Place) -> BTreeSet<usize> {
        let mut storage = BTreeSet::from([place.local]);
        for projection in &place.projections {
            if matches!(projection, k::Projection::Deref) {
                let mut next = BTreeSet::new();
                for local in storage {
                    if let Some(parents) = self.carriers.get(&local) {
                        next.extend(parents);
                    }
                }
                storage = next;
            }
        }
        storage
    }

    fn value_dependencies(&self, place: &k::Place) -> BTreeSet<usize> {
        let Some(ty) = place_type(&self.compiler.types, self.function, place) else {
            return BTreeSet::new();
        };
        if !self.carries_references(ty) {
            return BTreeSet::new();
        }
        let mut dependencies = BTreeSet::new();
        for local in self.storage_dependencies(place) {
            if let Some(parents) = self.carriers.get(&local) {
                dependencies.extend(parents);
            }
        }
        dependencies
    }

    fn carries_references(&self, ty: usize) -> bool {
        match self.compiler.types[ty].kind {
            k::TypeKind::Ref { .. } | k::TypeKind::Slice { .. } => true,
            k::TypeKind::Struct { .. } | k::TypeKind::Enum { .. } | k::TypeKind::Array { .. } => {
                self.compiler.has_view(ty)
            }
            _ => false,
        }
    }

    fn set_dependencies(&mut self, local: usize, dependencies: BTreeSet<usize>) {
        if dependencies.is_empty() {
            self.carriers.remove(&local);
        } else {
            self.carriers.insert(local, dependencies);
        }
    }

    fn store_dependencies(&mut self, place: &k::Place, dependencies: BTreeSet<usize>) {
        if place.projections.is_empty() {
            self.set_dependencies(place.local, dependencies);
        } else {
            // Field writes preserve dependencies of the other fields. This
            // scheduling approximation may keep carriers longer; K checks the
            // exact projected loans and moves independently.
            for local in self.storage_dependencies(place) {
                self.carriers
                    .entry(local)
                    .or_default()
                    .extend(&dependencies);
            }
        }
    }

    pub(super) fn record_carriers(&mut self, operation: &k::Op) {
        match operation {
            k::Op::Borrow { dst, place, .. } | k::Op::SliceBorrow { dst, place, .. } => {
                let dependencies = self.storage_dependencies(place);
                self.set_dependencies(*dst, dependencies);
            }
            k::Op::Read { dst, place, moving } => {
                let dependencies = self.value_dependencies(place);
                if *moving && place.projections.is_empty() {
                    self.carriers.remove(&place.local);
                }
                self.set_dependencies(*dst, dependencies);
            }
            k::Op::Assign { place, src } => {
                let dependencies = self.carriers.get(src).cloned().unwrap_or_default();
                self.carriers.remove(src);
                self.store_dependencies(place, dependencies);
            }
            k::Op::Struct { dst, fields }
            | k::Op::Variant { dst, fields, .. }
            | k::Op::Array {
                dst,
                elements: fields,
            } => {
                let mut dependencies = BTreeSet::new();
                for field in fields {
                    if let Some(parents) = self.carriers.remove(field) {
                        dependencies.extend(parents);
                    }
                }
                self.set_dependencies(*dst, dependencies);
            }
            k::Op::Call {
                dst,
                function,
                args,
            } => {
                let dependencies = match &self.compiler.functions[*function].return_source {
                    Some(k::ReturnSource::Parameter { index, projections }) => self
                        .value_dependencies(&k::Place {
                            local: args[*index],
                            projections: projections.clone(),
                        }),
                    _ => BTreeSet::new(),
                };
                for argument in args {
                    self.carriers.remove(argument);
                }
                self.set_dependencies(*dst, dependencies);
            }
            k::Op::Intrinsic { dst, name, args } => {
                let mut dependencies = BTreeSet::new();
                if name == "replace" || name == "clone" {
                    let source = k::Place {
                        local: args[0],
                        projections: vec![k::Projection::Deref],
                    };
                    dependencies = self.value_dependencies(&source);
                    if name == "replace" {
                        let incoming = self.carriers.get(&args[1]).cloned().unwrap_or_default();
                        let destinations = self.storage_dependencies(&source);
                        for destination in destinations {
                            self.set_dependencies(destination, incoming.clone());
                        }
                    }
                } else if self.carries_references(self.ty(*dst)) {
                    for argument in args {
                        if let Some(parents) = self.carriers.get(argument) {
                            dependencies.extend(parents);
                        }
                    }
                }
                for argument in args {
                    self.carriers.remove(argument);
                }
                self.set_dependencies(*dst, dependencies);
            }
            k::Op::Drop { place } if place.projections.is_empty() => {
                self.carriers.remove(&place.local);
            }
            k::Op::Return { src } => {
                self.carriers.remove(src);
            }
            _ => {}
        }
    }

    pub(super) fn merge_carriers(&mut self, state: &Snapshot) {
        for (local, parents) in &state.carriers {
            if self.live.get(*local) == Some(true) && state.live[*local] {
                self.carriers.entry(*local).or_default().extend(parents);
            }
        }
    }

    pub(super) fn drop_unused_references(&mut self, mut candidates: BTreeSet<usize>, line: usize) {
        // A live value outside the proposed drop set can depend on several
        // descriptors. Walk immediate edges once instead of expanding chains
        // into every value's metadata.
        let mut pending = Vec::new();
        for (local, parents) in &self.carriers {
            if self.live[*local] && !candidates.contains(local) {
                pending.extend(parents);
            }
        }
        let mut needed = BTreeSet::new();
        while let Some(local) = pending.pop() {
            if needed.insert(local) {
                if let Some(parents) = self.carriers.get(&local) {
                    pending.extend(parents);
                }
            }
        }
        candidates.retain(|local| !needed.contains(local));

        // Destroy a dead dependent before its carrier, including when a later
        // assignment made an older binding depend on a newer descriptor.
        let mut dependents: BTreeMap<usize, usize> =
            candidates.iter().map(|local| (*local, 0)).collect();
        for local in &candidates {
            if let Some(parents) = self.carriers.get(local) {
                for parent in parents {
                    if let Some(count) = dependents.get_mut(parent) {
                        *count += 1;
                    }
                }
            }
        }
        let mut ready: BTreeSet<usize> = dependents
            .iter()
            .filter_map(|(local, count)| (*count == 0).then_some(*local))
            .collect();
        while let Some(local) = ready.pop_last() {
            let parents = self.carriers.get(&local).cloned().unwrap_or_default();
            self.emit(
                k::Op::Drop {
                    place: k::Place::local(local),
                },
                line,
            );
            for parent in parents {
                if let Some(count) = dependents.get_mut(&parent) {
                    *count -= 1;
                    if *count == 0 {
                        ready.insert(parent);
                    }
                }
            }
        }
        // A dependency cycle has no early-drop order. Leave it to ordinary
        // scope cleanup and the K check rather than inventing a lifetime rule.
    }
}
