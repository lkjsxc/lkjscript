//! A typed view can restrict access without shortening the original storage.
//! These routes are source obligations, never executable provenance authority.
use super::*;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Route {
    from: Place,
    descendants: bool,
    bounded: bool,
}
#[derive(Default)]
pub(super) struct Views {
    declarations: BTreeMap<Place, BTreeMap<Region, (usize, usize)>>,
    demands: BTreeMap<Place, BTreeMap<Region, (usize, usize)>>,
    routes: BTreeMap<usize, BTreeMap<Vec<Projection>, BTreeMap<Route, usize>>>,
    records: Vec<(Place, Place)>,
}
enum Work {
    Visit(Place),
    Follow { source: Place, route: usize },
    Leave(usize),
}
impl Views {
    pub(super) fn new(scope: &Scope) -> Self {
        let mut result = Self::default();
        for (leaf, region) in &scope.parameters {
            result.declare(
                Place {
                    local: leaf.index,
                    projections: leaf.projections.clone(),
                },
                region.clone(),
                scope.parameter_sites.get(leaf).copied().unwrap_or((1, 1)),
            );
        }
        for annotation in &scope.annotations.bindings {
            result.declare(
                annotation.place.clone(),
                annotation.region.clone(),
                annotation.created,
            );
        }
        result
    }
    pub(super) fn demand(&mut self, place: Place, region: Region, created: (usize, usize)) {
        let regions = get_or_default(&mut self.demands, place);
        get_or_insert(regions, region, created);
    }
    pub(super) fn declare(&mut self, place: Place, region: Region, created: (usize, usize)) {
        get_or_default(&mut self.declarations, place.clone()).insert(region.clone(), created);
        self.demand(place, region, created);
    }
    pub(super) fn transfer(&mut self, from: &Place, to: &Place, descendants: bool, bounded: bool) {
        let paths = get_or_default(&mut self.routes, to.local);
        let routes = get_or_default(paths, to.projections.clone());
        let route = Route {
            from: from.clone(),
            descendants,
            bounded,
        };
        if !routes.contains_key(&route) {
            routes.insert(route, self.records.len());
            self.records.push((to.clone(), from.clone()));
        }
    }
    fn grows(&self, cycle: &[usize]) -> bool {
        let (mut required, mut produced) = self.records[cycle[0]].clone();
        for id in &cycle[1..] {
            let (destination, source) = &self.records[*id];
            if produced.local != destination.local {
                return false;
            }
            if produced.projections.starts_with(&destination.projections) {
                let mut next = source.clone();
                next.projections
                    .extend_from_slice(&produced.projections[destination.projections.len()..]);
                produced = next;
            } else if destination.projections.starts_with(&produced.projections) {
                required
                    .projections
                    .extend_from_slice(&destination.projections[produced.projections.len()..]);
                produced = source.clone();
            } else {
                return false;
            }
        }
        produced.local == required.local
            && produced.projections.len() > required.projections.len()
            && produced.projections.starts_with(&required.projections)
    }
    pub(super) fn check(
        &self,
        scope: &Scope,
        types: &[Type],
        function: &Function,
    ) -> Result<(), Diagnostic> {
        let mut seen: BTreeSet<(Place, Region)> = BTreeSet::new();
        for (place, requirements) in &self.demands {
            for (required, created) in requirements {
                let problem = |code, message: &str| {
                    let mut problem = error(created.0, code, message);
                    problem.column = created.1;
                    problem
                };
                let mut active = Vec::new();
                let mut positions = BTreeMap::<usize, Vec<usize>>::new();
                let mut pending = vec![Work::Visit(place.clone())];
                while let Some(work) = pending.pop() {
                    let place = match work {
                        Work::Leave(route) => {
                            positions.get_mut(&route).unwrap().pop();
                            active.pop();
                            continue;
                        }
                        Work::Follow { source, route } => {
                            if seen.contains(&(source.clone(), required.clone())) {
                                continue;
                            }
                            let prior: &mut Vec<usize> = get_or_default(&mut positions, route);
                            // Compose the actual compatible cycle. P -> P S
                            // can grow indefinitely; a longer intermediate path
                            // that fails a later prefix is merely a finite route.
                            if prior
                                .iter()
                                .any(|start: &usize| self.grows(&active[*start..]))
                            {
                                return Err(problem(
                                    "E_UNSUPPORTED",
                                    "recursive typed-view projection transport needs a region equation",
                                ));
                            }
                            prior.push(active.len());
                            active.push(route);
                            pending.push(Work::Leave(route));
                            pending.push(Work::Visit(source));
                            continue;
                        }
                        Work::Visit(place) => place,
                    };
                    if !seen.insert((place.clone(), required.clone())) {
                        continue;
                    }
                    if let Some(declared) = self.declarations.get(&place) {
                        if let Some((region, location)) = declared
                            .iter()
                            .find(|(region, _)| !scope.outlives(*region, required))
                        {
                            return Err(problem(
                                "E_BORROW",
                                &format!(
                                    "declared view region {} does not outlive {} (declaration at {}:{})",
                                    region.name, required.name, location.0, location.1,
                                ),
                            ));
                        }
                    }
                    let Some(routes) = self.routes.get(&place.local) else {
                        continue;
                    };
                    let mut prefix = Vec::new();
                    for cut in 0..=place.projections.len() {
                        if cut != 0 {
                            prefix.push(place.projections[cut - 1].clone());
                        }
                        let Some(edges) = routes.get(&prefix) else {
                            continue;
                        };
                        let suffix = &place.projections[cut..];
                        for (route, id) in edges {
                            if !suffix.is_empty() && !route.descendants {
                                if route.bounded {
                                    return Err(problem(
                                        "E_UNSUPPORTED",
                                        "bounded call results need a projected typed-view contract",
                                    ));
                                }
                                continue;
                            }
                            let mut source = route.from.clone();
                            source.projections.extend_from_slice(suffix);
                            if place_type(types, function, &source).is_none() {
                                return Err(problem(
                                    "E_UNSUPPORTED",
                                    "typed-view transport has no matching projected source",
                                ));
                            }
                            pending.push(Work::Follow { source, route: *id });
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
