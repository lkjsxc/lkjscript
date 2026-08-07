use std::collections::HashMap;

use crate::analyze::*;

pub(super) type PatternId = usize;
pub(super) type MatrixId = usize;
pub(super) type WitnessId = usize;

const WILDCARD_PATTERN: PatternId = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Constructor {
    Bool(bool),
    I64(i64),
    Variant(VariantId),
    Product(ProductId),
}

#[derive(Clone, Debug)]
pub(super) struct PatternNode {
    pub(super) constructor: Option<Constructor>,
    pub(super) fields: Vec<PatternId>,
}

#[derive(Clone, Debug)]
pub(super) enum WitnessNode {
    Wild(Type),
    Constructor {
        ty: Type,
        constructor: Constructor,
        fields: Vec<WitnessId>,
    },
}

pub(super) type Matrix = Vec<Vec<PatternId>>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct State {
    pub(super) matrix: MatrixId,
    pub(super) vector: Vec<PatternId>,
    pub(super) types: Vec<Type>,
}

enum Work {
    Evaluate(State),
    Specialized {
        state: State,
        constructor: Constructor,
        field_count: usize,
    },
    Complete {
        state: State,
        constructors: Vec<Constructor>,
        index: usize,
        field_count: usize,
    },
    Default {
        state: State,
        present: Vec<Constructor>,
    },
}

pub(super) struct Usefulness<'a> {
    pub(super) enums: &'a [EnumDefinition],
    pub(super) products: &'a [ProductDefinition],
    pub(super) patterns: Vec<PatternNode>,
    pub(super) pattern_ids: HashMap<usize, PatternId>,
    pub(super) matrices: Vec<Matrix>,
    pub(super) matrix_ids: HashMap<Matrix, MatrixId>,
    pub(super) specializations: HashMap<(MatrixId, Constructor, usize), MatrixId>,
    pub(super) defaults: HashMap<MatrixId, MatrixId>,
    pub(super) memo: HashMap<State, Option<Vec<WitnessId>>>,
    pub(super) witnesses: Vec<WitnessNode>,
    pub(super) wild_witnesses: HashMap<Type, WitnessId>,
}

impl<'a> Usefulness<'a> {
    pub(super) fn new(
        enums: &'a [EnumDefinition],
        products: &'a [ProductDefinition],
    ) -> Result<Self> {
        let mut patterns = Vec::new();
        reserve(&mut patterns, 1, "match usefulness pattern arena")?;
        patterns.push(PatternNode {
            constructor: None,
            fields: Vec::new(),
        });
        Ok(Self {
            enums,
            products,
            patterns,
            pattern_ids: HashMap::new(),
            matrices: Vec::new(),
            matrix_ids: HashMap::new(),
            specializations: HashMap::new(),
            defaults: HashMap::new(),
            memo: HashMap::new(),
            witnesses: Vec::new(),
            wild_witnesses: HashMap::new(),
        })
    }

    pub(super) fn useful(
        &mut self,
        matrix: &[&MatchPattern],
        vector: &[&MatchPattern],
        types: &[Type],
    ) -> Result<Option<Vec<WitnessId>>> {
        let initial = self.initial_state(matrix, vector, types)?;
        let mut work = Vec::new();
        reserve(&mut work, 1, "match usefulness continuation stack")?;
        work.push(Work::Evaluate(initial));
        let mut completed = Vec::new();

        while let Some(item) = work.pop() {
            match item {
                Work::Evaluate(state) => {
                    if let Some(cached) = self.memo.get(&state) {
                        push_result(
                            &mut completed,
                            clone_result(cached)?,
                            "match usefulness result stack",
                        )?;
                        continue;
                    }
                    if state.vector.is_empty() {
                        let result = self.matrix(state.matrix)?.is_empty().then(Vec::new);
                        self.publish(state, result, &mut completed)?;
                        continue;
                    }
                    let ty = state
                        .types
                        .first()
                        .ok_or_else(|| Error::msg("match usefulness lost column type"))?;
                    let head = *state
                        .vector
                        .first()
                        .ok_or_else(|| Error::msg("match usefulness lost candidate column"))?;
                    if let Some(constructor) = self.pattern(head)?.constructor {
                        let (next, field_count) = self.specialized_state(&state, constructor)?;
                        reserve(&mut work, 2, "match usefulness continuation stack")?;
                        work.push(Work::Specialized {
                            state,
                            constructor,
                            field_count,
                        });
                        work.push(Work::Evaluate(next));
                        continue;
                    }

                    let present = self.present_constructors(state.matrix)?;
                    if let Some(constructors) = self.complete_space(ty)? {
                        if constructors
                            .iter()
                            .all(|constructor| present.contains(constructor))
                        {
                            if constructors.is_empty() {
                                self.publish(state, None, &mut completed)?;
                                continue;
                            }
                            let constructor = constructors[0];
                            let (next, field_count) =
                                self.specialized_state(&state, constructor)?;
                            reserve(&mut work, 2, "match usefulness continuation stack")?;
                            work.push(Work::Complete {
                                state,
                                constructors,
                                index: 0,
                                field_count,
                            });
                            work.push(Work::Evaluate(next));
                            continue;
                        }
                    }

                    let next = self.default_state(&state)?;
                    reserve(&mut work, 2, "match usefulness continuation stack")?;
                    work.push(Work::Default { state, present });
                    work.push(Work::Evaluate(next));
                }
                Work::Specialized {
                    state,
                    constructor,
                    field_count,
                } => {
                    let result = pop_result(&mut completed)?;
                    let result = self.wrap_constructor(&state, constructor, field_count, result)?;
                    self.publish(state, result, &mut completed)?;
                }
                Work::Complete {
                    state,
                    constructors,
                    index,
                    field_count,
                } => {
                    let result = pop_result(&mut completed)?;
                    if result.is_some() {
                        let result = self.wrap_constructor(
                            &state,
                            constructors[index],
                            field_count,
                            result,
                        )?;
                        self.publish(state, result, &mut completed)?;
                        continue;
                    }
                    let next_index = index
                        .checked_add(1)
                        .ok_or_else(|| Error::host("match constructor index overflow"))?;
                    if next_index == constructors.len() {
                        self.publish(state, None, &mut completed)?;
                        continue;
                    }
                    let constructor = constructors[next_index];
                    let (next, next_field_count) = self.specialized_state(&state, constructor)?;
                    reserve(&mut work, 2, "match usefulness continuation stack")?;
                    work.push(Work::Complete {
                        state,
                        constructors,
                        index: next_index,
                        field_count: next_field_count,
                    });
                    work.push(Work::Evaluate(next));
                }
                Work::Default { state, present } => {
                    let result = pop_result(&mut completed)?;
                    let result = match result {
                        Some(mut witness) => {
                            let ty = state
                                .types
                                .first()
                                .ok_or_else(|| Error::msg("match usefulness lost column type"))?;
                            let missing = self.missing_witness(ty, &present)?;
                            reserve(&mut witness, 1, "match witness root vector")?;
                            witness.insert(0, missing);
                            Some(witness)
                        }
                        None => None,
                    };
                    self.publish(state, result, &mut completed)?;
                }
            }
        }

        let result = pop_result(&mut completed)?;
        if !completed.is_empty() {
            return Err(Error::msg("match usefulness left stale completed results"));
        }
        Ok(result)
    }

    fn publish(
        &mut self,
        state: State,
        result: Option<Vec<WitnessId>>,
        completed: &mut Vec<Option<Vec<WitnessId>>>,
    ) -> Result<()> {
        let output = clone_result(&result)?;
        self.memo
            .try_reserve(1)
            .map_err(|_| Error::host("match usefulness memo allocation failed"))?;
        self.memo.insert(state, result);
        push_result(completed, output, "match usefulness result stack")
    }

    fn wrap_constructor(
        &mut self,
        state: &State,
        constructor: Constructor,
        field_count: usize,
        result: Option<Vec<WitnessId>>,
    ) -> Result<Option<Vec<WitnessId>>> {
        let Some(mut witness) = result else {
            return Ok(None);
        };
        if witness.len() < field_count {
            return Err(Error::msg("match witness lost constructor fields"));
        }
        let mut fields = Vec::new();
        reserve(&mut fields, field_count, "match witness constructor fields")?;
        fields.extend_from_slice(&witness[..field_count]);
        witness.drain(..field_count);
        let ty = state
            .types
            .first()
            .ok_or_else(|| Error::msg("match witness lost constructor type"))?
            .clone();
        let root = self.push_witness(WitnessNode::Constructor {
            ty,
            constructor,
            fields,
        })?;
        reserve(&mut witness, 1, "match witness result vector")?;
        witness.insert(0, root);
        Ok(Some(witness))
    }

    pub(super) fn push_witness(&mut self, node: WitnessNode) -> Result<WitnessId> {
        let id = self.witnesses.len();
        reserve(&mut self.witnesses, 1, "match witness arena")?;
        self.witnesses.push(node);
        Ok(id)
    }

    pub(super) fn witness(&self, id: WitnessId) -> Result<&WitnessNode> {
        self.witnesses
            .get(id)
            .ok_or_else(|| Error::msg("match witness identity is stale"))
    }

    pub(super) fn pattern(&self, id: PatternId) -> Result<&PatternNode> {
        self.patterns
            .get(id)
            .ok_or_else(|| Error::msg("match usefulness pattern identity is stale"))
    }

    pub(super) fn matrix(&self, id: MatrixId) -> Result<&Matrix> {
        self.matrices
            .get(id)
            .ok_or_else(|| Error::msg("match usefulness matrix identity is stale"))
    }
}

pub(super) fn reserve<T>(values: &mut Vec<T>, additional: usize, context: &str) -> Result<()> {
    values
        .try_reserve(additional)
        .map_err(|_| Error::host(format!("{context} allocation failed")))
}

pub(super) fn checked_capacity(left: usize, right: usize, context: &str) -> Result<usize> {
    left.checked_add(right)
        .ok_or_else(|| Error::host(format!("{context} size overflow")))
}

fn clone_result(value: &Option<Vec<WitnessId>>) -> Result<Option<Vec<WitnessId>>> {
    value
        .as_ref()
        .map(|items| {
            let mut output = Vec::new();
            reserve(&mut output, items.len(), "match usefulness memo result")?;
            output.extend_from_slice(items);
            Ok(output)
        })
        .transpose()
}

fn push_result(
    completed: &mut Vec<Option<Vec<WitnessId>>>,
    result: Option<Vec<WitnessId>>,
    context: &str,
) -> Result<()> {
    reserve(completed, 1, context)?;
    completed.push(result);
    Ok(())
}

fn pop_result(completed: &mut Vec<Option<Vec<WitnessId>>>) -> Result<Option<Vec<WitnessId>>> {
    completed
        .pop()
        .ok_or_else(|| Error::msg("match usefulness continuation lost its result"))
}

pub(super) fn wildcard_pattern() -> PatternId {
    WILDCARD_PATTERN
}
