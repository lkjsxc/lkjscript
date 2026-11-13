//! Additional obligations for discarded definition-verification programs.
//! These annotations cannot create CheckedProgram authority or alter parsed K.
//! A fixed source region may retain a call argument after its descriptor dies.
use super::*;
mod locals;
mod views;
pub(super) use locals::LocalPass;
pub(crate) use locals::{Annotation, Annotations, Construction, Environment};

#[derive(Clone)]
pub(crate) struct ResultLeaf {
    pub projections: Vec<Projection>,
    pub region: Region,
    pub created: (usize, usize),
    pub invariant: bool,
}

pub(crate) fn invariant_path(types: &[Type], mut ty: usize, path: &[Projection]) -> bool {
    for projection in path {
        if *projection == Projection::Deref
            && matches!(types[ty].kind, TypeKind::Ref { mutable: true, .. })
        {
            return true;
        }
        let Some(next) = projected_type(types, ty, std::slice::from_ref(projection)) else {
            return false;
        };
        ty = next;
    }
    false
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Region {
    pub scope: usize,
    pub name: String,
}
impl Region {
    pub(crate) fn is_static(&self) -> bool {
        self.name == "'static"
    }
}

#[derive(Clone)]
pub(crate) struct Requirement {
    pub argument: usize,
    pub projections: Vec<Projection>,
    pub region: Region,
    // Shared references stored behind a borrow can be copied out. Exclusive
    // stored references cannot be moved out without a checked mutation contract.
    pub preserve: bool,
    pub mutable_storage: bool,
}
#[derive(Clone, Default)]
pub(crate) struct Scope {
    pub parameters: BTreeMap<InputLeaf, Region>,
    pub parameter_sites: BTreeMap<InputLeaf, (usize, usize)>,
    pub guarantees: BTreeMap<InputLeaf, BTreeSet<Region>>,
    pub outlives: BTreeMap<Region, BTreeSet<Region>>,
    pub calls: BTreeMap<usize, Vec<Requirement>>,
    pub results: Vec<ResultLeaf>,
    pub annotations: Annotations,
    pub constructions: BTreeMap<usize, Vec<Construction>>,
    pub environment: Option<Environment>,
}
impl Scope {
    fn outlives(&self, source: &Region, required: &Region) -> bool {
        if source == required || source.is_static() {
            return true;
        }
        let mut pending = vec![source];
        let mut seen = BTreeSet::new();
        while let Some(region) = pending.pop() {
            if !seen.insert(region) {
                continue;
            }
            if let Some(next) = self.outlives.get(region) {
                if next.contains(required) {
                    return true;
                }
                pending.extend(next);
            }
        }
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum EffectKind {
    Call,
    Local,
    LocalInput,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct EffectId {
    kind: EffectKind,
    instruction: usize,
    requirement: usize,
}
#[derive(Clone, PartialEq, Eq)]
struct Effect {
    region: Region,
    sources: Vec<SourceBound>,
    mutable: bool,
    created: (usize, usize),
}
#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct Effects {
    values: BTreeMap<EffectId, Effect>,
    roots: BTreeMap<Root, BTreeSet<EffectId>>,
}
impl Effects {
    fn insert(&mut self, id: EffectId, mut effect: Effect) {
        if let Some(previous) = self.values.get(&id) {
            effect.sources.extend_from_slice(&previous.sources);
            normalize_sources(&mut effect.sources);
        }
        for source in &effect.sources {
            self.roots
                .entry(source.origin.root.clone())
                .or_default()
                .insert(id);
        }
        self.values.insert(id, effect);
    }
    pub(super) fn join(&mut self, other: &Self) -> bool {
        let mut changed = false;
        for (id, effect) in &other.values {
            if self.values.get(id) == Some(effect) {
                continue;
            }
            let previous = self.values.get(id).cloned();
            self.insert(*id, effect.clone());
            changed |= previous.as_ref() != self.values.get(id);
        }
        changed
    }
    fn permits(&self, id: &EffectId, loan: &Loan) -> bool {
        loan.effects.contains(id)
            || self
                .values
                .get(id)
                .is_none_or(|effect| !overlaps_sources(&loan.sources, &effect.sources))
    }
    // A checked result may select among disjoint input leaves. Its existing
    // permission remains valid only if every overlapping alternative has that
    // permission; an unrelated alternative cannot revoke or create authority.
    pub(super) fn selection_permissions(&self, loans: &[Loan]) -> BTreeSet<EffectId> {
        let candidates: BTreeSet<_> = loans
            .iter()
            .flat_map(|loan| loan.effects.iter().copied())
            .collect();
        candidates
            .into_iter()
            .filter(|id| loans.iter().all(|loan| self.permits(id, loan)))
            .collect()
    }
    pub(super) fn check(
        &self,
        sources: &[SourceBound],
        permissions: &BTreeSet<EffectId>,
        mutable: bool,
        line: usize,
    ) -> Result<(), Diagnostic> {
        let mut candidates = BTreeSet::new();
        for source in sources {
            if let Some(ids) = self.roots.get(&source.origin.root) {
                candidates.extend(ids);
            }
        }
        for id in candidates {
            if permissions.contains(id) {
                continue;
            }
            let effect = &self.values[id];
            if (mutable || effect.mutable) && overlaps_sources(sources, &effect.sources) {
                return Err(error(
                    line,
                    "E_BORROW",
                    &format!(
                        "access conflicts with {} borrow retained for {} (created at {}:{})",
                        if effect.mutable {
                            "exclusive"
                        } else {
                            "shared"
                        },
                        effect.region.name,
                        effect.created.0,
                        effect.created.1,
                    ),
                ));
            }
        }
        Ok(())
    }
}

pub(super) struct CallContext<'a> {
    pub(super) state: &'a State,
    pub(super) function: &'a Function,
    pub(super) types: &'a [Type],
    pub(super) instruction: usize,
    pub(super) args: &'a [usize],
}

#[derive(Default)]
pub(super) struct InputRegions {
    roots: BTreeMap<Root, BTreeMap<Vec<Projection>, BTreeSet<Region>>>,
}
impl InputRegions {
    pub(super) fn new(
        state: &State,
        types: &[Type],
        function: &Function,
        scope: Option<&Scope>,
    ) -> Result<Self, Diagnostic> {
        let mut result = Self::default();
        let Some(scope) = scope else {
            return Ok(result);
        };
        let parameters = scope.parameters.iter().chain(
            scope
                .guarantees
                .iter()
                .flat_map(|(input, regions)| regions.iter().map(move |region| (input, region))),
        );
        for (input, region) in parameters {
            let place = Place {
                local: input.index,
                projections: input.projections.clone(),
            };
            let provenance = carried_provenance(state, types, function, &place, &place, 1)?;
            for loan in provenance.loans {
                for source in loan.sources {
                    result
                        .roots
                        .entry(source.origin.root)
                        .or_default()
                        .entry(source.origin.projections)
                        .or_default()
                        .insert(region.clone());
                }
            }
        }
        Ok(result)
    }
    fn proves(&self, source: &SourceBound, required: &Region, scope: &Scope) -> bool {
        if source.origin.root == Root::Static {
            return true;
        }
        let Some(paths) = self.roots.get(&source.origin.root) else {
            return false;
        };
        let mut path = source.origin.projections.as_slice();
        loop {
            if let Some(regions) = paths.get(path) {
                return regions
                    .iter()
                    .any(|region| scope.outlives(region, required));
            }
            let Some((_, parent)) = path.split_last() else {
                return false;
            };
            path = parent;
        }
    }
    pub(super) fn before_call(
        &self,
        scope: Option<&Scope>,
        call: &CallContext<'_>,
    ) -> Result<Retained, Diagnostic> {
        let CallContext {
            state,
            function,
            types,
            instruction,
            args,
        } = *call;
        let mut retained = Retained::default();
        let Some(scope) = scope else {
            return Ok(retained);
        };
        let Some(requirements) = scope.calls.get(&instruction) else {
            return Ok(retained);
        };
        let (line, column) = state.site.unwrap_or((1, 1));
        for (index, requirement) in requirements.iter().enumerate() {
            let place = Place {
                local: args[requirement.argument],
                projections: requirement.projections.clone(),
            };
            let value = carried_provenance(state, types, function, &place, &place, line)?;
            for loan in value.loans {
                if !loan_active(state, &loan) {
                    continue;
                }
                if !loan
                    .sources
                    .iter()
                    .all(|source| self.proves(source, &requirement.region, scope))
                {
                    return Err(error(
                        line,
                        "E_BORROW",
                        &format!(
                            "{} does not establish the fixed input region {}; its origin must outlive that region",
                            place_name(types, function, &place),
                            requirement.region.name,
                        ),
                    ));
                }
                if requirement.mutable_storage {
                    return Err(error(
                        line,
                        "E_UNSUPPORTED",
                        "opaque callable mutation of stored reference origins needs a checked mutation contract",
                    ));
                }
                {
                    retained.values.push((
                        EffectId {
                            kind: EffectKind::Call,
                            instruction,
                            requirement: index,
                        },
                        Effect {
                            region: requirement.region.clone(),
                            sources: loan.sources,
                            mutable: loan.mutable,
                            created: (line, column),
                        },
                        InputLeaf {
                            index: requirement.argument,
                            projections: requirement.projections.clone(),
                        },
                        if requirement.preserve {
                            stored_holders(state, &place)
                        } else {
                            Vec::new()
                        },
                    ));
                }
            }
        }
        Ok(retained)
    }
}
// A borrowed stored descriptor remains the current value after a read-only
// call. Its parent reborrow remains suspended; no name/type coincidence can
// grant that parent the current holder's authority.
fn stored_holders(state: &State, place: &Place) -> Vec<Place> {
    let resolved = resolve_origin(state, place);
    if resolved.exact() {
        if let Root::Local(local) = resolved.sources[0].origin.root {
            let holder = Place {
                local,
                projections: resolved.sources[0].origin.projections.clone(),
            };
            return state
                .loans
                .get(&holder)
                .map(|_| vec![holder])
                .unwrap_or_default();
        }
    }
    resolved
        .parents
        .into_iter()
        .filter(|holder| state.loans.get(holder).is_some())
        .collect()
}

#[derive(Default)]
pub(super) struct Retained {
    values: Vec<(EffectId, Effect, InputLeaf, Vec<Place>)>,
}
impl Retained {
    pub(super) fn permit_results(
        &self,
        call: &CallContext<'_>,
        callee: &Function,
        returned: &mut Provenance,
    ) -> Result<(), Diagnostic> {
        let line = call.state.site.map_or(1, |site| site.0);
        for loan in &mut returned.loans {
            let inputs = match &callee.return_source {
                Some(ReturnSource::Leaves(leaves)) => leaves
                    .iter()
                    .find(|leaf| leaf.projections == loan.holder.projections)
                    .and_then(|leaf| match &leaf.origin {
                        LeafOrigin::Inputs { inputs, .. } => Some(inputs.clone()),
                        LeafOrigin::Static => None,
                    })
                    .unwrap_or_default(),
                Some(ReturnSource::Parameter { index, projections }) => {
                    let source = Place {
                        local: *index,
                        projections: projections.clone(),
                    };
                    let ty = place_type(call.types, callee, &source).unwrap();
                    reference_paths(call.types, ty)
                        .into_iter()
                        .map(|(path, _)| {
                            let mut projections = projections.clone();
                            projections.extend(path);
                            InputLeaf {
                                index: *index,
                                projections,
                            }
                        })
                        .collect()
                }
                _ => Vec::new(),
            };
            for (id, effect, retained_input, _) in &self.values {
                if !overlaps_sources(&loan.sources, &effect.sources) {
                    continue;
                }
                loan.effects.remove(id);
                if !inputs.contains(retained_input) {
                    continue;
                }
                // The checked return mapping, not possible pointer overlap,
                // supplies authority. Other alternative inputs must be disjoint
                // from this retained source before the common result may use it.
                let mut authorized = true;
                for input in &inputs {
                    if input == retained_input {
                        continue;
                    }
                    let place = Place {
                        local: call.args[input.index],
                        projections: input.projections.clone(),
                    };
                    let value = carried_provenance(
                        call.state,
                        call.types,
                        call.function,
                        &place,
                        &place,
                        line,
                    )?;
                    if value
                        .loans
                        .iter()
                        .any(|candidate| overlaps_sources(&candidate.sources, &effect.sources))
                    {
                        authorized = false;
                        break;
                    }
                }
                if authorized {
                    loan.effects.insert(*id);
                }
            }
        }
        Ok(())
    }
    pub(super) fn install(self, state: &mut State) {
        for (id, effect, _, preserved) in self.values {
            // Reusing a static call site does not renew its older descendants'
            // authority. Suspend matching old grants before granting the newly
            // returned descendants. The reverse index visits only grantees.
            let holders: Vec<_> = state
                .loans
                .effects
                .get(&id)
                .into_iter()
                .flatten()
                .filter(|holder| {
                    overlaps_sources(&state.loans.values[*holder].sources, &effect.sources)
                })
                .cloned()
                .collect();
            for holder in holders {
                let mut loan = state.loans.remove(&holder).unwrap();
                loan.effects.remove(&id);
                state.loans.push(loan);
            }
            for holder in preserved {
                if let Some(mut loan) = state.loans.remove(&holder) {
                    loan.effects.insert(id);
                    state.loans.push(loan);
                }
            }
            state.effects.insert(id, effect);
        }
    }
}

// A possible effect constrains only incoming paths on which it exists. A
// returned descendant joined with an unaffected value retains the permission
// exactly when every feasible incoming effect/value pair authorizes the use.
pub(super) fn join_permissions(
    left: &Effects,
    right: &Effects,
    prior: Option<&Loan>,
    next: &Loan,
) -> BTreeSet<EffectId> {
    let mut candidates = next.effects.clone();
    if let Some(prior) = prior {
        candidates.extend(&prior.effects);
    }
    candidates
        .into_iter()
        .filter(|id| prior.is_none_or(|loan| left.permits(id, loan)) && right.permits(id, next))
        .collect()
}

pub(super) fn common_permissions<'a>(
    values: impl Iterator<Item = &'a BTreeSet<EffectId>>,
) -> BTreeSet<EffectId> {
    let mut values = values;
    let Some(first) = values.next() else {
        return BTreeSet::new();
    };
    values.fold(first.clone(), |prior, next| {
        prior.intersection(next).copied().collect()
    })
}

/// The generic verifier consumes these diagnostics. This API never returns the
/// capability required by an encoder, and ordinary K cannot supply a Scope.
pub(crate) fn check_definitions(
    program: &Program,
    scopes: &BTreeMap<usize, Scope>,
) -> Vec<Diagnostic> {
    if scopes.is_empty() {
        return Vec::new();
    }
    if let Err(problem) = check_structure(program) {
        return vec![problem];
    }
    let facts = TypeFacts::new(program);
    let mut problems = Vec::new();
    let mut waiting = BTreeMap::new();
    let mut parents = BTreeMap::<usize, BTreeSet<usize>>::new();
    for (index, scope) in scopes {
        let children: BTreeSet<_> = scope
            .constructions
            .values()
            .flatten()
            .map(|construction| construction.function)
            .filter(|child| scopes.contains_key(child))
            .collect();
        waiting.insert(*index, children.len());
        for child in children {
            parents.entry(child).or_default().insert(*index);
        }
    }
    let mut ready: BTreeSet<_> = waiting
        .iter()
        .filter_map(|(index, count)| (*count == 0).then_some(*index))
        .collect();
    let mut captures = BTreeMap::<usize, Vec<Annotation>>::new();
    let mut checked = BTreeSet::new();
    while let Some(index) = ready.pop_first() {
        let function = &program.functions[index];
        let mut scope = scopes[&index].clone();
        for (instruction, constructions) in &scope.constructions {
            for construction in constructions {
                if let Some(requirements) = captures.get(&construction.function) {
                    for requirement in requirements {
                        let mut annotation = requirement.clone();
                        annotation.place = construction.place.clone();
                        annotation
                            .place
                            .projections
                            .extend_from_slice(&requirement.place.projections);
                        scope
                            .annotations
                            .checkpoints
                            .entry(*instruction)
                            .or_default()
                            .push(annotation);
                    }
                }
            }
        }
        let result = if scope.annotations.bindings.is_empty()
            && scope.annotations.checkpoints.is_empty()
            && scope.results.is_empty()
            && scope.calls.is_empty()
        {
            Ok(())
        } else {
            let mut site = None;
            let mut observation = locals::Observation::new(&scope, &program.types, function);
            let mut observed = LocalPass::Observe(&mut observation);
            check_function_body(
                program,
                function,
                &facts,
                &mut site,
                None,
                None,
                Some(&mut observed),
            )
            .and_then(|()| {
                site = None;
                let requirements = observation.finish(&scope, &program.types, function)?;
                let contract = requirements.capture_contract(&mut scope, &program.types, function);
                captures.insert(index, contract);
                let mut checked = LocalPass::Verify {
                    requirements: &requirements,
                    changed: Vec::new(),
                };
                check_function_body(
                    program,
                    function,
                    &facts,
                    &mut site,
                    None,
                    Some(&scope),
                    Some(&mut checked),
                )
            })
            .map_err(|mut problem| {
                if let Some((line, column)) = site {
                    if problem.line == line {
                        problem.column = column;
                    }
                }
                problem
            })
        };
        if let Err(mut problem) = result {
            problem.file = function.file;
            problems.push(problem);
        }
        checked.insert(index);
        for parent in parents.get(&index).into_iter().flatten() {
            let count = waiting.get_mut(parent).unwrap();
            *count -= 1;
            if *count == 0 {
                ready.insert(*parent);
            }
        }
    }
    for index in scopes.keys().filter(|index| !checked.contains(index)) {
        let mut problem = error(
            1,
            "E_UNSUPPORTED",
            "recursive closure construction needs an explicit captured-region contract",
        );
        problem.file = program.functions[*index].file;
        problems.push(problem);
    }
    problems
}
