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
        let empty: Vec<usize> = Vec::new();
        while let Some((id, finish)) = work.pop() {
            if self.ids.contains_key(&id) {
                continue;
            }
            let children = match &terms[id] {
                Term::Named(_, children) | Term::DeclaredTrait(_, children) => children.as_slice(),
                _ => empty.as_slice(),
            };
            if !finish {
                work.push((id, true));
                let mut index = children.len();
                while index > 0 {
                    index -= 1;
                    work.push((children[index], false));
                }
                continue;
            }
            let mut child_ids = Vec::new();
            for child in children {
                child_ids.push(*self.ids.get(child).unwrap());
            }
            let key = match &terms[id] {
                Term::Named(name, _) => Term::Named(name.clone(), child_ids),
                Term::DeclaredTrait(declaration, _) => Term::DeclaredTrait(*declaration, child_ids),
                leaf => leaf.clone(),
            };
            let canonical = match self.terms.get(&key) {
                Some(previous) => *previous,
                None => {
                    self.terms.insert(key, id);
                    id
                }
            };
            self.ids.insert(id, canonical);
        }
        *self.ids.get(&first).unwrap()
    }
    fn goal(&mut self, terms: &[Term], goal: Goal) -> Goal {
        let (ty, bound) = goal;
        (self.term(terms, ty), self.term(terms, bound))
    }
}

/// Match only implementation variables. Query variables cannot be unified with
/// a concrete type merely to make an implementation appear applicable.
fn match_terms(
    terms: &[Term],
    pairs: Vec<Goal>,
    flexible: &BTreeSet<usize>,
    mut substitutions: BTreeMap<usize, usize>,
) -> Option<BTreeMap<usize, usize>> {
    let mut pending = pairs;
    while let Some((pattern, actual)) = pending.pop() {
        if let Term::Variable(variable) = terms[pattern] {
            if flexible.contains(&variable) {
                let previous = substitutions.get(&variable).copied();
                if let Some(previous) = previous {
                    if !equivalent(terms, previous, actual) {
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
                for index in 0..ac.len() {
                    pending.push((ac[index], bc[index]));
                }
            }
            (Term::DeclaredTrait(a, ac), Term::DeclaredTrait(b, bc))
                if a == b && ac.len() == bc.len() =>
            {
                for index in 0..ac.len() {
                    pending.push((ac[index], bc[index]));
                }
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
    goal: Goal,
    known: &[Goal],
) -> Result<Option<Vec<Goal>>, Diagnostic> {
    let (ty, bound) = goal;
    let declaration = match &terms[bound] {
        Term::DeclaredTrait(declaration, args) if args.is_empty() => *declaration,
        _ => return Ok(None),
    };
    let Some(candidates) = index.by_trait.get(&TraitIdentity::Declared(declaration)) else {
        return Ok(None);
    };
    let head = term_head(terms, ty);
    let mut selected = None;
    for candidate in candidates.matching(optional_name(&head)) {
        let implementation = &source.impls[candidate];
        let Some(trait_ty) = implementation.trait_ty else {
            continue;
        };
        // All existing variable identities are below the arena's next index.
        let mut next = terms.len();
        let environment = variables(source, &implementation.generics, terms, &mut next);
        let mut flexible = BTreeSet::new();
        for id in environment.values() {
            if let Term::Variable(variable) = terms[*id] {
                flexible.insert(variable);
            }
        }
        let target = signature_type(source, implementation.target, &environment, terms)?;
        let implemented = signature_type(source, trait_ty, &environment, terms)?;
        let Some(mut substitutions) = match_terms(
            terms,
            vec![(target, ty), (implemented, bound)],
            &flexible,
            BTreeMap::new(),
        ) else {
            continue;
        };
        // Callable output equations may determine an implementation parameter
        // absent from its nominal target. Only implementation variables are
        // flexible; the caller's type/region variables remain rigid.
        loop {
            let before = substitutions.len();
            let mut current = BTreeMap::new();
            for (name, id) in &environment {
                let value = match terms[*id] {
                    Term::Variable(variable) => {
                        substitutions.get(&variable).copied().unwrap_or(*id)
                    }
                    _ => *id,
                };
                current.insert(name.clone(), value);
            }
            for parameter in &implementation.generics {
                if parameter.lifetime {
                    continue;
                }
                let value = *current.get(&parameter.name).unwrap();
                if matches!(terms[value], Term::Variable(variable) if flexible.contains(&variable))
                {
                    continue;
                }
                for constraint in &parameter.bounds {
                    if !matches!(&source.types[*constraint].kind, Type::Path(path)
                        if path.declaration.is_none() && matches!(path.name.as_str(), "Fn" | "FnMut" | "FnOnce"))
                    {
                        continue;
                    }
                    let pattern = signature_type(source, *constraint, &current, terms)?;
                    let mut found: Option<BTreeMap<usize, usize>> = None;
                    let mut ambiguous = false;
                    for (known_ty, known_bound) in known {
                        if !equivalent(terms, value, *known_ty) {
                            continue;
                        }
                        let Some(candidate) = match_terms(
                            terms,
                            vec![(pattern, *known_bound)],
                            &flexible,
                            substitutions.clone(),
                        ) else {
                            continue;
                        };
                        if found.as_ref().is_some_and(|prior| {
                            prior.len() != candidate.len()
                                || prior.iter().any(|(variable, previous)| {
                                    candidate
                                        .get(variable)
                                        .is_none_or(|next| !equivalent(terms, *previous, *next))
                                })
                        }) {
                            ambiguous = true;
                            break;
                        }
                        found = Some(candidate);
                    }
                    if !ambiguous {
                        if let Some(found) = found {
                            substitutions = found;
                        }
                    }
                }
            }
            if substitutions.len() == before {
                break;
            }
        }
        let mut resolved = BTreeMap::new();
        let mut complete = true;
        for (name, id) in environment {
            let value = match terms[id] {
                Term::Variable(variable) => substitutions.get(&variable).copied(),
                _ => None,
            };
            if let Some(value) = value {
                resolved.insert(name, value);
            } else {
                complete = false;
                break;
            }
        }
        if !complete {
            continue;
        }
        let mut environment = resolved;
        environment.insert("Self".to_string(), ty);
        let mut children = Vec::new();
        for parameter in &implementation.generics {
            if parameter.lifetime {
                continue;
            }
            let value = *environment.get(&parameter.name).unwrap();
            for constraint in &parameter.bounds {
                children.push((
                    value,
                    signature_type(source, *constraint, &environment, terms)?,
                ));
            }
        }
        for constraint in &source.traits[declaration].bounds {
            children.push((
                ty,
                signature_type(source, *constraint, &environment, terms)?,
            ));
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
    let mut pending: Vec<(Goal, Option<Vec<Goal>>)> = vec![(first, None)];
    while let Some((goal, children)) = pending.pop() {
        if let Some(children) = children {
            let unfinished = state.get(&goal).copied() != Some(1u8);
            if unfinished {
                let mut valid = true;
                for child in &children {
                    if state.get(child).copied() != Some(2u8) {
                        valid = false;
                        break;
                    }
                }
                state.insert(goal, if valid { 2u8 } else { 1u8 });
            }
            continue;
        }
        let previous = state.get(&goal).copied();
        if let Some(previous) = previous {
            if previous == 0 {
                state.insert(goal, 1);
            }
            continue;
        }
        if symbolic_bound(terms, goal.0, goal.1, known) {
            state.insert(goal, 2);
            continue;
        }
        let Some(children) = prerequisites(source, index, terms, goal, known)? else {
            state.insert(goal, 1);
            continue;
        };
        let mut canonical_children: Vec<Goal> = Vec::new();
        for child in children {
            canonical_children.push(canonical.goal(terms, child));
        }
        let mut children = canonical_children;
        state.insert(goal, 0);
        pending.push((goal, Some(children.clone())));
        while let Some(child) = children.pop() {
            pending.push((child, None));
        }
    }
    Ok(state.get(&first).copied() == Some(2u8))
}
