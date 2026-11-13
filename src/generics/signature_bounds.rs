//! Declared bare-trait proofs for symbolic implementation signatures.
//!
//! Implementation parameters are flexible patterns; the caller's parameters
//! remain rigid. Every selected implementation's bounds are proved in turn.
//! Parameterized bounds still require a declared assumption, matching the
//! existing concrete bare-bound solver's contract. Thus recursive prerequisites
//! inspect existing type subterms rather than manufacturing expanding goals.
use super::*;

type Goal = (usize, usize);

#[derive(Default)]
struct Canonical {
    terms: BTreeMap<Term, usize>,
    ids: BTreeMap<usize, usize>,
}
impl Canonical {
    fn term(&mut self, terms: &[Term], first: usize) -> usize {
        let mut work = vec![(first, false)];
        while let Some((id, finish)) = work.pop() {
            if self.ids.contains_key(&id) {
                continue;
            }
            let children = match &terms[id] {
                Term::Named(_, children) | Term::DeclaredTrait(_, children) => children.as_slice(),
                _ => &[],
            };
            if !finish {
                work.push((id, true));
                for child in children.iter().rev() {
                    work.push((*child, false));
                }
                continue;
            }
            let key = match &terms[id] {
                Term::Named(name, children) => Term::Named(
                    name.clone(),
                    children.iter().map(|child| self.ids[child]).collect(),
                ),
                Term::DeclaredTrait(declaration, children) => Term::DeclaredTrait(
                    *declaration,
                    children.iter().map(|child| self.ids[child]).collect(),
                ),
                leaf => leaf.clone(),
            };
            let canonical = *self.terms.entry(key).or_insert(id);
            self.ids.insert(id, canonical);
        }
        self.ids[&first]
    }
    fn goal(&mut self, terms: &[Term], (ty, bound): Goal) -> Goal {
        (self.term(terms, ty), self.term(terms, bound))
    }
}

/// Match only implementation variables. Query variables cannot be unified with
/// a concrete type merely to make an implementation appear applicable.
fn match_terms(
    terms: &[Term],
    pairs: Vec<Goal>,
    flexible: &BTreeSet<usize>,
) -> Option<BTreeMap<usize, usize>> {
    let mut pending = pairs;
    let mut substitutions = BTreeMap::new();
    while let Some((pattern, actual)) = pending.pop() {
        if let Term::Variable(variable) = terms[pattern] {
            if flexible.contains(&variable) {
                if let Some(previous) = substitutions.get(&variable) {
                    if !equivalent(terms, *previous, actual) {
                        return None;
                    }
                } else {
                    substitutions.insert(variable, actual);
                }
                continue;
            }
        }
        match (&terms[pattern], &terms[actual]) {
            (Term::Variable(a), Term::Variable(b)) if a == b => {}
            (Term::Named(a, ac), Term::Named(b, bc)) if a == b && ac.len() == bc.len() => {
                pending.extend(ac.iter().copied().zip(bc.iter().copied()));
            }
            (Term::DeclaredTrait(a, ac), Term::DeclaredTrait(b, bc))
                if a == b && ac.len() == bc.len() =>
            {
                pending.extend(ac.iter().copied().zip(bc.iter().copied()));
            }
            _ => return None,
        }
    }
    Some(substitutions)
}

fn prerequisites(
    source: &Program,
    index: &ImplementationIndex,
    terms: &mut Vec<Term>,
    (ty, bound): Goal,
) -> Result<Option<Vec<Goal>>, Diagnostic> {
    let declaration = match &terms[bound] {
        Term::DeclaredTrait(declaration, args) if args.is_empty() => *declaration,
        _ => return Ok(None),
    };
    let Some(candidates) = index.by_trait.get(&TraitIdentity::Declared(declaration)) else {
        return Ok(None);
    };
    let head = term_head(terms, ty);
    let mut selected = None;
    for candidate in candidates.matching(head.as_deref()) {
        let implementation = &source.impls[candidate];
        let Some(trait_ty) = implementation.trait_ty else {
            continue;
        };
        // All existing variable identities are below the arena's next index.
        let mut next = terms.len();
        let environment = variables(source, &implementation.generics, terms, &mut next);
        let flexible: BTreeSet<_> = environment
            .values()
            .filter_map(|id| match terms[*id] {
                Term::Variable(variable) => Some(variable),
                _ => None,
            })
            .collect();
        let target = signature_type(source, implementation.target, &environment, terms)?;
        let implemented = signature_type(source, trait_ty, &environment, terms)?;
        let Some(substitutions) =
            match_terms(terms, vec![(target, ty), (implemented, bound)], &flexible)
        else {
            continue;
        };
        let mut environment = environment
            .into_iter()
            .map(|(name, id)| match terms[id] {
                Term::Variable(variable) => substitutions
                    .get(&variable)
                    .copied()
                    .map(|value| (name, value)),
                _ => None,
            })
            .collect::<Option<BTreeMap<_, _>>>();
        let Some(environment) = environment.as_mut() else {
            continue;
        };
        environment.insert("Self".to_string(), ty);
        let mut children = Vec::new();
        for parameter in &implementation.generics {
            if parameter.lifetime {
                continue;
            }
            let value = environment[&parameter.name];
            for constraint in &parameter.bounds {
                children.push((
                    value,
                    signature_type(source, *constraint, environment, terms)?,
                ));
            }
        }
        for constraint in &source.traits[declaration].bounds {
            children.push((ty, signature_type(source, *constraint, environment, terms)?));
        }
        // Coherence normally ensures uniqueness; never choose arbitrarily if a
        // caller provides source that has not passed that global validation.
        if selected.is_some() {
            return Ok(None);
        }
        selected = Some(children);
    }
    Ok(selected)
}

pub(super) fn prove(
    source: &Program,
    index: &ImplementationIndex,
    terms: &mut Vec<Term>,
    ty: usize,
    bound: usize,
    known: &[Goal],
) -> Result<bool, Diagnostic> {
    if symbolic_bound(terms, ty, bound, known) {
        return Ok(true);
    }
    let mut canonical = Canonical::default();
    let first = canonical.goal(terms, (ty, bound));
    // 0 is active, 1 disproved, 2 proved. A user-implementation cycle is not
    // evidence; structural builtin proofs are already handled independently.
    let mut state = BTreeMap::<Goal, u8>::new();
    let mut pending = vec![(first, None::<Vec<Goal>>)];
    while let Some((goal, children)) = pending.pop() {
        if let Some(children) = children {
            if state.get(&goal) != Some(&1) {
                let valid = children.iter().all(|child| state.get(child) == Some(&2));
                state.insert(goal, if valid { 2 } else { 1 });
            }
            continue;
        }
        if let Some(previous) = state.get(&goal) {
            if *previous == 0 {
                state.insert(goal, 1);
            }
            continue;
        }
        if symbolic_bound(terms, goal.0, goal.1, known) {
            state.insert(goal, 2);
            continue;
        }
        let Some(children) = prerequisites(source, index, terms, goal)? else {
            state.insert(goal, 1);
            continue;
        };
        let children: Vec<_> = children
            .into_iter()
            .map(|child| canonical.goal(terms, child))
            .collect();
        state.insert(goal, 0);
        pending.push((goal, Some(children.clone())));
        pending.extend(children.into_iter().rev().map(|child| (child, None)));
    }
    Ok(state.get(&first) == Some(&2))
}
