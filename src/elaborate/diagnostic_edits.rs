//! Source rewrites accompany diagnostics; every rewritten program is checked anew.
use super::{Builder, a, err, ir};
use crate::diagnostic::Edit;
use crate::generics::TraitIdentity;

fn receiver_edits(span: a::Span, suffix: &str) -> Vec<Edit> {
    vec![
        Edit { start: span.start, end: span.start, replacement: "(".to_string() },
        Edit { start: span.end, end: span.end, replacement: format!("){suffix}") },
    ]
}

impl Builder<'_, '_> {
    pub(super) fn receiver_diagnostic(
        &mut self,
        expression: a::ExprId,
        receiver: a::ExprId,
        base: usize,
        name: &str,
        span: a::Span,
    ) -> Option<a::Diagnostic> {
        let (no_types, no_args) = {
            let a::Expr::Method { types, args, .. } = &self.compiler.source.exprs[expression].kind else {
                return None;
            };
            (types.is_empty(), args.is_empty())
        };
        if !no_args { return None; }
        let receiver_span = self.compiler.source.exprs[receiver].span;
        if name == "to_vec" && no_types {
            if let ir::TypeKind::Array { inner, .. } = self.compiler.types[base].kind {
                if !self.compiler.satisfies_identity(inner, &TraitIdentity::Builtin("Clone".to_string())) {
                    return None;
                }
                let mut problem = err(
                    span,
                    "E_UNSUPPORTED",
                    "array.to_vec requires a slice receiver and Clone elements; help: write array[..].to_vec() (docs/language.md, Source library)",
                );
                problem.edits = receiver_edits(receiver_span, "[..]");
                return Some(problem);
            }
        }
        if name == "parse" && self.library_string(base, span) {
            let mut problem = err(
                span,
                "E_UNSUPPORTED",
                "String.parse requires a str receiver; help: write text.as_str().parse::<T>() (docs/language.md, Source library)",
            );
            problem.edits = receiver_edits(receiver_span, ".as_str()");
            return Some(problem);
        }
        None
    }

    fn primitive_edit_type(&self, ty: usize) -> Option<String> {
        match self.compiler.types[ty].kind {
            ir::TypeKind::Unit => Some("()".to_string()),
            ir::TypeKind::Bool => Some("bool".to_string()),
            ir::TypeKind::Char => Some("char".to_string()),
            ir::TypeKind::Int { .. } => Some(self.compiler.types[ty].name.clone()),
            ir::TypeKind::F64 => Some("f64".to_string()),
            _ => None,
        }
    }

    pub(super) fn method_context_diagnostic(
        &self,
        expression: a::ExprId,
        problem: a::Diagnostic,
    ) -> a::Diagnostic {
        if problem.code != "E_TYPE"
            || !problem.message.contains("closure parameter or result type needs context")
        {
            return problem;
        }
        let node = &self.compiler.source.exprs[expression];
        let a::Expr::Method { receiver, name, types, args } = &node.kind else {
            return problem;
        };
        if name != "map_err" || !types.is_empty() || args.len() != 1usize { return problem; }
        let closure = &self.compiler.source.exprs[args[0]];
        let a::Expr::Closure { params, patterns, result, body, .. } = &closure.kind else {
            return problem;
        };
        if params.len() != 1usize || patterns.len() != 1usize
            || problem.file != closure.span.file
            || problem.line != closure.span.line || problem.column != closure.span.column
        {
            return problem;
        }
        let Some(mut base) = self.inferred.expressions.get(receiver).copied() else {
            return problem;
        };
        let borrowed_receiver = matches!(self.compiler.types[base].kind, ir::TypeKind::Ref { .. });
        while let ir::TypeKind::Ref { inner, .. } = self.compiler.types[base].kind { base = inner; }
        let Some((application, arguments)) = self.compiler.applications.get(&base) else {
            return problem;
        };
        let declaration = self.compiler.names.library_type("prelude", "Result", node.span)
            .ok().and_then(|path| path.declaration);
        if declaration.is_none()
            || self.compiler.declaration_names.get(application).copied() != declaration
            || arguments.len() != 2usize
        {
            return problem;
        }
        let error_type = arguments[1];
        let input = self.inferred.bindings.get(&patterns[0]).copied();
        let output = self.inferred.expressions.get(body).copied();
        let input_text = match input {
            Some(ty) => self.compiler.display_type(ty),
            None => format!("unresolved (expected {})", self.compiler.display_type(error_type)),
        };
        let output_text = match output {
            Some(ty) => self.compiler.display_type(ty),
            None => "unresolved".to_string(),
        };
        let mut revised = err(
            closure.span,
            "E_TYPE",
            &format!(
                "Result::map_err closure needs known input and result types; input: {input_text}; result: {output_text}; help: annotate the missing closure type; a literal.to_string() body can use |_| -> std::String {{ literal.to_string() }} (docs/language.md, Source library)"
            ),
        );
        if input.is_none() && output.is_some()
            && matches!(self.compiler.source.types[params[0].ty].kind, a::Type::Infer)
        {
            if let Some(annotation) = self.primitive_edit_type(error_type) {
                let end = self.compiler.source.patterns[patterns[0]].span.end;
                revised.edits.push(Edit { start: end, end, replacement: format!(": {annotation}") });
                return revised;
            }
        }

        if borrowed_receiver {
            revised.message.push_str("; automatic result annotations are not offered for borrowed Result receivers");
            return revised;
        }

        // The existing text lowering always calls source String::from_str.
        // This exact body therefore has a known String result without inferring
        // arbitrary closure bodies or moving their captures/control-flow scope.
        let body = &self.compiler.source.exprs[*body];
        let a::Expr::Method { receiver: text, name, types, args } = &body.kind else {
            return revised;
        };
        if result.is_some() || !matches!(self.compiler.source.patterns[patterns[0]].kind, a::Pattern::Wild)
            || !matches!(self.compiler.source.types[params[0].ty].kind, a::Type::Infer)
            || name != "to_string" || !types.is_empty() || !args.is_empty()
            || !matches!(self.compiler.source.exprs[*text].kind, a::Expr::String(_))
        {
            return revised;
        }
        revised.edits = vec![
            Edit { start: body.span.start, end: body.span.start,
                replacement: "-> std::String { ".to_string() },
            Edit { start: body.span.end, end: body.span.end,
                replacement: " }".to_string() },
        ];
        revised
    }
}
