//! One exact closed task application. This certificate is not a language value.
use super::*;
use crate::platform::semantic_id::TypeParameterId;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
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
            || (!f.type_arguments.is_empty() && f.type_arguments.as_ref() != types.as_ref())
            || f.implementation_parameters.len() != f.implementation_arguments.len()
            || f.result_borrow.is_some()
        {
            return Err(reject());
        }
        let mut work = Work { control, nodes: 0 };
        for (ty, constraint) in types.iter().zip(f.type_parameter_constraints.iter()) {
            work.visit(0)?;
            let owned = closed_owned_type(program, *ty)?;
            if owned != constraint.has_owned()
                || (!owned && !program.buffer_free_types.contains(ty))
                || (constraint.requires_capture_safe() && !program.capture_safe_types.contains(ty))
            {
                return Err(reject());
            }
            if constraint.requires_transfer() {
                validate_type(program, *ty, 0, &mut work)?;
            }
            if constraint.requires_share() && !validate_shareable_type(program, *ty, 0, &mut work)?
            {
                return Err(reject());
            }
        }
        // Reserve invocation scratch and the retained resolved signature before
        // allocating either. No payload data is copied by signature preparation.
        if !types.is_empty() {
            reserve(super::super::super::value::collection_storage_bytes(
                types.len() as u64,
                (std::mem::size_of::<(TypeParameterId, TypeObjectDigest)>()
                    + 3 * std::mem::size_of::<usize>()) as u64,
                "normalized_parallel_application",
            )?)?;
        }
        let bindings: BTreeMap<_, _> = f
            .type_parameters
            .iter()
            .copied()
            .zip(types.iter().copied())
            .collect();
        if bindings.len() != types.len() {
            return Err(reject());
        }
        let mut witness_visited = BTreeSet::new();
        for (parameter, application) in f
            .implementation_parameters
            .iter()
            .zip(f.implementation_arguments.iter())
        {
            work.visit(0)?;
            validate_implementation(
                program,
                application,
                0,
                &mut work,
                &mut witness_visited,
                reserve,
            )?;
            for ty in application.implementation_type_arguments.iter() {
                if !closed_owned_type(program, *ty)? {
                    return Err(reject());
                }
            }
            if parameter.contract != application.contract
                || resolve_type(program, parameter.self_type, &bindings, control)?
                    != application.self_type
                || parameter.type_arguments.len() != application.type_arguments.len()
            {
                return Err(reject());
            }
            for (expected, actual) in parameter
                .type_arguments
                .iter()
                .zip(application.type_arguments.iter())
            {
                work.visit(0)?;
                if resolve_type(program, *expected, &bindings, control)? != *actual
                    || !closed_owned_type(program, *actual)?
                {
                    return Err(reject());
                }
            }
        }
        if types.is_empty() {
            validate_type(program, f.result, 0, &mut work)?;
            return Ok(Self {
                function,
                types,
                result: f.result,
                canonical_result: f.result,
                parameters: None,
            });
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
            validate_parameter_type(program, ty, p.use_mode, &mut work)?;
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

/// Metadata actuals prove their declared generic obligations; they do not gain
/// moving or sharing obligations merely by appearing in a callable application.
fn closed_owned_type(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
) -> Result<bool, ExecutionError> {
    let object = program.types.get(&ty).ok_or_else(reject)?;
    if matches!(object.form, TypeForm::TypeParameter { .. }) {
        return Err(reject());
    }
    Ok(matches!(
        object.form,
        TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::OwnedProduct { .. }
            | TypeForm::OwnedChoice { .. }
            | TypeForm::OwnedSequence { .. }
    ))
}

fn validate_parameter_type(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    mode: ParameterUse,
    work: &mut Work<'_>,
) -> Result<(), ExecutionError> {
    match mode {
        ParameterUse::Borrow if validate_shareable_type(program, ty, 0, work)? => Ok(()),
        ParameterUse::Consume if validate_type(program, ty, 0, work)? => Ok(()),
        ParameterUse::Unrestricted if !validate_type(program, ty, 0, work)? => Ok(()),
        _ => Err(reject()),
    }
}

fn validate_implementation(
    program: &NormalizedProgram,
    application: &super::super::super::prepare::NormalizedImplementationArgument,
    depth: usize,
    work: &mut Work<'_>,
    visited: &mut BTreeSet<u32>,
    reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
) -> Result<(), ExecutionError> {
    work.visit(depth)?;
    let canonical = program
        .implementation_applications
        .get(application.identity as usize)
        .ok_or_else(reject)?;
    // Only the exact immutable node admitted by this preparation is a valid handle.
    if canonical.identity != application.identity || !Arc::ptr_eq(canonical, application) {
        return Err(reject());
    }
    if depth.saturating_add(canonical.depth) > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH
    {
        return Err(reject());
    }
    if visited.contains(&application.identity) {
        return Ok(());
    }
    reserve((std::mem::size_of::<u32>() + 3 * std::mem::size_of::<usize>()) as u64)?;
    visited.insert(application.identity);
    for ty in std::iter::once(&application.self_type)
        .chain(application.implementation_type_arguments.iter())
        .chain(application.type_arguments.iter())
    {
        work.visit(0)?;
        if !closed_owned_type(program, *ty)? {
            return Err(reject());
        }
    }
    if application.implementations.len() != application.prerequisites.len() {
        return Err(reject());
    }
    for (child, obligation) in application
        .implementations
        .iter()
        .zip(application.prerequisites.iter())
    {
        if child.contract != obligation.contract
            || child.self_type != obligation.self_type
            || child.type_arguments != obligation.type_arguments
        {
            return Err(reject());
        }
        validate_implementation(program, child, depth + 1, work, visited, reserve)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::execution::normalized::tests::owned_implementation_scheme_tests::prepared_duplicate_dag;

    #[test]
    fn parallel_admission_visits_a_depth_24_binary_dag_linearly() {
        let program = prepared_duplicate_dag(24);
        let root = program.implementation_applications.last().unwrap();
        let control = ExecutionControl::uncancelled();
        let mut work = Work {
            control: &control,
            nodes: 0,
        };
        let mut visited = BTreeSet::new();
        let mut reservations = 0;
        validate_implementation(&program, root, 0, &mut work, &mut visited, &mut |_| {
            reservations += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(visited.len(), 25);
        assert_eq!(reservations, 25);
        assert!(work.nodes < 256, "type and edge admission remains linear");
        validate_implementation(&program, root, 0, &mut work, &mut visited, &mut |_| {
            panic!("repeated supplying witnesses reuse the same admission ledger")
        })
        .unwrap();
    }

    #[test]
    fn parallel_witness_handle_rejects_equal_record_before_memo_skip() {
        let program = prepared_duplicate_dag(24);
        let root = program.implementation_applications.last().unwrap();
        let equal_copy = Arc::new((**root).clone());
        let control = ExecutionControl::uncancelled();
        let mut work = Work {
            control: &control,
            nodes: 0,
        };
        let mut visited = BTreeSet::from([root.identity]);
        assert!(
            validate_implementation(
                &program,
                &equal_copy,
                0,
                &mut work,
                &mut visited,
                &mut |_| panic!("equal copied record is not this canonical handle")
            )
            .is_err()
        );
        assert_eq!(visited, BTreeSet::from([root.identity]));
    }

    #[test]
    fn parallel_dag_admission_rejects_a_private_mutated_duplicate_before_memo_skip() {
        let program = prepared_duplicate_dag(24);
        let root = program.implementation_applications.last().unwrap();
        let mut forged = root.clone();
        let duplicate =
            Arc::make_mut(&mut Arc::make_mut(&mut Arc::make_mut(&mut forged).implementations)[1]);
        Arc::make_mut(&mut Arc::make_mut(&mut duplicate.implementations)[0]).contract =
            root.implementation;
        let control = ExecutionControl::uncancelled();
        let mut work = Work {
            control: &control,
            nodes: 0,
        };
        let mut visited = BTreeSet::from([root.identity]);
        assert!(
            validate_implementation(&program, &forged, 0, &mut work, &mut visited, &mut |_| {
                Ok(())
            })
            .is_err()
        );
        assert_eq!(visited.len(), 1);
    }

    #[test]
    fn parallel_dag_depth_checks_the_full_shared_suffix_before_memo_skip() {
        let maximum = crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH;
        let program = prepared_duplicate_dag(maximum);
        let root = program.implementation_applications.last().unwrap();
        let control = ExecutionControl::uncancelled();
        let mut work = Work {
            control: &control,
            nodes: 0,
        };
        let mut visited = BTreeSet::new();
        validate_implementation(&program, root, 0, &mut work, &mut visited, &mut |_| Ok(()))
            .unwrap();
        assert_eq!(visited.len(), maximum + 1);
        assert!(
            validate_implementation(&program, root, 1, &mut work, &mut visited, &mut |_| {
                panic!("an over-depth shared path is refused before memo allocation")
            })
            .is_err()
        );
    }
}
