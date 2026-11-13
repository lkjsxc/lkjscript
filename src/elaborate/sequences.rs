//! Vec operations call ordinary, resolved source-library functions.
use super::{Builder, Compiler, Result, a, err, k};
use std::collections::BTreeMap;

impl Builder<'_, '_> {
    fn sequence_call(
        &mut self,
        name: &str,
        element: usize,
        arguments: Vec<usize>,
        result: usize,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let function = self.compiler.sequence_instance(name, element, span)?;
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
                "source sequence helper signature differs from its operation",
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

    pub(super) fn vector_repeat(
        &mut self,
        value: usize,
        count: usize,
        element: usize,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        if !self.compiler.satisfies_bound(element, "Clone") {
            return Err(err(
                span,
                "E_TRAIT",
                "Vec repetition requires element Clone",
            ));
        }
        let vector = self
            .compiler
            .intern(String::new(), k::TypeKind::Vec { inner: element });
        self.sequence_call(
            "__lkj_vec_repeat",
            element,
            vec![value, count],
            vector,
            span,
        )
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
}

impl Compiler<'_> {
    pub(super) fn sequence_instance(
        &mut self,
        name: &str,
        element: usize,
        span: a::Span,
    ) -> Result<usize> {
        let source = self.names.sequence_function(name, span)?;
        let parameters: Vec<_> = self.source.functions[source]
            .generics
            .iter()
            .filter(|parameter| !parameter.lifetime)
            .collect();
        if parameters.len() != 1 {
            return Err(err(
                span,
                "E_TYPE",
                "source sequence helper requires one element type parameter",
            ));
        }
        let mut substitutions = BTreeMap::new();
        substitutions.insert(parameters[0].name.clone(), element);
        let function = self.instantiate(source, substitutions)?;
        let vector = self.intern(String::new(), k::TypeKind::Vec { inner: element });
        let reference = self.reference(vector, false);
        let slice = self.intern(
            String::new(),
            k::TypeKind::Slice {
                inner: element,
                mutable: false,
            },
        );
        let slice_reference = self.reference(slice, false);
        let (parameters, result) = match name {
            "__lkj_vec_clone" => (vec![reference], vector),
            "__lkj_vec_equal" => (vec![reference, reference], k::BOOL),
            "__lkj_vec_order" => (vec![reference, reference], k::I32),
            "__lkj_slice_equal" => (vec![slice_reference, slice_reference], k::BOOL),
            "__lkj_slice_order" => (vec![slice_reference, slice_reference], k::I32),
            "__lkj_vec_repeat" => (vec![element, k::U64], vector),
            _ => {
                return Err(err(
                    span,
                    "E_TYPE",
                    "unknown source sequence helper operation",
                ));
            }
        };
        let contract = &self.functions[function];
        if contract.params != parameters.len()
            || contract.result != result
            || contract
                .locals
                .iter()
                .zip(&parameters)
                .any(|(local, expected)| local.ty != *expected)
        {
            return Err(err(
                span,
                "E_TYPE",
                "source sequence helper signature differs from its operation",
            ));
        }
        Ok(function)
    }
}
