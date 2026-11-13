//! Source-local region constraints. Ordinary checked transfers supply a graph
//! of descriptor storage identities. The graph can only add source obligations.
use super::*;

#[derive(Clone)]
pub(crate) struct Annotation {
    pub place: Place,
    pub region: Region,
    pub created: (usize, usize),
    pub invariant: bool,
}
#[derive(Clone, Default)]
pub(crate) struct Annotations {
    pub bindings: Vec<Annotation>,
    pub checkpoints: BTreeMap<usize, Vec<Annotation>>,
}
#[derive(Clone)]
pub(crate) struct Construction {
    pub function: usize,
    pub place: Place,
}
#[derive(Clone)]
pub(crate) struct Environment {
    pub prefix: Vec<Projection>,
}

type Demands = BTreeMap<Origin, BTreeMap<Region, (usize, usize)>>;
type Paths = Vec<(Vec<Projection>, bool)>;

pub(in super::super) struct Observation {
    annotations: Annotations,
    indirect: BTreeMap<usize, Vec<Annotation>>,
    demands: Demands,
    declared: BTreeMap<Origin, BTreeSet<Region>>,
    edges: BTreeMap<Origin, BTreeSet<Origin>>,
    blocked: BTreeMap<Origin, usize>,
    paths: BTreeMap<usize, Paths>,
    changed: Vec<Place>,
    views: super::views::Views,
    invariant: BTreeMap<Origin, BTreeSet<Region>>,
    parameter_references: BTreeSet<usize>,
    calls: BTreeMap<usize, Vec<Requirement>>,
    results: Vec<ResultLeaf>,
}
fn append(place: &Place, suffix: &[Projection]) -> Place {
    let mut result = place.clone();
    result.projections.extend_from_slice(suffix);
    result
}
fn storage(state: &State, place: &Place, line: usize) -> Result<Vec<Origin>, Diagnostic> {
    let resolved = resolve_origin(state, place);
    if resolved.sources.iter().any(|source| !source.exact) {
        return Err(error(
            line,
            "E_UNSUPPORTED",
            "named local regions need an exact descriptor storage contract",
        ));
    }
    Ok(source_origins(&resolved.sources))
}
impl Observation {
    pub(in super::super) fn new(scope: &Scope, types: &[Type], function: &Function) -> Self {
        let annotations = &scope.annotations;
        let mut parameter_references: BTreeSet<usize> = BTreeSet::new();
        for index in 0..function.params {
            if matches!(types[function.locals[index].ty].kind, TypeKind::Ref { .. }) {
                parameter_references.insert(index);
            }
        }
        let mut result = Self {
            annotations: annotations.clone(),
            indirect: BTreeMap::new(),
            demands: BTreeMap::new(),
            declared: BTreeMap::new(),
            edges: BTreeMap::new(),
            blocked: BTreeMap::new(),
            paths: BTreeMap::new(),
            changed: Vec::new(),
            views: super::views::Views::new(scope),
            invariant: BTreeMap::new(),
            parameter_references,
            calls: scope.calls.clone(),
            results: scope.results.clone(),
        };
        for annotation in &annotations.bindings {
            if annotation.place.projections.contains(&Projection::Deref) {
                get_or_default(&mut result.indirect, annotation.place.local)
                    .push(annotation.clone());
            } else {
                let node = Origin {
                    root: Root::Local(annotation.place.local),
                    projections: annotation.place.projections.clone(),
                };
                get_or_default(&mut result.declared, node.clone())
                    .insert(annotation.region.clone());
                get_or_default(&mut result.demands, node)
                    .insert(annotation.region.clone(), annotation.created);
            }
        }
        result
    }
    fn paths(&mut self, types: &[Type], ty: usize) -> Paths {
        if !self.paths.contains_key(&ty) {
            self.paths.insert(ty, reference_paths(types, ty));
        }
        self.paths.get(&ty).expect("cached reference paths").clone()
    }
    fn canonical_place(&self, node: &Origin) -> Option<Place> {
        let local = match node.root {
            Root::Local(local) | Root::Parameter(local) => local,
            Root::Static => return None,
        };
        let mut projections = Vec::new();
        if matches!(node.root, Root::Parameter(_)) && self.parameter_references.contains(&local) {
            projections.push(Projection::Deref);
        }
        projections.extend_from_slice(&node.projections);
        Some(Place { local, projections })
    }
    fn seed(&mut self, state: &State, annotation: &Annotation) -> Result<(), Diagnostic> {
        self.views.demand(
            annotation.place.clone(),
            annotation.region.clone(),
            annotation.created,
        );
        for node in storage(state, &annotation.place, annotation.created.0)? {
            if annotation.invariant {
                get_or_default(&mut self.declared, node.clone()).insert(annotation.region.clone());
                get_or_default(&mut self.invariant, node.clone()).insert(annotation.region.clone());
                if let Some(place) = self.canonical_place(&node) {
                    self.views
                        .declare(place, annotation.region.clone(), annotation.created);
                }
            }
            get_or_default(&mut self.demands, node)
                .insert(annotation.region.clone(), annotation.created);
        }
        Ok(())
    }
    fn edge(
        &mut self,
        state: &State,
        from: &Place,
        to: &Place,
        line: usize,
    ) -> Result<(), Diagnostic> {
        self.views.transfer(from, to, false, false);
        let sources = storage(state, from, line);
        for destination in storage(state, to, line)? {
            if let Ok(sources) = &sources {
                let edges = get_or_default(&mut self.edges, destination);
                for source in sources {
                    edges.insert(source.clone());
                }
            } else {
                self.blocked.insert(destination, line);
            }
        }
        Ok(())
    }
    fn transfer(
        &mut self,
        state: &State,
        types: &[Type],
        function: &Function,
        from: &Place,
        to: &Place,
        line: usize,
    ) -> Result<(), Diagnostic> {
        let ty = place_type(types, function, from).unwrap();
        let paths = self.paths(types, ty);
        if paths.is_empty() {
            return Ok(());
        }
        self.views.transfer(from, to, true, false);
        for node in storage(state, to, line)? {
            if let Some(place) = self.canonical_place(&node) {
                self.views.transfer(from, &place, true, false);
            }
        }
        for (path, _) in paths.iter() {
            self.edge(state, &append(from, path), &append(to, path), line)?;
        }
        Ok(())
    }
    fn borrow(
        &mut self,
        state: &State,
        types: &[Type],
        function: &Function,
        place: &Place,
        dst: usize,
        line: usize,
    ) -> Result<(), Diagnostic> {
        if matches!(types[function.locals[dst].ty].kind, TypeKind::Ref { inner, .. } if Some(inner) == place_type(types, function, place))
        {
            self.views.transfer(
                place,
                &Place {
                    local: dst,
                    projections: vec![Projection::Deref],
                },
                true,
                false,
            );
        }
        let mut prefix = Place::local(place.local);
        let mut descriptors: Vec<(Place, bool)> = Vec::new();
        for projection in &place.projections {
            if *projection == Projection::Deref {
                if let TypeKind::Ref { mutable, .. } =
                    types[place_type(types, function, &prefix).unwrap()].kind
                {
                    descriptors.push((prefix.clone(), mutable));
                }
            }
            prefix.projections.push(projection.clone());
        }
        // Shared descriptors can be copied independently of their carrier.
        // Reborrowing an exclusive stored descriptor also needs the issuing
        // carrier's region, up to that first independently copyable descriptor.
        while let Some((descriptor, mutable)) = descriptors.pop() {
            self.edge(state, &descriptor, &Place::local(dst), line)?;
            if !mutable {
                break;
            }
        }
        Ok(())
    }
    fn changed(&mut self, state: &State, place: &Place, line: usize) -> Result<(), Diagnostic> {
        for target in storage(state, place, line)? {
            if let Root::Local(local) = target.root {
                self.changed.push(Place {
                    local,
                    projections: target.projections,
                });
            }
        }
        Ok(())
    }
    fn before(
        &mut self,
        program: &Program,
        function: &Function,
        state: &State,
        index: usize,
    ) -> Result<(), Diagnostic> {
        self.changed.clear();
        let annotations = self.annotations.checkpoints.get(&index).cloned();
        if let Some(annotations) = annotations {
            for annotation in annotations {
                self.seed(state, &annotation)?;
            }
        }
        let instruction = &function.body[index];
        let line = instruction.line;
        let types = &program.types;
        if let Op::Call {
            function: target,
            args,
            ..
        } = &instruction.op
        {
            let requirements = self.calls.get(&index).cloned();
            if let Some(requirements) = requirements {
                for requirement in requirements {
                    let formal = &program.functions[*target];
                    let annotation = Annotation {
                        place: Place {
                            local: args[requirement.argument],
                            projections: requirement.projections.clone(),
                        },
                        region: requirement.region,
                        created: (line, instruction.column),
                        invariant: invariant_path(
                            types,
                            formal.locals[requirement.argument].ty,
                            &requirement.projections,
                        ),
                    };
                    self.seed(state, &annotation)?;
                }
            }
        }
        if let Op::Return { src } = instruction.op {
            for result in self.results.clone() {
                self.seed(
                    state,
                    &Annotation {
                        place: Place {
                            local: src,
                            projections: result.projections,
                        },
                        region: result.region,
                        created: result.created,
                        invariant: result.invariant,
                    },
                )?;
            }
        }
        let dst = match &instruction.op {
            Op::Read { dst, place, .. } => {
                self.transfer(state, types, function, place, &Place::local(*dst), line)?;
                Some(*dst)
            }
            Op::Borrow { dst, place, .. }
            | Op::SliceBorrow { dst, place, .. }
            | Op::ViewRange { dst, place, .. } => {
                self.borrow(state, types, function, place, *dst, line)?;
                Some(*dst)
            }
            Op::Assign { place, src } => {
                self.transfer(state, types, function, &Place::local(*src), place, line)?;
                if !self.paths(types, function.locals[*src].ty).is_empty() {
                    self.changed(state, place, line)?;
                }
                None
            }
            Op::Struct { dst, fields } | Op::Variant { dst, fields, .. } => {
                for (i, field) in fields.iter().enumerate() {
                    let projection = match instruction.op {
                        Op::Variant { variant, .. } => Projection::VariantField(variant, i),
                        _ => Projection::Field(i),
                    };
                    self.transfer(
                        state,
                        types,
                        function,
                        &Place::local(*field),
                        &Place {
                            local: *dst,
                            projections: vec![projection],
                        },
                        line,
                    )?;
                }
                Some(*dst)
            }
            Op::Call {
                dst,
                function: target,
                args,
            } => {
                let callee = &program.functions[*target];
                match &callee.return_source {
                    Some(ReturnSource::Leaves(leaves)) => {
                        for leaf in leaves {
                            if let LeafOrigin::Inputs { inputs, exact } = &leaf.origin {
                                for input in inputs {
                                    self.views.transfer(
                                        &Place {
                                            local: args[input.index],
                                            projections: input.projections.clone(),
                                        },
                                        &Place {
                                            local: *dst,
                                            projections: leaf.projections.clone(),
                                        },
                                        *exact,
                                        !*exact,
                                    );
                                    self.edge(
                                        state,
                                        &Place {
                                            local: args[input.index],
                                            projections: input.projections.clone(),
                                        },
                                        &Place {
                                            local: *dst,
                                            projections: leaf.projections.clone(),
                                        },
                                        line,
                                    )?;
                                }
                            }
                        }
                    }
                    Some(ReturnSource::Parameter { index, projections }) => {
                        let source = Place {
                            local: args[*index],
                            projections: projections.clone(),
                        };
                        let ty = place_type(types, function, &source).unwrap();
                        let inputs = self.paths(types, ty);
                        for (output, _) in self.paths(types, function.locals[*dst].ty).iter() {
                            for (input, _) in inputs.iter() {
                                self.views.transfer(
                                    &append(&source, input),
                                    &Place {
                                        local: *dst,
                                        projections: output.clone(),
                                    },
                                    false,
                                    true,
                                );
                                self.edge(
                                    state,
                                    &append(&source, input),
                                    &Place {
                                        local: *dst,
                                        projections: output.clone(),
                                    },
                                    line,
                                )?;
                            }
                        }
                    }
                    _ => {}
                }
                Some(*dst)
            }
            Op::Intrinsic { dst, name, args } => {
                if name == "replace" || name == "clone" {
                    let place = Place {
                        local: args[0],
                        projections: vec![Projection::Deref],
                    };
                    self.transfer(state, types, function, &place, &Place::local(*dst), line)?;
                    if name == "replace" {
                        self.transfer(
                            state,
                            types,
                            function,
                            &Place::local(args[1]),
                            &place,
                            line,
                        )?;
                        if !self.paths(types, function.locals[*dst].ty).is_empty() {
                            self.changed(state, &place, line)?;
                        }
                    }
                } else if name == "str_as_bytes" || name == "str_from_utf8" {
                    let to = Place {
                        local: *dst,
                        projections: if name == "str_from_utf8" {
                            vec![Projection::VariantField(0, 0)]
                        } else {
                            Vec::new()
                        },
                    };
                    self.edge(state, &Place::local(args[0]), &to, line)?;
                }
                Some(*dst)
            }
            Op::Text { dst, .. } => Some(*dst),
            _ => None,
        };
        if let Some(dst) = dst {
            self.changed.push(Place::local(dst));
        }
        Ok(())
    }
    fn after(&mut self, state: &State) -> Result<(), Diagnostic> {
        let mut roots: BTreeSet<usize> = BTreeSet::new();
        for place in &self.changed {
            roots.insert(place.local);
        }
        for root in roots {
            let annotations = self.indirect.get(&root).cloned();
            if let Some(annotations) = annotations {
                for annotation in annotations {
                    if state.live.at(annotation.place.local) {
                        self.seed(state, &annotation)?;
                    }
                }
            }
        }
        Ok(())
    }
    pub(in super::super) fn finish(
        mut self,
        scope: &Scope,
        types: &[Type],
        function: &Function,
    ) -> Result<Requirements, Diagnostic> {
        self.views.check(scope, types, function)?;
        let mut pending: Vec<(Origin, Region, (usize, usize))> = Vec::new();
        for (node, regions) in &self.demands {
            for (region, site) in regions {
                pending.push((node.clone(), region.clone(), *site));
            }
        }
        while let Some((node, region, site)) = pending.pop() {
            if let Some(line) = self.blocked.get(&node) {
                return Err(error(
                    *line,
                    "E_UNSUPPORTED",
                    "named local regions need an exact descriptor storage contract",
                ));
            }
            let mut outlives = true;
            if let Some(declared) = self.declared.get(&node) {
                for prior in declared {
                    if !scope.outlives(prior, &region) {
                        outlives = false;
                        break;
                    }
                }
            }
            if !outlives {
                let mut problem = error(
                    site.0,
                    "E_BORROW",
                    &format!(
                        "declared local reference region does not outlive {}",
                        region.name
                    ),
                );
                problem.column = site.1;
                return Err(problem);
            }
            if let Some(sources) = self.edges.get(&node) {
                for source in sources {
                    let required = get_or_default(&mut self.demands, source.clone());
                    if !required.contains_key(&region) {
                        required.insert(region.clone(), site);
                        pending.push((source.clone(), region.clone(), site));
                    }
                }
            }
        }
        let mut count = 0usize;
        let mut values: BTreeMap<Root, BTreeMap<Vec<Projection>, Vec<Constraint>>> =
            BTreeMap::new();
        for (node, regions) in self.demands {
            let mut constraints: Vec<Constraint> = Vec::new();
            for (region, created) in regions {
                let id = count;
                count += 1;
                constraints.push(Constraint {
                    id,
                    region,
                    created,
                });
            }
            get_or_default(&mut values, node.root).insert(node.projections, constraints);
        }
        Ok(Requirements {
            values,
            invariant: self.invariant,
        })
    }
}

pub(in super::super) struct Constraint {
    id: usize,
    region: Region,
    created: (usize, usize),
}
pub(in super::super) struct Requirements {
    values: BTreeMap<Root, BTreeMap<Vec<Projection>, Vec<Constraint>>>,
    invariant: BTreeMap<Origin, BTreeSet<Region>>,
}
fn parameter_place(function: &Function, types: &[Type], node: &Origin) -> Option<Place> {
    match node.root {
        Root::Local(local) if local < function.params => Some(Place {
            local,
            projections: node.projections.clone(),
        }),
        Root::Parameter(local) => {
            let mut projections = Vec::new();
            if matches!(types[function.locals[local].ty].kind, TypeKind::Ref { .. }) {
                projections.push(Projection::Deref);
            }
            projections.extend_from_slice(&node.projections);
            let place = Place { local, projections };
            if matches!(
                types[place_type(types, function, &place)?].kind,
                TypeKind::Ref { .. }
            ) {
                Some(place)
            } else {
                None
            }
        }
        _ => None,
    }
}
struct RetentionContext<'a> {
    scope: &'a Scope,
    inputs: &'a InputRegions,
    instruction: usize,
    entry: bool,
}

impl Requirements {
    pub(in super::super) fn capture_contract(
        &self,
        scope: &mut Scope,
        types: &[Type],
        function: &Function,
    ) -> Vec<Annotation> {
        let Some(environment) = &scope.environment else {
            return Vec::new();
        };
        let prefix = environment.prefix.clone();
        let mut contract = Vec::new();
        for (root, paths) in &self.values {
            for (projections, requirements) in paths {
                let Some(place) = parameter_place(
                    function,
                    types,
                    &Origin {
                        root: root.clone(),
                        projections: projections.clone(),
                    },
                ) else {
                    continue;
                };
                if place.local != 0
                    || !place.projections.starts_with(&prefix)
                    || !matches!(
                        place.projections.get(prefix.len()),
                        Some(Projection::Field(_))
                    )
                {
                    continue;
                }
                for requirement in requirements {
                    get_or_default(
                        &mut scope.guarantees,
                        InputLeaf {
                            index: 0,
                            projections: place.projections.clone(),
                        },
                    )
                    .insert(requirement.region.clone());
                    contract.push(Annotation {
                        place: Place {
                            local: 0,
                            projections: place.projections[prefix.len()..].to_vec(),
                        },
                        region: requirement.region.clone(),
                        created: requirement.created,
                        invariant: self
                            .invariant
                            .get(&Origin {
                                root: root.clone(),
                                projections: projections.clone(),
                            })
                            .is_some_and(|regions| regions.contains(&requirement.region)),
                    });
                }
            }
        }
        contract
    }
    fn retain(
        &self,
        context: &RetentionContext<'_>,
        state: &mut State,
        loan: Loan,
        requirements: &[Constraint],
        preserved: Vec<Place>,
    ) -> Result<(), Diagnostic> {
        if !loan_active(state, &loan) {
            return Ok(());
        }
        let mut retained = Retained::default();
        for requirement in requirements {
            if !loan.sources.iter().all(|source| {
                context
                    .inputs
                    .proves(source, &requirement.region, context.scope)
            }) {
                let (line, column) = state.site.unwrap_or(requirement.created);
                let mut problem = error(
                    line,
                    "E_BORROW",
                    &format!(
                        "reference origin does not outlive named local region {} (annotation at {}:{})",
                        requirement.region.name, requirement.created.0, requirement.created.1
                    ),
                );
                problem.column = column;
                return Err(problem);
            }
            retained.values.push((
                EffectId {
                    instruction: context.instruction,
                    requirement: requirement.id,
                    kind: if context.entry {
                        EffectKind::LocalInput
                    } else {
                        EffectKind::Local
                    },
                },
                Effect {
                    region: requirement.region.clone(),
                    sources: loan.sources.clone(),
                    mutable: loan.mutable,
                    created: state.site.unwrap_or(requirement.created),
                },
                InputLeaf {
                    index: loan.holder.local,
                    projections: loan.holder.projections.clone(),
                },
                preserved.clone(),
            ));
        }
        retained.install(state);
        Ok(())
    }
    fn entry(
        &self,
        scope: &Scope,
        inputs: &InputRegions,
        state: &mut State,
        types: &[Type],
        function: &Function,
    ) -> Result<(), Diagnostic> {
        for (root, paths) in &self.values {
            for (projections, requirements) in paths {
                let Some(place) = parameter_place(
                    function,
                    types,
                    &Origin {
                        root: root.clone(),
                        projections: projections.clone(),
                    },
                ) else {
                    continue;
                };
                let value = carried_provenance(state, types, function, &place, &place, 1)?;
                for loan in value.loans {
                    let holders = stored_holders(state, &place);
                    self.retain(
                        &RetentionContext {
                            scope,
                            inputs,
                            instruction: 0,
                            entry: true,
                        },
                        state,
                        loan,
                        requirements,
                        holders,
                    )?;
                }
            }
        }
        Ok(())
    }
    fn after(
        &self,
        scope: &Scope,
        inputs: &InputRegions,
        state: &mut State,
        changed: &[Place],
        index: usize,
    ) -> Result<(), Diagnostic> {
        let mut loans: BTreeMap<Place, Loan> = BTreeMap::new();
        for place in changed {
            let Some(paths) = self.values.get(&Root::Local(place.local)) else {
                continue;
            };
            for holder in state.loans.for_holder(place.local) {
                let loan = state.loans.get(&holder).unwrap();
                if within(&loan.holder, place) && paths.contains_key(&loan.holder.projections) {
                    loans.insert(loan.holder.clone(), loan.clone());
                }
            }
        }
        for (holder, loan) in loans {
            let requirements = self
                .values
                .get(&Root::Local(holder.local))
                .expect("declared local requirements")
                .get(&holder.projections)
                .expect("declared projection requirements");
            self.retain(
                &RetentionContext {
                    scope,
                    inputs,
                    instruction: index,
                    entry: false,
                },
                state,
                loan,
                requirements,
                vec![holder],
            )?;
        }
        Ok(())
    }
}

pub(in super::super) enum LocalPass<'a> {
    Observe(&'a mut Observation),
    Verify {
        requirements: &'a Requirements,
        changed: Vec<Place>,
    },
}
impl LocalPass<'_> {
    pub(in super::super) fn entry(
        &self,
        scope: Option<&Scope>,
        inputs: &InputRegions,
        state: &mut State,
        types: &[Type],
        function: &Function,
    ) -> Result<(), Diagnostic> {
        if let Self::Verify { requirements, .. } = self {
            requirements.entry(scope.unwrap(), inputs, state, types, function)?;
        }
        Ok(())
    }
    pub(in super::super) fn before(
        &mut self,
        program: &Program,
        function: &Function,
        state: &State,
        index: usize,
    ) -> Result<(), Diagnostic> {
        match self {
            Self::Observe(observation) => observation.before(program, function, state, index),
            Self::Verify { changed, .. } => {
                changed.clear();
                let operation = &function.body[index].op;
                let destination = match operation {
                    Op::Read { dst, .. }
                    | Op::Borrow { dst, .. }
                    | Op::SliceBorrow { dst, .. }
                    | Op::ViewRange { dst, .. }
                    | Op::Struct { dst, .. }
                    | Op::Variant { dst, .. }
                    | Op::Call { dst, .. }
                    | Op::Text { dst, .. } => Some(*dst),
                    Op::Intrinsic { dst, name, args } => {
                        if name == "replace" && state.views[function.locals[*dst].ty] {
                            for target in storage(
                                state,
                                &Place {
                                    local: args[0],
                                    projections: vec![Projection::Deref],
                                },
                                function.body[index].line,
                            )? {
                                if let Root::Local(local) = target.root {
                                    changed.push(Place {
                                        local,
                                        projections: target.projections,
                                    });
                                }
                            }
                        }
                        Some(*dst)
                    }
                    Op::Assign { place, .. } => {
                        if !state.views[place_type(&program.types, function, place).unwrap()] {
                            return Ok(());
                        }
                        for target in storage(state, place, function.body[index].line)? {
                            if let Root::Local(local) = target.root {
                                changed.push(Place {
                                    local,
                                    projections: target.projections,
                                });
                            }
                        }
                        None
                    }
                    _ => None,
                };
                if let Some(dst) = destination {
                    changed.push(Place::local(dst));
                }
                Ok(())
            }
        }
    }
    pub(in super::super) fn after(
        &mut self,
        scope: Option<&Scope>,
        inputs: &InputRegions,
        state: &mut State,
        index: usize,
    ) -> Result<(), Diagnostic> {
        match self {
            Self::Observe(observation) => observation.after(state),
            Self::Verify {
                requirements,
                changed,
            } => requirements.after(scope.unwrap(), inputs, state, changed, index),
        }
    }
}
