//! Pattern binding mode depends on the subject's reference shape. Keep lexical
//! names available while a producer or associated projection determines it.
use super::*;
use std::collections::BTreeSet;

impl Infer<'_, '_> {
    pub(super) fn pattern_test_pending(&self, mut subject: Variable) -> bool {
        loop {
            match self.shape(subject) {
                Some(Shape::Ref(_, inner)) => subject = inner,
                None => return self.definition.nodes[self.definition.root(subject)].producers != 0,
                Some(Shape::Rigid(_) | Shape::Projected(_, _) | Shape::Source(_)) => return true,
                _ => return false,
            }
        }
    }

    pub(super) fn pattern_pending(&self, pattern: a::PatternId, mut subject: Variable) -> bool {
        if !matches!(
            self.source.patterns[pattern].kind,
            a::Pattern::Tuple(_)
                | a::Pattern::Variant { .. }
                | a::Pattern::Struct { .. }
                | a::Pattern::Path(_)
        ) {
            return false;
        }
        loop {
            match self.shape(subject) {
                Some(Shape::Ref(_, inner)) => subject = inner,
                None | Some(Shape::Rigid(_) | Shape::Projected(_, _) | Shape::Source(_)) => {
                    return true;
                }
                _ => return false,
            }
        }
    }

    pub(super) fn defer_pattern(
        &mut self,
        pattern: a::PatternId,
        subject: Variable,
        mode: Option<bool>,
    ) {
        let mut names = Vec::new();
        let mut seen = BTreeSet::new();
        let mut pending = vec![pattern];
        while let Some(pattern) = pending.pop() {
            match self.source.patterns[pattern].kind.clone() {
                a::Pattern::Bind { name, .. } => {
                    if seen.insert(name.clone()) {
                        let value = self.operation_type();
                        names.push((name.clone(), value));
                        self.bindings.push((name, value));
                    }
                }
                a::Pattern::Tuple(children) | a::Pattern::Or(children) => {
                    pending.extend(children.into_iter().rev())
                }
                a::Pattern::Variant { fields, .. } => pending.extend(fields.into_iter().rev()),
                a::Pattern::Struct { fields, .. } => {
                    pending.extend(fields.into_iter().rev().map(|(_, pattern)| pattern))
                }
                a::Pattern::Ref { inner, .. } => pending.push(inner),
                _ => {}
            }
        }
        self.obligation(Obligation::Pattern {
            pattern,
            subject,
            mode,
            names,
        });
    }

    pub(super) fn solve_pattern(
        &mut self,
        id: usize,
        pattern: a::PatternId,
        subject: Variable,
        mode: Option<bool>,
        names: Vec<(String, Variable)>,
    ) -> Result<bool> {
        if self.pattern_pending(pattern, subject) {
            self.watch_shape(subject, id);
            return Ok(false);
        }
        let scope = self.bindings.len();
        self.bind_pattern_mode(pattern, subject, mode)?;
        let actual: BTreeMap<_, _> = self.bindings.entries[scope..].iter().cloned().collect();
        self.bindings.truncate(scope);
        let span = self.source.patterns[pattern].span;
        for (name, placeholder) in names {
            let value = actual
                .get(&name)
                .copied()
                .ok_or_else(|| err(span, "E_TYPE", "pattern binding names differ"))?;
            self.unify(placeholder, value, span)?;
            self.finish_operation(placeholder);
        }
        Ok(true)
    }
}
