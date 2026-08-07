use std::collections::{HashMap, HashSet};

use lkjscript_core::{Error, Result};

use crate::hir::{
    Binding, BindingId, BindingKind, BindingStorage, Expr, ExprKind, MatchEdgeTarget, MatchPattern,
    Operation, Origin, Program, SourceId, Type,
};

pub(super) fn program(program: &Program) -> Result<()> {
    validate_sources(program)?;
    validate_bindings(program)?;
    validate_declarations(program)?;
    validate_main(program)?;
    validate_functions(program)?;
    validate_global_layout(program)?;
    validate_match_plans(program)?;
    validate_expressions(program)?;
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
    if program.sources.is_empty() {
        return Err(Error::msg("complete HIR program has no source origins"));
    }
    Ok(())
}

fn validate_bindings(program: &Program) -> Result<()> {
    for (index, binding) in program.bindings.iter().enumerate() {
        require_dense(binding.id.raw(), index, "HIR binding")?;
        if binding.name.is_empty() {
            return Err(Error::msg("HIR binding has an empty name"));
        }
        validate_origin(program, binding.origin)?;
        validate_type(program, &binding.ty)?;
        match (&binding.kind, binding.origin) {
            (BindingKind::BuiltinOperation(_), Origin::Builtin) => {}
            (BindingKind::BuiltinOperation(_), Origin::Source(_)) => {
                return Err(Error::msg("builtin operation has a source origin"));
            }
            (_, Origin::Builtin) => {
                return Err(Error::msg("ordinary binding has a builtin origin"));
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_declarations(program: &Program) -> Result<()> {
    for (index, product) in program.products.iter().enumerate() {
        require_dense(product.id.raw(), index, "HIR product")?;
        validate_source(program, product.origin)?;
        if product.identity == [0; 32] || product.name.is_empty() {
            return Err(Error::msg("HIR product identity or name is invalid"));
        }
        for (field_index, field) in product.fields.iter().enumerate() {
            require_dense(field.source_order, field_index, "HIR product field")?;
            if field.identity == [0; 32] || field.name.is_empty() {
                return Err(Error::msg("HIR product field identity or name is invalid"));
            }
            validate_type(program, &field.ty)?;
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
        if let Some(origin) = definition.origin {
            validate_source(program, origin)?;
        }
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
                validate_type(program, &field.ty)?;
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

fn validate_main(program: &Program) -> Result<()> {
    validate_source(program, program.main.origin)?;
    if program.main.arity != program.main.params.len()
        || program.main.params.len() != program.main.param_places.len()
        || program.main.params.len() != program.main.param_types.len()
    {
        return Err(Error::msg("HIR main signature lengths are inconsistent"));
    }
    validate_type(program, &program.main.return_type)?;
    for (parameter, expected) in program.main.params.iter().zip(&program.main.param_types) {
        let binding = require_binding(program, *parameter, "HIR main parameter")?;
        if binding.kind != BindingKind::Parameter
            || binding.origin != Origin::Source(program.main.origin)
            || binding.ty != *expected
        {
            return Err(Error::msg("HIR main parameter signature is stale"));
        }
        validate_type(program, expected)?;
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
        validate_source(program, function.origin)?;
        let binding = require_binding(program, function.binding, "HIR function")?;
        if binding.kind != BindingKind::Function
            || binding.origin != Origin::Source(function.origin)
            || function.arity != function.params.len()
            || function.params.len() != function.param_places.len()
        {
            return Err(Error::msg("HIR function header is inconsistent"));
        }
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
                || local.origin != Origin::Source(function.origin)
                || local.ty != *expected
            {
                return Err(Error::msg("HIR function parameter signature is stale"));
            }
            let _ = place;
        }
        for bound in &function.bounds {
            require_index(
                bound.trait_id.raw(),
                program.traits.len(),
                "HIR trait bound",
            )?;
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

fn validate_match_plans(program: &Program) -> Result<()> {
    for (index, plan) in program.match_plans.iter().enumerate() {
        require_dense(plan.id.raw(), index, "HIR match plan")?;
        validate_source(program, plan.origin)?;
        if plan.arms.is_empty() || !plan.exhaustive || plan.witness.is_some() {
            return Err(Error::msg(
                "HIR complete match plan has stale completeness facts",
            ));
        }
        validate_match_local(program, plan.origin, &plan.scrutinee)?;
        validate_type(program, &plan.result_type)?;
        for (arm_index, arm) in plan.arms.iter().enumerate() {
            require_dense(arm.id, arm_index, "HIR match arm")?;
            validate_type(program, &arm.body_type)?;
            if Type::join_control(&arm.body_type, &plan.result_type)
                != Some(plan.result_type.clone())
            {
                return Err(Error::msg("HIR match arm result type is stale"));
            }
            validate_pattern(program, plan.origin, &arm.pattern, &plan.scrutinee.ty)?;
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
    origin: SourceId,
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
            MatchPattern::Wildcard { ty } => validate_type(program, ty)?,
            MatchPattern::Binding { local } => validate_match_local(program, origin, local)?,
            MatchPattern::Bool(_) if expected == Type::Bool => {}
            MatchPattern::I64(_) if expected == Type::I64 => {}
            MatchPattern::Variant {
                enum_id,
                variant,
                layout,
                fields,
                ..
            } => {
                let definition = program
                    .enums
                    .iter()
                    .find(|definition| definition.id == *enum_id)
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
                pending
                    .try_reserve(fields.len())
                    .map_err(|_| Error::host("HIR match pattern work allocation failed"))?;
                for (field, declared) in fields.iter().zip(&selected.fields) {
                    if field.name != declared.name || field.field_index != declared.source_order {
                        return Err(Error::msg("HIR match field identity is stale"));
                    }
                    if let Some(local) = &field.projection {
                        validate_match_local(program, origin, local)?;
                    }
                    pending.push((&field.pattern, field.pattern.ty()));
                }
            }
            MatchPattern::Product {
                product, fields, ..
            } => {
                let definition = program
                    .products
                    .get(index_of(product.raw(), "HIR match product")?)
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
                    if let Some(local) = &field.projection {
                        validate_match_local(program, origin, local)?;
                    }
                    pending.push((&field.pattern, field.pattern.ty()));
                }
            }
            _ => return Err(Error::msg("HIR match literal type is stale")),
        }
    }
    Ok(())
}

fn validate_match_local(
    program: &Program,
    origin: SourceId,
    local: &crate::hir::MatchLocal,
) -> Result<()> {
    let binding = require_binding(program, local.binding, "HIR match local")?;
    if binding.kind != BindingKind::ImmutableLocal
        || binding.origin != Origin::Source(origin)
        || binding.ty != local.ty
    {
        return Err(Error::msg("HIR match local signature or origin is stale"));
    }
    validate_type(program, &local.ty)
}

fn validate_expressions(program: &Program) -> Result<()> {
    let mut unreachable_counts = Vec::new();
    unreachable_counts
        .try_reserve(program.match_plans.len())
        .map_err(|_| Error::host("HIR match marker consistency allocation failed"))?;
    unreachable_counts.resize(program.match_plans.len(), 0_u64);
    validate_expression_root(
        program,
        &program.main.body,
        program.main.origin,
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
            &function.body,
            function.origin,
            function.local_count,
            return_type,
            &mut unreachable_counts,
        )?;
    }
    if unreachable_counts.iter().any(|count| *count != 1) {
        return Err(Error::msg(
            "HIR match plan has missing or duplicate unreachable provenance",
        ));
    }
    Ok(())
}

fn validate_expression_root(
    program: &Program,
    root: &Expr,
    origin: SourceId,
    local_count: usize,
    return_type: &Type,
    unreachable_counts: &mut [u64],
) -> Result<()> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| Error::host("HIR consistency work allocation failed"))?;
    pending.push(root);
    while let Some(expression) = pending.pop() {
        validate_source(program, expression.origin)?;
        validate_type(program, &expression.ty)?;
        validate_expression_kind(
            program,
            expression,
            origin,
            local_count,
            return_type,
            unreachable_counts,
        )?;
        push_children(&mut pending, expression)?;
    }
    Ok(())
}

fn validate_expression_kind(
    program: &Program,
    expression: &Expr,
    origin: SourceId,
    local_count: usize,
    return_type: &Type,
    unreachable_counts: &mut [u64],
) -> Result<()> {
    match &expression.kind {
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
                &binding.ty,
                args,
                instantiation.as_ref(),
                &expression.ty,
            )?;
        }
        ExprKind::Operation {
            binding,
            resolved_signature,
            args,
            ..
        } => {
            let binding = require_binding(program, *binding, "HIR operation binding")?;
            if !matches!(binding.kind, BindingKind::BuiltinOperation(_)) {
                return Err(Error::msg("HIR operation target is not builtin"));
            }
            let (parameters, result) = function_signature(resolved_signature)
                .ok_or_else(|| Error::msg("HIR operation signature is not a function"))?;
            if parameters.len() != args.len()
                || parameters
                    .iter()
                    .zip(args)
                    .any(|(parameter, argument)| *parameter != argument.ty)
                || Type::join_control(&expression.ty, result) != Some(result.clone())
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
            let expected = values
                .last()
                .map_or_else(|| Type::Unit, |value| value.ty.clone());
            if Type::join_control(&expected, &expression.ty) != Some(expression.ty.clone()) {
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
        ExprKind::While { condition, .. } if condition.ty == Type::Bool => {}
        ExprKind::Loop { result_type, .. } if *result_type == expression.ty => {}
        ExprKind::Return { value }
            if Type::join_control(&value.ty, return_type) == Some(return_type.clone()) => {}
        ExprKind::Break { value, .. }
            if value.ty != Type::Never && expression.ty == Type::Never => {}
        ExprKind::Continue { .. } if expression.ty == Type::Never => {}
        ExprKind::Trap { value } if value.ty == Type::Str && expression.ty == Type::Never => {}
        ExprKind::Exit { code } if code.ty == Type::I64 && expression.ty == Type::Never => {}
        ExprKind::Let { bindings, body } => {
            for local in bindings {
                validate_local_binding(
                    program,
                    local.binding,
                    local.slot,
                    origin,
                    local_count,
                    &local.value.ty,
                )?;
            }
            if Type::join_control(&body.ty, &expression.ty) != Some(expression.ty.clone()) {
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
            validate_local_binding(program, *binding, *slot, origin, local_count, &initial.ty)?;
            if Type::join_control(&body.ty, &expression.ty) != Some(expression.ty.clone()) {
                return Err(Error::msg("HIR mutable-local body type is stale"));
            }
        }
        ExprKind::SetLocal {
            target,
            slot,
            value,
        } => {
            validate_local_binding(program, *target, *slot, origin, local_count, &value.ty)?;
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
            validate_enum_use(program, *enum_id, *variant, *layout, fields.len())?;
            validate_enum_type(&expression.ty, *enum_id)?;
        }
        ExprKind::EnumIsVariant {
            enum_id,
            variant,
            layout,
            value,
        } => {
            validate_enum_use(program, *enum_id, *variant, *layout, 0)?;
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
            let selected = validate_enum_use(program, *enum_id, *variant, *layout, 0)?;
            validate_enum_type(&value.ty, *enum_id)?;
            let selected_field = selected
                .fields
                .get(index_of(*field_index, "HIR enum field")?)
                .ok_or_else(|| Error::msg("HIR enum field identity is stale"))?;
            if selected_field.id != *field {
                return Err(Error::msg("HIR enum field stable identity is stale"));
            }
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
            unreachable_counts[index] = unreachable_counts[index]
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

fn validate_call_signature(
    program: &Program,
    signature: &Type,
    arguments: &[Expr],
    instantiation: Option<&crate::hir::GenericInstantiation>,
    result: &Type,
) -> Result<()> {
    let (parameters, declared_result) = function_signature(signature)
        .ok_or_else(|| Error::msg("HIR call target has no function signature"))?;
    if parameters.len() != arguments.len() {
        return Err(Error::msg("HIR call argument count is stale"));
    }
    let mut substitutions = HashMap::new();
    if let Some(instantiation) = instantiation {
        substitutions
            .try_reserve(instantiation.substitutions.len())
            .map_err(|_| Error::host("HIR call substitution allocation failed"))?;
        for item in &instantiation.substitutions {
            validate_type(program, &item.ty)?;
            if substitutions
                .insert(item.parameter.clone(), item.ty.clone())
                .is_some()
            {
                return Err(Error::msg("HIR call substitution is duplicated"));
            }
        }
        for witness in &instantiation.witnesses {
            require_index(
                witness.trait_id.raw(),
                program.traits.len(),
                "HIR call trait witness",
            )?;
            validate_type(program, &witness.ty)?;
            if let crate::hir::TraitWitnessKind::Explicit(implementation) = &witness.kind {
                require_index(
                    implementation.raw(),
                    program.implementations.len(),
                    "HIR call implementation witness",
                )?;
            }
        }
    }
    for (parameter, argument) in parameters.iter().zip(arguments) {
        let expected = parameter.subst(&substitutions);
        if argument.ty != expected {
            return Err(Error::msg("HIR call argument type is stale"));
        }
    }
    let expected_result = declared_result.subst(&substitutions);
    if Type::join_control(result, &expected_result) != Some(expected_result) {
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

fn validate_local_binding(
    program: &Program,
    binding: BindingId,
    slot: usize,
    origin: SourceId,
    local_count: usize,
    value_type: &Type,
) -> Result<()> {
    let binding = require_binding(program, binding, "HIR local binding")?;
    if slot >= local_count
        || binding.origin != Origin::Source(origin)
        || binding.ty != *value_type
        || !matches!(
            binding.kind,
            BindingKind::ImmutableLocal | BindingKind::StaticBytesLocal | BindingKind::MutableLocal
        )
    {
        return Err(Error::msg(
            "HIR local binding slot, type, or origin is stale",
        ));
    }
    Ok(())
}

fn validate_enum_type(ty: &Type, expected: crate::hir::EnumId) -> Result<()> {
    if !matches!(ty, Type::Enum { id, .. } if *id == expected) {
        return Err(Error::msg("HIR enum expression type identity is stale"));
    }
    Ok(())
}

fn validate_enum_use(
    program: &Program,
    id: crate::hir::EnumId,
    variant: crate::hir::VariantId,
    layout: crate::hir::RuntimeLayoutId,
    field_count: usize,
) -> Result<&crate::hir::EnumVariant> {
    let definition = program
        .enums
        .iter()
        .find(|item| item.id == id)
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

fn push_children<'a>(pending: &mut Vec<&'a Expr>, expression: &'a Expr) -> Result<()> {
    let additional = child_count(&expression.kind)?;
    pending
        .try_reserve(additional)
        .map_err(|_| Error::host("HIR consistency work allocation failed"))?;
    match &expression.kind {
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::Loop { body: args, .. }
        | ExprKind::ProductValue { fields: args, .. }
        | ExprKind::EnumValue { fields: args, .. } => pending.extend(args.iter().rev()),
        ExprKind::While {
            condition, body, ..
        } => {
            pending.extend(body.iter().rev());
            pending.push(condition);
        }
        ExprKind::F64FromI64Exact(value)
        | ExprKind::F64FromI64Rounded(value)
        | ExprKind::I64FromF64Exact(value)
        | ExprKind::I64FromF64Trunc(value)
        | ExprKind::Return { value }
        | ExprKind::Break { value, .. }
        | ExprKind::Trap { value }
        | ExprKind::Exit { code: value }
        | ExprKind::SetLocal { value, .. }
        | ExprKind::ProductField { value, .. }
        | ExprKind::EnumIsVariant { value, .. }
        | ExprKind::EnumField { value, .. }
        | ExprKind::EnumUnwrap { value, .. } => pending.push(value),
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            pending.push(else_branch);
            pending.push(then_branch);
            pending.push(condition);
        }
        ExprKind::Let { bindings, body } => {
            pending.push(body);
            pending.extend(bindings.iter().rev().map(|binding| &binding.value));
        }
        ExprKind::MutableLocal { initial, body, .. }
        | ExprKind::WithProductField {
            value: initial,
            replacement: body,
            ..
        } => {
            pending.push(body);
            pending.push(initial);
        }
        _ => {}
    }
    Ok(())
}

fn child_count(kind: &ExprKind) -> Result<usize> {
    match kind {
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::Loop { body: args, .. }
        | ExprKind::ProductValue { fields: args, .. }
        | ExprKind::EnumValue { fields: args, .. } => Ok(args.len()),
        ExprKind::While { body, .. } => body
            .len()
            .checked_add(1)
            .ok_or_else(|| Error::host("HIR child count overflow")),
        ExprKind::If { .. } => Ok(3),
        ExprKind::Let { bindings, .. } => bindings
            .len()
            .checked_add(1)
            .ok_or_else(|| Error::host("HIR child count overflow")),
        ExprKind::MutableLocal { .. } | ExprKind::WithProductField { .. } => Ok(2),
        ExprKind::F64FromI64Exact(_)
        | ExprKind::F64FromI64Rounded(_)
        | ExprKind::I64FromF64Exact(_)
        | ExprKind::I64FromF64Trunc(_)
        | ExprKind::Return { .. }
        | ExprKind::Break { .. }
        | ExprKind::Trap { .. }
        | ExprKind::Exit { .. }
        | ExprKind::SetLocal { .. }
        | ExprKind::ProductField { .. }
        | ExprKind::EnumIsVariant { .. }
        | ExprKind::EnumField { .. }
        | ExprKind::EnumUnwrap { .. } => Ok(1),
        _ => Ok(0),
    }
}

fn validate_type(program: &Program, root: &Type) -> Result<()> {
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| Error::host("HIR type consistency allocation failed"))?;
    pending.push(root);
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Product(name) => {
                if !program.products.iter().any(|product| product.name == *name) {
                    return Err(Error::msg("HIR type references an unknown product"));
                }
            }
            Type::Enum {
                id,
                name,
                arguments,
            } => {
                let definition = program
                    .enums
                    .iter()
                    .find(|definition| definition.id == *id)
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
        Origin::Builtin => Ok(()),
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
