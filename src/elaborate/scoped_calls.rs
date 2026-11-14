//! Fixed callable regions in the discarded definition program. Region variables
//! belong to declarations; type equality never identifies their scopes.
use super::origins::Label;
use super::{Compiler, Instance, PendingFunction, Result, a, err, ir};
use crate::check::scoped::{Region, Requirement, Scope};
use std::collections::{BTreeMap, BTreeSet};

type RegionBindings = BTreeMap<Label, BTreeSet<Region>>;

fn region(scope: usize, name: &str) -> Region {
    Region {
        scope: if name == "'static" { 0 } else { scope },
        name: name.to_string(),
    }
}
fn named_region(scope: usize, label: &Label) -> Option<Region> {
    match label {
        Label::Lifetime(name) => Some(region(scope, name)),
        Label::Static => Some(region(0, "'static")),
        _ => None,
    }
}
fn insert_outlives(edges: &mut BTreeMap<Region, BTreeSet<Region>>, long: Region, short: Region) {
    if edges.contains_key(&long) {
        edges.get_mut(&long).unwrap().insert(short);
    } else {
        let mut shorts = BTreeSet::new();
        shorts.insert(short);
        edges.insert(long, shorts);
    }
}
fn insert_region(bindings: &mut RegionBindings, label: Label, region: Region) {
    if bindings.contains_key(&label) {
        bindings.get_mut(&label).unwrap().insert(region);
    } else {
        let mut regions = BTreeSet::new();
        regions.insert(region);
        bindings.insert(label, regions);
    }
}
fn normalize(requirements: Vec<Requirement>) -> Vec<Requirement> {
    let mut unique: BTreeMap<(usize, Vec<ir::Projection>, Region), (bool, bool)> = BTreeMap::new();
    for requirement in requirements {
        let key = (
            requirement.argument,
            requirement.projections,
            requirement.region,
        );
        if unique.contains_key(&key) {
            let entry = unique.get_mut(&key).unwrap();
            entry.0 |= requirement.preserve;
            entry.1 |= requirement.mutable_storage;
        } else {
            unique.insert(key, (requirement.preserve, requirement.mutable_storage));
        }
    }
    let mut normalized = Vec::new();
    for ((argument, projections, region), (preserve, mutable_storage)) in unique {
        normalized.push(Requirement {
            argument,
            projections,
            region,
            preserve,
            mutable_storage,
        });
    }
    normalized
}

impl Compiler<'_> {
    pub(super) fn scoped_parameters(&self, index: usize) -> Vec<a::GenericParam> {
        let mut parameters = self.source.functions[index].generics.clone();
        match self.source.function_owner(a::FunctionId { index }) {
            a::FunctionOwner::Implementation(owner) => {
                for parameter in &self.source.impls[owner.index].generics {
                    parameters.push(parameter.clone());
                }
            }
            a::FunctionOwner::Trait(owner) => {
                for parameter in &self.source.traits[owner.index].generics {
                    parameters.push(parameter.clone());
                }
            }
            a::FunctionOwner::Free => {}
        }
        parameters
    }
    fn region_edges(&self, index: usize) -> BTreeMap<Region, BTreeSet<Region>> {
        let mut edges = BTreeMap::<Region, BTreeSet<Region>>::new();
        for parameter in self.scoped_parameters(index) {
            if !parameter.lifetime {
                continue;
            }
            for bound in &parameter.bounds {
                if let a::Type::Path(path) = &self.source.types[*bound].kind {
                    if path.name.starts_with('\'') {
                        insert_outlives(
                            &mut edges,
                            region(index, &parameter.name),
                            region(index, &path.name),
                        );
                    }
                }
            }
        }
        edges
    }
    fn source_scope(&self, instance: &Instance, function: &ir::Function) -> Result<Scope> {
        let source = &self.source.functions[instance.source];
        let mut scope = Scope {
            outlives: self.region_edges(instance.source),
            ..Scope::default()
        };
        let mut elisions: BTreeMap<Label, Region> = BTreeMap::new();
        let mut input_regions: BTreeMap<Label, Region> = BTreeMap::new();
        for (index, parameter) in source.params.iter().enumerate() {
            let leaves =
                self.region_leaves(instance.source, parameter.ty, function.locals[index].ty);
            for (leaf_index, leaf) in leaves.into_iter().enumerate() {
                let region = if let Some(named) = named_region(instance.source, &leaf.label) {
                    named
                } else if matches!(leaf.label, Label::Elided { .. }) {
                    let known = elisions.get(&leaf.label).cloned();
                    if let Some(found) = known {
                        found
                    } else {
                        let found =
                            region(instance.source, &format!("@input{index}:leaf{leaf_index}"));
                        elisions.insert(leaf.label.clone(), found.clone());
                        found
                    }
                } else {
                    region(instance.source, &format!("@input{index}:leaf{leaf_index}"))
                };
                if matches!(
                    leaf.label,
                    Label::Lifetime(_) | Label::Static | Label::Elided { .. }
                ) {
                    input_regions.insert(leaf.label.clone(), region.clone());
                }
                let input = ir::InputLeaf {
                    index,
                    projections: leaf.path,
                };
                scope.parameter_sites.insert(
                    input.clone(),
                    (
                        self.source.types[parameter.ty].span.line,
                        self.source.types[parameter.ty].span.column,
                    ),
                );
                scope.parameters.insert(input, region);
            }
        }
        let elided_result = self
            .return_elision(instance.source, &function.locals, function.result)?
            .and_then(|label| input_regions.get(&label).cloned());
        for leaf in self.region_leaves(instance.source, source.result, function.result) {
            let required = if matches!(leaf.label, Label::Elided { .. }) {
                Some(elided_result.clone().ok_or_else(|| {
                    err(
                        self.source.types[source.result].span,
                        "E_BORROW",
                        "elided output region needs one source region or a borrowed receiver",
                    )
                })?)
            } else {
                named_region(instance.source, &leaf.label)
            };
            if let Some(region) = required {
                scope.results.push(crate::check::scoped::ResultLeaf {
                    invariant: crate::check::scoped::invariant_path(
                        &self.types,
                        function.result,
                        &leaf.path,
                    ),
                    projections: leaf.path,
                    region,
                    created: (
                        self.source.types[source.result].span.line,
                        self.source.types[source.result].span.column,
                    ),
                });
            }
        }
        // A nested reference in a well-formed input must outlive the reference
        // through which that stored value is borrowed. These are signature
        // implications, not facts manufactured by a call needing a region.
        for (leaf, inner) in &scope.parameters {
            for cut in 0..leaf.projections.len() {
                if leaf.projections[cut] != ir::Projection::Deref {
                    continue;
                }
                let outer = ir::InputLeaf {
                    index: leaf.index,
                    projections: leaf.projections[..cut].to_vec(),
                };
                if let Some(outer) = scope.parameters.get(&outer) {
                    insert_outlives(&mut scope.outlives, inner.clone(), outer.clone());
                }
            }
        }
        Ok(scope)
    }
    fn opaque_callable(&self, mut ty: usize) -> Option<usize> {
        loop {
            if self.closures.region_bounds.contains_key(&ty) {
                return Some(ty);
            }
            if let ir::TypeKind::Ref { inner, .. } = self.types[ty].kind {
                ty = inner;
            } else {
                return None;
            }
        }
    }
    fn opaque_requirements(&self, ty: usize) -> Vec<Requirement> {
        let signature = self.closures.assumptions.get(&ty).unwrap();
        let mut requirements = Vec::new();
        // Every registered assumption has already passed ordinary callable
        // signature agreement. Multiple declarations conservatively retain all
        // their fixed regions; fresh inputs contribute no retained obligation.
        for bound in self.closures.region_bounds.get(&ty).unwrap() {
            for (argument, (syntax, concrete)) in
                bound.arguments.iter().zip(&signature.arguments).enumerate()
            {
                for leaf in self.region_leaves(bound.scope, *syntax, *concrete) {
                    if let Some(region) = named_region(bound.scope, &leaf.label) {
                        requirements.push(Requirement {
                            argument: argument + 1,
                            projections: leaf.path,
                            region,
                            preserve: leaf.preserve,
                            mutable_storage: leaf.mutable_storage,
                        });
                    }
                }
            }
        }
        normalize(requirements)
    }
    fn bind_callable_regions(
        &self,
        instance: &Instance,
        bindings: &mut RegionBindings,
    ) -> Result<()> {
        for parameter in self.scoped_parameters(instance.source) {
            let Some(actual_ty) = instance.substitutions.get(&parameter.name) else {
                continue;
            };
            let actual_bounds = self.callable_region_bounds(*actual_ty);
            if actual_bounds.is_empty() {
                continue;
            }
            let Some(signature) = self.callable_signature(*actual_ty) else {
                continue;
            };
            for bound in &parameter.bounds {
                if !self.is_callable_bound(*bound) {
                    continue;
                }
                let a::Type::Path(formal) = &self.source.types[*bound].kind else {
                    unreachable!()
                };
                let a::Type::Tuple(parameters) = &self.source.types[formal.args[0]].kind else {
                    unreachable!()
                };
                for actual in &actual_bounds {
                    for ((formal, actual_syntax), concrete) in parameters
                        .iter()
                        .zip(&actual.arguments)
                        .zip(&signature.arguments)
                    {
                        let mut actual_leaves: BTreeMap<Vec<ir::Projection>, Label> =
                            BTreeMap::new();
                        for leaf in self.region_leaves(actual.scope, *actual_syntax, *concrete) {
                            actual_leaves.insert(leaf.path, leaf.label);
                        }
                        for leaf in self.region_leaves(instance.source, *formal, *concrete) {
                            let Some(actual_label) = actual_leaves.get(&leaf.path) else {
                                continue;
                            };
                            if matches!(leaf.label, Label::Elided { .. })
                                && !matches!(actual_label, Label::Elided { .. } | Label::Unknown)
                            {
                                return Err(err(
                                    self.source.types[*bound].span,
                                    "E_TRAIT",
                                    "fixed-region callable does not establish a fresh higher-ranked input",
                                ));
                            }
                            let Some(required) = named_region(actual.scope, actual_label) else {
                                continue;
                            };
                            if matches!(
                                leaf.label,
                                Label::Lifetime(_)
                                    | Label::Parameter(_, _)
                                    | Label::Associated(_, _)
                            ) {
                                insert_region(bindings, leaf.label, required);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn source_requirements(
        &self,
        instance: &Instance,
        function: &ir::Function,
    ) -> Result<Vec<Requirement>> {
        let mut bindings = RegionBindings::new();
        self.bind_callable_regions(instance, &mut bindings)?;
        // If 'long: 'short and the call fixes 'short to an enclosing region,
        // every 'long argument must outlive that same enclosing obligation.
        let edges = self.region_edges(instance.source);
        loop {
            let mut changed = false;
            for (long, shorts) in &edges {
                for short in shorts {
                    let required = bindings
                        .get(&Label::Lifetime(short.name.clone()))
                        .cloned()
                        .unwrap_or(BTreeSet::new());
                    let label = Label::Lifetime(long.name.clone());
                    if !bindings.contains_key(&label) {
                        bindings.insert(label.clone(), BTreeSet::new());
                    }
                    let entry = bindings.get_mut(&label).unwrap();
                    for region in required {
                        changed |= entry.insert(region);
                    }
                }
            }
            if !changed {
                break;
            }
        }
        let mut requirements = Vec::new();
        for (argument, parameter) in self.source.functions[instance.source]
            .params
            .iter()
            .enumerate()
        {
            for leaf in
                self.region_leaves(instance.source, parameter.ty, function.locals[argument].ty)
            {
                let regions = if leaf.label == Label::Static {
                    let mut regions = BTreeSet::new();
                    regions.insert(region(0, "'static"));
                    regions
                } else {
                    bindings
                        .get(&leaf.label)
                        .cloned()
                        .unwrap_or(BTreeSet::new())
                };
                for region in regions {
                    requirements.push(Requirement {
                        argument,
                        projections: leaf.path.clone(),
                        region,
                        preserve: leaf.preserve,
                        mutable_storage: leaf.mutable_storage,
                    });
                }
            }
        }
        Ok(normalize(requirements))
    }

    pub(super) fn scoped_definitions(
        &self,
        checked: &BTreeSet<usize>,
    ) -> (BTreeMap<usize, Scope>, Vec<a::Diagnostic>) {
        if checked.is_empty() {
            return (BTreeMap::new(), Vec::new());
        }
        let mut instances: BTreeMap<usize, Instance> = BTreeMap::new();
        for (function, pending) in &self.pending_functions {
            if let PendingFunction::Source(instance) = pending {
                instances.insert(*function, instance.clone());
            }
        }
        let mut opaque: BTreeMap<usize, usize> = BTreeMap::new();
        for (ty, function) in &self.closures.assumed_functions {
            opaque.insert(*function, *ty);
        }
        for ((ty, _), function) in &self.closures.wrappers {
            if let Some(ty) = self.opaque_callable(*ty) {
                opaque.insert(*function, ty);
            }
        }
        let mut calls = BTreeMap::new();
        let mut failures = BTreeMap::new();
        for (function, instance) in &instances {
            match self.source_requirements(instance, &self.functions[*function]) {
                Ok(requirements) => {
                    if !requirements.is_empty() {
                        calls.insert(*function, requirements);
                    }
                }
                Err(problem) => {
                    failures.insert(*function, problem);
                }
            }
        }
        for (function, ty) in &opaque {
            let requirements = self.opaque_requirements(*ty);
            if !requirements.is_empty() {
                calls.insert(*function, requirements);
            }
        }
        for (function, body) in &self.scoped_bodies {
            if !body.parameters.is_empty() {
                if !calls.contains_key(function) {
                    calls.insert(*function, Vec::new());
                }
                let requirements = calls.get_mut(function).unwrap();
                for parameter in &body.parameters {
                    requirements.push(Requirement {
                        argument: parameter.place.local,
                        projections: parameter.place.projections.clone(),
                        region: parameter.region.clone(),
                        preserve: parameter.place.projections.contains(&ir::Projection::Deref),
                        // Concrete closure bodies receive the ordinary external-store
                        // identity check; this is not an opaque mutation promise.
                        mutable_storage: false,
                    });
                }
            }
        }
        let mut scopes = BTreeMap::new();
        let mut problems = Vec::new();
        for function_index in checked {
            let function = &self.functions[*function_index];
            let mut scope = if let Some(instance) = instances.get(function_index) {
                match self.source_scope(instance, function) {
                    Ok(scope) => scope,
                    Err(problem) => {
                        problems.push(problem);
                        continue;
                    }
                }
            } else if let Some(ty) = opaque.get(function_index) {
                // Internal borrowed-callable wrappers forward exactly these
                // declared arguments. Their caller must prove the same regions.
                let mut scope = Scope::default();
                for bound in self.closures.region_bounds.get(ty).unwrap() {
                    for (long, shorts) in self.region_edges(bound.scope) {
                        scope.outlives.insert(long, shorts);
                    }
                }
                for requirement in self.opaque_requirements(*ty) {
                    scope.parameters.insert(
                        ir::InputLeaf {
                            index: requirement.argument,
                            projections: requirement.projections,
                        },
                        requirement.region,
                    );
                }
                scope
            } else {
                // Captured origin families need their own checked environment
                // equation. Do not infer a region from a capture's concrete type
                // or a coincidentally equal source variable name.
                Scope::default()
            };
            if let Some(body) = self.scoped_bodies.get(function_index) {
                if let Some(owner) = body.source_scope {
                    for (long, shorts) in self.region_edges(owner) {
                        scope.outlives.insert(long, shorts);
                    }
                }
                for parameter in &body.parameters {
                    scope.parameter_sites.insert(
                        ir::InputLeaf {
                            index: parameter.place.local,
                            projections: parameter.place.projections.clone(),
                        },
                        parameter.created,
                    );
                    scope.parameters.insert(
                        ir::InputLeaf {
                            index: parameter.place.local,
                            projections: parameter.place.projections.clone(),
                        },
                        parameter.region.clone(),
                    );
                }
                if body.environment.is_some() {
                    scope.results = body.results.clone();
                }
                scope.annotations = body.annotations.clone();
                scope.constructions = body.constructions.clone();
                scope.environment = body.environment.clone();
            }
            for (instruction_index, instruction) in function.body.iter().enumerate() {
                let ir::Op::Call {
                    function: target, ..
                } = instruction.op
                else {
                    continue;
                };
                if let Some(problem) = failures.get(&target) {
                    let mut problem = problem.clone();
                    problem.file = function.file;
                    problem.line = instruction.line;
                    problem.column = instruction.column;
                    problems.push(problem);
                }
                if let Some(requirements) = calls.get(&target) {
                    scope.calls.insert(instruction_index, requirements.clone());
                }
            }
            if !scope.calls.is_empty()
                || !scope.annotations.bindings.is_empty()
                || !scope.annotations.checkpoints.is_empty()
                || !scope.constructions.is_empty()
                || scope.environment.is_some()
            {
                scopes.insert(*function_index, scope);
            }
        }
        (scopes, problems)
    }
}
