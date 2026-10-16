//! One exact closed task application. This certificate is not a language value.
use super::*;
use crate::platform::semantic_id::TypeParameterId;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;

pub(in super::super::super) struct TaskApplication {
    identity: ValueOrigin,
    function: FunctionIndex,
    types: Arc<[TypeObjectDigest]>,
    result: TypeObjectDigest,
    canonical_result: crate::platform::kernel::CallableResult<u32>,
    parameters: Option<Vec<TypeObjectDigest>>,
    bindings: BTreeMap<TypeParameterId, TypeObjectDigest>,
    canonical_parameters: Arc<[super::super::super::prepare::NormalizedParameter]>,
    implementations: Arc<[super::super::super::prepare::NormalizedImplementationArgument]>,
    scope_parameters: Arc<[crate::platform::kernel::ReadScopeParameter]>,
    scope_sources: Arc<[crate::platform::kernel::ReadScopeSource<u32>]>,
    scopes: Arc<[crate::platform::kernel::ReadScopeArgument]>,
}
impl TaskApplication {
    pub(in super::super::super) fn application_identity(&self) -> ValueOrigin {
        self.identity
    }
    pub(in super::super::super) fn read_scope_arguments(
        &self,
    ) -> &Arc<[crate::platform::kernel::ReadScopeArgument]> {
        &self.scopes
    }
    pub(in super::super::super) fn read_scope_source_count(&self, position: usize) -> usize {
        self.scope_sources
            .iter()
            .filter(|source| source.source as usize == position)
            .count()
    }
    pub(in super::super::super) fn visit_read_scope_keys(
        &self,
        position: usize,
        visit: &mut impl FnMut(crate::platform::kernel::ReadScopeArgument) -> Result<(), ExecutionError>,
    ) -> Result<(), ExecutionError> {
        for source in self
            .scope_sources
            .iter()
            .filter(|source| source.source as usize == position)
        {
            let crate::platform::kernel::ReadScopeArgument::Parameter(reference) = source.scope
            else {
                return Err(reject());
            };
            let index = self
                .scope_parameters
                .iter()
                .position(|parameter| parameter.id == reference.parameter)
                .ok_or_else(reject)?;
            visit(*self.scopes.get(index).ok_or_else(reject)?)?;
        }
        Ok(())
    }
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
            || !matches!(
                f.result.relation,
                crate::platform::kernel::ResultRelation::Value
            )
            || f.read_scope_parameters.len() != f.read_scope_arguments.len()
        {
            return Err(reject());
        }
        let identity = ValueOrigin::fresh().ok_or_else(reject)?;
        let mut work = Work { control, nodes: 0 };
        for (position, scope) in f.read_scope_parameters.iter().enumerate() {
            work.visit(0)?;
            for earlier in &f.read_scope_parameters[..position] {
                work.visit(0)?;
                if earlier.id == scope.id {
                    return Err(reject());
                }
            }
        }
        for (position, source) in f.read_scope_sources.iter().enumerate() {
            work.visit(0)?;
            work_scope_source(f, position, &mut work)?;
            for earlier in &f.read_scope_sources[..position] {
                work.visit(0)?;
                if earlier == source {
                    return Err(reject());
                }
            }
        }
        for (ty, constraint) in types.iter().zip(f.type_parameter_constraints.iter()) {
            work.visit(0)?;
            let region = matches!(
                program.types.get(ty).map(|t| &t.form),
                Some(TypeForm::RegionBrand { .. })
            );
            if region || constraint.is_region() {
                if region != constraint.is_region() {
                    return Err(reject());
                }
                continue;
            }
            let owned = closed_owned_type(program, *ty, &mut work)?;
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
        let context = super::super::super::prepare::PreparedApplicationContext::new(
            program, function, &bindings, control,
        )
        .map_err(types::context_error)?;
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
            if parameter.contract != application.contract
                || types::resolve_application_in(program, parameter.self_type, &context, &mut work)?
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
                if types::resolve_application_in(program, *expected, &context, &mut work)?
                    != *actual
                {
                    return Err(reject());
                }
            }
        }
        if types.is_empty()
            && f.implementation_arguments.is_empty()
            && f.read_scope_parameters.is_empty()
        {
            for parameter in f.parameters.iter() {
                let ty = types::resolve_application_in(program, parameter.ty, &context, &mut work)?;
                if ty != parameter.ty {
                    return Err(reject());
                }
                validate_parameter_type(program, ty, parameter.use_mode, &mut work)?;
            }
            let result = types::resolve_application_in(program, f.result.ty, &context, &mut work)?;
            validate_type(program, result, 0, &mut work)?;
            return Ok(Self {
                identity,
                function,
                types,
                result,
                canonical_result: f.result,
                parameters: None,
                bindings,
                canonical_parameters: Arc::clone(&f.parameters),
                implementations: Arc::clone(&f.implementation_arguments),
                scope_parameters: Arc::clone(&f.read_scope_parameters),
                scope_sources: Arc::clone(&f.read_scope_sources),
                scopes: Arc::clone(&f.read_scope_arguments),
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
            let ty = types::resolve_application_in(program, p.ty, &context, &mut work)?;
            validate_parameter_type(program, ty, p.use_mode, &mut work)?;
            parameters.push(ty);
        }
        let result = types::resolve_application_in(program, f.result.ty, &context, &mut work)?;
        validate_type(program, result, 0, &mut work)?;
        Ok(Self {
            identity,
            function,
            types,
            result,
            canonical_result: f.result,
            parameters: Some(parameters),
            bindings,
            canonical_parameters: Arc::clone(&f.parameters),
            implementations: Arc::clone(&f.implementation_arguments),
            scope_parameters: Arc::clone(&f.read_scope_parameters),
            scope_sources: Arc::clone(&f.read_scope_sources),
            scopes: Arc::clone(&f.read_scope_arguments),
        })
    }

    pub(super) fn matches(
        &self,
        program: &NormalizedProgram,
        control: &ExecutionControl,
    ) -> Result<bool, ExecutionError> {
        control.check()?;
        let Some(function) = program.functions.get(self.function.0 as usize) else {
            return Ok(false);
        };
        if self.function.1 != program.value_origin
            || !function.graph_function
            || !matches!(&function.effect, FunctionEffect::Task { requirements, effect_parameters } if requirements.is_empty() && effect_parameters.is_empty())
            || !function.task_requirements.is_empty()
            || !function.effect_parameters.is_empty()
            || !function.requirement_parameters.is_empty()
            || !matches!(
                function.result.relation,
                crate::platform::kernel::ResultRelation::Value
            )
            || function.result != self.canonical_result
            || function.type_parameters.len() != self.types.len()
            || !Arc::ptr_eq(&function.parameters, &self.canonical_parameters)
            || !Arc::ptr_eq(&function.implementation_arguments, &self.implementations)
            || !Arc::ptr_eq(&function.read_scope_parameters, &self.scope_parameters)
            || !Arc::ptr_eq(&function.read_scope_sources, &self.scope_sources)
            || !Arc::ptr_eq(&function.read_scope_arguments, &self.scopes)
        {
            return Ok(false);
        }
        let context = super::super::super::prepare::PreparedApplicationContext::new(
            program,
            self.function,
            &self.bindings,
            control,
        )
        .map_err(types::context_error)?;
        let mut work = Work { control, nodes: 0 };
        if types::resolve_application_in(program, function.result.ty, &context, &mut work)?
            != self.result
        {
            return Ok(false);
        }
        for (index, parameter) in function.parameters.iter().enumerate() {
            if types::resolve_application_in(program, parameter.ty, &context, &mut work)?
                != self.parameter(program, index)?
            {
                return Ok(false);
            }
        }
        Ok(true)
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

fn work_scope_source(
    function: &super::super::super::prepare::NormalizedFunction,
    position: usize,
    work: &mut Work<'_>,
) -> Result<(), ExecutionError> {
    let source = function
        .read_scope_sources
        .get(position)
        .ok_or_else(reject)?;
    let parameter = function
        .parameters
        .get(source.source as usize)
        .ok_or_else(reject)?;
    if parameter.use_mode != ParameterUse::Borrow || parameter.resource_requirement.is_some() {
        return Err(reject());
    }
    let crate::platform::kernel::ReadScopeArgument::Parameter(reference) = source.scope else {
        return Err(reject());
    };
    if reference.binder
        != crate::platform::kernel::ReadScopeBinder::Declaration(function.declaration)
    {
        return Err(reject());
    }
    for parameter in function.read_scope_parameters.iter() {
        work.visit(0)?;
        if parameter.id == reference.parameter {
            return Ok(());
        }
    }
    Err(reject())
}

/// Metadata actuals prove their declared generic obligations; they do not gain
/// moving or sharing obligations merely by appearing in a callable application.
fn closed_owned_type(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    work: &mut Work<'_>,
) -> Result<bool, ExecutionError> {
    if types::resolve_in(program, ty, &BTreeMap::new(), work)? != ty {
        return Err(reject());
    }
    let object = program.types.get(&ty).ok_or_else(reject)?;
    if matches!(object.form, TypeForm::TypeParameter { .. }) {
        return Err(reject());
    }
    Ok(program.is_owned_type(ty))
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
    if !application.associated_types.is_empty() {
        types::validate_associated_bindings_in(program, application, work)?;
    }
    if visited.contains(&application.identity) {
        return Ok(());
    }
    reserve((std::mem::size_of::<u32>() + 3 * std::mem::size_of::<usize>()) as u64)?;
    visited.insert(application.identity);
    if !closed_owned_type(program, application.self_type, work)? {
        return Err(reject());
    }
    for (arguments, constraints, contract) in [
        (
            &application.implementation_type_arguments,
            &application.implementation_type_parameter_constraints,
            false,
        ),
        (
            &application.type_arguments,
            &application.contract_type_parameter_constraints,
            true,
        ),
    ] {
        if arguments.len() != constraints.len() {
            return Err(reject());
        }
        for (ty, constraint) in arguments.iter().zip(constraints.iter()) {
            work.visit(0)?;
            let owned = closed_owned_type(program, *ty, work)?;
            if (contract
                && !matches!(constraint,
                crate::platform::kernel::TypeParameterConstraints::Owned
                | crate::platform::kernel::TypeParameterConstraints::Transferable
                | crate::platform::kernel::TypeParameterConstraints::CaptureSafeTransferable))
                || (!constraint.has_owned()
                    && !matches!(constraint,
                    crate::platform::kernel::TypeParameterConstraints::Transferable
                    | crate::platform::kernel::TypeParameterConstraints::CaptureSafeTransferable))
                || owned != constraint.has_owned()
                || (!owned
                    && (!program.buffer_free_types.contains(ty)
                        || !program.comparable_types.contains(ty)))
                || (constraint.requires_capture_safe() && !program.capture_safe_types.contains(ty))
            {
                return Err(reject());
            }
            if constraint.requires_transfer() {
                validate_type(program, *ty, 0, work)?;
            }
            if constraint.requires_share() && !validate_shareable_type(program, *ty, 0, work)? {
                return Err(reject());
            }
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
    fn parallel_mixed_witness_admission_requires_complete_exact_formal_categories() {
        use crate::platform::execution::normalized::tests::mixed_storage_contract_tests::prepared_mixed;
        use crate::platform::kernel::TypeParameterConstraints as C;
        let base = prepared_mixed();
        let control = ExecutionControl::uncancelled();
        for fault in 0..5 {
            let mut program = base.clone();
            let node = &mut Arc::make_mut(&mut program.implementation_applications)[0];
            match fault {
                1 => Arc::make_mut(node).implementation_type_parameter_constraints = Arc::from([]),
                2 => {
                    Arc::make_mut(node).implementation_type_parameter_constraints =
                        Arc::from([C::Owned])
                }
                3 => Arc::make_mut(node).contract_type_parameter_constraints = Arc::from([]),
                4 => {
                    Arc::make_mut(node).contract_type_parameter_constraints = Arc::from([C::Owned])
                }
                _ => {}
            }
            let node = &program.implementation_applications[0];
            let result = validate_implementation(
                &program,
                node,
                0,
                &mut Work {
                    control: &control,
                    nodes: 0,
                },
                &mut BTreeSet::new(),
                &mut |_| Ok(()),
            );
            assert_eq!(result.is_ok(), fault == 0, "fault {fault}");
        }
    }

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
