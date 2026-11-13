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
        let one = [projection.clone()];
        let Some(next) = projected_type(types, ty, &one) else {
            return false;
        };
        ty = next;
    }
    false
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
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
        let mut pending = vec![source.clone()];
        let mut seen: BTreeSet<Region> = BTreeSet::new();
        while let Some(region) = pending.pop() {
            if !seen.insert(region.clone()) {
                continue;
            }
            if let Some(next) = self.outlives.get(&region) {
                if next.contains(required) {
                    return true;
                }
                for next_region in next {
                    pending.push(next_region.clone());
                }
            }
        }
        false
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EffectKind {
    Call,
    Local,
    LocalInput,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
            get_or_default(&mut self.roots, source.origin.root.clone()).insert(id);
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
        let mut candidates: BTreeSet<EffectId> = BTreeSet::new();
        for loan in loans {
            for id in &loan.effects {
                candidates.insert(*id);
            }
        }
        let mut permitted: BTreeSet<EffectId> = BTreeSet::new();
        for id in candidates {
            if loans.iter().all(|loan| self.permits(&id, loan)) {
                permitted.insert(id);
            }
        }
        permitted
    }
    pub(super) fn check(
        &self,
        sources: &[SourceBound],
        permissions: &BTreeSet<EffectId>,
        mutable: bool,
        line: usize,
    ) -> Result<(), Diagnostic> {
        let mut candidates: BTreeSet<EffectId> = BTreeSet::new();
        for source in sources {
            if let Some(ids) = self.roots.get(&source.origin.root) {
                for id in ids {
                    candidates.insert(*id);
                }
            }
        }
        for id in candidates {
            if permissions.contains(&id) {
                continue;
            }
            let effect = self.values.get(&id).expect("indexed effect");
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
        for (input, region) in &scope.parameters {
            result.record(state, types, function, input, region)?;
        }
        for (input, regions) in &scope.guarantees {
            for region in regions {
                result.record(state, types, function, input, region)?;
            }
        }
        Ok(result)
    }
    fn record(
        &mut self,
        state: &State,
        types: &[Type],
        function: &Function,
        input: &InputLeaf,
        region: &Region,
    ) -> Result<(), Diagnostic> {
        let place = Place {
            local: input.index,
            projections: input.projections.clone(),
        };
        let provenance = carried_provenance(state, types, function, &place, &place, 1)?;
        for loan in provenance.loans {
            for source in loan.sources {
                let paths = get_or_default(&mut self.roots, source.origin.root);
                get_or_default(paths, source.origin.projections).insert(region.clone());
            }
        }
        Ok(())
    }
    fn proves(&self, source: &SourceBound, required: &Region, scope: &Scope) -> bool {
        if source.origin.root == Root::Static {
            return true;
        }
        let Some(paths) = self.roots.get(&source.origin.root) else {
            return false;
        };
        let mut path = source.origin.projections.clone();
        loop {
            if let Some(regions) = paths.get(&path) {
                return regions
                    .iter()
                    .any(|region| scope.outlives(region, required));
            }
            if path.pop().is_none() {
                return false;
            }
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
            return if state.loans.get(&holder).is_some() {
                vec![holder]
            } else {
                Vec::new()
            };
        }
    }
    let mut holders: Vec<Place> = Vec::new();
    for holder in resolved.parents {
        if state.loans.get(&holder).is_some() {
            holders.push(holder);
        }
    }
    holders
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
        for index in 0..returned.loans.len() {
            let loan = &mut returned.loans[index];
            let mut inputs: Vec<InputLeaf> = Vec::new();
            match &callee.return_source {
                Some(ReturnSource::Leaves(leaves)) => {
                    for leaf in leaves {
                        if leaf.projections == loan.holder.projections {
                            if let LeafOrigin::Inputs {
                                inputs: declared, ..
                            } = &leaf.origin
                            {
                                inputs = declared.clone();
                            }
                            break;
                        }
                    }
                }
                Some(ReturnSource::Parameter { index, projections }) => {
                    let source = Place {
                        local: *index,
                        projections: projections.clone(),
                    };
                    let ty = place_type(call.types, callee, &source).unwrap();
                    for (path, _) in reference_paths(call.types, ty) {
                        let mut selected = projections.clone();
                        for projection in path {
                            selected.push(projection);
                        }
                        inputs.push(InputLeaf {
                            index: *index,
                            projections: selected,
                        });
                    }
                }
                _ => {}
            }
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
            let mut holders: Vec<Place> = Vec::new();
            if let Some(grantees) = state.loans.effects.get(&id) {
                for holder in grantees {
                    let loan = state
                        .loans
                        .values
                        .get(holder)
                        .expect("indexed effect holder");
                    if overlaps_sources(&loan.sources, &effect.sources) {
                        holders.push(holder.clone());
                    }
                }
            }
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
        for id in &prior.effects {
            candidates.insert(*id);
        }
    }
    let mut permitted = BTreeSet::new();
    for id in candidates {
        let left_permits = match prior {
            Some(loan) => left.permits(&id, loan),
            None => true,
        };
        if left_permits && right.permits(&id, next) {
            permitted.insert(id);
        }
    }
    permitted
}

pub(super) fn intersect_permissions(
    left: &BTreeSet<EffectId>,
    right: &BTreeSet<EffectId>,
) -> BTreeSet<EffectId> {
    let mut result: BTreeSet<EffectId> = BTreeSet::new();
    for id in left {
        if right.contains(id) {
            result.insert(*id);
        }
    }
    result
}
pub(super) fn common_permissions(values: &[BTreeSet<EffectId>]) -> BTreeSet<EffectId> {
    if values.is_empty() {
        return BTreeSet::new();
    }
    let mut result = values[0].clone();
    for next in &values[1..] {
        result = intersect_permissions(&result, next);
    }
    result
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
    let mut waiting: BTreeMap<usize, usize> = BTreeMap::new();
    let mut parents = BTreeMap::<usize, BTreeSet<usize>>::new();
    for (index, scope) in scopes {
        let mut children: BTreeSet<usize> = BTreeSet::new();
        for constructions in scope.constructions.values() {
            for construction in constructions {
                if scopes.contains_key(&construction.function) {
                    children.insert(construction.function);
                }
            }
        }
        waiting.insert(*index, children.len());
        for child in children {
            get_or_default(&mut parents, child).insert(*index);
        }
    }
    let mut ready: BTreeSet<usize> = BTreeSet::new();
    for (index, count) in &waiting {
        if *count == 0 {
            ready.insert(*index);
        }
    }
    let mut captures = BTreeMap::<usize, Vec<Annotation>>::new();
    let mut checked: BTreeSet<usize> = BTreeSet::new();
    while let Some(index) = ready.pop_first() {
        let function = &program.functions[index];
        let mut scope = scopes.get(&index).expect("scheduled scope").clone();
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
                        get_or_default(&mut scope.annotations.checkpoints, *instruction)
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
            let result = {
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
            };
            let result = match result {
                Ok(()) => {
                    site = None;
                    match observation.finish(&scope, &program.types, function) {
                        Ok(requirements) => {
                            let contract =
                                requirements.capture_contract(&mut scope, &program.types, function);
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
                        }
                        Err(problem) => Err(problem),
                    }
                }
                Err(problem) => Err(problem),
            };
            match result {
                Err(mut problem) => {
                    if let Some((line, column)) = site {
                        if problem.line == line {
                            problem.column = column;
                        }
                    }
                    Err(problem)
                }
                Ok(()) => Ok(()),
            }
        };
        if let Err(mut problem) = result {
            problem.file = function.file;
            problems.push(problem);
        }
        checked.insert(index);
        if let Some(dependents) = parents.get(&index) {
            for parent in dependents {
                let count = waiting.get_mut(parent).unwrap();
                *count -= 1;
                if *count == 0 {
                    ready.insert(*parent);
                }
            }
        }
    }
    for index in scopes.keys().filter(|index| !checked.contains(*index)) {
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
