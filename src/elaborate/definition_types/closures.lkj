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
        for index in 0..self.capture_frames.len() {
            let frame = &mut self.capture_frames[index];
            if entry < frame.start {
                if !frame.captures.contains_key(&entry) {
                    frame.captures.insert(
                        entry,
                        Capture {
                            name: name.to_string(),
                            value,
                            uses: Vec::new(),
                        },
                    );
                }
                frame
                    .captures
                    .get_mut(&entry)
                    .unwrap()
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
        let mut index = 0;
        while index < params.len() && index < patterns.len() {
            let value = self.source_type(params[index].ty, &environment)?;
            arguments.push(value);
            self.definition.bindings.insert(patterns[index], value);
            self.bind_pattern(patterns[index], value, true)?;
            index += 1;
        }
        let result = match annotation {
            Some(ty) => self.source_type(ty, &environment)?,
            None => self.unknown(),
        };
        let enclosing_result = std::mem::replace(&mut self.result, result);
        let enclosing_loops = std::mem::replace(&mut self.loops, Vec::new());
        let value = self.expr(body)?;
        self.coerce(value, result, span)?;
        self.result = enclosing_result;
        self.loops = enclosing_loops;
        self.bindings.truncate(start);
        let mut frame = self.capture_frames.pop().unwrap();
        // The entry key breaks equal source positions exactly as the original
        // stable sort of the map's ordered values did.
        let mut order: Vec<(usize, usize)> = Vec::new();
        for (entry, capture) in &frame.captures {
            order.push((self.source.exprs[capture.uses[0]].span.start, *entry));
        }
        order.sort();
        let mut captures = Vec::new();
        let mut children = Vec::new();
        for (_, entry) in order {
            let capture = frame.captures.remove(&entry).unwrap();
            children.push(capture.value);
            captures.push(capture);
        }
        for argument in &arguments {
            children.push(*argument);
        }
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
        let mut arguments = Vec::new();
        for ty in parameters {
            arguments.push(self.source_type(ty, environment)?);
        }
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
        produced: bool,
    ) -> Result<bool> {
        let Some((_, shape)) = self.strip_references(receiver, id) else {
            return Ok(false);
        };
        let mut span = span;
        let signature = match shape {
            Shape::Closure(expression, _) => {
                // Callable bounds are declared in library headers, but a mismatch
                // in a supplied closure belongs to that caller's expression.
                if !produced {
                    span = self.source.exprs[expression].span;
                }
                let closure = self.definition.closures.get(&expression).unwrap();
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
        for index in 0..arguments.len() {
            self.coerce(arguments[index], parameters[index], span)?;
        }
        self.unify(result, output, span)?;
        Ok(true)
    }
}
