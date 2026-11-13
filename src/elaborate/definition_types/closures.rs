//! Closure signatures participate in the same definition-local graph as calls.
//! Capture paths are recorded by lexical binding identity, before specialization.
use super::*;

#[derive(Clone)]
pub(super) struct Capture {
    pub(super) name: String,
    pub(super) value: Variable,
    pub(super) uses: Vec<a::ExprId>,
}
#[derive(Clone)]
pub(super) struct Closure {
    pub(super) source_function: usize,
    pub(super) captures: Vec<Capture>,
    pub(super) arguments: Vec<Variable>,
    pub(super) result: Variable,
}
pub(super) struct Frame {
    pub(super) start: usize,
    pub(super) captures: BTreeMap<usize, Capture>,
}

impl Infer<'_, '_> {
    pub(super) fn capture_path(&mut self, expression: a::ExprId, name: &str) {
        let Some((entry, value)) = self.bindings.entry(name) else {
            return;
        };
        for frame in &mut self.capture_frames {
            if entry < frame.start {
                frame
                    .captures
                    .entry(entry)
                    .or_insert_with(|| Capture {
                        name: name.to_string(),
                        value,
                        uses: Vec::new(),
                    })
                    .uses
                    .push(expression);
            }
        }
    }

    pub(super) fn closure_expression(
        &mut self,
        expression: a::ExprId,
        params: &[a::Param],
        patterns: &[a::PatternId],
        annotation: Option<a::TypeId>,
        body: a::ExprId,
    ) -> Result<Variable> {
        let span = self.source.exprs[expression].span;
        let environment = self.environment.clone();
        let mut arguments = Vec::new();
        let start = self.bindings.len();
        self.capture_frames.push(Frame {
            start,
            captures: BTreeMap::new(),
        });
        for (parameter, pattern) in params.iter().zip(patterns) {
            let value = self.source_type(parameter.ty, &environment)?;
            arguments.push(value);
            self.definition.bindings.insert(*pattern, value);
            self.bind_pattern(*pattern, value, true)?;
        }
        let result = match annotation {
            Some(ty) => self.source_type(ty, &environment)?,
            None => self.unknown(),
        };
        let enclosing_result = std::mem::replace(&mut self.result, result);
        let enclosing_loops = std::mem::take(&mut self.loops);
        let value = self.expr(body)?;
        self.coerce(value, result, span)?;
        self.result = enclosing_result;
        self.loops = enclosing_loops;
        self.bindings.truncate(start);
        let frame = self.capture_frames.pop().unwrap();
        // Environment fields follow first source use, independently of names or
        // inference-variable union representatives.
        let mut captures: Vec<_> = frame.captures.into_values().collect();
        captures.sort_by_key(|capture| self.source.exprs[capture.uses[0]].span.start);
        let mut children: Vec<_> = captures.iter().map(|capture| capture.value).collect();
        children.extend_from_slice(&arguments);
        children.push(result);
        self.definition.closures.insert(
            expression,
            Closure {
                source_function: self.source_function,
                captures,
                arguments,
                result,
            },
        );
        Ok(self.shaped(Shape::Closure(expression, children)))
    }

    pub(super) fn callable_bound(
        &mut self,
        receiver: Variable,
        bound: a::TypeId,
        environment: &BTreeMap<String, Variable>,
    ) -> Result<()> {
        let node = self.source.types[bound].clone();
        let a::Type::Path(path) = node.kind else {
            return Ok(());
        };
        if path.declaration.is_some()
            || !matches!(path.name.as_str(), "Fn" | "FnMut" | "FnOnce")
            || path.args.len() != 2
        {
            return Ok(());
        }
        let a::Type::Tuple(parameters) = self.source.types[path.args[0]].kind.clone() else {
            return Ok(());
        };
        let arguments = parameters
            .into_iter()
            .map(|ty| self.source_type(ty, environment))
            .collect::<Result<Vec<_>>>()?;
        let result = self.source_type(path.args[1], environment)?;
        if let Some(Shape::Rigid(name)) = self.shape(receiver) {
            self.callable_parameters.insert(name, (arguments, result));
        } else {
            self.obligation(Obligation::Callable {
                receiver,
                arguments,
                result,
                span: node.span,
                produced: false,
            });
        }
        Ok(())
    }

    pub(super) fn solve_callable(
        &mut self,
        id: usize,
        receiver: Variable,
        arguments: Vec<Variable>,
        result: Variable,
        span: a::Span,
    ) -> Result<bool> {
        let Some((_, shape)) = self.strip_references(receiver, id) else {
            return Ok(false);
        };
        let signature = match shape {
            Shape::Closure(expression, _) => {
                let closure = &self.definition.closures[&expression];
                Some((closure.arguments.clone(), closure.result))
            }
            Shape::Rigid(name) => self.callable_parameters.get(&name).cloned(),
            _ => None,
        };
        let Some((parameters, output)) = signature else {
            return Ok(false);
        };
        if parameters.len() != arguments.len() {
            return Err(err(span, "E_TYPE", "callable argument count differs"));
        }
        for (value, parameter) in arguments.into_iter().zip(parameters) {
            self.coerce(value, parameter, span)?;
        }
        self.unify(result, output, span)?;
        Ok(true)
    }
}
