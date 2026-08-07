use std::collections::HashMap;
use std::sync::Arc;

use lkjscript_core::{Error, Result};

use crate::hir::{BindingId, BindingKind, Expr, ExprKind, GenericInstantiation, Program, Type};

use super::model::{EntityAddress, NodeAddress, NodeKey};
use super::{
    CallEdge, ContainmentEdge, DependencyEdge, EntityHeader, EntityId, EntityKind, NodeHeader,
    NodeId, NodeKind, ReferenceEdge, SemanticChild, SemanticOwner, SnapshotIndexes,
    WorkspaceNamespace,
};

const INITIAL_GENERATION: u64 = 1;

type EnumEntityMap = (EntityId, Vec<(EntityId, Vec<EntityId>)>);

struct EntityMaps {
    main: EntityId,
    bindings: Vec<EntityId>,
    products: Vec<(EntityId, Vec<EntityId>)>,
    enums: Vec<EnumEntityMap>,
    traits: Vec<EntityId>,
    implementations: Vec<EntityId>,
}

struct PendingExpression<'a> {
    expression: &'a Expr,
    expected: Option<Type>,
    owner: SemanticOwner,
    enclosing: EntityId,
    return_type: &'a Type,
    local_count: usize,
}

pub(super) fn build(program: &Program, namespace: WorkspaceNamespace) -> Result<SnapshotIndexes> {
    let (mut indexes, maps) = build_entities(program, namespace)?;
    add_entity_dependencies(program, &maps, &mut indexes)?;
    indexes
        .dependencies
        .sort_by_key(|edge| (edge.dependent, edge.dependency));
    indexes.dependencies.dedup();
    let declaration_count = indexes.dependencies.len();
    reserve(
        &mut indexes.declaration_dependencies,
        declaration_count,
        "workspace declaration dependency index",
    )?;
    indexes
        .declaration_dependencies
        .extend(indexes.dependencies.iter().copied());

    set_parameter_owners(&mut indexes, &maps, maps.main, &program.main.params)?;
    walk_root(
        program,
        &maps,
        &mut indexes,
        &program.main.body,
        maps.main,
        &program.main.return_type,
        program.main.local_count,
    )?;
    for function in &program.functions {
        let owner = binding_entity(&maps, function.binding)?;
        set_parameter_owners(&mut indexes, &maps, owner, &function.params)?;
        let return_type = function_return_type(program, function.binding)?;
        walk_root(
            program,
            &maps,
            &mut indexes,
            &function.body,
            owner,
            return_type,
            function.local_count,
        )?;
    }

    indexes
        .references
        .sort_by_key(|edge| (edge.site, edge.target));
    indexes.references.dedup();
    indexes
        .calls
        .sort_by_key(|edge| (edge.caller, edge.callee, edge.site));
    indexes.calls.dedup();
    indexes
        .dependencies
        .sort_by_key(|edge| (edge.dependent, edge.dependency));
    indexes.dependencies.dedup();
    finish_private_indexes(&mut indexes)?;
    Ok(indexes)
}

fn build_entities(
    program: &Program,
    namespace: WorkspaceNamespace,
) -> Result<(SnapshotIndexes, EntityMaps)> {
    let mut indexes = SnapshotIndexes {
        entities: Vec::new(),
        nodes: Vec::new(),
        containment: Vec::new(),
        references: Vec::new(),
        calls: Vec::new(),
        dependencies: Vec::new(),
        declaration_dependencies: Vec::new(),
        diagnostics: Vec::new(),
        entity_addresses: Vec::new(),
        node_addresses: Vec::new(),
        node_keys: Vec::new(),
        node_fingerprints: Vec::new(),
        node_expected_types: Vec::new(),
        entity_lookup: HashMap::new(),
        node_lookup: HashMap::new(),
        address_entities: HashMap::new(),
        address_nodes: HashMap::new(),
    };
    let main = push_entity(&mut indexes, namespace, EntityKind::Main, "main", None)?;

    let mut bindings = Vec::new();
    reserve(&mut bindings, program.bindings.len(), "binding entity map")?;
    for binding in &program.bindings {
        bindings.push(push_entity(
            &mut indexes,
            namespace,
            binding_kind(&binding.kind),
            &binding.name,
            None,
        )?);
    }

    let mut products = Vec::new();
    reserve(&mut products, program.products.len(), "product entity map")?;
    for product in &program.products {
        let entity = push_entity(
            &mut indexes,
            namespace,
            EntityKind::Product,
            &product.name,
            None,
        )?;
        let mut fields = Vec::new();
        reserve(
            &mut fields,
            product.fields.len(),
            "product field entity map",
        )?;
        for field in &product.fields {
            let field_entity = push_entity(
                &mut indexes,
                namespace,
                EntityKind::ProductField,
                &field.name,
                Some(entity),
            )?;
            push_containment(
                &mut indexes,
                SemanticOwner::Entity(entity),
                SemanticChild::Entity(field_entity),
            )?;
            fields.push(field_entity);
        }
        products.push((entity, fields));
    }

    let mut enums = Vec::new();
    reserve(&mut enums, program.enums.len(), "enum entity map")?;
    for definition in &program.enums {
        let entity = push_entity(
            &mut indexes,
            namespace,
            EntityKind::Enum,
            &definition.name,
            None,
        )?;
        let mut variants = Vec::new();
        reserve(
            &mut variants,
            definition.variants.len(),
            "enum variant entity map",
        )?;
        for variant in &definition.variants {
            let variant_entity = push_entity(
                &mut indexes,
                namespace,
                EntityKind::EnumVariant,
                &variant.name,
                Some(entity),
            )?;
            push_containment(
                &mut indexes,
                SemanticOwner::Entity(entity),
                SemanticChild::Entity(variant_entity),
            )?;
            let mut fields = Vec::new();
            reserve(&mut fields, variant.fields.len(), "enum field entity map")?;
            for field in &variant.fields {
                let field_entity = push_entity(
                    &mut indexes,
                    namespace,
                    EntityKind::EnumField,
                    &field.name,
                    Some(variant_entity),
                )?;
                push_containment(
                    &mut indexes,
                    SemanticOwner::Entity(variant_entity),
                    SemanticChild::Entity(field_entity),
                )?;
                fields.push(field_entity);
            }
            variants.push((variant_entity, fields));
        }
        enums.push((entity, variants));
    }

    let mut traits = Vec::new();
    reserve(&mut traits, program.traits.len(), "trait entity map")?;
    for definition in &program.traits {
        traits.push(push_entity(
            &mut indexes,
            namespace,
            EntityKind::Trait,
            &definition.name,
            None,
        )?);
    }

    let mut implementations = Vec::new();
    reserve(
        &mut implementations,
        program.implementations.len(),
        "implementation entity map",
    )?;
    for implementation in &program.implementations {
        let trait_name = program
            .traits
            .get(index_of(implementation.trait_id.raw(), "trait")?)
            .map(|item| item.name.as_str())
            .unwrap_or("<stale-trait>");
        let product_name = program
            .products
            .get(index_of(implementation.product.raw(), "product")?)
            .map(|item| item.name.as_str())
            .unwrap_or("<stale-product>");
        let name = format!("{trait_name} for {product_name}");
        implementations.push(push_entity(
            &mut indexes,
            namespace,
            EntityKind::Implementation,
            &name,
            None,
        )?);
    }

    Ok((
        indexes,
        EntityMaps {
            main,
            bindings,
            products,
            enums,
            traits,
            implementations,
        },
    ))
}

fn add_entity_dependencies(
    program: &Program,
    maps: &EntityMaps,
    indexes: &mut SnapshotIndexes,
) -> Result<()> {
    add_types_dependencies(
        program,
        maps,
        indexes,
        maps.main,
        program
            .main
            .param_types
            .iter()
            .chain(std::iter::once(&program.main.return_type)),
    )?;
    for binding in &program.bindings {
        add_types_dependencies(
            program,
            maps,
            indexes,
            binding_entity(maps, binding.id)?,
            std::iter::once(&binding.ty),
        )?;
    }
    for (product_index, product) in program.products.iter().enumerate() {
        let owner = maps.products[product_index].0;
        add_types_dependencies(
            program,
            maps,
            indexes,
            owner,
            product.fields.iter().map(|field| &field.ty),
        )?;
    }
    for (enum_index, definition) in program.enums.iter().enumerate() {
        let owner = maps.enums[enum_index].0;
        add_types_dependencies(
            program,
            maps,
            indexes,
            owner,
            definition
                .variants
                .iter()
                .flat_map(|variant| variant.fields.iter().map(|field| &field.ty)),
        )?;
    }
    for (index, implementation) in program.implementations.iter().enumerate() {
        let owner = maps.implementations[index];
        push_dependency(
            indexes,
            owner,
            maps.traits[index_of(implementation.trait_id.raw(), "trait")?],
        )?;
        push_dependency(
            indexes,
            owner,
            maps.products[index_of(implementation.product.raw(), "product")?].0,
        )?;
    }
    Ok(())
}

fn add_types_dependencies<'a>(
    program: &Program,
    maps: &EntityMaps,
    indexes: &mut SnapshotIndexes,
    owner: EntityId,
    types: impl Iterator<Item = &'a Type>,
) -> Result<()> {
    let mut pending = Vec::new();
    for ty in types {
        reserve(&mut pending, 1, "type dependency work stack")?;
        pending.push(ty);
    }
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Product(name) => {
                if let Some((index, _)) = program
                    .products
                    .iter()
                    .enumerate()
                    .find(|(_, product)| product.name == *name)
                {
                    push_dependency(indexes, owner, maps.products[index].0)?;
                }
            }
            Type::Enum { id, arguments, .. } => {
                if let Some((index, _)) = program
                    .enums
                    .iter()
                    .enumerate()
                    .find(|(_, definition)| definition.id == *id)
                {
                    push_dependency(indexes, owner, maps.enums[index].0)?;
                }
                reserve(&mut pending, arguments.len(), "type dependency work stack")?;
                pending.extend(arguments);
            }
            Type::List(inner) => {
                reserve(&mut pending, 1, "type dependency work stack")?;
                pending.push(inner);
            }
            Type::Fn { params, ret } => {
                let additional = params
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| Error::host("type dependency child count overflow"))?;
                reserve(&mut pending, additional, "type dependency work stack")?;
                pending.push(ret);
                pending.extend(params);
            }
            Type::Forall { body, .. } => {
                reserve(&mut pending, 1, "type dependency work stack")?;
                pending.push(body);
            }
            _ => {}
        }
    }
    Ok(())
}

fn walk_root(
    program: &Program,
    maps: &EntityMaps,
    indexes: &mut SnapshotIndexes,
    root: &Expr,
    owner: EntityId,
    return_type: &Type,
    local_count: usize,
) -> Result<()> {
    let mut pending = Vec::new();
    reserve(&mut pending, 1, "workspace expression work stack")?;
    pending.push(PendingExpression {
        expression: root,
        expected: Some(return_type.clone()),
        owner: SemanticOwner::Entity(owner),
        enclosing: owner,
        return_type,
        local_count,
    });
    while let Some(item) = pending.pop() {
        let expression = item.expression;
        let node = push_node(indexes, item.owner, expression, item.expected.as_ref())?;
        push_containment(indexes, item.owner, SemanticChild::Node(node))?;
        add_expression_relations(
            program,
            maps,
            indexes,
            expression,
            node,
            item.enclosing,
            item.local_count,
        )?;
        let children =
            expression_children(program, expression, item.return_type, item.local_count)?;
        reserve(
            &mut pending,
            children.len(),
            "workspace expression work stack",
        )?;
        pending.extend(
            children
                .into_iter()
                .rev()
                .map(|(child, expected)| PendingExpression {
                    expression: child,
                    expected,
                    owner: SemanticOwner::Node(node),
                    enclosing: item.enclosing,
                    return_type: item.return_type,
                    local_count: item.local_count,
                }),
        );
    }
    Ok(())
}

fn add_expression_relations(
    program: &Program,
    maps: &EntityMaps,
    indexes: &mut SnapshotIndexes,
    expression: &Expr,
    node: NodeId,
    enclosing: EntityId,
    local_count: usize,
) -> Result<()> {
    let mut reference = |binding: BindingId| -> Result<EntityId> {
        let target = binding_entity(maps, binding)?;
        push_reference(indexes, node, target)?;
        push_dependency(indexes, enclosing, target)?;
        Ok(target)
    };
    match &expression.kind {
        ExprKind::Load(binding)
        | ExprKind::Move { binding, .. }
        | ExprKind::Borrow { binding, .. }
        | ExprKind::BorrowBytes { binding, .. } => {
            reference(binding.binding)?;
        }
        ExprKind::Call { callee, .. } => {
            let target = reference(callee.binding)?;
            reserve(&mut indexes.calls, 1, "workspace call index")?;
            indexes.calls.push(CallEdge {
                caller: enclosing,
                callee: target,
                site: node,
            });
        }
        ExprKind::Operation { binding, .. } => {
            reference(*binding)?;
        }
        ExprKind::Let { bindings, .. } => {
            for local in bindings {
                if local.slot >= local_count {
                    return Err(Error::msg("local binding slot exceeds owner local count"));
                }
                set_entity_owner(indexes, binding_entity(maps, local.binding)?, enclosing)?;
            }
        }
        ExprKind::MutableLocal { binding, slot, .. } => {
            if *slot >= local_count {
                return Err(Error::msg("mutable local slot exceeds owner local count"));
            }
            set_entity_owner(indexes, binding_entity(maps, *binding)?, enclosing)?;
        }
        ExprKind::SetLocal { target, slot, .. } => {
            if *slot >= local_count {
                return Err(Error::msg("set-local slot exceeds owner local count"));
            }
            reference(*target)?;
        }
        ExprKind::ProductValue { product, .. }
        | ExprKind::ProductField { product, .. }
        | ExprKind::WithProductField { product, .. } => {
            let target = product_entity(maps, *product)?;
            push_reference(indexes, node, target)?;
            push_dependency(indexes, enclosing, target)?;
        }
        ExprKind::EnumValue { enum_id, .. }
        | ExprKind::EnumIsVariant { enum_id, .. }
        | ExprKind::EnumField { enum_id, .. }
        | ExprKind::EnumUnwrap { enum_id, .. } => {
            let target = enum_entity(program, maps, *enum_id)?;
            push_reference(indexes, node, target)?;
            push_dependency(indexes, enclosing, target)?;
        }
        ExprKind::MatchUnreachable { plan } => {
            let plan = program
                .match_plans
                .get(index_of(plan.raw(), "match plan")?)
                .ok_or_else(|| Error::msg("match plan identity is stale"))?;
            set_entity_owner(
                indexes,
                binding_entity(maps, plan.scrutinee.binding)?,
                enclosing,
            )?;
            for arm in &plan.arms {
                let mut patterns = Vec::new();
                reserve(&mut patterns, 1, "workspace match pattern work stack")?;
                patterns.push(&arm.pattern);
                while let Some(pattern) = patterns.pop() {
                    match pattern {
                        crate::hir::MatchPattern::Binding { local } => {
                            set_entity_owner(
                                indexes,
                                binding_entity(maps, local.binding)?,
                                enclosing,
                            )?;
                        }
                        crate::hir::MatchPattern::Variant { fields, .. }
                        | crate::hir::MatchPattern::Product { fields, .. } => {
                            for field in fields {
                                if let Some(local) = &field.projection {
                                    set_entity_owner(
                                        indexes,
                                        binding_entity(maps, local.binding)?,
                                        enclosing,
                                    )?;
                                }
                                patterns.push(&field.pattern);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn expression_children<'a>(
    program: &Program,
    expression: &'a Expr,
    return_type: &Type,
    _local_count: usize,
) -> Result<Vec<(&'a Expr, Option<Type>)>> {
    let mut children = Vec::new();
    match &expression.kind {
        ExprKind::Call {
            callee,
            args,
            instantiation,
        } => {
            let parameters = match program.binding(callee.binding) {
                Some(binding) => signature_parameters(&binding.ty, instantiation)?,
                None => None,
            };
            reserve(&mut children, args.len(), "workspace call children")?;
            for (index, argument) in args.iter().enumerate() {
                children.push((
                    argument,
                    parameters
                        .as_ref()
                        .and_then(|items| items.get(index))
                        .cloned(),
                ));
            }
        }
        ExprKind::Operation {
            resolved_signature,
            args,
            ..
        } => {
            let parameters = signature_parameters(resolved_signature, &None)?;
            reserve(&mut children, args.len(), "workspace operation children")?;
            for (index, argument) in args.iter().enumerate() {
                children.push((
                    argument,
                    parameters
                        .as_ref()
                        .and_then(|items| items.get(index))
                        .cloned(),
                ));
            }
        }
        ExprKind::Do(values) => {
            reserve(&mut children, values.len(), "workspace sequence children")?;
            for (index, value) in values.iter().enumerate() {
                let expected = (index + 1 == values.len()).then(|| expression.ty.clone());
                children.push((value, expected));
            }
        }
        ExprKind::While {
            condition, body, ..
        } => {
            let additional = body
                .len()
                .checked_add(1)
                .ok_or_else(|| Error::host("workspace while child count overflow"))?;
            reserve(&mut children, additional, "workspace while children")?;
            children.push((condition, Some(Type::Bool)));
            children.extend(body.iter().map(|value| (value, None)));
        }
        ExprKind::Loop { body, .. }
        | ExprKind::ProductValue { fields: body, .. }
        | ExprKind::EnumValue { fields: body, .. } => {
            reserve(&mut children, body.len(), "workspace expression children")?;
            if matches!(expression.kind, ExprKind::ProductValue { .. }) {
                if let ExprKind::ProductValue { product, fields } = &expression.kind {
                    let definition = program.products.get(index_of(product.raw(), "product")?);
                    for (index, field) in fields.iter().enumerate() {
                        children.push((
                            field,
                            definition
                                .and_then(|item| item.fields.get(index))
                                .map(|item| item.ty.clone()),
                        ));
                    }
                }
            } else if let ExprKind::EnumValue {
                enum_id,
                variant,
                fields,
                ..
            } = &expression.kind
            {
                let definition = program.enums.iter().find(|item| item.id == *enum_id);
                let selected = definition
                    .and_then(|item| item.variants.iter().find(|item| item.id == *variant));
                for (index, field) in fields.iter().enumerate() {
                    children.push((
                        field,
                        selected
                            .and_then(|item| item.fields.get(index))
                            .map(|item| item.ty.clone()),
                    ));
                }
            } else {
                children.extend(body.iter().map(|value| (value, None)));
            }
        }
        ExprKind::F64FromI64Exact(value) | ExprKind::F64FromI64Rounded(value) => {
            children.push((value, Some(Type::I64)));
        }
        ExprKind::I64FromF64Exact(value) | ExprKind::I64FromF64Trunc(value) => {
            children.push((value, Some(Type::F64)));
        }
        ExprKind::Return { value } => children.push((value, Some(return_type.clone()))),
        ExprKind::Break { value, .. } => children.push((value, Some(value.ty.clone()))),
        ExprKind::Trap { value } => children.push((value, Some(Type::Str))),
        ExprKind::Exit { code } => children.push((code, Some(Type::I64))),
        ExprKind::SetLocal { target, value, .. } => children.push((
            value,
            program.binding(*target).map(|binding| binding.ty.clone()),
        )),
        ExprKind::ProductField { product, value, .. }
        | ExprKind::WithProductField { product, value, .. } => {
            let expected = program
                .products
                .get(index_of(product.raw(), "product")?)
                .map(|item| Type::Product(item.name.clone()));
            children.push((value, expected));
            if let ExprKind::WithProductField {
                field, replacement, ..
            } = &expression.kind
            {
                let field_type = program
                    .products
                    .get(index_of(product.raw(), "product")?)
                    .and_then(|item| item.fields.get(index_of(*field, "product field").ok()?))
                    .map(|item| item.ty.clone());
                children.push((replacement, field_type));
            }
        }
        ExprKind::EnumIsVariant { value, .. }
        | ExprKind::EnumField { value, .. }
        | ExprKind::EnumUnwrap { value, .. } => children.push((value, Some(value.ty.clone()))),
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            reserve(&mut children, 3, "workspace conditional children")?;
            children.push((condition, Some(Type::Bool)));
            children.push((then_branch, Some(expression.ty.clone())));
            children.push((else_branch, Some(expression.ty.clone())));
        }
        ExprKind::Let { bindings, body } => {
            let additional = bindings
                .len()
                .checked_add(1)
                .ok_or_else(|| Error::host("workspace let child count overflow"))?;
            reserve(&mut children, additional, "workspace let children")?;
            for binding in bindings {
                children.push((
                    &binding.value,
                    program.binding(binding.binding).map(|item| item.ty.clone()),
                ));
            }
            children.push((body, Some(expression.ty.clone())));
        }
        ExprKind::MutableLocal {
            binding,
            initial,
            body,
            ..
        } => {
            reserve(&mut children, 2, "workspace mutable-local children")?;
            children.push((
                initial,
                program.binding(*binding).map(|item| item.ty.clone()),
            ));
            children.push((body, Some(expression.ty.clone())));
        }
        _ => {}
    }
    Ok(children)
}

fn signature_parameters(
    ty: &Type,
    instantiation: &Option<GenericInstantiation>,
) -> Result<Option<Vec<Type>>> {
    let ty = match ty {
        Type::Forall { body, .. } => body.as_ref(),
        other => other,
    };
    let Type::Fn { params, .. } = ty else {
        return Ok(None);
    };
    let Some(instantiation) = instantiation else {
        return Ok(Some(params.clone()));
    };
    let mut substitutions = HashMap::new();
    substitutions
        .try_reserve(instantiation.substitutions.len())
        .map_err(|_| Error::host("workspace type substitution allocation failed"))?;
    for item in &instantiation.substitutions {
        substitutions.insert(item.parameter.clone(), item.ty.clone());
    }
    let mut resolved = Vec::new();
    reserve(
        &mut resolved,
        params.len(),
        "workspace resolved signature parameters",
    )?;
    for parameter in params {
        resolved.push(parameter.subst(&substitutions));
    }
    Ok(Some(resolved))
}

fn function_return_type(program: &Program, binding: BindingId) -> Result<&Type> {
    let binding = program
        .binding(binding)
        .ok_or_else(|| Error::msg("function binding is stale"))?;
    let signature = match &binding.ty {
        Type::Forall { body, .. } => body.as_ref(),
        other => other,
    };
    let Type::Fn { ret, .. } = signature else {
        return Err(Error::msg("function binding lost its function signature"));
    };
    Ok(ret)
}

fn push_entity(
    indexes: &mut SnapshotIndexes,
    namespace: WorkspaceNamespace,
    kind: EntityKind,
    name: &str,
    owner: Option<EntityId>,
) -> Result<EntityId> {
    let slot = u64::try_from(indexes.entities.len())
        .map_err(|_| Error::host("workspace entity identity exceeds u64"))?;
    reserve(&mut indexes.entities, 1, "workspace entity map")?;
    let id = EntityId::new(namespace, slot, INITIAL_GENERATION);
    indexes.entities.push(EntityHeader {
        id,
        kind,
        name: Arc::from(name),
        owner,
    });
    Ok(id)
}

fn push_node(
    indexes: &mut SnapshotIndexes,
    owner: SemanticOwner,
    expression: &Expr,
    expected: Option<&Type>,
) -> Result<NodeId> {
    let namespace = match owner {
        SemanticOwner::Entity(id) => id.namespace(),
        SemanticOwner::Node(id) => id.namespace(),
    };
    let slot = u64::try_from(indexes.nodes.len())
        .map_err(|_| Error::host("workspace node identity exceeds u64"))?;
    reserve(&mut indexes.nodes, 1, "workspace node map")?;
    let id = NodeId::new(namespace, slot, INITIAL_GENERATION);
    indexes.nodes.push(NodeHeader {
        id,
        kind: node_kind(&expression.kind),
        owner,
        actual_type: Arc::from(expression.ty.to_string()),
        expected_type: expected.map(|ty| Arc::from(ty.to_string())),
    });
    reserve(
        &mut indexes.node_fingerprints,
        1,
        "workspace node fingerprint index",
    )?;
    indexes
        .node_fingerprints
        .push(expression_fingerprint(expression)?);
    reserve(
        &mut indexes.node_expected_types,
        1,
        "workspace typed expectation index",
    )?;
    indexes.node_expected_types.push(expected.cloned());
    Ok(id)
}

fn push_containment(
    indexes: &mut SnapshotIndexes,
    owner: SemanticOwner,
    child: SemanticChild,
) -> Result<()> {
    reserve(&mut indexes.containment, 1, "workspace containment index")?;
    indexes.containment.push(ContainmentEdge { owner, child });
    Ok(())
}

fn push_reference(indexes: &mut SnapshotIndexes, site: NodeId, target: EntityId) -> Result<()> {
    reserve(&mut indexes.references, 1, "workspace reference index")?;
    indexes.references.push(ReferenceEdge { site, target });
    Ok(())
}

fn push_dependency(
    indexes: &mut SnapshotIndexes,
    dependent: EntityId,
    dependency: EntityId,
) -> Result<()> {
    if dependent == dependency {
        return Ok(());
    }
    reserve(&mut indexes.dependencies, 1, "workspace dependency index")?;
    indexes.dependencies.push(DependencyEdge {
        dependent,
        dependency,
    });
    Ok(())
}

fn set_parameter_owners(
    indexes: &mut SnapshotIndexes,
    maps: &EntityMaps,
    owner: EntityId,
    parameters: &[BindingId],
) -> Result<()> {
    for parameter in parameters {
        set_entity_owner(indexes, binding_entity(maps, *parameter)?, owner)?;
    }
    Ok(())
}

fn set_entity_owner(
    indexes: &mut SnapshotIndexes,
    entity: EntityId,
    owner: EntityId,
) -> Result<()> {
    let index = index_of(entity.slot(), "workspace entity")?;
    let header = indexes
        .entities
        .get_mut(index)
        .filter(|header| header.id == entity)
        .ok_or_else(|| Error::msg("workspace entity owner target is stale"))?;
    match header.owner {
        Some(current) if current != owner => {
            return Err(Error::msg(
                "semantic entity has multiple containment owners",
            ));
        }
        Some(_) => return Ok(()),
        None => header.owner = Some(owner),
    }
    push_containment(
        indexes,
        SemanticOwner::Entity(owner),
        SemanticChild::Entity(entity),
    )
}

fn binding_entity(maps: &EntityMaps, binding: BindingId) -> Result<EntityId> {
    maps.bindings
        .get(index_of(binding.raw(), "binding")?)
        .copied()
        .ok_or_else(|| Error::msg("binding identity is stale"))
}

fn product_entity(maps: &EntityMaps, product: lkjscript_core::ProductId) -> Result<EntityId> {
    maps.products
        .get(index_of(product.raw(), "product")?)
        .map(|item| item.0)
        .ok_or_else(|| Error::msg("product identity is stale"))
}

fn enum_entity(program: &Program, maps: &EntityMaps, id: crate::hir::EnumId) -> Result<EntityId> {
    program
        .enums
        .iter()
        .position(|item| item.id == id)
        .and_then(|index| maps.enums.get(index).map(|item| item.0))
        .ok_or_else(|| Error::msg("enum identity is stale"))
}

fn binding_kind(kind: &BindingKind) -> EntityKind {
    match kind {
        BindingKind::Parameter => EntityKind::Parameter,
        BindingKind::ImmutableLocal => EntityKind::ImmutableLocal,
        BindingKind::StaticBytesLocal => EntityKind::StaticBytesLocal,
        BindingKind::MutableLocal => EntityKind::MutableLocal,
        BindingKind::Function => EntityKind::Function,
        BindingKind::BuiltinOperation(_) => EntityKind::BuiltinOperation,
    }
}

fn node_kind(kind: &ExprKind) -> NodeKind {
    match kind {
        ExprKind::LitI64(_)
        | ExprKind::LitF64(_)
        | ExprKind::LitBool(_)
        | ExprKind::LitUnit
        | ExprKind::EmptyList
        | ExprKind::LitStr(_)
        | ExprKind::LitBytes(_) => NodeKind::Literal,
        ExprKind::Load(_) => NodeKind::Load,
        ExprKind::Move { .. } => NodeKind::Move,
        ExprKind::Borrow { .. } | ExprKind::BorrowBytes { .. } => NodeKind::Borrow,
        ExprKind::Call { .. } => NodeKind::Call,
        ExprKind::Operation { .. } => NodeKind::Operation,
        ExprKind::F64FromI64Exact(_)
        | ExprKind::F64FromI64Rounded(_)
        | ExprKind::I64FromF64Exact(_)
        | ExprKind::I64FromF64Trunc(_) => NodeKind::Conversion,
        ExprKind::Do(_) => NodeKind::Sequence,
        ExprKind::If { .. } => NodeKind::Conditional,
        ExprKind::While { .. } => NodeKind::While,
        ExprKind::Loop { .. } => NodeKind::Loop,
        ExprKind::Return { .. } => NodeKind::Return,
        ExprKind::Break { .. } => NodeKind::Break,
        ExprKind::Continue { .. } => NodeKind::Continue,
        ExprKind::Trap { .. } => NodeKind::Trap,
        ExprKind::Exit { .. } => NodeKind::Exit,
        ExprKind::Let { .. } => NodeKind::Let,
        ExprKind::MutableLocal { .. } => NodeKind::MutableLocal,
        ExprKind::SetLocal { .. } => NodeKind::SetLocal,
        ExprKind::ProductValue { .. }
        | ExprKind::ProductField { .. }
        | ExprKind::WithProductField { .. } => NodeKind::Product,
        ExprKind::EnumValue { .. }
        | ExprKind::EnumIsVariant { .. }
        | ExprKind::EnumField { .. }
        | ExprKind::EnumUnwrap { .. } => NodeKind::Enum,
        ExprKind::MatchUnreachable { .. } => NodeKind::MatchUnreachable,
        ExprKind::QuoteSymbol(_) => NodeKind::Symbol,
    }
}

fn finish_private_indexes(indexes: &mut SnapshotIndexes) -> Result<()> {
    reserve(
        &mut indexes.entity_addresses,
        indexes.entities.len(),
        "workspace entity addresses",
    )?;
    for slot in 0..indexes.entities.len() {
        indexes.entity_addresses.push(EntityAddress(
            u64::try_from(slot).map_err(|_| Error::host("workspace entity address exceeds u64"))?,
        ));
    }

    reserve(
        &mut indexes.node_addresses,
        indexes.nodes.len(),
        "workspace node addresses",
    )?;
    reserve(
        &mut indexes.node_keys,
        indexes.nodes.len(),
        "workspace node keys",
    )?;
    let mut root_counts: HashMap<EntityId, u64> = HashMap::new();
    let mut child_counts: HashMap<SemanticOwner, u64> = HashMap::new();
    let mut node_roots: HashMap<NodeId, EntityId> = HashMap::new();
    root_counts
        .try_reserve(indexes.entities.len())
        .map_err(|_| Error::host("workspace root address allocation failed"))?;
    child_counts
        .try_reserve(indexes.nodes.len())
        .map_err(|_| Error::host("workspace child key allocation failed"))?;
    node_roots
        .try_reserve(indexes.nodes.len())
        .map_err(|_| Error::host("workspace node root allocation failed"))?;
    for header in &indexes.nodes {
        let ordinal = child_counts.entry(header.owner).or_insert(0);
        indexes.node_keys.push(NodeKey {
            owner: header.owner,
            ordinal: *ordinal,
        });
        *ordinal = ordinal
            .checked_add(1)
            .ok_or_else(|| Error::host("workspace child ordinal exceeds u64"))?;

        let root = match header.owner {
            SemanticOwner::Entity(entity) => entity,
            SemanticOwner::Node(parent) => node_roots
                .get(&parent)
                .copied()
                .ok_or_else(|| Error::msg("workspace node owner is stale"))?,
        };
        node_roots.insert(header.id, root);
        let preorder = root_counts.entry(root).or_insert(0);
        indexes.node_addresses.push(NodeAddress {
            root: EntityAddress(root.slot()),
            preorder: *preorder,
        });
        *preorder = preorder
            .checked_add(1)
            .ok_or_else(|| Error::host("workspace root preorder exceeds u64"))?;
    }
    indexes.rebuild_maps()
}

fn expression_fingerprint(expression: &Expr) -> Result<[u8; 32]> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve(96)
        .map_err(|_| Error::host("workspace expression fingerprint allocation failed"))?;
    bytes.extend_from_slice(expression.ty.to_string().as_bytes());
    bytes.extend_from_slice(&expression.effects.bits().to_be_bytes());
    let tag = match &expression.kind {
        ExprKind::LitI64(value) => {
            bytes.extend_from_slice(&value.to_be_bytes());
            0
        }
        ExprKind::LitF64(value) => {
            bytes.extend_from_slice(&value.to_bits().to_be_bytes());
            1
        }
        ExprKind::LitBool(value) => {
            bytes.push(u8::from(*value));
            2
        }
        ExprKind::LitUnit => 3,
        ExprKind::EmptyList => 4,
        ExprKind::LitStr(value) => {
            bytes.extend_from_slice(value.as_bytes());
            5
        }
        ExprKind::LitBytes(value) => {
            bytes.extend_from_slice(value);
            6
        }
        ExprKind::Load(value) => {
            bytes.extend_from_slice(&value.binding.raw().to_be_bytes());
            7
        }
        ExprKind::Move { binding, .. } => {
            bytes.extend_from_slice(&binding.binding.raw().to_be_bytes());
            8
        }
        ExprKind::Borrow { binding, .. } => {
            bytes.extend_from_slice(&binding.binding.raw().to_be_bytes());
            9
        }
        ExprKind::BorrowBytes { binding, .. } => {
            bytes.extend_from_slice(&binding.binding.raw().to_be_bytes());
            10
        }
        ExprKind::Call { callee, .. } => {
            bytes.extend_from_slice(&callee.binding.raw().to_be_bytes());
            11
        }
        ExprKind::Operation { binding, .. } => {
            bytes.extend_from_slice(&binding.raw().to_be_bytes());
            12
        }
        ExprKind::F64FromI64Exact(_) => 13,
        ExprKind::F64FromI64Rounded(_) => 14,
        ExprKind::I64FromF64Exact(_) => 15,
        ExprKind::I64FromF64Trunc(_) => 16,
        ExprKind::Do(_) => 17,
        ExprKind::If { .. } => 18,
        ExprKind::While { .. } => 19,
        ExprKind::Loop { .. } => 20,
        ExprKind::Return { .. } => 21,
        ExprKind::Break { .. } => 22,
        ExprKind::Continue { .. } => 23,
        ExprKind::Trap { .. } => 24,
        ExprKind::Exit { .. } => 25,
        ExprKind::Let { .. } => 26,
        ExprKind::MutableLocal { binding, .. } => {
            bytes.extend_from_slice(&binding.raw().to_be_bytes());
            27
        }
        ExprKind::SetLocal { target, .. } => {
            bytes.extend_from_slice(&target.raw().to_be_bytes());
            28
        }
        ExprKind::ProductValue { product, .. } => {
            bytes.extend_from_slice(&product.raw().to_be_bytes());
            29
        }
        ExprKind::ProductField { product, field, .. } => {
            bytes.extend_from_slice(&product.raw().to_be_bytes());
            bytes.extend_from_slice(&field.to_be_bytes());
            30
        }
        ExprKind::WithProductField { product, field, .. } => {
            bytes.extend_from_slice(&product.raw().to_be_bytes());
            bytes.extend_from_slice(&field.to_be_bytes());
            31
        }
        ExprKind::EnumValue {
            enum_id, variant, ..
        } => {
            bytes.extend_from_slice(&enum_id.bytes());
            bytes.extend_from_slice(&variant.bytes());
            32
        }
        ExprKind::EnumIsVariant {
            enum_id, variant, ..
        } => {
            bytes.extend_from_slice(&enum_id.bytes());
            bytes.extend_from_slice(&variant.bytes());
            33
        }
        ExprKind::EnumField {
            enum_id,
            variant,
            field,
            ..
        } => {
            bytes.extend_from_slice(&enum_id.bytes());
            bytes.extend_from_slice(&variant.bytes());
            bytes.extend_from_slice(&field.bytes());
            34
        }
        ExprKind::EnumUnwrap {
            enum_id,
            variant,
            field,
            ..
        } => {
            bytes.extend_from_slice(&enum_id.bytes());
            bytes.extend_from_slice(&variant.bytes());
            bytes.extend_from_slice(&field.bytes());
            35
        }
        ExprKind::MatchUnreachable { plan } => {
            bytes.extend_from_slice(&plan.raw().to_be_bytes());
            36
        }
        ExprKind::QuoteSymbol(value) => {
            bytes.extend_from_slice(value.as_bytes());
            37
        }
    };
    bytes.push(tag);
    Ok(lkjscript_core::sha256(&bytes))
}

fn index_of(raw: u64, kind: &str) -> Result<usize> {
    usize::try_from(raw).map_err(|_| Error::msg(format!("{kind} identity is not host-addressable")))
}

fn reserve<T>(values: &mut Vec<T>, additional: usize, context: &str) -> Result<()> {
    values
        .try_reserve(additional)
        .map_err(|_| Error::host(format!("{context} allocation failed")))
}
