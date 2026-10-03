//! One exact closed task application. This certificate is not a language value.
use super::*;
use crate::platform::semantic_id::TypeParameterId;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(in super::super::super) struct TaskApplication {
    function: FunctionIndex,
    types: Arc<[TypeObjectDigest]>,
    result: TypeObjectDigest,
    canonical_result: TypeObjectDigest,
    parameters: Option<Vec<TypeObjectDigest>>,
}
impl TaskApplication {
    pub(in super::super::super) fn function(&self) -> FunctionIndex {
        self.function
    }
    pub(in super::super::super) fn types(&self) -> &Arc<[TypeObjectDigest]> {
        &self.types
    }
    pub(in super::super::super) fn result(&self) -> TypeObjectDigest {
        self.result
    }

    pub(in super::super::super) fn bind(
        program: &NormalizedProgram,
        function: FunctionIndex,
        types: Arc<[TypeObjectDigest]>,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        let f = program
            .functions
            .get(function.0 as usize)
            .filter(|_| function.1 == program.value_origin)
            .ok_or_else(reject)?;
        if !f.graph_function
            || !matches!(&f.effect, FunctionEffect::Task { requirements, effect_parameters } if requirements.is_empty() && effect_parameters.is_empty())
            || !f.task_requirements.is_empty()
            || !f.effect_parameters.is_empty()
            || !f.requirement_parameters.is_empty()
            || f.type_parameters.len() != types.len()
            || f.type_parameter_constraints.len() != types.len()
            || f.implementation_parameters.len() != f.implementation_arguments.len()
        {
            return Err(reject());
        }
        let mut work = Work { control, nodes: 0 };
        for (ty, constraint) in types.iter().zip(f.type_parameter_constraints.iter()) {
            let owned = validate_type(program, *ty, 0, &mut work)?;
            if owned != constraint.has_owned()
                || (constraint.requires_capture_safe() && !program.capture_safe_types.contains(ty))
            {
                return Err(reject());
            }
        }
        if types.is_empty() {
            if !f.implementation_arguments.is_empty() {
                return Err(reject());
            }
            validate_type(program, f.result, 0, &mut work)?;
            return Ok(Self {
                function,
                types,
                result: f.result,
                canonical_result: f.result,
                parameters: None,
            });
        }
        // Reserve invocation scratch and the retained resolved signature before
        // allocating either. No payload data is copied by signature preparation.
        reserve(super::super::super::value::collection_storage_bytes(
            types.len() as u64,
            (std::mem::size_of::<(TypeParameterId, TypeObjectDigest)>()
                + 3 * std::mem::size_of::<usize>()) as u64,
            "normalized_parallel_application",
        )?)?;
        let bindings: BTreeMap<_, _> = f
            .type_parameters
            .iter()
            .copied()
            .zip(types.iter().copied())
            .collect();
        if bindings.len() != types.len() {
            return Err(reject());
        }
        for (parameter, (_, self_type)) in f
            .implementation_parameters
            .iter()
            .zip(f.implementation_arguments.iter())
        {
            work.visit(0)?;
            if resolve_type(program, parameter.self_type, &bindings, control)? != *self_type {
                return Err(reject());
            }
        }
        reserve(super::super::super::value::collection_storage_bytes(
            f.parameters.len() as u64,
            std::mem::size_of::<TypeObjectDigest>() as u64,
            "normalized_parallel_application",
        )?)?;
        let mut parameters = Vec::with_capacity(f.parameters.len());
        for p in f.parameters.iter() {
            work.visit(0)?;
            let ty = resolve_type(program, p.ty, &bindings, control)?;
            validate_type(program, ty, 0, &mut work)?;
            parameters.push(ty);
        }
        let result = resolve_type(program, f.result, &bindings, control)?;
        validate_type(program, result, 0, &mut work)?;
        Ok(Self {
            function,
            types,
            result,
            canonical_result: f.result,
            parameters: Some(parameters),
        })
    }

    pub(super) fn matches(&self, program: &NormalizedProgram) -> bool {
        self.function.1 == program.value_origin
            && program
                .functions
                .get(self.function.0 as usize)
                .is_some_and(|f| {
                    f.result == self.canonical_result && f.type_parameters.len() == self.types.len()
                })
    }

    pub(in super::super::super) fn parameter(
        &self,
        program: &NormalizedProgram,
        index: usize,
    ) -> Result<TypeObjectDigest, ExecutionError> {
        match &self.parameters {
            Some(parameters) => parameters.get(index).copied(),
            None => program
                .functions
                .get(self.function.0 as usize)
                .and_then(|f| f.parameters.get(index))
                .map(|p| p.ty),
        }
        .ok_or_else(reject)
    }
}
