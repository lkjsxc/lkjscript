//! Assignment has a different operand order from ordinary binary expressions:
//! first compute the incoming value, then the destination, then replace it.
use super::{Bindings, Builder, Facts, LoopScope, Result, a, err, k, place_type};
use std::collections::BTreeMap;

impl Builder<'_, '_> {
    /// Structural place types do not evaluate their index expressions. In
    /// particular, a method's spelling is never evidence of its result type.
    fn assignment_type_hint(&self, mut expression: usize) -> Option<usize> {
        enum Step<'a> {
            Deref,
            Field(&'a str),
            Index,
        }
        let mut steps = Vec::new();
        let mut ty = loop {
            match &self.compiler.source.exprs[expression].kind {
                a::Expr::Path(path) => {
                    break self
                        .guard_aliases
                        .get(&path.name)
                        .and_then(|place| place_type(&self.compiler.types, self.function, place))
                        .or_else(|| self.binding(&path.name).map(|local| self.ty(local)))?;
                }
                a::Expr::Unary { op, value } if op == "*" => {
                    steps.push(Step::Deref);
                    expression = *value;
                }
                a::Expr::Field { value, name } => {
                    steps.push(Step::Field(name));
                    expression = *value;
                }
                a::Expr::Index { value, .. } => {
                    steps.push(Step::Index);
                    expression = *value;
                }
                _ => return None,
            }
        };
        while let Some(step) = steps.pop() {
            ty = match step {
                Step::Deref => match self.compiler.types[ty].kind {
                    k::TypeKind::Ref { inner, .. } | k::TypeKind::Box { inner } => inner,
                    _ => return None,
                },
                Step::Field(name) => {
                    while let k::TypeKind::Ref { inner, .. } | k::TypeKind::Box { inner } =
                        self.compiler.types[ty].kind
                    {
                        ty = inner;
                    }
                    match &self.compiler.types[ty].kind {
                        k::TypeKind::Struct { fields, .. } => {
                            fields.iter().find(|field| field.name == name)?.ty
                        }
                        _ => return None,
                    }
                }
                Step::Index => {
                    while let k::TypeKind::Ref { inner, .. } = self.compiler.types[ty].kind {
                        ty = inner;
                    }
                    match self.compiler.types[ty].kind {
                        k::TypeKind::Vec { inner }
                        | k::TypeKind::Slice { inner, .. }
                        | k::TypeKind::Array { inner, .. } => inner,
                        _ => return None,
                    }
                }
            };
        }
        Some(ty)
    }

    fn assignment_type(&mut self, expression: usize) -> Result<usize> {
        if let Some(ty) = self.inferred.expressions.get(&expression) {
            return Ok(*ty);
        }
        if let Some(ty) = self.assignment_types.get(&expression) {
            return Ok(*ty);
        }
        if let Some(ty) = self.assignment_type_hint(expression) {
            return Ok(ty);
        }
        // An effectful destination can still supply essential context, such as
        // the u8 type of `*choose_slot() = 255`. Reuse ordinary elaboration in a
        // discarded builder to obtain that type; do not emit its effects before
        // the RHS or reproduce method selection in a separate type checker.
        // Only source-visible bindings are needed, not the function's transient
        // locals or existing instructions. The common structural path above
        // needs no copy of this environment.
        let mut function = k::Function {
            file: self.file,
            name: "$assignment_type".to_string(),
            params: 0,
            locals: Vec::new(),
            result: self.function.result,
            return_source: self.function.return_source.clone(),
            body: Vec::new(),
            test: false,
        };
        let mut bindings = Bindings::default();
        let mut live = Vec::new();
        let mut definite_live = Vec::new();
        let mut written = Vec::new();
        for (name, original) in &self.bindings.entries {
            let local = function.locals.len();
            function
                .locals
                .push(self.function.locals[*original].clone());
            bindings.push((name.clone(), local));
            live.push(self.live[*original]);
            definite_live.push(self.definite_live[*original]);
            written.push(self.written[*original]);
        }
        let references = Facts::new(function.locals.len());
        let loops = self
            .loops
            .iter()
            .map(|scope| LoopScope {
                synthetic: scope.synthetic,
                start: 0,
                range: None,
                exits: Vec::new(),
                result: None,
                result_typed: true,
            })
            .collect();
        // The memo belongs to this one function instance. Nested probes share
        // it by transfer, so an effectful target nested inside another target is
        // elaborated for its type once rather than expanding repeated work.
        let assignment_types = std::mem::take(&mut self.assignment_types);
        let (result, assignment_types) = {
            let mut probe = Builder {
                source_function: self.source_function,
                scope: self.scope,
                file: self.file,
                compiler: self.compiler,
                function: &mut function,
                sub: self.sub.clone(),
                bindings,
                guard_aliases: BTreeMap::new(),
                guard_shells: Vec::new(),
                capture_aliases: self.capture_aliases.clone(),
                capture_locals: self.capture_locals.clone(),
                active_span: self.active_span,
                reference_updates: references.clone(),
                reference_schedules: BTreeMap::new(),
                references,
                carriers: BTreeMap::new(),
                dependents: BTreeMap::new(),
                assignment_types,
                range_borrows: BTreeMap::new(),
                inferred: self.inferred.clone(),
                live: live.into_iter().collect(),
                definite_live: definite_live.into_iter().collect(),
                written: written.into_iter().collect(),
                loops,
                reachable: self.reachable,
            };
            let result = probe
                .place_with_mutability(expression, true)
                .and_then(|place| {
                    place_type(&probe.compiler.types, probe.function, &place).ok_or_else(|| {
                        err(
                            probe.compiler.source.exprs[expression].span,
                            "E_TYPE",
                            "invalid assignment place",
                        )
                    })
                });
            (result, probe.assignment_types)
        };
        self.assignment_types = assignment_types;
        let ty = result?;
        self.assignment_types.insert(expression, ty);
        Ok(ty)
    }

    pub(super) fn assignment_expr(
        &mut self,
        operation: &str,
        place: usize,
        value: usize,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let ty = self.assignment_type(place)?;
        let incoming = self.expr(value, Some(ty))?;
        if incoming.1 != ty {
            return Err(err(span, "E_TYPE", "assignment type differs"));
        }
        let place = self.place_with_mutability(place, true)?;
        self.guard_write(&place, span)?;
        if place.projections.is_empty()
            && !self.function.locals[place.local].mutable
            && self.written[place.local]
        {
            return Err(err(span, "E_TYPE", "assignment to an immutable binding"));
        }
        let value = if operation == "=" {
            incoming.0
        } else {
            // Only primitive compound operations are currently supported. Their
            // RHS is evaluated before both the destination and its scalar read.
            let left = self.read(place.clone(), span.line)?.0;
            let dst = self.temp(ty);
            self.emit(
                k::Op::Binary {
                    dst,
                    op: operation.trim_end_matches('=').to_string(),
                    left,
                    right: incoming.0,
                },
                span.line,
            );
            dst
        };
        self.emit(k::Op::Assign { place, src: value }, span.line);
        Ok(self.unit(span.line))
    }
}
