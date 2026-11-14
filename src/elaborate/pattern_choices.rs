//! Pattern tests are pure. Cache their outcomes before guards, then enumerate
//! shared alternatives at runtime without copying their Cartesian product.
use super::patterns::{PatternGuardInput, PatternKind, PatternPlan};
use super::{Builder, LoopScope, Result, ir};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct PatternCache {
    local: usize,
    slots: BTreeMap<usize, usize>,
    possible: BTreeMap<usize, usize>,
}
#[derive(Clone, Copy)]
pub(super) struct Choice {
    pub(super) position: usize,
    active: usize,
    alternatives: usize,
}
pub(super) struct PatternChoices {
    pub(super) slots: BTreeMap<usize, Choice>,
    order: Vec<usize>,
}

impl Builder<'_, '_> {
    fn pattern_scalar(&mut self, ty: usize, value: i64, line: usize) -> usize {
        let dst = self.temp(ty);
        self.emit(ir::Op::Int { dst, value }, line);
        dst
    }
    fn pattern_assign_scalar(&mut self, local: usize, value: i64, line: usize) {
        let ty = self.ty(local);
        let src = self.pattern_scalar(ty, value, line);
        self.emit(
            ir::Op::Assign {
                place: ir::Place::local(local),
                src,
            },
            line,
        );
    }
    fn cache_place(&mut self, cache: &PatternCache, slot: usize, line: usize) -> ir::Place {
        let index = self.pattern_scalar(ir::U64, slot as i64, line);
        ir::Place {
            local: cache.local,
            projections: vec![ir::Projection::Index(index)],
        }
    }
    fn cache_test(&mut self, cache: &PatternCache, id: usize, value: usize, line: usize) {
        let place = self.cache_place(cache, *cache.slots.get(&id).unwrap(), line);
        self.emit(ir::Op::Assign { place, src: value }, line);
    }
    fn cached_test(&mut self, cache: &PatternCache, id: usize, line: usize) -> Result<usize> {
        let place = self.cache_place(cache, *cache.slots.get(&id).unwrap(), line);
        Ok(self.read(place, line)?.0)
    }
    fn cached_possible(&mut self, cache: &PatternCache, id: usize, line: usize) -> Result<usize> {
        let place = self.cache_place(cache, *cache.possible.get(&id).unwrap(), line);
        Ok(self.read(place, line)?.0)
    }
    fn fold_possible(
        &mut self,
        cache: &PatternCache,
        mut result: usize,
        children: &[usize],
        or: bool,
        line: usize,
    ) -> Result<usize> {
        for child in children {
            let right = self.cached_possible(cache, *child, line)?;
            let dst = self.temp(ir::BOOL);
            self.emit(
                ir::Op::Binary {
                    dst,
                    op: if or { "|" } else { "&" }.to_string(),
                    left: result,
                    right,
                },
                line,
            );
            result = dst;
        }
        Ok(result)
    }
    pub(super) fn prepare_pattern_cache(
        &mut self,
        plan: &PatternPlan,
        roots: &[usize],
        place: ir::Place,
        line: usize,
    ) -> Result<PatternCache> {
        let mut slots = BTreeMap::new();
        let mut pending = roots.to_vec();
        let mut visited = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            match &plan.nodes[id].kind {
                PatternKind::Variant { fields, .. } => {
                    let index = slots.len();
                    slots.insert(id, index);
                    for (_, child) in fields {
                        pending.push(*child);
                    }
                }
                PatternKind::Literal(_) | PatternKind::Range(_) => {
                    let index = slots.len();
                    slots.insert(id, index);
                }
                PatternKind::Product(fields) => {
                    for (_, child) in fields {
                        pending.push(*child);
                    }
                }
                PatternKind::Deref(inner) => pending.push(*inner),
                PatternKind::Or(alternatives) => pending.extend_from_slice(alternatives),
                _ => {}
            }
        }
        let mut possible = BTreeMap::new();
        for id in &visited {
            let position = slots.len() + possible.len();
            possible.insert(*id, position);
        }
        let length = slots.len() + possible.len();
        let ty = self.compiler.intern(
            String::new(),
            ir::TypeKind::Array {
                inner: ir::BOOL,
                len: length,
            },
        );
        let mut elements = Vec::new();
        for _ in 0..length {
            elements.push(self.pattern_scalar(ir::BOOL, 0, line));
        }
        let local = self.local(ty, "$pattern_tests", true);
        self.emit(
            ir::Op::Array {
                dst: local,
                elements,
            },
            line,
        );
        let cache = PatternCache {
            local,
            slots,
            possible,
        };
        let start = self.live.len();
        for root in roots {
            self.fill_pattern_cache(plan, *root, place.clone(), &cache)?;
        }
        // Children precede parents in the arena. Cache feasibility once so a
        // failed subtree never expands unrelated matching alternatives.
        for id in visited {
            let value = match &plan.nodes[id].kind {
                PatternKind::Any | PatternKind::Bind { .. } => {
                    self.pattern_scalar(ir::BOOL, 1, line)
                }
                PatternKind::Literal(_) | PatternKind::Range(_) => {
                    self.cached_test(&cache, id, line)?
                }
                PatternKind::Deref(inner) => self.cached_possible(&cache, *inner, line)?,
                PatternKind::Product(fields) | PatternKind::Variant { fields, .. } => {
                    let initial = if matches!(plan.nodes[id].kind, PatternKind::Variant { .. }) {
                        self.cached_test(&cache, id, line)?
                    } else {
                        self.pattern_scalar(ir::BOOL, 1, line)
                    };
                    let mut children = Vec::new();
                    for (_, child) in fields {
                        children.push(*child);
                    }
                    self.fold_possible(&cache, initial, &children, false, line)?
                }
                PatternKind::Or(alternatives) => {
                    let initial = self.pattern_scalar(ir::BOOL, 0, line);
                    self.fold_possible(&cache, initial, alternatives, true, line)?
                }
            };
            let destination = self.cache_place(&cache, *cache.possible.get(&id).unwrap(), line);
            self.emit(
                ir::Op::Assign {
                    place: destination,
                    src: value,
                },
                line,
            );
        }
        self.cleanup(start, None, line);
        Ok(cache)
    }
    fn fill_pattern_cache(
        &mut self,
        plan: &PatternPlan,
        id: usize,
        place: ir::Place,
        cache: &PatternCache,
    ) -> Result<()> {
        let node = &plan.nodes[id];
        let line = node.span.line;
        match &node.kind {
            PatternKind::Literal(_) | PatternKind::Range(_) => {
                let value = self.pattern_condition(plan, id, place)?;
                self.cache_test(cache, id, value, line);
            }
            PatternKind::Variant { tag, fields } => {
                let condition = self.temp(ir::BOOL);
                self.emit(
                    ir::Op::TagTest {
                        dst: condition,
                        place: place.clone(),
                        variant: *tag,
                    },
                    line,
                );
                let value = self.read(ir::Place::local(condition), line)?.0;
                self.cache_test(cache, id, value, line);
                self.emit(ir::Op::If { condition }, line);
                let before = self.snapshot();
                let start = self.live.len();
                for (projection, child) in fields {
                    let mut field = place.clone();
                    field.projections.push(projection.clone());
                    self.fill_pattern_cache(plan, *child, field, cache)?;
                }
                self.cleanup(start, None, line);
                self.emit(ir::Op::End, line);
                self.restore(&before);
            }
            PatternKind::Product(fields) => {
                for (projection, child) in fields {
                    let mut field = place.clone();
                    field.projections.push(projection.clone());
                    self.fill_pattern_cache(plan, *child, field, cache)?;
                }
            }
            PatternKind::Deref(inner) => {
                let mut field = place;
                field.projections.push(ir::Projection::Deref);
                self.fill_pattern_cache(plan, *inner, field, cache)?;
            }
            PatternKind::Or(alternatives) => {
                for alternative in alternatives {
                    self.fill_pattern_cache(plan, *alternative, place.clone(), cache)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub(super) fn pattern_choice_equals(
        &mut self,
        position: usize,
        index: usize,
        line: usize,
    ) -> Result<usize> {
        let left = self.read(ir::Place::local(position), line)?.0;
        let right = self.pattern_scalar(ir::U64, index as i64, line);
        let dst = self.temp(ir::BOOL);
        self.emit(
            ir::Op::Binary {
                dst,
                op: "==".to_string(),
                left,
                right,
            },
            line,
        );
        Ok(dst)
    }
    pub(super) fn cached_pattern_condition(
        &mut self,
        plan: &PatternPlan,
        id: usize,
        cache: &PatternCache,
        choices: Option<&PatternChoices>,
    ) -> Result<usize> {
        let node = &plan.nodes[id];
        let line = node.span.line;
        if choices.is_none() {
            return self.cached_possible(cache, id, line);
        }
        match &node.kind {
            PatternKind::Any | PatternKind::Bind { .. } => {
                Ok(self.pattern_scalar(ir::BOOL, 1, line))
            }
            PatternKind::Literal(_) | PatternKind::Range(_) => self.cached_test(cache, id, line),
            PatternKind::Deref(inner) => {
                self.cached_pattern_condition(plan, *inner, cache, choices)
            }
            PatternKind::Product(fields) => {
                self.cached_product_condition(plan, fields, cache, choices, line)
            }
            PatternKind::Variant { fields, .. } => {
                let condition = self.cached_test(cache, id, line)?;
                if fields.is_empty() {
                    return Ok(condition);
                }
                let result = self.local(ir::BOOL, "$pattern", true);
                self.pattern_assign_scalar(result, 0, line);
                self.emit(ir::Op::If { condition }, line);
                let before = self.snapshot();
                let start = self.live.len();
                let src = self.cached_product_condition(plan, fields, cache, choices, line)?;
                self.emit(
                    ir::Op::Assign {
                        place: ir::Place::local(result),
                        src,
                    },
                    line,
                );
                self.cleanup(start, None, line);
                self.emit(ir::Op::End, line);
                self.restore(&before);
                Ok(result)
            }
            PatternKind::Or(alternatives) => {
                if let Some(choice) = choices.and_then(|choices| choices.slots.get(&id)) {
                    self.pattern_assign_scalar(choice.active, 1, line);
                    let result = self.local(ir::BOOL, "$pattern", true);
                    self.pattern_assign_scalar(result, 0, line);
                    for (index, alternative) in alternatives.iter().enumerate() {
                        let left = self.pattern_choice_equals(choice.position, index, line)?;
                        let right = self.cached_possible(cache, *alternative, line)?;
                        let condition = self.temp(ir::BOOL);
                        self.emit(
                            ir::Op::Binary {
                                dst: condition,
                                op: "&".to_string(),
                                left,
                                right,
                            },
                            line,
                        );
                        self.emit(ir::Op::If { condition }, line);
                        let before = self.snapshot();
                        let start = self.live.len();
                        let src =
                            self.cached_pattern_condition(plan, *alternative, cache, choices)?;
                        self.emit(
                            ir::Op::Assign {
                                place: ir::Place::local(result),
                                src,
                            },
                            line,
                        );
                        self.cleanup(start, None, line);
                        self.emit(ir::Op::End, line);
                        self.restore(&before);
                    }
                    Ok(result)
                } else {
                    let mut result = self.pattern_scalar(ir::BOOL, 0, line);
                    for alternative in alternatives {
                        let right =
                            self.cached_pattern_condition(plan, *alternative, cache, None)?;
                        let dst = self.temp(ir::BOOL);
                        self.emit(
                            ir::Op::Binary {
                                dst,
                                op: "|".to_string(),
                                left: result,
                                right,
                            },
                            line,
                        );
                        result = dst;
                    }
                    Ok(result)
                }
            }
        }
    }
    fn cached_product_condition(
        &mut self,
        plan: &PatternPlan,
        fields: &[(ir::Projection, usize)],
        cache: &PatternCache,
        choices: Option<&PatternChoices>,
        line: usize,
    ) -> Result<usize> {
        let result = self.local(ir::BOOL, "$pattern", true);
        self.pattern_assign_scalar(result, 1, line);
        for (_, child) in fields {
            let condition = self.read(ir::Place::local(result), line)?.0;
            self.emit(ir::Op::If { condition }, line);
            let before = self.snapshot();
            let start = self.live.len();
            let src = self.cached_pattern_condition(plan, *child, cache, choices)?;
            self.emit(
                ir::Op::Assign {
                    place: ir::Place::local(result),
                    src,
                },
                line,
            );
            self.cleanup(start, None, line);
            self.emit(ir::Op::End, line);
            self.restore(&before);
        }
        Ok(result)
    }
    fn prepare_pattern_choices(
        &mut self,
        plan: &PatternPlan,
        root: usize,
        line: usize,
    ) -> PatternChoices {
        let mut choices = PatternChoices {
            slots: BTreeMap::new(),
            order: Vec::new(),
        };
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            match &plan.nodes[id].kind {
                PatternKind::Or(alternatives) => {
                    let position = self.local(ir::U64, "$alternative", true);
                    let active = self.local(ir::BOOL, "$alternative_active", true);
                    self.pattern_assign_scalar(position, 0, line);
                    self.pattern_assign_scalar(active, 0, line);
                    choices.slots.insert(
                        id,
                        Choice {
                            position,
                            active,
                            alternatives: alternatives.len(),
                        },
                    );
                    choices.order.push(id);
                    let mut index = alternatives.len();
                    while index != 0 {
                        index -= 1;
                        pending.push(alternatives[index]);
                    }
                }
                PatternKind::Product(fields) | PatternKind::Variant { fields, .. } => {
                    let mut index = fields.len();
                    while index != 0 {
                        index -= 1;
                        pending.push(fields[index].1);
                    }
                }
                PatternKind::Deref(inner) => pending.push(*inner),
                _ => {}
            }
        }
        choices
    }
    fn advance_pattern_choices(&mut self, choices: &PatternChoices, line: usize) -> Result<usize> {
        let carry = self.local(ir::BOOL, "$alternative_carry", true);
        self.pattern_assign_scalar(carry, 1, line);
        let mut index = choices.order.len();
        while index != 0 {
            index -= 1;
            let id = choices.order[index];
            let choice = *choices.slots.get(&id).unwrap();
            let left = self.read(ir::Place::local(carry), line)?.0;
            let right = self.read(ir::Place::local(choice.active), line)?.0;
            let condition = self.temp(ir::BOOL);
            self.emit(
                ir::Op::Binary {
                    dst: condition,
                    op: "&".to_string(),
                    left,
                    right,
                },
                line,
            );
            self.emit(ir::Op::If { condition }, line);
            let before = self.snapshot();
            let start = self.live.len();
            let left = self.read(ir::Place::local(choice.position), line)?.0;
            let right = self.pattern_scalar(ir::U64, 1, line);
            let next = self.temp(ir::U64);
            self.emit(
                ir::Op::Binary {
                    dst: next,
                    op: "+".to_string(),
                    left,
                    right,
                },
                line,
            );
            let left = self.read(ir::Place::local(next), line)?.0;
            let right = self.pattern_scalar(ir::U64, choice.alternatives as i64, line);
            let condition = self.temp(ir::BOOL);
            self.emit(
                ir::Op::Binary {
                    dst: condition,
                    op: "<".to_string(),
                    left,
                    right,
                },
                line,
            );
            self.emit(ir::Op::If { condition }, line);
            let selection = self.snapshot();
            self.emit(
                ir::Op::Assign {
                    place: ir::Place::local(choice.position),
                    src: next,
                },
                line,
            );
            self.pattern_assign_scalar(carry, 0, line);
            self.cleanup(start, None, line);
            let advanced = self.snapshot();
            self.emit(ir::Op::Else, line);
            self.restore(&selection);
            self.pattern_assign_scalar(choice.position, 0, line);
            self.cleanup(start, None, line);
            self.emit(ir::Op::End, line);
            self.merge_carriers(&advanced);
            self.emit(ir::Op::End, line);
            self.restore(&before);
        }
        Ok(carry)
    }
    pub(super) fn retry_pattern_guard(
        &mut self,
        plan: &PatternPlan,
        root: usize,
        place: ir::Place,
        guard: usize,
        cache: &PatternCache,
        tested: &[ir::Place],
    ) -> Result<(usize, PatternChoices)> {
        let line = plan.nodes[root].span.line;
        let choices = self.prepare_pattern_choices(plan, root, line);
        let found = self.local(ir::BOOL, "$guard_match", true);
        self.pattern_assign_scalar(found, 0, line);
        let possible = self.cached_possible(cache, root, line)?;
        self.emit(
            ir::Op::If {
                condition: possible,
            },
            line,
        );
        let skipped = self.snapshot();
        let start = self.live.len();
        let entry = self.snapshot();
        self.loops.push(LoopScope {
            synthetic: true,
            start,
            range: None,
            result: None,
            result_typed: true,
            result_expected: None,
            exits: Vec::new(),
        });
        self.emit(ir::Op::Loop, line);
        for choice in choices.slots.values() {
            self.pattern_assign_scalar(choice.active, 0, line);
        }
        let condition = self.cached_pattern_condition(plan, root, cache, Some(&choices))?;
        let accepted = self.pattern_guard_with_choices(
            plan,
            root,
            place,
            PatternGuardInput {
                expression: guard,
                condition,
                tested,
                choices: Some(&choices),
            },
        )?;
        self.emit(
            ir::Op::If {
                condition: accepted,
            },
            line,
        );
        let rejected = self.snapshot();
        self.pattern_assign_scalar(found, 1, line);
        self.cleanup(start, None, line);
        self.emit(ir::Op::Break, line);
        self.emit(ir::Op::End, line);
        self.restore(&rejected);
        let exhausted = self.advance_pattern_choices(&choices, line)?;
        self.emit(
            ir::Op::If {
                condition: exhausted,
            },
            line,
        );
        let continued = self.snapshot();
        self.cleanup(start, None, line);
        self.emit(ir::Op::Break, line);
        self.emit(ir::Op::End, line);
        self.restore(&continued);
        self.cleanup(start, None, line);
        self.emit(ir::Op::Continue, line);
        self.emit(ir::Op::End, line);
        let scope = self.loops.pop().unwrap();
        if let Some(exit) = scope.exits.first() {
            self.restore(exit);
            for other in &scope.exits[1..] {
                self.merge_carriers(other);
            }
        } else {
            self.restore(&entry);
            self.reachable = false;
        }
        let retried = self.snapshot();
        self.emit(ir::Op::End, line);
        self.restore(&skipped);
        self.merge_carriers(&retried);
        Ok((found, choices))
    }
}
