//! Conditional associated-type equations, indexed by the declaring trait.
//!
//! A unique structural implementation can determine a type without proving that
//! the implementation is eligible. These equations never become trait evidence:
//! ordinary definition checking and concrete selection still prove every bound.
use super::*;
use std::collections::BTreeSet;

pub(super) enum Match {
    No,
    Deferred,
    Yes(BTreeMap<String, Variable>),
}

impl Infer<'_, '_> {
    pub(super) fn parameter_equations(
        &mut self,
        parameters: &[a::GenericParam],
        environment: &mut BTreeMap<String, Variable>,
        rigid: bool,
    ) -> Result<()> {
        // Establish projected names before callable signatures use them. A
        // declaration's bounds, not the spelling `Item`, choose its identity.
        for parameter in parameters {
            let receiver = match environment.get(&parameter.name) {
                Some(receiver) => Some(*receiver),
                None => None,
            };
            if let Some(receiver) = receiver {
                for bound in &parameter.bounds {
                    self.trait_equations(receiver, &parameter.name, *bound, environment, rigid)?;
                }
            }
        }
        for parameter in parameters {
            let receiver = match environment.get(&parameter.name) {
                Some(receiver) => Some(*receiver),
                None => None,
            };
            if let Some(receiver) = receiver {
                for bound in &parameter.bounds {
                    self.callable_bound(receiver, *bound, environment)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn trait_equations(
        &mut self,
        receiver: Variable,
        alias: &str,
        bound: a::TypeId,
        environment: &mut BTreeMap<String, Variable>,
        rigid: bool,
    ) -> Result<()> {
        let mut pending = vec![bound];
        let mut seen = BTreeSet::<Variable>::new();
        while let Some(bound) = pending.pop() {
            let declaration = match &self.source.types[bound].kind {
                a::Type::Path(path) if path.args.is_empty() => path.declaration,
                _ => None,
            };
            let Some(declaration) = declaration else {
                continue;
            };
            let a::Item::Trait(index) = self.source.declarations[declaration].item else {
                continue;
            };
            // Parameterized associated families need their argument identity in
            // the key. Leave that existing boundary unresolved rather than
            // conflating two applications of the same trait.
            if !seen.insert(declaration) {
                continue;
            }
            let definition = self.source.traits[index].clone();
            for bound in &definition.bounds {
                pending.push(*bound);
            }
            for associated in definition.associated {
                let name = format!("{alias}::{}", associated.name);
                let root = self.root(receiver);
                let key = (declaration, associated.name.clone());
                let previous = match self.projections[root].get(&key) {
                    Some(previous) => Some(*previous),
                    None => None,
                };
                let value = if let Some(previous) = previous {
                    previous
                } else {
                    let result = if rigid {
                        self.shaped(Shape::Projected(declaration, name.clone()))
                    } else {
                        self.operation_type()
                    };
                    self.projections[root].insert(key, result);
                    if !rigid {
                        self.obligation(Obligation::Projection {
                            receiver,
                            declaration,
                            member: associated.name,
                            result,
                            span: associated.span,
                        });
                    }
                    result
                };
                // Ambiguous same-spelled associated members do not establish an
                // equation. The declaration checker diagnoses that source path.
                let previous = match environment.get(&name) {
                    Some(previous) => Some(*previous),
                    None => None,
                };
                if let Some(previous) = previous {
                    if self.definition.root(previous) != self.definition.root(value) {
                        environment.insert(name, self.blocked());
                    }
                } else {
                    environment.insert(name, value);
                }
            }
        }
        Ok(())
    }

    pub(super) fn shape_head(&self, receiver: Variable) -> Option<String> {
        match self.shape(receiver) {
            Some(Shape::Nominal(declaration, _)) => {
                Some(self.source.declarations[declaration].name.clone())
            }
            Some(Shape::Vec(_)) => Some("Vec".to_string()),
            Some(Shape::Box(_)) => Some("Box".to_string()),
            Some(Shape::Atom(name)) => Some(name),
            _ => None,
        }
    }

    pub(super) fn watch_shape(&mut self, variable: Variable, obligation: usize) {
        let mut pending = vec![variable];
        let mut seen = BTreeSet::<Variable>::new();
        while let Some(variable) = pending.pop() {
            let root = self.root(variable);
            if !seen.insert(root) {
                continue;
            }
            self.watch(root, obligation);
            if let Some(shape) = self.shape(root) {
                for child in shape.children() {
                    pending.push(child);
                }
            }
        }
    }

    /// Only implementation variables are flexible. Unknown caller positions
    /// cannot be fixed to a concrete pattern merely to select its header.
    pub(super) fn implementation_environment(
        &mut self,
        index: usize,
        receiver: Variable,
    ) -> Result<Match> {
        let implementation = self.source.impls[index].clone();
        let mut environment = BTreeMap::new();
        for parameter in &implementation.generics {
            if !parameter.lifetime {
                environment.insert(parameter.name.clone(), self.unknown());
            }
        }
        let mut flexible = BTreeSet::<Variable>::new();
        for value in environment.values() {
            flexible.insert(*value);
        }
        let target = self.source_type(implementation.target, &environment)?;
        let mut substitutions: BTreeMap<Variable, Variable> = BTreeMap::new();
        let mut pending = vec![(target, receiver)];
        let mut deferred = false;
        while let Some((pattern, actual)) = pending.pop() {
            let pattern = self.definition.root(pattern);
            let actual = self.definition.root(actual);
            if flexible.contains(&pattern) {
                let previous = match substitutions.get(&pattern) {
                    Some(previous) => Some(*previous),
                    None => None,
                };
                if let Some(previous) = previous {
                    pending.push((previous, actual));
                } else {
                    substitutions.insert(pattern, actual);
                }
                continue;
            }
            if pattern == actual {
                continue;
            }
            match (self.shape(pattern), self.shape(actual)) {
                (None, _) | (_, None) => deferred = true,
                (Some(Shape::Vec(a)), Some(Shape::Vec(b)))
                | (Some(Shape::Box(a)), Some(Shape::Box(b)))
                | (Some(Shape::Slice(a)), Some(Shape::Slice(b))) => pending.push((a, b)),
                (Some(Shape::Ref(am, a)), Some(Shape::Ref(bm, b))) if am == bm => {
                    pending.push((a, b))
                }
                (Some(Shape::Array(a, an)), Some(Shape::Array(b, bn))) if an == bn => {
                    pending.push((a, b))
                }
                (Some(Shape::Tuple(a)), Some(Shape::Tuple(b))) if a.len() == b.len() => {
                    for index in 0..a.len() {
                        pending.push((a[index], b[index]));
                    }
                }
                (Some(Shape::Nominal(ad, a)), Some(Shape::Nominal(bd, b)))
                    if ad == bd && a.len() == b.len() =>
                {
                    for index in 0..a.len() {
                        pending.push((a[index], b[index]));
                    }
                }
                (Some(a), Some(b)) => {
                    if a != b {
                        return Ok(Match::No);
                    }
                }
            }
        }
        if deferred {
            return Ok(Match::Deferred);
        }
        let mut resolved = BTreeMap::new();
        for (name, value) in environment {
            let actual = match substitutions.get(&value) {
                Some(actual) => *actual,
                None => value,
            };
            resolved.insert(name, actual);
        }
        resolved.insert("Self".to_string(), receiver);
        Ok(Match::Yes(resolved))
    }

    pub(super) fn solve_projection(
        &mut self,
        id: usize,
        receiver: Variable,
        declaration: a::DeclarationId,
        member: &str,
        result: Variable,
        span: a::Span,
    ) -> Result<bool> {
        let Some(shape) = self.shape(receiver) else {
            self.watch(receiver, id);
            return Ok(false);
        };
        if matches!(shape, Shape::Rigid(_)) {
            // Joining the receiver with a declared parameter has already joined
            // the corresponding projection slot; no concrete impl is guessed.
            return Ok(matches!(self.shape(result), Some(Shape::Projected(_, _))));
        }
        let a::Item::Trait(trait_index) = self.source.declarations[declaration].item else {
            return Ok(false);
        };
        let head = self.shape_head(receiver);
        let head_view = match &head {
            Some(head) => Some(head.as_str()),
            None => None,
        };
        let candidates = self
            .implementations
            .trait_candidates_by_head(trait_index, head_view);
        let mut selected = None;
        for candidate in candidates {
            match self.implementation_environment(candidate, receiver)? {
                Match::No => {}
                Match::Deferred => {
                    self.watch_shape(receiver, id);
                    return Ok(false);
                }
                Match::Yes(environment) => {
                    if selected.is_some() {
                        return Ok(false);
                    }
                    selected = Some((candidate, environment));
                }
            }
        }
        let Some((candidate, mut environment)) = selected else {
            return Ok(false);
        };
        let implementation = self.source.impls[candidate].clone();
        self.parameter_equations(&implementation.generics, &mut environment, false)?;
        if let Some(bound) = implementation.trait_ty {
            self.trait_equations(receiver, "Self", bound, &mut environment, false)?;
        }
        let mut associated = None;
        for item in &implementation.associated {
            if item.name == member {
                associated = item.value;
                break;
            }
        }
        if associated.is_none() {
            for item in &self.source.traits[trait_index].associated {
                if item.name == member {
                    associated = item.value;
                    break;
                }
            }
        }
        let Some(associated) = associated else {
            return Ok(false);
        };
        let value = self.source_type(associated, &environment)?;
        self.unify(result, value, span)?;
        Ok(true)
    }
}
