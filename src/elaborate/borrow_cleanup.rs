//! Scheduling metadata for early reference cleanup. These storage dependencies
//! keep a reference descriptor alive while another live value points into it.
//! They do not prove lifetime validity: every emitted operation still passes IR.
use super::{Builder, Snapshot, ir, place_type};
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Dependencies = BTreeMap<usize, BTreeSet<usize>>;

#[derive(Default)]
pub(super) struct ReferenceSchedule {
    pub(super) deadlines: BTreeMap<usize, BTreeSet<usize>>,
    pub(super) pending: BTreeSet<usize>,
}

impl Builder<'_, '_> {
    fn storage_dependencies(&self, place: &ir::Place) -> BTreeSet<usize> {
        let mut storage = BTreeSet::new();
        storage.insert(place.local);
        for projection in &place.projections {
            if matches!(projection, ir::Projection::Deref) {
                let mut next = BTreeSet::new();
                for local in storage {
                    if let Some(parents) = self.carriers.get(&local) {
                        for parent in parents {
                            next.insert(*parent);
                        }
                    }
                }
                storage = next;
            }
        }
        storage
    }

    fn value_dependencies(&self, place: &ir::Place) -> BTreeSet<usize> {
        let Some(ty) = place_type(&self.compiler.types, self.function, place) else {
            return BTreeSet::new();
        };
        if !self.carries_references(ty) {
            return BTreeSet::new();
        }
        let mut dependencies = BTreeSet::new();
        for local in self.storage_dependencies(place) {
            if let Some(parents) = self.carriers.get(&local) {
                for parent in parents {
                    dependencies.insert(*parent);
                }
            }
        }
        dependencies
    }

    fn carries_references(&self, ty: usize) -> bool {
        match self.compiler.types[ty].kind {
            ir::TypeKind::Ref { .. } | ir::TypeKind::Slice { .. } => true,
            ir::TypeKind::Struct { .. }
            | ir::TypeKind::Enum { .. }
            | ir::TypeKind::Array { .. } => self.compiler.has_view(ty),
            _ => false,
        }
    }

    fn remove_dependencies(&mut self, local: usize) -> Option<BTreeSet<usize>> {
        let parents = self.carriers.remove(&local)?;
        for parent in &parents {
            let empty = if let Some(children) = self.dependents.get_mut(parent) {
                children.remove(&local);
                children.is_empty()
            } else {
                false
            };
            if empty {
                self.dependents.remove(parent);
            }
        }
        Some(parents)
    }
    fn set_dependencies(&mut self, local: usize, dependencies: BTreeSet<usize>) {
        self.remove_dependencies(local);
        if !dependencies.is_empty() {
            for parent in &dependencies {
                if self.dependents.contains_key(parent) {
                    self.dependents.get_mut(parent).unwrap().insert(local);
                } else {
                    let mut children = BTreeSet::new();
                    children.insert(local);
                    self.dependents.insert(*parent, children);
                }
            }
            self.carriers.insert(local, dependencies);
        }
    }

    fn store_dependencies(&mut self, place: &ir::Place, dependencies: BTreeSet<usize>) {
        if place.projections.is_empty() {
            self.set_dependencies(place.local, dependencies);
        } else {
            // Field writes preserve dependencies of the other fields. This
            // scheduling approximation may keep carriers longer; IR checks the
            // exact projected loans and moves independently.
            for local in self.storage_dependencies(place) {
                let mut parents = self
                    .carriers
                    .get(&local)
                    .cloned()
                    .unwrap_or(BTreeSet::new());
                for dependency in &dependencies {
                    parents.insert(*dependency);
                }
                self.set_dependencies(local, parents);
            }
        }
    }

    pub(super) fn record_carriers(&mut self, operation: &ir::Op) {
        match operation {
            ir::Op::Borrow { dst, place, .. }
            | ir::Op::SliceBorrow { dst, place, .. }
            | ir::Op::ViewRange { dst, place, .. } => {
                let dependencies = self.storage_dependencies(place);
                self.set_dependencies(*dst, dependencies);
            }
            ir::Op::Read { dst, place, moving } => {
                let dependencies = self.value_dependencies(place);
                if *moving && place.projections.is_empty() {
                    self.remove_dependencies(place.local);
                }
                self.set_dependencies(*dst, dependencies);
            }
            ir::Op::Assign { place, src } => {
                let dependencies = self.carriers.get(src).cloned().unwrap_or(BTreeSet::new());
                self.remove_dependencies(*src);
                self.store_dependencies(place, dependencies);
            }
            ir::Op::Struct { dst, fields }
            | ir::Op::Variant { dst, fields, .. }
            | ir::Op::Array {
                dst,
                elements: fields,
            } => {
                let mut dependencies = BTreeSet::new();
                for field in fields {
                    if let Some(parents) = self.remove_dependencies(*field) {
                        for parent in parents {
                            dependencies.insert(parent);
                        }
                    }
                }
                self.set_dependencies(*dst, dependencies);
            }
            ir::Op::Call {
                dst,
                function,
                args,
            } => {
                let dependencies = match &self.compiler.functions[*function].return_source {
                    Some(ir::ReturnSource::Parameter { index, projections }) => self
                        .value_dependencies(&ir::Place {
                            local: args[*index],
                            projections: projections.clone(),
                        }),
                    Some(ir::ReturnSource::Leaves(leaves)) => {
                        let mut dependencies = BTreeSet::new();
                        for leaf in leaves {
                            if let ir::LeafOrigin::Inputs { inputs, .. } = &leaf.origin {
                                for input in inputs {
                                    for parent in self.value_dependencies(&ir::Place {
                                        local: args[input.index],
                                        projections: input.projections.clone(),
                                    }) {
                                        dependencies.insert(parent);
                                    }
                                }
                            }
                        }
                        dependencies
                    }
                    _ => BTreeSet::new(),
                };
                for argument in args {
                    self.remove_dependencies(*argument);
                }
                self.set_dependencies(*dst, dependencies);
            }
            ir::Op::Intrinsic { dst, name, args } => {
                let mut dependencies = BTreeSet::new();
                if *name == ir::IntrinsicOp::Replace || *name == ir::IntrinsicOp::Clone {
                    let source = ir::Place {
                        local: args[0],
                        projections: vec![ir::Projection::Deref],
                    };
                    dependencies = self.value_dependencies(&source);
                    if *name == ir::IntrinsicOp::Replace {
                        let incoming = self
                            .carriers
                            .get(&args[1])
                            .cloned()
                            .unwrap_or(BTreeSet::new());
                        let destinations = self.storage_dependencies(&source);
                        for destination in destinations {
                            self.set_dependencies(destination, incoming.clone());
                        }
                    }
                } else if self.carries_references(self.ty(*dst)) {
                    for argument in args {
                        if let Some(parents) = self.carriers.get(argument) {
                            for parent in parents {
                                dependencies.insert(*parent);
                            }
                        }
                    }
                }
                for argument in args {
                    self.remove_dependencies(*argument);
                }
                self.set_dependencies(*dst, dependencies);
            }
            ir::Op::Drop { place } | ir::Op::DropIf { place } if place.projections.is_empty() => {
                self.remove_dependencies(place.local);
            }
            ir::Op::Return { src } => {
                self.remove_dependencies(*src);
            }
            _ => {}
        }
    }

    pub(super) fn merge_carriers(&mut self, state: &Snapshot) {
        // Source construction tracks possible owners so lexical cleanup is
        // emitted for either path. IR independently checks definite availability.
        self.live = self.live.union(&state.live);
        self.definite_live = self.definite_live.intersection(&state.definite_live);
        self.written = self.written.union(&state.written);
        let changed = self.references.symmetric_difference(&state.references);
        self.reference_updates = self.reference_updates.union(&changed);
        self.references = self.references.union(&state.references);
        for (local, parents) in &state.carriers {
            if state.live.at(*local) {
                let mut combined = self.carriers.get(local).cloned().unwrap_or(BTreeSet::new());
                for parent in parents {
                    combined.insert(*parent);
                }
                self.set_dependencies(*local, combined);
            }
        }
    }

    pub(super) fn drop_unused_references(&mut self, mut candidates: BTreeSet<usize>, line: usize) {
        if candidates.is_empty() {
            return;
        }
        // Reverse edges limit this query to the proposed descriptors. A live
        // value outside the set protects its carriers and their ancestors.
        let mut pending = Vec::new();
        for local in &candidates {
            if let Some(children) = self.dependents.get(local) {
                for child in children {
                    if self.live.at(*child) && !candidates.contains(child) {
                        pending.push(*local);
                        break;
                    }
                }
            }
        }
        let mut needed: BTreeSet<usize> = BTreeSet::new();
        while let Some(local) = pending.pop() {
            if needed.insert(local) {
                if let Some(parents) = self.carriers.get(&local) {
                    for parent in parents {
                        pending.push(*parent);
                    }
                }
            }
        }
        let mut remaining = BTreeSet::new();
        for local in candidates {
            if !needed.contains(&local) {
                remaining.insert(local);
            }
        }
        candidates = remaining;

        // Destroy a dead dependent before its carrier, including when a later
        // assignment made an older binding depend on a newer descriptor.
        let mut dependents: BTreeMap<usize, usize> = BTreeMap::new();
        for local in &candidates {
            dependents.insert(*local, 0);
        }
        for local in &candidates {
            if let Some(parents) = self.carriers.get(local) {
                for parent in parents {
                    if let Some(count) = dependents.get_mut(parent) {
                        *count += 1;
                    }
                }
            }
        }
        // Inverting the key lets the ordinary ascending cursor select the
        // greatest local first, retaining the original cleanup priority.
        let mut ready: BTreeSet<usize> = BTreeSet::new();
        for (local, count) in &dependents {
            if *count == 0 {
                ready.insert(!*local);
            }
        }
        while let Some(key) = ready.pop_first() {
            let local = !key;
            let parents = self
                .carriers
                .get(&local)
                .cloned()
                .unwrap_or(BTreeSet::new());
            let fields = self.compiler.closure_reference_fields(self.ty(local));
            if fields.is_empty() {
                self.emit(
                    ir::Op::Drop {
                        place: ir::Place::local(local),
                    },
                    line,
                );
            } else {
                // Release only capture descriptors after the final call. Owned
                // fields retain their lexical destructor point and drop order.
                for field in fields {
                    self.emit(
                        ir::Op::DropIf {
                            place: ir::Place {
                                local,
                                projections: vec![ir::Projection::Field(field)],
                            },
                        },
                        line,
                    );
                }
                self.references.set(local, false);
                self.reference_updates.set(local, false);
                self.remove_dependencies(local);
            }
            for parent in parents {
                if let Some(count) = dependents.get_mut(&parent) {
                    *count -= 1;
                    if *count == 0 {
                        ready.insert(!parent);
                    }
                }
            }
        }
        // A dependency cycle has no early-drop order. Leave it to ordinary
        // scope cleanup and the IR check rather than inventing a lifetime rule.
    }
}
