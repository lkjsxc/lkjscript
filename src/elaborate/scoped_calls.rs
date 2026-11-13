//! Fixed callable regions in the discarded definition program. Region variables
//! belong to declarations; type equality never identifies their scopes.
use super::origins::Label;
use super::{Compiler, Instance, PendingFunction, Result, a, err, k};
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
fn normalize(requirements: Vec<Requirement>) -> Vec<Requirement> {
    let mut unique = BTreeMap::new();
    for requirement in requirements {
        let entry = unique
            .entry((
                requirement.argument,
                requirement.projections,
                requirement.region,
            ))
            .or_insert((false, false));
        entry.0 |= requirement.preserve;
        entry.1 |= requirement.mutable_storage;
    }
    unique
        .into_iter()
        .map(
            |((argument, projections, region), (preserve, mutable_storage))| Requirement {
                argument,
                projections,
                region,
                preserve,
                mutable_storage,
            },
        )
        .collect()
}

impl Compiler<'_> {
    fn scoped_parameters(&self, index: usize) -> Vec<&a::GenericParam> {
        let mut parameters: Vec<_> = self.source.functions[index].generics.iter().collect();
        match self.source.function_owner(a::FunctionId { index }) {
            a::FunctionOwner::Implementation(owner) => {
                parameters.extend(&self.source.impls[owner.index].generics)
            }
            a::FunctionOwner::Trait(owner) => {
                parameters.extend(&self.source.traits[owner.index].generics)
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
                        edges
                            .entry(region(index, &parameter.name))
                            .or_default()
                            .insert(region(index, &path.name));
                    }
                }
            }
        }
        edges
    }
    fn source_scope(&self, instance: &Instance, function: &k::Function) -> Scope {
        let source = &self.source.functions[instance.source];
        let mut scope = Scope {
            outlives: self.region_edges(instance.source),
            ..Scope::default()
        };
        for (index, parameter) in source.params.iter().enumerate() {
            let leaves =
                self.region_leaves(instance.source, parameter.ty, function.locals[index].ty);
            for (leaf_index, leaf) in leaves.into_iter().enumerate() {
                let region = named_region(instance.source, &leaf.label).unwrap_or_else(|| {
                    region(instance.source, &format!("@input{index}:leaf{leaf_index}"))
                });
                scope.parameters.insert(
                    k::InputLeaf {
                        index,
                        projections: leaf.path,
                    },
                    region,
                );
            }
        }
        // A nested reference in a well-formed input must outlive the reference
        // through which that stored value is borrowed. These are signature
        // implications, not facts manufactured by a call needing a region.
        for (leaf, inner) in &scope.parameters {
            for cut in 0..leaf.projections.len() {
                if leaf.projections[cut] != k::Projection::Deref {
                    continue;
                }
                let outer = k::InputLeaf {
                    index: leaf.index,
                    projections: leaf.projections[..cut].to_vec(),
                };
                if let Some(outer) = scope.parameters.get(&outer) {
                    scope
                        .outlives
                        .entry(inner.clone())
                        .or_default()
                        .insert(outer.clone());
                }
            }
        }
        scope
    }
    fn opaque_callable(&self, mut ty: usize) -> Option<usize> {
        loop {
            if self.closures.region_bounds.contains_key(&ty) {
                return Some(ty);
            }
            if let k::TypeKind::Ref { inner, .. } = self.types[ty].kind {
                ty = inner;
            } else {
                return None;
            }
        }
    }
    fn opaque_requirements(&self, ty: usize) -> Vec<Requirement> {
        let signature = &self.closures.assumptions[&ty];
        let mut requirements = Vec::new();
        // Every registered assumption has already passed ordinary callable
        // signature agreement. Multiple declarations conservatively retain all
        // their fixed regions; fresh inputs contribute no retained obligation.
        for bound in &self.closures.region_bounds[&ty] {
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
                        let actual_leaves: BTreeMap<_, _> = self
                            .region_leaves(actual.scope, *actual_syntax, *concrete)
                            .into_iter()
                            .map(|leaf| (leaf.path, leaf.label))
                            .collect();
                        for leaf in self.region_leaves(instance.source, *formal, *concrete) {
                            let Some(actual_label) = actual_leaves.get(&leaf.path) else {
                                continue;
                            };
                            if matches!(leaf.label, Label::Elided)
                                && !matches!(actual_label, Label::Elided | Label::Unknown)
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
                                bindings.entry(leaf.label).or_default().insert(required);
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
        function: &k::Function,
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
                        .unwrap_or_default();
                    let entry = bindings
                        .entry(Label::Lifetime(long.name.clone()))
                        .or_default();
                    let before = entry.len();
                    entry.extend(required);
                    changed |= before != entry.len();
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
                    BTreeSet::from([region(0, "'static")])
                } else {
                    bindings.get(&leaf.label).cloned().unwrap_or_default()
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
        let instances: BTreeMap<_, _> = self
            .pending_functions
            .iter()
            .filter_map(|(function, pending)| {
                if let PendingFunction::Source(instance) = pending {
                    Some((*function, instance.clone()))
                } else {
                    None
                }
            })
            .collect();
        let mut opaque: BTreeMap<_, _> = self
            .closures
            .assumed_functions
            .iter()
            .map(|(ty, function)| (*function, *ty))
            .collect();
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
        let mut scopes = BTreeMap::new();
        let mut problems = Vec::new();
        for function_index in checked {
            let function = &self.functions[*function_index];
            let mut scope = if let Some(instance) = instances.get(function_index) {
                self.source_scope(instance, function)
            } else if let Some(ty) = opaque.get(function_index) {
                // Internal borrowed-callable wrappers forward exactly these
                // declared arguments. Their caller must prove the same regions.
                let mut scope = Scope::default();
                for bound in &self.closures.region_bounds[ty] {
                    scope.outlives.extend(self.region_edges(bound.scope));
                }
                for requirement in self.opaque_requirements(*ty) {
                    scope.parameters.insert(
                        k::InputLeaf {
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
            for (instruction_index, instruction) in function.body.iter().enumerate() {
                let k::Op::Call {
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
            if !scope.calls.is_empty() {
                scopes.insert(*function_index, scope);
            }
        }
        (scopes, problems)
    }
}
