//! Vec operations call ordinary, resolved source-library functions.
use super::{Builder, Result, a, err, k};
use std::collections::BTreeMap;

impl Builder<'_, '_> {
    /// Ordinary lookup includes the implicit Vec trait implementation alongside
    /// visible source traits. An inherent method still has ordinary priority.
    pub(super) fn vector_builtin_available(
        &mut self,
        receiver: usize,
        name: &str,
        span: a::Span,
    ) -> Result<bool> {
        let required = if name == "clone" {
            "Clone"
        } else {
            "PartialEq"
        };
        let builtin = self.compiler.satisfies_bound(receiver, required);
        let mut declared_trait = false;
        for implementation in self.compiler.source.impls.clone() {
            if !implementation
                .methods
                .iter()
                .any(|method| self.compiler.source.functions[*method].name == name)
            {
                continue;
            }
            let mut substitutions = BTreeMap::new();
            if !self.infer_type_matches(implementation.target, receiver, &mut substitutions)? {
                continue;
            }
            substitutions.insert("Self".to_string(), receiver);
            if self
                .compiler
                .resolve(implementation.target, &substitutions)
                .ok()
                != Some(receiver)
                || !self.trait_available(&implementation, receiver)
                || self
                    .compiler
                    .bounds(&implementation.generics, &substitutions)
                    .is_err()
            {
                continue;
            }
            if implementation.trait_ty.is_none() {
                return Ok(false);
            }
            declared_trait = true;
        }
        if declared_trait && builtin {
            return Err(err(
                span,
                "E_TRAIT",
                &format!(
                    "Vec method {name} is ambiguous between builtin and source trait implementations"
                ),
            ));
        }
        Ok(!declared_trait)
    }

    fn sequence_call(
        &mut self,
        name: &str,
        element: usize,
        arguments: Vec<usize>,
        result: usize,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let source = self.compiler.names.sequence_function(name, span)?;
        let parameters: Vec<_> = self.compiler.source.functions[source]
            .generics
            .iter()
            .filter(|parameter| !parameter.lifetime)
            .collect();
        if parameters.len() != 1 {
            return Err(err(
                span,
                "E_TYPE",
                "source Vec helper requires one element type parameter",
            ));
        }
        let mut substitutions = BTreeMap::new();
        substitutions.insert(parameters[0].name.clone(), element);
        let function = self.compiler.instantiate(source, substitutions)?;
        let contract = &self.compiler.functions[function];
        if contract.params != arguments.len()
            || contract.result != result
            || arguments
                .iter()
                .enumerate()
                .any(|(index, argument)| contract.locals[index].ty != self.ty(*argument))
        {
            return Err(err(
                span,
                "E_TYPE",
                "source Vec helper signature differs from its operation",
            ));
        }
        let destination = self.temp(result);
        self.emit(
            k::Op::Call {
                dst: destination,
                function,
                args: arguments,
            },
            span.line,
        );
        Ok((destination, result))
    }

    pub(super) fn vector_equality_values(
        &mut self,
        operation: &str,
        left: usize,
        right: usize,
        element: usize,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        if !self.compiler.satisfies_bound(element, "PartialEq") {
            return Err(err(
                span,
                "E_TRAIT",
                "Vec equality requires element PartialEq",
            ));
        }
        let compared =
            self.sequence_call("__lkj_vec_equal", element, vec![left, right], k::BOOL, span)?;
        if operation == "==" {
            return Ok(compared);
        }
        let destination = self.temp(k::BOOL);
        self.emit(
            k::Op::Unary {
                dst: destination,
                op: "!".to_string(),
                src: compared.0,
            },
            span.line,
        );
        Ok((destination, k::BOOL))
    }

    /// Both normal method lookup and a retained generic trait choice enter here.
    pub(super) fn vector_trait_method(
        &mut self,
        target: &k::Place,
        receiver: usize,
        name: &str,
        types: &[usize],
        arguments: &[usize],
        span: a::Span,
    ) -> Result<(usize, usize)> {
        if !types.is_empty() {
            return Err(err(
                span,
                "E_TYPE",
                "Vec trait methods take no generic arguments",
            ));
        }
        let expected_arguments = if name == "clone" { 0 } else { 1 };
        if arguments.len() != expected_arguments {
            return Err(err(
                span,
                "E_TYPE",
                "Vec method argument count differs from its trait contract",
            ));
        }
        let element = match self.compiler.types[receiver].kind {
            k::TypeKind::Vec { inner } => inner,
            _ => return Err(err(span, "E_TYPE", "Vec trait method needs a Vec receiver")),
        };
        if name == "clone" && !self.compiler.satisfies_bound(element, "Clone") {
            return Err(err(span, "E_TRAIT", "Vec cloning requires element Clone"));
        }
        let borrowed = self.borrow(target.clone(), false, span.line)?.0;
        if name == "clone" {
            return self.sequence_call("__lkj_vec_clone", element, vec![borrowed], receiver, span);
        }
        let expected = self.compiler.reference(receiver, false);
        let right = self.expr(arguments[0], Some(expected))?;
        if right.1 != expected {
            return Err(err(
                span,
                "E_TYPE",
                "Vec equality argument must borrow the same vector type",
            ));
        }
        self.vector_equality_values(
            if name == "eq" { "==" } else { "!=" },
            borrowed,
            right.0,
            element,
            span,
        )
    }
}
