use std::collections::{HashMap, HashSet};

use lkjscript_core::{Error, Result};

use crate::hir::{
    Binding, BindingId, BindingKind, BindingStorage, Expr, ExprKind, LexicalLoopContext,
    MatchEdgeTarget, MatchPattern, Operation, Origin, Program, SourceId, Type,
};

struct DeclarationIndexes<'a> {
    products_by_name: HashMap<&'a str, usize>,
    product_ids_by_name: HashMap<String, lkjscript_core::ProductId>,
    implementation_index:
        HashMap<(crate::hir::TraitId, lkjscript_core::ProductId), crate::hir::ImplId>,
    enums_by_id: HashMap<crate::hir::EnumId, usize>,
}

impl<'a> DeclarationIndexes<'a> {
    fn build(program: &'a Program) -> Result<Self> {
        let mut products_by_name = HashMap::new();
        products_by_name
            .try_reserve(program.products.len())
            .map_err(|_| Error::host("HIR product name index allocation failed"))?;
        for (index, product) in program.products.iter().enumerate() {
            if products_by_name
                .insert(product.name.as_str(), index)
                .is_some()
            {
                return Err(Error::msg("HIR product declaration name is duplicated"));
            }
        }
        let mut product_ids_by_name = HashMap::new();
        product_ids_by_name
            .try_reserve(program.products.len())
            .map_err(|_| Error::host("HIR product identity index allocation failed"))?;
        for product in &program.products {
            product_ids_by_name.insert(product.name.clone(), product.id);
        }
        let mut implementation_index = HashMap::new();
        implementation_index
            .try_reserve(program.implementations.len())
            .map_err(|_| Error::host("HIR implementation index allocation failed"))?;
        for implementation in &program.implementations {
            if implementation_index
                .insert(
                    (implementation.trait_id, implementation.product),
                    implementation.id,
                )
                .is_some()
            {
                return Err(Error::msg("HIR implementation facts overlap"));
            }
        }
        let mut enums_by_id = HashMap::new();
        enums_by_id
            .try_reserve(program.enums.len())
            .map_err(|_| Error::host("HIR enum identity index allocation failed"))?;
        for (index, definition) in program.enums.iter().enumerate() {
            if enums_by_id.insert(definition.id, index).is_some() {
                return Err(Error::msg("HIR enum identity is duplicated"));
            }
        }
        Ok(Self {
            products_by_name,
            product_ids_by_name,
            implementation_index,
            enums_by_id,
        })
    }
}

pub(super) fn program(program: &Program) -> Result<()> {
    let declarations = DeclarationIndexes::build(program)?;
    validate_sources(program)?;
    validate_bindings(program, &declarations)?;
    validate_declarations(program, &declarations)?;
    validate_main(program, &declarations)?;
    validate_functions(program)?;
    validate_global_layout(program)?;
    validate_match_plans(program, &declarations)?;
    validate_expressions(program, &declarations)?;
    Ok(())
}

fn validate_sources(program: &Program) -> Result<()> {
    let mut paths = HashSet::new();
    paths
        .try_reserve(program.sources.len())
        .map_err(|_| Error::host("HIR source consistency allocation failed"))?;
    for (index, source) in program.sources.iter().enumerate() {
        require_dense(source.id.raw(), index, "HIR source")?;
        if !paths.insert(&source.path) {
            return Err(Error::msg("HIR source paths are not unique"));
        }
    }
    Ok(())
}

fn validate_bindings(program: &Program, declarations: &DeclarationIndexes) -> Result<()> {
    for (index, binding) in program.bindings.iter().enumerate() {
        require_dense(binding.id.raw(), index, "HIR binding")?;
        if binding.name.is_empty() {
            return Err(Error::msg("HIR binding has an empty name"));
        }
        validate_origin(program, binding.origin)?;
        validate_type(program, declarations, &binding.ty)?;
        match (&binding.kind, binding.origin) {
            (BindingKind::BuiltinOperation(_), Origin::Builtin) => {}
            (BindingKind::BuiltinOperation(_), Origin::Source(_) | Origin::Semantic) => {
                return Err(Error::msg("builtin operation has a non-builtin origin"));
            }
            (_, Origin::Builtin) => {
                return Err(Error::msg("ordinary binding has a builtin origin"));
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_declarations(program: &Program, declarations: &DeclarationIndexes) -> Result<()> {
    let mut product_identities = HashSet::new();
    product_identities
        .try_reserve(program.products.len())
        .map_err(|_| Error::host("HIR product identity allocation failed"))?;
    let product_field_count = program
        .products
        .iter()
        .try_fold(0_usize, |count, product| {
            count.checked_add(product.fields.len())
        })
        .ok_or_else(|| Error::host("HIR product field identity count overflow"))?;
    let mut product_field_identities = HashSet::new();
    product_field_identities
        .try_reserve(product_field_count)
        .map_err(|_| Error::host("HIR product field identity allocation failed"))?;
    for (index, product) in program.products.iter().enumerate() {
        require_dense(product.id.raw(), index, "HIR product")?;
        validate_origin(program, product.origin)?;
        if product.origin == Origin::Builtin {
            return Err(Error::msg("user product has a builtin origin"));
        }
        if product.identity == [0; 32]
            || !product_identities.insert(product.identity)
            || product.name.is_empty()
        {
            return Err(Error::msg("HIR product identity or name is invalid"));
        }
        for (field_index, field) in product.fields.iter().enumerate() {
            require_dense(field.source_order, field_index, "HIR product field")?;
            if field.identity == [0; 32]
                || !product_field_identities.insert(field.identity)
                || field.name.is_empty()
            {
                return Err(Error::msg("HIR product field identity or name is invalid"));
            }
            validate_type(program, declarations, &field.ty)?;
        }
    }

    let mut enum_ids = HashSet::new();
    enum_ids
        .try_reserve(program.enums.len())
        .map_err(|_| Error::host("HIR enum identity consistency allocation failed"))?;
    for definition in &program.enums {
        if !definition.id.is_resolved()
            || !enum_ids.insert(definition.id)
            || !definition.layout.identity.is_resolved()
            || definition.name.is_empty()
        {
            return Err(Error::msg("HIR enum identity, layout, or name is invalid"));
        }
        validate_origin(program, definition.origin)?;
        let mut variants = HashSet::new();
        variants
            .try_reserve(definition.variants.len())
            .map_err(|_| Error::host("HIR variant identity consistency allocation failed"))?;
        for (variant_index, variant) in definition.variants.iter().enumerate() {
            require_dense(variant.source_order, variant_index, "HIR enum variant")?;
            if !variant.id.is_resolved() || !variants.insert(variant.id) || variant.name.is_empty()
            {
                return Err(Error::msg("HIR enum variant identity or name is invalid"));
            }
            let mut fields = HashSet::new();
            fields
                .try_reserve(variant.fields.len())
                .map_err(|_| Error::host("HIR enum field consistency allocation failed"))?;
            for (field_index, field) in variant.fields.iter().enumerate() {
                require_dense(field.source_order, field_index, "HIR enum field")?;
                if !field.id.is_resolved() || !fields.insert(field.id) || field.name.is_empty() {
                    return Err(Error::msg("HIR enum field identity or name is invalid"));
                }
                validate_type(program, declarations, &field.ty)?;
            }
        }
    }

    for (index, definition) in program.traits.iter().enumerate() {
        require_dense(definition.id.raw(), index, "HIR trait")?;
        validate_origin(program, definition.origin)?;
        if definition.name.is_empty() {
            return Err(Error::msg("HIR trait has an empty name"));
        }
    }
    let mut pairs = HashSet::new();
    pairs
        .try_reserve(program.implementations.len())
        .map_err(|_| Error::host("HIR implementation consistency allocation failed"))?;
    for (index, implementation) in program.implementations.iter().enumerate() {
        require_dense(implementation.id.raw(), index, "HIR implementation")?;
        validate_source(program, implementation.origin)?;
        require_index(
            implementation.trait_id.raw(),
            program.traits.len(),
            "HIR implementation trait",
        )?;
        require_index(
            implementation.product.raw(),
            program.products.len(),
            "HIR implementation product",
        )?;
        if !pairs.insert((implementation.trait_id, implementation.product)) {
            return Err(Error::msg("duplicate HIR trait implementation"));
        }
    }
    Ok(())
}

fn validate_main(program: &Program, declarations: &DeclarationIndexes) -> Result<()> {
    validate_program_origin(program, program.main.origin)?;
    if program.main.arity != program.main.params.len()
        || program.main.params.len() != program.main.param_places.len()
        || program.main.params.len() != program.main.param_types.len()
    {
        return Err(Error::msg("HIR main signature lengths are inconsistent"));
    }
    validate_type(program, declarations, &program.main.return_type)?;
    for (parameter, expected) in program.main.params.iter().zip(&program.main.param_types) {
        let binding = require_binding(program, *parameter, "HIR main parameter")?;
        if binding.kind != BindingKind::Parameter
            || binding.origin != program.main.origin
            || binding.ty != *expected
        {
            return Err(Error::msg("HIR main parameter signature is stale"));
        }
        validate_type(program, declarations, expected)?;
    }
    if Type::join_control(&program.main.body.ty, &program.main.return_type)
        != Some(program.main.return_type.clone())
    {
        return Err(Error::msg("HIR main body and return type disagree"));
    }
    Ok(())
}

fn validate_functions(program: &Program) -> Result<()> {
    let mut function_bindings = HashSet::new();
    function_bindings
        .try_reserve(program.functions.len())
        .map_err(|_| Error::host("HIR function consistency allocation failed"))?;
    for function in &program.functions {
        if !function_bindings.insert(function.binding) {
            return Err(Error::msg("HIR function binding is duplicated"));
        }
        if !function.summary.is_known() {
            return Err(Error::msg("complete HIR function has unknown effects"));
        }
        validate_program_origin(program, function.origin)?;
        let binding = require_binding(program, function.binding, "HIR function")?;
        if binding.kind != BindingKind::Function
            || binding.origin != function.origin
            || function.arity != function.params.len()
            || function.params.len() != function.param_places.len()
        {
            return Err(Error::msg("HIR function header is inconsistent"));
        }
        let (variables, signature) = match &binding.ty {
            Type::Forall { vars, body } => (vars.as_slice(), body.as_ref()),
            other => (&[][..], other),
        };
        validate_function_type_parameters(variables, signature, &function.bounds)?;
        let (parameters, result) = function_signature(&binding.ty)
            .ok_or_else(|| Error::msg("HIR function binding has no function signature"))?;
        if parameters.len() != function.params.len()
            || Type::join_control(&function.body.ty, result) != Some(result.clone())
        {
            return Err(Error::msg("HIR function signature or body type is stale"));
        }
        for ((parameter, expected), place) in function
            .params
            .iter()
            .zip(parameters)
            .zip(&function.param_places)
        {
            let local = require_binding(program, *parameter, "HIR function parameter")?;
            if local.kind != BindingKind::Parameter
                || local.origin != function.origin
                || local.ty != *expected
            {
                return Err(Error::msg("HIR function parameter signature is stale"));
            }
            let _ = place;
        }
        for bound in &function.bounds {
            let index = index_of(bound.trait_id.raw(), "HIR trait bound")?;
            if program
                .traits
                .get(index)
                .is_none_or(|definition| definition.id != bound.trait_id)
            {
                return Err(Error::msg("HIR trait bound identity is stale"));
            }
        }
    }
    Ok(())
}

fn validate_function_type_parameters(
    variables: &[String],
    signature: &Type,
    bounds: &[crate::hir::TraitBound],
) -> Result<()> {
    let mut declared = HashSet::new();
    declared
        .try_reserve(variables.len())
        .map_err(|_| Error::host("HIR type-parameter set allocation failed"))?;
    for variable in variables {
        if !declared.insert(variable.as_str()) {
            return Err(Error::msg("HIR function type parameter is duplicated"));
        }
    }
    let mut used = HashSet::new();
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| Error::host("HIR type-parameter work allocation failed"))?;
    pending.push(signature);
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Param(parameter) => {
                if !declared.contains(parameter.as_str()) {
                    return Err(Error::msg(
                        "HIR function signature references an undeclared type parameter",
                    ));
                }
                used.try_reserve(1)
                    .map_err(|_| Error::host("HIR used type-parameter allocation failed"))?;
                used.insert(parameter.as_str());
            }
            Type::Enum { arguments, .. } => {
                pending
                    .try_reserve(arguments.len())
                    .map_err(|_| Error::host("HIR type-parameter work allocation failed"))?;
                pending.extend(arguments);
            }
            Type::List(inner) => {
                pending
                    .try_reserve(1)
                    .map_err(|_| Error::host("HIR type-parameter work allocation failed"))?;
                pending.push(inner);
            }
            Type::Fn { params, ret } => {
                let additional = params
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| Error::host("HIR type-parameter child count overflow"))?;
                pending
                    .try_reserve(additional)
                    .map_err(|_| Error::host("HIR type-parameter work allocation failed"))?;
                pending.push(ret);
                pending.extend(params);
            }
            Type::Forall { .. } => {
                return Err(Error::msg(
                    "HIR function signature contains an unsupported nested universal type",
                ));
            }
            _ => {}
        }
    }
    if variables
        .iter()
        .any(|variable| !used.contains(variable.as_str()))
    {
        return Err(Error::msg("HIR function type parameter is unused"));
    }
    let mut seen_bounds = HashSet::new();
    seen_bounds
        .try_reserve(bounds.len())
        .map_err(|_| Error::host("HIR trait-bound set allocation failed"))?;
    for bound in bounds {
        if !declared.contains(bound.parameter.as_str()) {
            return Err(Error::msg(
                "HIR trait bound references an undeclared type parameter",
            ));
        }
        if !seen_bounds.insert((bound.parameter.as_str(), bound.trait_id)) {
            return Err(Error::msg("HIR trait bound is duplicated"));
        }
    }
    Ok(())
}

fn validate_global_layout(program: &Program) -> Result<()> {
    if program.global_layout.len() != program.functions.len() {
        return Err(Error::msg(
            "HIR global function layout has the wrong number of entries",
        ));
    }
    let mut seen = HashSet::new();
    seen.try_reserve(program.global_layout.len())
        .map_err(|_| Error::host("HIR global layout consistency allocation failed"))?;
    for binding in &program.global_layout {
        let binding = require_binding(program, *binding, "HIR global function layout")?;
        if binding.kind != BindingKind::Function || !seen.insert(binding.id) {
            return Err(Error::msg(
                "HIR global function layout is stale or duplicated",
            ));
        }
    }
    if program
        .functions
        .iter()
        .any(|function| !seen.contains(&function.binding))
    {
        return Err(Error::msg("HIR global function layout omits a function"));
    }
    Ok(())
}

fn validate_match_plans(program: &Program, declarations: &DeclarationIndexes) -> Result<()> {
    for (index, plan) in program.match_plans.iter().enumerate() {
        require_dense(plan.id.raw(), index, "HIR match plan")?;
        validate_program_origin(program, plan.origin)?;
        if plan.arms.is_empty() || !plan.exhaustive || plan.witness.is_some() {
            return Err(Error::msg(
                "HIR complete match plan has stale completeness facts",
            ));
        }
        validate_match_local(
            program,
            declarations,
            plan.origin,
            &plan.scrutinee,
            BindingKind::MatchTemporary,
        )?;
        validate_type(program, declarations, &plan.result_type)?;
        for (arm_index, arm) in plan.arms.iter().enumerate() {
            require_dense(arm.id, arm_index, "HIR match arm")?;
            validate_type(program, declarations, &arm.body_type)?;
            if Type::join_control(&arm.body_type, &plan.result_type)
                != Some(plan.result_type.clone())
            {
                return Err(Error::msg("HIR match arm result type is stale"));
            }
            validate_pattern(
                program,
                declarations,
                plan.origin,
                &arm.pattern,
                &plan.scrutinee.ty,
            )?;
        }
        let expected_edges = plan
            .arms
            .len()
            .checked_add(1)
            .ok_or_else(|| Error::host("HIR match edge count overflow"))?;
        if plan.edges.len() != expected_edges {
            return Err(Error::msg("HIR match edge facts have the wrong length"));
        }
        for (edge_index, edge) in plan.edges.iter().enumerate() {
            let expected = if edge_index + 2 < plan.edges.len() {
                MatchEdgeTarget::Arm(
                    u64::try_from(edge_index + 1)
                        .map_err(|_| Error::host("HIR match arm edge exceeds u64"))?,
                )
            } else if edge_index + 1 < plan.edges.len() {
                MatchEdgeTarget::Default
            } else {
                MatchEdgeTarget::Unreachable
            };
            if *edge != expected {
                return Err(Error::msg("HIR match edge facts are stale"));
            }
        }
    }
    Ok(())
}

fn validate_pattern(
    program: &Program,
    declarations: &DeclarationIndexes,
    origin: Origin,
    root: &MatchPattern,
    expected: &Type,
) -> Result<()> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| Error::host("HIR match pattern work allocation failed"))?;
    pending.push((root, expected.clone()));
    while let Some((pattern, expected)) = pending.pop() {
        if pattern.ty() != expected {
            return Err(Error::msg("HIR match pattern type is stale"));
        }
        match pattern {
            MatchPattern::Wildcard { ty } => validate_type(program, declarations, ty)?,
            MatchPattern::Binding { local } => {
                validate_match_local(
                    program,
                    declarations,
                    origin,
                    local,
                    BindingKind::ImmutableLocal,
                )?;
            }
            MatchPattern::Bool(_) if expected == Type::Bool => {}
            MatchPattern::I64(_) if expected == Type::I64 => {}
            MatchPattern::Variant {
                ty,
                enum_id,
                variant,
                layout,
                fields,
            } => {
                let Type::Enum { id, arguments, .. } = ty else {
                    return Err(Error::msg("HIR variant pattern lost its enum type"));
                };
                if id != enum_id {
                    return Err(Error::msg("HIR match pattern enum identity is stale"));
                }
                let definition = declarations
                    .enums_by_id
                    .get(enum_id)
                    .and_then(|index| program.enums.get(*index))
                    .ok_or_else(|| Error::msg("HIR match pattern has a stale enum identity"))?;
                if definition.layout.identity != *layout {
                    return Err(Error::msg("HIR match pattern has a stale enum layout"));
                }
                let selected = definition
                    .variants
                    .iter()
                    .find(|item| item.id == *variant)
                    .ok_or_else(|| Error::msg("HIR match pattern has a stale variant identity"))?;
                if fields.len() != selected.fields.len() {
                    return Err(Error::msg("HIR match pattern has stale variant fields"));
                }
                if definition.type_parameters.len() != arguments.len() {
                    return Err(Error::msg("HIR match enum arguments are stale"));
                }
                let mut substitutions = HashMap::new();
                substitutions
                    .try_reserve(definition.type_parameters.len())
                    .map_err(|_| Error::host("HIR match substitution allocation failed"))?;
                for (parameter, argument) in definition.type_parameters.iter().zip(arguments) {
                    substitutions.insert(parameter.as_str(), argument);
                }
                pending
                    .try_reserve(fields.len())
                    .map_err(|_| Error::host("HIR match pattern work allocation failed"))?;
                for (field, declared) in fields.iter().zip(&selected.fields) {
                    if field.name != declared.name || field.field_index != declared.source_order {
                        return Err(Error::msg("HIR match field identity is stale"));
                    }
                    let field_type = substitute_hir_type(&declared.ty, &substitutions)?;
                    match (&field.projection, &field.pattern) {
                        (None, MatchPattern::Wildcard { .. }) => {}
                        (Some(local), pattern)
                            if !matches!(pattern, MatchPattern::Wildcard { .. })
                                && local.ty == field_type =>
                        {
                            validate_match_local(
                                program,
                                declarations,
                                origin,
                                local,
                                BindingKind::MatchTemporary,
                            )?;
                        }
                        _ => {
                            return Err(Error::msg(
                                "HIR match wildcard/projection metadata is stale",
                            ))
                        }
                    }
                    pending.push((&field.pattern, field_type));
                }
            }
            MatchPattern::Product {
                ty,
                product,
                fields,
            } => {
                let Type::Product(name) = ty else {
                    return Err(Error::msg("HIR product pattern lost its product type"));
                };
                let definition = program
                    .products
                    .get(index_of(product.raw(), "HIR match product")?)
                    .filter(|definition| definition.id == *product && definition.name == *name)
                    .ok_or_else(|| Error::msg("HIR match pattern has a stale product identity"))?;
                if fields.len() != definition.fields.len() {
                    return Err(Error::msg("HIR match pattern has stale product fields"));
                }
                pending
                    .try_reserve(fields.len())
                    .map_err(|_| Error::host("HIR match pattern work allocation failed"))?;
                for (field, declared) in fields.iter().zip(&definition.fields) {
                    if field.name != declared.name || field.field_index != declared.source_order {
                        return Err(Error::msg("HIR match field identity is stale"));
                    }
                    match (&field.projection, &field.pattern) {
                        (None, MatchPattern::Wildcard { .. }) => {}
                        (Some(local), pattern)
                            if !matches!(pattern, MatchPattern::Wildcard { .. })
                                && local.ty == declared.ty =>
                        {
                            validate_match_local(
                                program,
                                declarations,
                                origin,
                                local,
                                BindingKind::MatchTemporary,
                            )?;
                        }
                        _ => {
                            return Err(Error::msg(
                                "HIR match wildcard/projection metadata is stale",
                            ))
                        }
                    }
                    pending.push((&field.pattern, declared.ty.clone()));
                }
            }
            _ => return Err(Error::msg("HIR match literal type is stale")),
        }
    }
    Ok(())
}

fn validate_match_local(
    program: &Program,
    declarations: &DeclarationIndexes,
    origin: Origin,
    local: &crate::hir::MatchLocal,
    kind: BindingKind,
) -> Result<()> {
    let binding = require_binding(program, local.binding, "HIR match local")?;
    if binding.kind != kind || binding.origin != origin || binding.ty != local.ty {
        return Err(Error::msg("HIR match local signature or origin is stale"));
    }
    validate_type(program, declarations, &local.ty)
}

fn validate_expressions(program: &Program, declarations: &DeclarationIndexes) -> Result<()> {
    let mut unreachable_counts = Vec::new();
    unreachable_counts
        .try_reserve(program.match_plans.len())
        .map_err(|_| Error::host("HIR match marker consistency allocation failed"))?;
    unreachable_counts.resize(program.match_plans.len(), (0_u64, 0_u64));
    validate_expression_root(
        program,
        declarations,
        &program.main.body,
        program.main.local_count,
        &program.main.return_type,
        &mut unreachable_counts,
    )?;
    for function in &program.functions {
        let binding = require_binding(program, function.binding, "HIR function")?;
        let (_, return_type) = function_signature(&binding.ty)
            .ok_or_else(|| Error::msg("HIR function binding has no function signature"))?;
        validate_expression_root(
            program,
            declarations,
            &function.body,
            function.local_count,
            return_type,
            &mut unreachable_counts,
        )?;
    }
    if unreachable_counts
        .iter()
        .any(|(semantic, lowered)| semantic.checked_add(*lowered) != Some(1))
    {
        return Err(Error::msg(
            "HIR match plan has missing or duplicate semantic/lowered provenance",
        ));
    }
    Ok(())
}

fn validate_expression_root(
    program: &Program,
    declarations: &DeclarationIndexes,
    root: &Expr,
    local_count: usize,
    return_type: &Type,
    unreachable_counts: &mut [(u64, u64)],
) -> Result<()> {
    enum Work<'a> {
        Visit(&'a Expr),
        EnterLoop(LexicalLoopContext),
        ExitLoop(crate::hir::LoopId),
    }

    let mut work = Vec::new();
    work.try_reserve(1)
        .map_err(|_| Error::host("HIR consistency work allocation failed"))?;
    work.push(Work::Visit(root));
    let mut active_loops: Vec<LexicalLoopContext> = Vec::new();
    let mut declared_loops = HashSet::new();
    while let Some(item) = work.pop() {
        let expression = match item {
            Work::EnterLoop(context) => {
                active_loops
                    .try_reserve(1)
                    .map_err(|_| Error::host("HIR lexical-loop context allocation failed"))?;
                active_loops.push(context);
                continue;
            }
            Work::ExitLoop(expected) => {
                let actual = active_loops
                    .pop()
                    .ok_or_else(|| Error::msg("HIR lexical-loop context is invalid"))?;
                if actual.loop_id != expected {
                    return Err(Error::msg(
                        "HIR lexical-loop contexts close out of semantic order",
                    ));
                }
                continue;
            }
            Work::Visit(expression) => expression,
        };

        validate_program_origin(program, expression.origin)?;
        if !expression.effects.is_known() {
            return Err(Error::msg("complete HIR expression has unknown effects"));
        }
        validate_type(program, declarations, &expression.ty)?;
        validate_expression_kind(
            program,
            declarations,
            expression,
            local_count,
            return_type,
            unreachable_counts,
        )?;
        match &expression.kind {
            ExprKind::Break { loop_id, value } => {
                let target = active_loops
                    .last()
                    .ok_or_else(|| Error::msg("HIR break is outside a lexical loop"))?;
                if *loop_id != target.loop_id {
                    return Err(Error::msg(
                        "HIR break does not target the nearest lexical loop",
                    ));
                }
                if value.ty != target.result_type {
                    return Err(Error::msg(
                        "HIR break value does not exactly equal its loop result type",
                    ));
                }
            }
            ExprKind::Continue { loop_id } => {
                let target = active_loops
                    .last()
                    .ok_or_else(|| Error::msg("HIR continue is outside a lexical loop"))?;
                if *loop_id != target.loop_id {
                    return Err(Error::msg(
                        "HIR continue does not target the nearest lexical loop",
                    ));
                }
            }
            ExprKind::While {
                loop_id,
                condition,
                body,
            } => {
                declared_loops
                    .try_reserve(1)
                    .map_err(|_| Error::host("HIR loop-identity allocation failed"))?;
                if !declared_loops.insert(*loop_id) {
                    return Err(Error::msg(
                        "HIR loop identity is duplicated in one callable",
                    ));
                }
                let additional = body
                    .len()
                    .checked_add(4)
                    .ok_or_else(|| Error::host("HIR consistency work count overflow"))?;
                work.try_reserve(additional)
                    .map_err(|_| Error::host("HIR consistency work allocation failed"))?;
                work.push(Work::ExitLoop(*loop_id));
                work.extend(body.iter().rev().map(Work::Visit));
                work.push(Work::EnterLoop(LexicalLoopContext {
                    loop_id: *loop_id,
                    result_type: Type::Unit,
                    is_while: true,
                }));
                work.push(Work::Visit(condition));
                continue;
            }
            ExprKind::Loop {
                loop_id,
                result_type,
                body,
            } => {
                declared_loops
                    .try_reserve(1)
                    .map_err(|_| Error::host("HIR loop-identity allocation failed"))?;
                if !declared_loops.insert(*loop_id) {
                    return Err(Error::msg(
                        "HIR loop identity is duplicated in one callable",
                    ));
                }
                let additional = body
                    .len()
                    .checked_add(3)
                    .ok_or_else(|| Error::host("HIR consistency work count overflow"))?;
                work.try_reserve(additional)
                    .map_err(|_| Error::host("HIR consistency work allocation failed"))?;
                work.push(Work::ExitLoop(*loop_id));
                work.extend(body.iter().rev().map(Work::Visit));
                work.push(Work::EnterLoop(LexicalLoopContext {
                    loop_id: *loop_id,
                    result_type: result_type.clone(),
                    is_while: false,
                }));
                continue;
            }
            _ => {}
        }
        let children = crate::hir::try_expression_children(expression, "HIR consistency")?;
        work.try_reserve(children.len())
            .map_err(|_| Error::host("HIR consistency work allocation failed"))?;
        work.extend(children.into_iter().rev().map(Work::Visit));
    }
    if active_loops.is_empty() {
        Ok(())
    } else {
        Err(Error::msg("HIR lexical-loop context did not close"))
    }
}

fn validate_expression_kind(
    program: &Program,
    declarations: &DeclarationIndexes,
    expression: &Expr,
    local_count: usize,
    return_type: &Type,
    unreachable_counts: &mut [(u64, u64)],
) -> Result<()> {
    match &expression.kind {
        ExprKind::Hole => return Err(Error::msg("complete HIR contains a hole")),
        ExprKind::UnresolvedValueReference { .. } => {
            return Err(Error::msg(
                "complete HIR contains an unresolved value reference",
            ));
        }
        ExprKind::LitI64(_) if expression.ty == Type::I64 => {}
        ExprKind::LitF64(_) if expression.ty == Type::F64 => {}
        ExprKind::LitBool(_) if expression.ty == Type::Bool => {}
        ExprKind::LitUnit if expression.ty == Type::Unit => {}
        ExprKind::EmptyList if matches!(expression.ty, Type::List(_)) => {}
        ExprKind::LitStr(_) if expression.ty == Type::Str => {}
        ExprKind::LitBytes(_) if expression.ty == Type::Bytes => {}
        ExprKind::Load(reference)
        | ExprKind::Move {
            binding: reference, ..
        } => {
            validate_binding_reference(program, *reference, local_count, Some(&expression.ty))?;
        }
        ExprKind::Borrow {
            binding: reference, ..
        }
        | ExprKind::BorrowBytes {
            binding: reference, ..
        } => {
            validate_binding_reference(program, *reference, local_count, None)?;
        }
        ExprKind::Call {
            callee,
            args,
            instantiation,
        } => {
            let binding = validate_binding_reference(program, *callee, local_count, None)?;
            if binding.kind != BindingKind::Function {
                return Err(Error::msg("HIR call target is not a function"));
            }
            validate_call_signature(
                program,
                declarations,
                binding.id,
                &binding.ty,
                args,
                instantiation.as_ref(),
                &expression.ty,
            )?;
        }
        ExprKind::Operation {
            operation,
            resolved_signature,
            args,
        } => {
            let mut argument_types = Vec::new();
            argument_types
                .try_reserve(args.len())
                .map_err(|_| Error::host("HIR operation type allocation failed"))?;
            argument_types.extend(args.iter().map(|argument| argument.ty.clone()));
            let (canonical_signature, canonical_result) = operation
                .resolve_types(&argument_types)
                .map_err(|message| Error::msg(format!("HIR operation is invalid: {message}")))?;
            if *resolved_signature != canonical_signature
                || Type::join_control(&expression.ty, &canonical_result) != Some(canonical_result)
            {
                return Err(Error::msg("HIR operation signature is stale"));
            }
        }
        ExprKind::F64FromI64Exact(value) => {
            validate_conversion(Operation::F64FromI64Exact, value, expression)?;
        }
        ExprKind::F64FromI64Rounded(value) => {
            validate_conversion(Operation::F64FromI64Rounded, value, expression)?;
        }
        ExprKind::I64FromF64Exact(value) => {
            validate_conversion(Operation::I64FromF64Exact, value, expression)?;
        }
        ExprKind::I64FromF64Trunc(value) => {
            validate_conversion(Operation::I64FromF64Trunc, value, expression)?;
        }
        ExprKind::Do(values) => {
            validate_ordered_control_body(
                values,
                "HIR sequence contains an expression after a control terminator",
            )?;
            let expected = values
                .last()
                .map_or_else(|| Type::Unit, |value| value.ty.clone());
            if expected != expression.ty {
                return Err(Error::msg("HIR sequence result type is stale"));
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            if condition.ty != Type::Bool
                || Type::join_control(&then_branch.ty, &else_branch.ty)
                    != Some(expression.ty.clone())
            {
                return Err(Error::msg("HIR conditional type facts are stale"));
            }
        }
        ExprKind::While {
            condition, body, ..
        } if condition.ty == Type::Bool && expression.ty == Type::Unit => {
            validate_ordered_control_body(
                body,
                "HIR while body contains an expression after a control terminator",
            )?;
        }
        ExprKind::Loop {
            result_type, body, ..
        } if *result_type == expression.ty && !result_type.contains_never() => {
            validate_ordered_control_body(
                body,
                "HIR loop body contains an expression after a control terminator",
            )?;
        }
        ExprKind::Return { value }
            if value.ty != Type::Never
                && value.ty == *return_type
                && expression.ty == Type::Never => {}
        ExprKind::Break { value, .. }
            if value.ty != Type::Never && expression.ty == Type::Never => {}
        ExprKind::Continue { .. } if expression.ty == Type::Never => {}
        ExprKind::Trap { value } if value.ty == Type::Str && expression.ty == Type::Never => {}
        ExprKind::Exit { code } if code.ty == Type::I64 && expression.ty == Type::Never => {}
        ExprKind::Let { bindings, body } => {
            for local in bindings {
                let binding = validate_local_binding(
                    program,
                    local.binding,
                    local.slot,
                    Some(expression.origin),
                    local_count,
                    &local.value.ty,
                )?;
                if !matches!(
                    binding.kind,
                    BindingKind::ImmutableLocal
                        | BindingKind::MatchTemporary
                        | BindingKind::StaticBytesLocal
                ) {
                    return Err(Error::msg("HIR let binding kind is stale"));
                }
            }
            if body.ty != expression.ty {
                return Err(Error::msg("HIR let body type is stale"));
            }
        }
        ExprKind::MutableLocal {
            binding,
            slot,
            initial,
            body,
            ..
        } => {
            let binding = validate_local_binding(
                program,
                *binding,
                *slot,
                Some(expression.origin),
                local_count,
                &initial.ty,
            )?;
            if binding.kind != BindingKind::MutableLocal {
                return Err(Error::msg("HIR mutable-local binding kind is stale"));
            }
            if body.ty != expression.ty {
                return Err(Error::msg("HIR mutable-local body type is stale"));
            }
        }
        ExprKind::SetLocal {
            target,
            slot,
            value,
        } => {
            let binding =
                validate_local_binding(program, *target, *slot, None, local_count, &value.ty)?;
            if binding.kind != BindingKind::MutableLocal {
                return Err(Error::msg("HIR set-local target kind is stale"));
            }
            if expression.ty != Type::Unit {
                return Err(Error::msg("HIR set-local result type is stale"));
            }
        }
        ExprKind::ProductValue { product, fields } => {
            let definition = require_product(program, *product)?;
            if fields.len() != definition.fields.len()
                || !matches!(&expression.ty, Type::Product(name) if *name == definition.name)
            {
                return Err(Error::msg("HIR product construction facts are stale"));
            }
        }
        ExprKind::ProductField {
            product,
            field,
            value,
        } => {
            let definition = require_product(program, *product)?;
            let field = definition
                .fields
                .get(index_of(*field, "HIR product field")?)
                .ok_or_else(|| Error::msg("HIR product field identity is stale"))?;
            if expression.ty != field.ty
                || !matches!(&value.ty, Type::Product(name) if *name == definition.name)
            {
                return Err(Error::msg("HIR product projection facts are stale"));
            }
        }
        ExprKind::WithProductField {
            product,
            field,
            value,
            replacement,
        } => {
            let definition = require_product(program, *product)?;
            let field = definition
                .fields
                .get(index_of(*field, "HIR product field")?)
                .ok_or_else(|| Error::msg("HIR product field identity is stale"))?;
            if replacement.ty != field.ty
                || expression.ty != value.ty
                || !matches!(&value.ty, Type::Product(name) if *name == definition.name)
            {
                return Err(Error::msg("HIR product update facts are stale"));
            }
        }
        ExprKind::EnumValue {
            enum_id,
            variant,
            layout,
            fields,
        } => {
            validate_enum_use(
                program,
                declarations,
                *enum_id,
                *variant,
                *layout,
                fields.len(),
            )?;
            validate_enum_type(&expression.ty, *enum_id)?;
        }
        ExprKind::EnumIsVariant {
            enum_id,
            variant,
            layout,
            value,
        } => {
            validate_enum_use(program, declarations, *enum_id, *variant, *layout, 0)?;
            validate_enum_type(&value.ty, *enum_id)?;
            if expression.ty != Type::Bool {
                return Err(Error::msg("HIR enum test result type is stale"));
            }
        }
        ExprKind::EnumField {
            enum_id,
            variant,
            field,
            field_index,
            layout,
            value,
        }
        | ExprKind::EnumUnwrap {
            enum_id,
            variant,
            field,
            field_index,
            layout,
            value,
            ..
        } => {
            let selected =
                validate_enum_use(program, declarations, *enum_id, *variant, *layout, 0)?;
            validate_enum_type(&value.ty, *enum_id)?;
            let selected_field = selected
                .fields
                .get(index_of(*field_index, "HIR enum field")?)
                .ok_or_else(|| Error::msg("HIR enum field identity is stale"))?;
            if selected_field.id != *field {
                return Err(Error::msg("HIR enum field stable identity is stale"));
            }
        }
        ExprKind::Match {
            plan,
            scrutinee,
            arms,
        } => {
            let index = index_of(plan.raw(), "HIR match plan")?;
            let planned = program
                .match_plans
                .get(index)
                .filter(|item| item.id == *plan)
                .ok_or_else(|| Error::msg("HIR semantic match plan identity is stale"))?;
            if expression.origin != planned.origin
                || expression.ty != planned.result_type
                || scrutinee.ty != planned.scrutinee.ty
                || arms.len() != planned.arms.len()
                || arms
                    .iter()
                    .zip(&planned.arms)
                    .any(|(body, arm)| body.ty != arm.body_type)
            {
                return Err(Error::msg("HIR semantic match facts are stale"));
            }
            unreachable_counts[index].0 = unreachable_counts[index]
                .0
                .checked_add(1)
                .ok_or_else(|| Error::msg("HIR semantic match marker count overflow"))?;
        }
        ExprKind::MatchUnreachable { plan } => {
            let index = index_of(plan.raw(), "HIR match plan")?;
            let planned = program
                .match_plans
                .get(index)
                .filter(|item| item.id == *plan)
                .ok_or_else(|| Error::msg("HIR match marker plan identity is stale"))?;
            if expression.ty != Type::Never || expression.origin != planned.origin {
                return Err(Error::msg("HIR match marker type or origin is stale"));
            }
            unreachable_counts[index].1 = unreachable_counts[index]
                .1
                .checked_add(1)
                .ok_or_else(|| Error::msg("HIR match marker count overflow"))?;
        }
        ExprKind::QuoteSymbol(_) if expression.ty == Type::Symbol => {}
        _ => {
            return Err(Error::msg(
                "HIR expression kind and type facts are inconsistent",
            ))
        }
    }
    Ok(())
}

fn substitute_hir_type(ty: &Type, substitutions: &HashMap<&str, &Type>) -> Result<Type> {
    crate::generic_call::substitute_type(ty, substitutions).map_err(|error| match error {
        crate::generic_call::GenericCallError::Host(message) => Error::host(message),
        other => Error::msg(format!("HIR type substitution is invalid: {other}")),
    })
}

fn validate_call_signature(
    program: &Program,
    declarations: &DeclarationIndexes,
    callee: BindingId,
    signature: &Type,
    arguments: &[Expr],
    instantiation: Option<&crate::hir::GenericInstantiation>,
    result: &Type,
) -> Result<()> {
    let mut substitutions = Vec::new();
    if let Some(instantiation) = instantiation {
        substitutions
            .try_reserve(instantiation.substitutions.len())
            .map_err(|_| Error::host("HIR call substitution allocation failed"))?;
        substitutions.extend(instantiation.substitutions.iter().cloned());
    }
    for substitution in &substitutions {
        validate_type(program, declarations, &substitution.ty)?;
    }
    let bounds = program
        .functions
        .iter()
        .find(|function| function.binding == callee)
        .map(|function| function.bounds.as_slice())
        .ok_or_else(|| Error::msg("HIR call target function is stale"))?;
    let mut argument_types = Vec::new();
    argument_types
        .try_reserve(arguments.len())
        .map_err(|_| Error::host("HIR call argument type allocation failed"))?;
    argument_types.extend(arguments.iter().map(|argument| argument.ty.clone()));
    let facts = crate::generic_call::GenericFacts {
        traits: &program.traits,
        products: &program.products,
        implementations: &program.implementations,
        product_names: &declarations.product_ids_by_name,
        implementation_index: &declarations.implementation_index,
    };
    let exact = crate::generic_call::resolve_exact(
        signature,
        substitutions,
        &argument_types,
        bounds,
        &facts,
    )
    .map_err(|error| match error {
        crate::generic_call::GenericCallError::Host(message) => Error::host(message),
        other => Error::msg(format!("HIR call instantiation is invalid: {other}")),
    })?;
    if exact.instantiation.as_ref() != instantiation {
        return Err(Error::msg(
            "HIR call instantiation metadata is not canonical",
        ));
    }
    if Type::join_control(result, &exact.result) != Some(exact.result) {
        return Err(Error::msg("HIR call result type is stale"));
    }
    Ok(())
}

fn validate_conversion(operation: Operation, value: &Expr, expression: &Expr) -> Result<()> {
    let (_, result) = operation
        .resolve_types(std::slice::from_ref(&value.ty))
        .map_err(Error::msg)?;
    if expression.ty != result {
        return Err(Error::msg("HIR numeric conversion result type is stale"));
    }
    Ok(())
}

fn validate_binding_reference<'a>(
    program: &'a Program,
    reference: crate::hir::BindingRef,
    local_count: usize,
    expression_type: Option<&Type>,
) -> Result<&'a Binding> {
    let binding = require_binding(program, reference.binding, "HIR binding reference")?;
    match reference.storage {
        BindingStorage::Local(slot) if slot < local_count => {}
        BindingStorage::Function if binding.kind == BindingKind::Function => {}
        _ => return Err(Error::msg("HIR binding storage fact is stale")),
    }
    if expression_type.is_some_and(|ty| binding.ty != *ty) {
        return Err(Error::msg("HIR binding reference type is stale"));
    }
    Ok(binding)
}

fn validate_local_binding<'a>(
    program: &'a Program,
    binding: BindingId,
    slot: usize,
    origin: Option<Origin>,
    local_count: usize,
    value_type: &Type,
) -> Result<&'a Binding> {
    let binding = require_binding(program, binding, "HIR local binding")?;
    if slot >= local_count
        || origin.is_some_and(|origin| binding.origin != origin)
        || binding.ty != *value_type
        || !matches!(
            binding.kind,
            BindingKind::ImmutableLocal
                | BindingKind::MatchTemporary
                | BindingKind::StaticBytesLocal
                | BindingKind::MutableLocal
        )
    {
        return Err(Error::msg(
            "HIR local binding slot, type, or origin is stale",
        ));
    }
    Ok(binding)
}

fn validate_enum_type(ty: &Type, expected: crate::hir::EnumId) -> Result<()> {
    if !matches!(ty, Type::Enum { id, .. } if *id == expected) {
        return Err(Error::msg("HIR enum expression type identity is stale"));
    }
    Ok(())
}

fn validate_enum_use<'a>(
    program: &'a Program,
    declarations: &DeclarationIndexes,
    id: crate::hir::EnumId,
    variant: crate::hir::VariantId,
    layout: crate::hir::RuntimeLayoutId,
    field_count: usize,
) -> Result<&'a crate::hir::EnumVariant> {
    let definition = declarations
        .enums_by_id
        .get(&id)
        .and_then(|index| program.enums.get(*index))
        .ok_or_else(|| Error::msg("HIR enum identity is stale"))?;
    if definition.layout.identity != layout {
        return Err(Error::msg("HIR enum layout identity is stale"));
    }
    let selected = definition
        .variants
        .iter()
        .find(|item| item.id == variant)
        .ok_or_else(|| Error::msg("HIR enum variant identity is stale"))?;
    if field_count != 0 && selected.fields.len() != field_count {
        return Err(Error::msg("HIR enum field count is stale"));
    }
    Ok(selected)
}

fn validate_type(program: &Program, declarations: &DeclarationIndexes, root: &Type) -> Result<()> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| Error::host("HIR type consistency allocation failed"))?;
    pending.push(root);
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Product(name) => {
                if !declarations.products_by_name.contains_key(name.as_str()) {
                    return Err(Error::msg("HIR type references an unknown product"));
                }
            }
            Type::Enum {
                id,
                name,
                arguments,
            } => {
                let definition = declarations
                    .enums_by_id
                    .get(id)
                    .and_then(|index| program.enums.get(*index))
                    .ok_or_else(|| Error::msg("HIR type references an unknown enum"))?;
                if definition.name != *name || definition.type_parameters.len() != arguments.len() {
                    return Err(Error::msg("HIR enum type identity or arity is stale"));
                }
                pending
                    .try_reserve(arguments.len())
                    .map_err(|_| Error::host("HIR type consistency allocation failed"))?;
                pending.extend(arguments);
            }
            Type::List(inner) => pending.push(inner),
            Type::Fn { params, ret } => {
                let additional = params
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| Error::host("HIR function type child count overflow"))?;
                pending
                    .try_reserve(additional)
                    .map_err(|_| Error::host("HIR type consistency allocation failed"))?;
                pending.push(ret);
                pending.extend(params);
            }
            Type::Forall { vars, body } => {
                if vars.is_empty() {
                    return Err(Error::msg("HIR universal type has no variables"));
                }
                pending.push(body);
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_ordered_control_body(body: &[Expr], message: &'static str) -> Result<()> {
    if body
        .iter()
        .take(body.len().saturating_sub(1))
        .any(|expression| expression.ty == Type::Never)
    {
        Err(Error::msg(message))
    } else {
        Ok(())
    }
}

fn function_signature(ty: &Type) -> Option<(&[Type], &Type)> {
    let ty = match ty {
        Type::Forall { body, .. } => body.as_ref(),
        other => other,
    };
    let Type::Fn { params, ret } = ty else {
        return None;
    };
    Some((params, ret))
}

fn require_binding<'a>(program: &'a Program, id: BindingId, kind: &str) -> Result<&'a Binding> {
    program
        .bindings
        .get(index_of(id.raw(), kind)?)
        .filter(|binding| binding.id == id)
        .ok_or_else(|| Error::msg(format!("{kind} identity is stale")))
}

fn require_product(
    program: &Program,
    id: lkjscript_core::ProductId,
) -> Result<&crate::hir::ProductDefinition> {
    program
        .products
        .get(index_of(id.raw(), "HIR product")?)
        .filter(|product| product.id == id)
        .ok_or_else(|| Error::msg("HIR product identity is stale"))
}

fn validate_origin(program: &Program, origin: Origin) -> Result<()> {
    match origin {
        Origin::Source(source) => validate_source(program, source),
        Origin::Semantic | Origin::Builtin => Ok(()),
    }
}

fn validate_program_origin(program: &Program, origin: Origin) -> Result<()> {
    match origin {
        Origin::Source(source) => validate_source(program, source),
        Origin::Semantic => Ok(()),
        Origin::Builtin => Err(Error::msg("ordinary HIR program item has a builtin origin")),
    }
}

fn validate_source(program: &Program, source: SourceId) -> Result<()> {
    program
        .sources
        .get(index_of(source.raw(), "HIR source")?)
        .filter(|item| item.id == source)
        .map(|_| ())
        .ok_or_else(|| Error::msg("HIR source origin is stale"))
}

fn require_dense(raw: u64, expected: usize, kind: &str) -> Result<()> {
    let expected =
        u64::try_from(expected).map_err(|_| Error::host(format!("{kind} index exceeds u64")))?;
    if raw != expected {
        return Err(Error::msg(format!("{kind} dense identity is stale")));
    }
    Ok(())
}

fn require_index(raw: u64, len: usize, kind: &str) -> Result<()> {
    let index = index_of(raw, kind)?;
    if index >= len {
        return Err(Error::msg(format!("{kind} identity is stale")));
    }
    Ok(())
}

fn index_of(raw: u64, kind: &str) -> Result<usize> {
    usize::try_from(raw).map_err(|_| Error::msg(format!("{kind} identity is not host-addressable")))
}
