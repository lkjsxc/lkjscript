use std::collections::HashMap;

use crate::ownership::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::ownership) struct ExprRange {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy, Debug)]
struct ExprFacts {
    range: ExprRange,
    contains_ownership_action: bool,
    uses_reference_binding: bool,
}

pub(in crate::ownership) struct OwnershipPlan {
    facts: Vec<ExprFacts>,
    uses: HashMap<BindingId, Vec<usize>>,
    places: HashMap<BindingId, PlaceId>,
    place_bindings: HashMap<PlaceId, BindingId>,
}

#[derive(Default)]
pub(in crate::ownership) struct ExprCursor {
    next: usize,
}

#[derive(Default)]
pub(in crate::ownership) struct FutureUses {
    ranges: Vec<ExprRange>,
}

pub(in crate::ownership) enum FutureCheckpoint {
    Unchanged,
    Added,
    Extended { previous_start: usize },
}

impl OwnershipPlan {
    pub(in crate::ownership) fn build(
        program: &Program,
        expression: &Expr,
        declared_places: impl IntoIterator<Item = (BindingId, PlaceId)>,
    ) -> Result<Self> {
        let mut plan = Self {
            facts: Vec::new(),
            uses: HashMap::new(),
            places: HashMap::new(),
            place_bindings: HashMap::new(),
        };
        for (binding, place) in declared_places {
            plan.record_place(program, binding, place)?;
        }
        plan.build_expression_facts(program, expression)?;
        Ok(plan)
    }

    fn build_expression_facts(&mut self, program: &Program, root: &Expr) -> Result<()> {
        enum Work<'a> {
            Visit {
                expression: &'a Expr,
                parent: Option<usize>,
            },
            Finish {
                expression: usize,
                parent: Option<usize>,
            },
        }

        let mut work = Vec::new();
        work.try_reserve(1)
            .map_err(|_| Error::host("ownership liveness work allocation failed"))?;
        work.push(Work::Visit {
            expression: root,
            parent: None,
        });

        while let Some(item) = work.pop() {
            match item {
                Work::Visit { expression, parent } => {
                    let id = self.facts.len();
                    let end = id
                        .checked_add(1)
                        .ok_or_else(|| Error::host("ownership expression identity overflow"))?;
                    let direct_use = direct_binding_use(expression);
                    if let Some(binding) = direct_use {
                        self.record_use(binding, id)?;
                    }
                    let uses_reference_binding = direct_use
                        .and_then(|binding| program.binding(binding))
                        .is_some_and(|binding| is_ref(&binding.ty) || is_ref_mut(&binding.ty));
                    let contains_ownership_action = matches!(
                        expression.kind,
                        ExprKind::Move { .. } | ExprKind::Borrow { .. }
                    );
                    self.facts
                        .try_reserve(1)
                        .map_err(|_| Error::host("ownership expression-fact allocation failed"))?;
                    self.facts.push(ExprFacts {
                        range: ExprRange { start: id, end },
                        contains_ownership_action,
                        uses_reference_binding,
                    });
                    self.record_expression_places(program, expression)?;

                    work.try_reserve(1)
                        .map_err(|_| Error::host("ownership liveness work allocation failed"))?;
                    work.push(Work::Finish {
                        expression: id,
                        parent,
                    });
                    let children_start = work.len();
                    let mut allocation_failed = false;
                    crate::hir::for_each_expression_child(expression, &mut |child| {
                        if allocation_failed {
                            return;
                        }
                        if work.try_reserve(1).is_err() {
                            allocation_failed = true;
                        } else {
                            work.push(Work::Visit {
                                expression: child,
                                parent: Some(id),
                            });
                        }
                    });
                    if allocation_failed {
                        return Err(Error::host("ownership liveness child allocation failed"));
                    }
                    work[children_start..].reverse();
                }
                Work::Finish { expression, parent } => {
                    let end = self.facts.len();
                    let facts = self
                        .facts
                        .get_mut(expression)
                        .ok_or_else(|| Error::msg("ownership liveness lost expression facts"))?;
                    facts.range.end = end;
                    let contains_ownership_action = facts.contains_ownership_action;
                    let uses_reference_binding = facts.uses_reference_binding;
                    if let Some(parent) = parent {
                        let parent = self
                            .facts
                            .get_mut(parent)
                            .ok_or_else(|| Error::msg("ownership liveness lost parent facts"))?;
                        parent.contains_ownership_action |= contains_ownership_action;
                        parent.uses_reference_binding |= uses_reference_binding;
                    }
                }
            }
        }
        Ok(())
    }

    fn record_expression_places(&mut self, program: &Program, expression: &Expr) -> Result<()> {
        match &expression.kind {
            ExprKind::Let { bindings, .. } => {
                for binding in bindings {
                    self.record_place(program, binding.binding, binding.place)?;
                }
            }
            ExprKind::MutableLocal { binding, place, .. } => {
                self.record_place(program, *binding, *place)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn record_place(
        &mut self,
        program: &Program,
        binding: BindingId,
        place: PlaceId,
    ) -> Result<()> {
        if binding.index().is_none() || program.binding(binding).is_none() {
            return Err(Error::msg("ownership place references unknown binding"));
        }
        if self.places.contains_key(&binding) || self.place_bindings.contains_key(&place) {
            return Err(Error::msg("ownership analysis found duplicate PlaceId"));
        }
        self.places
            .try_reserve(1)
            .map_err(|_| Error::host("ownership place-index allocation failed"))?;
        self.place_bindings
            .try_reserve(1)
            .map_err(|_| Error::host("ownership place reverse-index allocation failed"))?;
        self.places.insert(binding, place);
        self.place_bindings.insert(place, binding);
        Ok(())
    }

    fn record_use(&mut self, binding: BindingId, expression: usize) -> Result<()> {
        if !self.uses.contains_key(&binding) {
            self.uses
                .try_reserve(1)
                .map_err(|_| Error::host("ownership use-index allocation failed"))?;
            self.uses.insert(binding, Vec::new());
        }
        let uses = self
            .uses
            .get_mut(&binding)
            .ok_or_else(|| Error::msg("ownership use index lost a binding"))?;
        uses.try_reserve(1)
            .map_err(|_| Error::host("ownership binding-use allocation failed"))?;
        uses.push(expression);
        Ok(())
    }

    pub(in crate::ownership) fn range(&self, expression: usize) -> Result<ExprRange> {
        self.facts
            .get(expression)
            .map(|facts| facts.range)
            .ok_or_else(|| Error::msg("ownership liveness expression is out of range"))
    }

    pub(in crate::ownership) fn contains_ownership_action(
        &self,
        expression: usize,
    ) -> Result<bool> {
        self.facts
            .get(expression)
            .map(|facts| facts.contains_ownership_action)
            .ok_or_else(|| Error::msg("ownership liveness expression is out of range"))
    }

    pub(in crate::ownership) fn uses_reference_binding(&self, expression: usize) -> Result<bool> {
        self.facts
            .get(expression)
            .map(|facts| facts.uses_reference_binding)
            .ok_or_else(|| Error::msg("ownership liveness expression is out of range"))
    }

    pub(in crate::ownership) fn place(&self, binding: BindingId) -> Option<PlaceId> {
        self.places.get(&binding).copied()
    }

    pub(in crate::ownership) fn binding_for_place(&self, place: PlaceId) -> Option<BindingId> {
        self.place_bindings.get(&place).copied()
    }

    pub(in crate::ownership) fn binding_live(
        &self,
        binding: BindingId,
        current: Option<ExprRange>,
        future: &FutureUses,
    ) -> bool {
        current.is_some_and(|range| self.binding_used_in_range(binding, range))
            || future
                .ranges
                .iter()
                .copied()
                .any(|range| self.binding_used_in_range(binding, range))
    }

    fn binding_used_in_range(&self, binding: BindingId, range: ExprRange) -> bool {
        let Some(uses) = self.uses.get(&binding) else {
            return false;
        };
        let index = uses.partition_point(|position| *position < range.start);
        uses.get(index)
            .is_some_and(|position| *position < range.end)
    }

    pub(in crate::ownership) fn expression_count(&self) -> usize {
        self.facts.len()
    }
}

impl ExprCursor {
    pub(in crate::ownership) fn enter(&mut self, plan: &OwnershipPlan) -> Result<usize> {
        let expression = self.next;
        let _ = plan.range(expression)?;
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| Error::host("ownership liveness cursor overflow"))?;
        Ok(expression)
    }

    pub(in crate::ownership) fn peek_range(&self, plan: &OwnershipPlan) -> Result<ExprRange> {
        plan.range(self.next)
    }

    pub(in crate::ownership) fn finish(&self, plan: &OwnershipPlan) -> Result<()> {
        if self.next == plan.expression_count() {
            Ok(())
        } else {
            Err(Error::msg(
                "ownership checker did not consume the complete liveness plan",
            ))
        }
    }
}

impl FutureUses {
    pub(in crate::ownership) fn push_suffix(
        &mut self,
        child: ExprRange,
        parent: ExprRange,
    ) -> Result<FutureCheckpoint> {
        if child.end > parent.end {
            return Err(Error::msg(
                "ownership child range exceeds its parent expression",
            ));
        }
        self.push(ExprRange {
            start: child.end,
            end: parent.end,
        })
    }

    fn push(&mut self, range: ExprRange) -> Result<FutureCheckpoint> {
        if range.start == range.end {
            return Ok(FutureCheckpoint::Unchanged);
        }
        if range.start > range.end {
            return Err(Error::msg("ownership future-use range is reversed"));
        }
        if let Some(last) = self.ranges.last_mut() {
            if range.end == last.start {
                let previous_start = last.start;
                last.start = range.start;
                return Ok(FutureCheckpoint::Extended { previous_start });
            }
        }
        self.ranges
            .try_reserve(1)
            .map_err(|_| Error::host("ownership continuation allocation failed"))?;
        self.ranges.push(range);
        Ok(FutureCheckpoint::Added)
    }

    pub(in crate::ownership) fn restore(&mut self, checkpoint: FutureCheckpoint) {
        match checkpoint {
            FutureCheckpoint::Unchanged => {}
            FutureCheckpoint::Added => {
                self.ranges.pop();
            }
            FutureCheckpoint::Extended { previous_start } => {
                if let Some(last) = self.ranges.last_mut() {
                    last.start = previous_start;
                }
            }
        }
    }
}

fn direct_binding_use(expression: &Expr) -> Option<BindingId> {
    match &expression.kind {
        ExprKind::Load(reference)
        | ExprKind::Move {
            binding: reference, ..
        } => Some(reference.binding),
        ExprKind::Borrow { binding, .. } | ExprKind::BorrowBytes { binding, .. } => {
            Some(binding.binding)
        }
        _ => None,
    }
}
