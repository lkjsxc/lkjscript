use crate::analyze::*;

use super::usefulness::{
    checked_capacity, reserve, wildcard_pattern, Constructor, Matrix, MatrixId, PatternId,
    PatternNode, State, Usefulness,
};

impl Usefulness<'_> {
    pub(super) fn initial_state(
        &mut self,
        matrix: &[&MatchPattern],
        vector: &[&MatchPattern],
        types: &[Type],
    ) -> Result<State> {
        let mut rows = Vec::new();
        reserve(&mut rows, matrix.len(), "match usefulness initial matrix")?;
        for pattern in matrix {
            let id = self.intern_pattern(pattern)?;
            let mut row = Vec::new();
            reserve(&mut row, 1, "match usefulness initial matrix row")?;
            row.push(id);
            rows.push(row);
        }
        let matrix = self.intern_matrix(rows)?;
        let mut candidate = Vec::new();
        reserve(
            &mut candidate,
            vector.len(),
            "match usefulness initial candidate",
        )?;
        for pattern in vector {
            candidate.push(self.intern_pattern(pattern)?);
        }
        let types = clone_types(types, "match usefulness initial types")?;
        if candidate.len() != types.len() {
            return Err(Error::msg(
                "match usefulness candidate/type column count differs",
            ));
        }
        Ok(State {
            matrix,
            vector: candidate,
            types,
        })
    }

    pub(super) fn present_constructors(&self, matrix: MatrixId) -> Result<Vec<Constructor>> {
        let matrix = self.matrix(matrix)?;
        let mut result = Vec::new();
        reserve(&mut result, matrix.len(), "match constructor presence set")?;
        for row in matrix {
            let Some(pattern) = row.first() else {
                continue;
            };
            if let Some(constructor) = self.pattern(*pattern)?.constructor {
                if !result.contains(&constructor) {
                    result.push(constructor);
                }
            }
        }
        Ok(result)
    }

    pub(super) fn specialized_state(
        &mut self,
        state: &State,
        constructor: Constructor,
    ) -> Result<(State, usize)> {
        let ty = state
            .types
            .first()
            .ok_or_else(|| Error::msg("match specialization lost column type"))?;
        let fields = self.field_types(ty, &constructor)?;
        let field_count = fields.len();
        let matrix = self.specialize_matrix(state.matrix, constructor, field_count)?;
        let head = *state
            .vector
            .first()
            .ok_or_else(|| Error::msg("match specialization lost candidate column"))?;
        let mut candidate = self
            .specialize_pattern(head, constructor, field_count)?
            .ok_or_else(|| Error::msg("candidate constructor specialization failed"))?;
        let candidate_tail = state
            .vector
            .len()
            .checked_sub(1)
            .ok_or_else(|| Error::msg("match specialization lost candidate column"))?;
        let candidate_capacity = checked_capacity(
            candidate.len(),
            candidate_tail,
            "match specialized candidate",
        )?;
        let candidate_additional = candidate_capacity
            .checked_sub(candidate.len())
            .ok_or_else(|| Error::host("match specialized candidate size underflow"))?;
        reserve(
            &mut candidate,
            candidate_additional,
            "match specialized candidate",
        )?;
        candidate.extend_from_slice(&state.vector[1..]);

        let mut types = fields;
        let type_tail = state
            .types
            .len()
            .checked_sub(1)
            .ok_or_else(|| Error::msg("match specialization lost column type"))?;
        let type_capacity =
            checked_capacity(types.len(), type_tail, "match specialized type vector")?;
        let type_additional = type_capacity
            .checked_sub(types.len())
            .ok_or_else(|| Error::host("match specialized type vector size underflow"))?;
        reserve(&mut types, type_additional, "match specialized type vector")?;
        types.extend(state.types[1..].iter().cloned());
        if candidate.len() != types.len() {
            return Err(Error::msg(
                "match specialization candidate/type column count differs",
            ));
        }
        Ok((
            State {
                matrix,
                vector: candidate,
                types,
            },
            field_count,
        ))
    }

    pub(super) fn default_state(&mut self, state: &State) -> Result<State> {
        if state.vector.is_empty() || state.types.is_empty() {
            return Err(Error::msg("match default specialization lost column"));
        }
        let matrix = self.default_matrix(state.matrix)?;
        let mut vector = Vec::new();
        reserve(
            &mut vector,
            state.vector.len() - 1,
            "match default candidate",
        )?;
        vector.extend_from_slice(&state.vector[1..]);
        let types = clone_types(&state.types[1..], "match default type vector")?;
        Ok(State {
            matrix,
            vector,
            types,
        })
    }

    fn intern_pattern(&mut self, root: &MatchPattern) -> Result<PatternId> {
        enum PatternWork<'a> {
            Visit(&'a MatchPattern),
            Finish {
                address: usize,
                constructor: Constructor,
                child_count: usize,
            },
        }

        let mut work = Vec::new();
        reserve(&mut work, 1, "match pattern interning work stack")?;
        work.push(PatternWork::Visit(root));
        let mut completed = Vec::new();
        while let Some(item) = work.pop() {
            match item {
                PatternWork::Visit(pattern) => {
                    let address = std::ptr::from_ref(pattern).addr();
                    if let Some(id) = self.pattern_ids.get(&address) {
                        reserve(&mut completed, 1, "match pattern interning result stack")?;
                        completed.push(*id);
                        continue;
                    }
                    let (constructor, fields) = match pattern {
                        MatchPattern::Wildcard { .. } | MatchPattern::Binding { .. } => {
                            self.pattern_ids.try_reserve(1).map_err(|_| {
                                Error::host("match pattern identity index allocation failed")
                            })?;
                            self.pattern_ids.insert(address, wildcard_pattern());
                            reserve(&mut completed, 1, "match pattern interning result stack")?;
                            completed.push(wildcard_pattern());
                            continue;
                        }
                        MatchPattern::Bool(value) => (Constructor::Bool(*value), None),
                        MatchPattern::I64(value) => (Constructor::I64(*value), None),
                        MatchPattern::Variant {
                            variant, fields, ..
                        } => (Constructor::Variant(*variant), Some(fields.as_slice())),
                        MatchPattern::Product {
                            product, fields, ..
                        } => (Constructor::Product(*product), Some(fields.as_slice())),
                    };
                    let child_count = fields.map_or(0, <[MatchFieldPattern]>::len);
                    let additional =
                        checked_capacity(1, child_count, "match pattern interning work stack")?;
                    reserve(&mut work, additional, "match pattern interning work stack")?;
                    work.push(PatternWork::Finish {
                        address,
                        constructor,
                        child_count,
                    });
                    if let Some(fields) = fields {
                        work.extend(
                            fields
                                .iter()
                                .rev()
                                .map(|field| PatternWork::Visit(&field.pattern)),
                        );
                    }
                }
                PatternWork::Finish {
                    address,
                    constructor,
                    child_count,
                } => {
                    let start = completed
                        .len()
                        .checked_sub(child_count)
                        .ok_or_else(|| Error::msg("match pattern interning lost child results"))?;
                    let mut fields = Vec::new();
                    reserve(&mut fields, child_count, "match pattern arena fields")?;
                    fields.extend_from_slice(&completed[start..]);
                    completed.truncate(start);
                    let id = self.patterns.len();
                    reserve(&mut self.patterns, 1, "match usefulness pattern arena")?;
                    self.patterns.push(PatternNode {
                        constructor: Some(constructor),
                        fields,
                    });
                    self.pattern_ids.try_reserve(1).map_err(|_| {
                        Error::host("match pattern identity index allocation failed")
                    })?;
                    self.pattern_ids.insert(address, id);
                    reserve(&mut completed, 1, "match pattern interning result stack")?;
                    completed.push(id);
                }
            }
        }
        if completed.len() != 1 {
            return Err(Error::msg(
                "match pattern interning produced the wrong root count",
            ));
        }
        completed
            .pop()
            .ok_or_else(|| Error::msg("match pattern interning omitted its root"))
    }

    fn intern_matrix(&mut self, matrix: Matrix) -> Result<MatrixId> {
        if let Some(id) = self.matrix_ids.get(&matrix) {
            return Ok(*id);
        }
        let key = clone_matrix(&matrix)?;
        let id = self.matrices.len();
        reserve(&mut self.matrices, 1, "match usefulness matrix arena")?;
        self.matrices.push(matrix);
        self.matrix_ids
            .try_reserve(1)
            .map_err(|_| Error::host("match usefulness matrix index allocation failed"))?;
        self.matrix_ids.insert(key, id);
        Ok(id)
    }

    fn specialize_matrix(
        &mut self,
        matrix: MatrixId,
        constructor: Constructor,
        field_count: usize,
    ) -> Result<MatrixId> {
        let cache_key = (matrix, constructor, field_count);
        if let Some(id) = self.specializations.get(&cache_key) {
            return Ok(*id);
        }
        let mut specialized = Vec::new();
        let source = self.matrix(matrix)?;
        reserve(&mut specialized, source.len(), "match specialized matrix")?;
        for row in source {
            let Some(head) = row.first() else {
                continue;
            };
            let Some(mut next) = specialize_node(self.pattern(*head)?, constructor, field_count)?
            else {
                continue;
            };
            let row_tail = row
                .len()
                .checked_sub(1)
                .ok_or_else(|| Error::msg("match specialization lost matrix column"))?;
            let capacity = checked_capacity(next.len(), row_tail, "match specialized matrix row")?;
            let additional = capacity
                .checked_sub(next.len())
                .ok_or_else(|| Error::host("match specialized matrix row size underflow"))?;
            reserve(&mut next, additional, "match specialized matrix row")?;
            next.extend_from_slice(&row[1..]);
            specialized.push(next);
        }
        let id = self.intern_matrix(specialized)?;
        self.specializations
            .try_reserve(1)
            .map_err(|_| Error::host("match specialization cache allocation failed"))?;
        self.specializations.insert(cache_key, id);
        Ok(id)
    }

    fn default_matrix(&mut self, matrix: MatrixId) -> Result<MatrixId> {
        if let Some(id) = self.defaults.get(&matrix) {
            return Ok(*id);
        }
        let mut default = Vec::new();
        let source = self.matrix(matrix)?;
        reserve(&mut default, source.len(), "match default matrix")?;
        for row in source {
            let Some(head) = row.first() else {
                continue;
            };
            if self.pattern(*head)?.constructor.is_none() {
                let mut next = Vec::new();
                let row_tail = row
                    .len()
                    .checked_sub(1)
                    .ok_or_else(|| Error::msg("match default specialization lost matrix column"))?;
                reserve(&mut next, row_tail, "match default matrix row")?;
                next.extend_from_slice(&row[1..]);
                default.push(next);
            }
        }
        let id = self.intern_matrix(default)?;
        self.defaults
            .try_reserve(1)
            .map_err(|_| Error::host("match default matrix cache allocation failed"))?;
        self.defaults.insert(matrix, id);
        Ok(id)
    }

    fn specialize_pattern(
        &self,
        pattern: PatternId,
        constructor: Constructor,
        field_count: usize,
    ) -> Result<Option<Vec<PatternId>>> {
        specialize_node(self.pattern(pattern)?, constructor, field_count)
    }
}

fn specialize_node(
    pattern: &PatternNode,
    constructor: Constructor,
    field_count: usize,
) -> Result<Option<Vec<PatternId>>> {
    match pattern.constructor {
        None => {
            let mut fields = Vec::new();
            reserve(
                &mut fields,
                field_count,
                "match wildcard specialization fields",
            )?;
            fields.resize(field_count, wildcard_pattern());
            Ok(Some(fields))
        }
        Some(found) if found == constructor => {
            if pattern.fields.len() != field_count {
                return Err(Error::msg(
                    "match constructor field count changed during specialization",
                ));
            }
            let mut fields = Vec::new();
            reserve(
                &mut fields,
                pattern.fields.len(),
                "match constructor specialization fields",
            )?;
            fields.extend_from_slice(&pattern.fields);
            Ok(Some(fields))
        }
        Some(_) => Ok(None),
    }
}

fn clone_types(types: &[Type], context: &str) -> Result<Vec<Type>> {
    let mut output = Vec::new();
    reserve(&mut output, types.len(), context)?;
    output.extend(types.iter().cloned());
    Ok(output)
}

fn clone_matrix(matrix: &Matrix) -> Result<Matrix> {
    let mut output = Vec::new();
    reserve(
        &mut output,
        matrix.len(),
        "match usefulness matrix index key",
    )?;
    for row in matrix {
        let mut cloned = Vec::new();
        reserve(&mut cloned, row.len(), "match usefulness matrix index row")?;
        cloned.extend_from_slice(row);
        output.push(cloned);
    }
    Ok(output)
}
