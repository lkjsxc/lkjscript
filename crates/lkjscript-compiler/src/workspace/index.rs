use std::collections::HashMap;
use std::sync::Arc;

use lkjscript_core::{Error, Result};

use crate::hir::{BindingId, BindingKind, Expr, ExprKind, GenericInstantiation, Type};

use super::program::SemanticProgram;

use super::model::{EntityAddress, NodeAddress, NodeKey};
use super::{
    CallEdge, ContainmentEdge, DependencyEdge, EntityHeader, EntityId, EntityKind, NodeHeader,
    NodeId, NodeKind, ReferenceEdge, SemanticChild, SemanticOwner, SnapshotIndexes,
    WorkspaceNamespace,
};

const INITIAL_GENERATION: u64 = 1;

#[cfg(test)]
thread_local! {
    static ROOT_ADDRESS_LOOKUPS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn reset_root_address_lookups() {
    ROOT_ADDRESS_LOOKUPS.with(|count| count.set(0));
}

#[cfg(test)]
pub(super) fn root_address_lookups() -> u64 {
    ROOT_ADDRESS_LOOKUPS.with(std::cell::Cell::get)
}

type EnumEntityMap = Option<(EntityId, Vec<(EntityId, Vec<EntityId>)>)>;

struct EntityMaps {
    main: Option<EntityId>,
    bindings: Vec<Option<EntityId>>,
    function_type_parameters: Vec<Vec<EntityId>>,
    products: Vec<(EntityId, Vec<EntityId>)>,
    enums: Vec<EnumEntityMap>,
    enum_type_parameters: Vec<Vec<EntityId>>,
    enum_indices: HashMap<crate::hir::EnumId, usize>,
    variant_indices: HashMap<(crate::hir::EnumId, crate::hir::VariantId), usize>,
    enum_field_indices: HashMap<
        (
            crate::hir::EnumId,
            crate::hir::VariantId,
            crate::hir::VariantFieldId,
        ),
        usize,
    >,
    traits: Vec<Option<EntityId>>,
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

pub(super) fn build(
    program: &SemanticProgram,
    namespace: WorkspaceNamespace,
) -> Result<SnapshotIndexes> {
    let (mut indexes, maps) = build_entities(program, namespace)?;
    install_entity_types(program, &maps, &mut indexes)?;
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

    if let (Some(main_entity), Some(main)) = (maps.main, program.main.as_ref()) {
        set_parameter_owners(&mut indexes, &maps, main_entity, &main.params)?;
        walk_root(
            program,
            &maps,
            &mut indexes,
            &main.body,
            main_entity,
            &main.return_type,
            main.local_count,
        )?;
    }
    for function in &program.functions {
        let owner = require_binding_entity(&maps, function.binding)?;
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
    program: &SemanticProgram,
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
        entity_addresses: Vec::new(),
        node_addresses: Vec::new(),
        node_keys: Vec::new(),
        node_match_plans: Vec::new(),
        node_enclosing_entities: Vec::new(),
        node_actual_types: Vec::new(),
        node_expected_types: Vec::new(),
        node_operations: Vec::new(),
        node_effects: Vec::new(),
        entity_types: Vec::new(),
        entity_lookup: HashMap::new(),
        node_lookup: HashMap::new(),
        node_children: HashMap::new(),
        product_name_indices: HashMap::new(),
        enum_identity_indices: HashMap::new(),
        variant_identity_indices: HashMap::new(),
        address_entities: HashMap::new(),
        address_nodes: HashMap::new(),
        type_parameter_entities: HashMap::new(),
    };
    let main = program
        .main
        .as_ref()
        .map(|_| {
            push_entity(
                &mut indexes,
                namespace,
                EntityAddress::Main,
                EntityKind::Main,
                "main",
                None,
            )
        })
        .transpose()?;

    let mut bindings = Vec::new();
    reserve(&mut bindings, program.bindings.len(), "binding entity map")?;
    let mut function_type_parameters = Vec::new();
    reserve(
        &mut function_type_parameters,
        program.bindings.len(),
        "function type-parameter entity map",
    )?;
    for binding in &program.bindings {
        let entity = if matches!(
            binding.kind,
            BindingKind::BuiltinOperation(_) | BindingKind::MatchTemporary
        ) {
            None
        } else {
            Some(push_entity(
                &mut indexes,
                namespace,
                EntityAddress::Binding(binding.id.raw()),
                binding_kind(&binding.kind),
                &binding.name,
                None,
            )?)
        };
        let mut type_parameters = Vec::new();
        if let (Some(owner), BindingKind::Function, Type::Forall { vars, .. }) =
            (entity, &binding.kind, &binding.ty)
        {
            reserve(
                &mut type_parameters,
                vars.len(),
                "function type-parameter entity map",
            )?;
            for (ordinal, name) in vars.iter().enumerate() {
                let ordinal = u64::try_from(ordinal)
                    .map_err(|_| Error::host("function type-parameter ordinal exceeds u64"))?;
                let parameter = push_entity(
                    &mut indexes,
                    namespace,
                    EntityAddress::FunctionTypeParameter {
                        function: binding.id.raw(),
                        ordinal,
                    },
                    EntityKind::TypeParameter,
                    name,
                    Some(owner),
                )?;
                push_containment(
                    &mut indexes,
                    SemanticOwner::Entity(owner),
                    SemanticChild::Entity(parameter),
                )?;
                type_parameters.push(parameter);
            }
        }
        bindings.push(entity);
        function_type_parameters.push(type_parameters);
    }

    let mut products = Vec::new();
    reserve(&mut products, program.products.len(), "product entity map")?;
    indexes
        .product_name_indices
        .try_reserve(program.products.len())
        .map_err(|_| Error::host("product name index allocation failed"))?;
    for (product_index, product) in program.products.iter().enumerate() {
        let mut product_name = String::new();
        product_name
            .try_reserve(product.name.len())
            .map_err(|_| Error::host("product name copy allocation failed"))?;
        product_name.push_str(&product.name);
        if indexes
            .product_name_indices
            .insert(product_name, product_index)
            .is_some()
        {
            return Err(Error::msg("product declaration name is duplicated"));
        }
        let product_index = u64::try_from(product_index)
            .map_err(|_| Error::host("workspace product address exceeds u64"))?;
        let entity = push_entity(
            &mut indexes,
            namespace,
            EntityAddress::Product(product_index),
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
        for (field_index, field) in product.fields.iter().enumerate() {
            let field_index = u64::try_from(field_index)
                .map_err(|_| Error::host("workspace product field address exceeds u64"))?;
            let field_entity = push_entity(
                &mut indexes,
                namespace,
                EntityAddress::ProductField {
                    product: product_index,
                    field: field_index,
                },
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
    let mut enum_type_parameters = Vec::new();
    reserve(
        &mut enum_type_parameters,
        program.enums.len(),
        "enum type-parameter entity map",
    )?;
    let mut enum_indices = HashMap::new();
    enum_indices
        .try_reserve(program.enums.len())
        .map_err(|_| Error::host("enum identity index allocation failed"))?;
    indexes
        .enum_identity_indices
        .try_reserve(program.enums.len())
        .map_err(|_| Error::host("workspace enum query index allocation failed"))?;
    let variant_count = program
        .enums
        .iter()
        .try_fold(0_usize, |count, definition| {
            count.checked_add(definition.variants.len())
        })
        .ok_or_else(|| Error::host("enum variant identity count overflow"))?;
    let mut variant_indices = HashMap::new();
    variant_indices
        .try_reserve(variant_count)
        .map_err(|_| Error::host("enum variant identity index allocation failed"))?;
    indexes
        .variant_identity_indices
        .try_reserve(variant_count)
        .map_err(|_| Error::host("workspace variant query index allocation failed"))?;
    let field_count = program
        .enums
        .iter()
        .flat_map(|definition| &definition.variants)
        .try_fold(0_usize, |count, variant| {
            count.checked_add(variant.fields.len())
        })
        .ok_or_else(|| Error::host("enum field identity count overflow"))?;
    let mut enum_field_indices = HashMap::new();
    enum_field_indices
        .try_reserve(field_count)
        .map_err(|_| Error::host("enum field identity index allocation failed"))?;
    for (enum_index, definition) in program.enums.iter().enumerate() {
        enum_indices.insert(definition.id, enum_index);
        indexes
            .enum_identity_indices
            .insert(definition.id, enum_index);
        for (variant_index, variant) in definition.variants.iter().enumerate() {
            variant_indices.insert((definition.id, variant.id), variant_index);
            indexes
                .variant_identity_indices
                .insert((definition.id, variant.id), (enum_index, variant_index));
            for (field_index, field) in variant.fields.iter().enumerate() {
                enum_field_indices.insert((definition.id, variant.id, field.id), field_index);
            }
        }
        if definition.origin == crate::hir::Origin::Builtin {
            enums.push(None);
            enum_type_parameters.push(Vec::new());
            continue;
        }
        let enum_index = u64::try_from(enum_index)
            .map_err(|_| Error::host("workspace enum address exceeds u64"))?;
        let entity = push_entity(
            &mut indexes,
            namespace,
            EntityAddress::Enum(enum_index),
            EntityKind::Enum,
            &definition.name,
            None,
        )?;
        let mut type_parameters = Vec::new();
        reserve(
            &mut type_parameters,
            definition.type_parameters.len(),
            "enum type-parameter entity map",
        )?;
        for (ordinal, name) in definition.type_parameters.iter().enumerate() {
            let ordinal = u64::try_from(ordinal)
                .map_err(|_| Error::host("enum type-parameter ordinal exceeds u64"))?;
            let parameter = push_entity(
                &mut indexes,
                namespace,
                EntityAddress::EnumTypeParameter {
                    enumeration: enum_index,
                    ordinal,
                },
                EntityKind::TypeParameter,
                name,
                Some(entity),
            )?;
            push_containment(
                &mut indexes,
                SemanticOwner::Entity(entity),
                SemanticChild::Entity(parameter),
            )?;
            type_parameters.push(parameter);
        }
        let mut variants = Vec::new();
        reserve(
            &mut variants,
            definition.variants.len(),
            "enum variant entity map",
        )?;
        for (variant_index, variant) in definition.variants.iter().enumerate() {
            let variant_index = u64::try_from(variant_index)
                .map_err(|_| Error::host("workspace enum variant address exceeds u64"))?;
            let variant_entity = push_entity(
                &mut indexes,
                namespace,
                EntityAddress::EnumVariant {
                    enumeration: enum_index,
                    variant: variant_index,
                },
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
            for (field_index, field) in variant.fields.iter().enumerate() {
                let field_index = u64::try_from(field_index)
                    .map_err(|_| Error::host("workspace enum field address exceeds u64"))?;
                let field_entity = push_entity(
                    &mut indexes,
                    namespace,
                    EntityAddress::EnumField {
                        enumeration: enum_index,
                        variant: variant_index,
                        field: field_index,
                    },
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
        enums.push(Some((entity, variants)));
        enum_type_parameters.push(type_parameters);
    }

    let mut traits = Vec::new();
    reserve(&mut traits, program.traits.len(), "trait entity map")?;
    for (trait_index, definition) in program.traits.iter().enumerate() {
        let entity = if definition.origin == crate::hir::Origin::Builtin {
            None
        } else {
            let trait_index = u64::try_from(trait_index)
                .map_err(|_| Error::host("workspace trait address exceeds u64"))?;
            Some(push_entity(
                &mut indexes,
                namespace,
                EntityAddress::Trait(trait_index),
                EntityKind::Trait,
                &definition.name,
                None,
            )?)
        };
        traits.push(entity);
    }

    let mut implementations = Vec::new();
    reserve(
        &mut implementations,
        program.implementations.len(),
        "implementation entity map",
    )?;
    for (implementation_index, implementation) in program.implementations.iter().enumerate() {
        let implementation_index = u64::try_from(implementation_index)
            .map_err(|_| Error::host("workspace implementation address exceeds u64"))?;
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
            EntityAddress::Implementation(implementation_index),
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
            function_type_parameters,
            products,
            enums,
            enum_type_parameters,
            enum_indices,
            variant_indices,
            enum_field_indices,
            traits,
            implementations,
        },
    ))
}

fn install_entity_types(
    program: &SemanticProgram,
    maps: &EntityMaps,
    indexes: &mut SnapshotIndexes,
) -> Result<()> {
    indexes
        .entity_types
        .try_reserve(indexes.entities.len())
        .map_err(|_| Error::host("workspace entity type allocation failed"))?;
    indexes.entity_types.resize(indexes.entities.len(), None);
    let mut set = |entity: EntityId, ty: Type| -> Result<()> {
        let index = index_of(entity.slot(), "workspace entity type")?;
        let slot = indexes
            .entity_types
            .get_mut(index)
            .ok_or_else(|| Error::msg("workspace entity type identity is stale"))?;
        *slot = Some(ty);
        Ok(())
    };
    if let (Some(entity), Some(main)) = (maps.main, program.main.as_ref()) {
        set(entity, main.return_type.clone())?;
    }
    for (index, binding) in program.bindings.iter().enumerate() {
        if let Some(entity) = maps.bindings[index] {
            set(entity, binding.ty.clone())?;
            if let Type::Forall { vars, .. } = &binding.ty {
                for (parameter, name) in maps.function_type_parameters[index].iter().zip(vars) {
                    set(*parameter, Type::Param(name.clone()))?;
                }
            }
        }
    }
    for (index, product) in program.products.iter().enumerate() {
        let (entity, fields) = &maps.products[index];
        let ty = Type::Product(product.name.clone());
        set(*entity, ty)?;
        for (field, entity) in product.fields.iter().zip(fields) {
            set(*entity, field.ty.clone())?;
        }
    }
    for (index, definition) in program.enums.iter().enumerate() {
        let Some((entity, variants)) = &maps.enums[index] else {
            continue;
        };
        let ty = Type::Enum {
            id: definition.id,
            name: definition.name.clone(),
            arguments: definition
                .type_parameters
                .iter()
                .cloned()
                .map(Type::Param)
                .collect(),
        };
        set(*entity, ty.clone())?;
        for (parameter, name) in maps.enum_type_parameters[index]
            .iter()
            .zip(&definition.type_parameters)
        {
            set(*parameter, Type::Param(name.clone()))?;
        }
        for (variant, (variant_entity, fields)) in definition.variants.iter().zip(variants) {
            set(*variant_entity, ty.clone())?;
            for (field, entity) in variant.fields.iter().zip(fields) {
                set(*entity, field.ty.clone())?;
            }
        }
    }
    Ok(())
}

fn add_entity_dependencies(
    program: &SemanticProgram,
    maps: &EntityMaps,
    indexes: &mut SnapshotIndexes,
) -> Result<()> {
    if let (Some(main_entity), Some(main)) = (maps.main, program.main.as_ref()) {
        add_types_dependencies(
            program,
            maps,
            indexes,
            main_entity,
            main.param_types
                .iter()
                .chain(std::iter::once(&main.return_type)),
        )?;
    }
    for binding in &program.bindings {
        if let Some(owner) = binding_entity(maps, binding.id)? {
            add_types_dependencies(program, maps, indexes, owner, std::iter::once(&binding.ty))?;
        }
    }
    for function in &program.functions {
        let owner = require_binding_entity(maps, function.binding)?;
        for bound in &function.bounds {
            if let Some(trait_entity) = maps
                .traits
                .get(index_of(bound.trait_id.raw(), "function bound trait")?)
                .copied()
                .flatten()
            {
                push_dependency(indexes, owner, trait_entity)?;
            }
        }
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
        if let Some((owner, _)) = &maps.enums[enum_index] {
            add_types_dependencies(
                program,
                maps,
                indexes,
                *owner,
                definition
                    .variants
                    .iter()
                    .flat_map(|variant| variant.fields.iter().map(|field| &field.ty)),
            )?;
        }
    }
    for (index, implementation) in program.implementations.iter().enumerate() {
        let owner = maps.implementations[index];
        let trait_entity = maps.traits[index_of(implementation.trait_id.raw(), "trait")?]
            .ok_or_else(|| Error::msg("explicit implementation targets a compiler-owned trait"))?;
        push_dependency(indexes, owner, trait_entity)?;
        push_dependency(
            indexes,
            owner,
            maps.products[index_of(implementation.product.raw(), "product")?].0,
        )?;
    }
    Ok(())
}

fn add_types_dependencies<'a>(
    _program: &SemanticProgram,
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
                if let Some(index) = indexes.product_name_indices.get(name).copied() {
                    push_dependency(indexes, owner, maps.products[index].0)?;
                }
            }
            Type::Enum { id, arguments, .. } => {
                if let Some(index) = maps.enum_indices.get(id).copied() {
                    if let Some((target, _)) = &maps.enums[index] {
                        push_dependency(indexes, owner, *target)?;
                    }
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
    program: &SemanticProgram,
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
        let node = push_node(
            indexes,
            item.owner,
            item.enclosing,
            expression,
            item.expected.as_ref(),
        )?;
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
        let children = expression_children(
            program,
            expression,
            item.expected.as_ref(),
            item.return_type,
            item.local_count,
        )?;
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
    program: &SemanticProgram,
    maps: &EntityMaps,
    indexes: &mut SnapshotIndexes,
    expression: &Expr,
    node: NodeId,
    enclosing: EntityId,
    local_count: usize,
) -> Result<()> {
    add_types_dependencies(
        program,
        maps,
        indexes,
        enclosing,
        std::iter::once(&expression.ty),
    )?;
    let mut reference = |binding: BindingId| -> Result<Option<EntityId>> {
        let Some(target) = binding_entity(maps, binding)? else {
            return Ok(None);
        };
        push_reference(indexes, node, target)?;
        push_dependency(indexes, enclosing, target)?;
        Ok(Some(target))
    };
    match &expression.kind {
        ExprKind::Load(binding)
        | ExprKind::Move { binding, .. }
        | ExprKind::Borrow { binding, .. }
        | ExprKind::BorrowBytes { binding, .. } => {
            reference(binding.binding)?;
        }
        ExprKind::Call {
            callee,
            instantiation,
            ..
        } => {
            let target = reference(callee.binding)?
                .ok_or_else(|| Error::msg("workspace call target is compiler-owned"))?;
            reserve(&mut indexes.calls, 1, "workspace call index")?;
            indexes.calls.push(CallEdge {
                caller: enclosing,
                callee: target,
                site: node,
            });
            if let Some(instantiation) = instantiation {
                add_types_dependencies(
                    program,
                    maps,
                    indexes,
                    enclosing,
                    instantiation
                        .substitutions
                        .iter()
                        .map(|item| &item.ty)
                        .chain(instantiation.witnesses.iter().map(|item| &item.ty)),
                )?;
                for witness in &instantiation.witnesses {
                    if let Some(trait_entity) = maps
                        .traits
                        .get(index_of(witness.trait_id.raw(), "trait witness")?)
                        .copied()
                        .flatten()
                    {
                        push_reference(indexes, node, trait_entity)?;
                        push_dependency(indexes, enclosing, trait_entity)?;
                    }
                    if let crate::hir::TraitWitnessKind::Explicit(implementation) = witness.kind {
                        let target = maps
                            .implementations
                            .get(index_of(implementation.raw(), "implementation witness")?)
                            .copied()
                            .ok_or_else(|| Error::msg("implementation witness is stale"))?;
                        push_reference(indexes, node, target)?;
                        push_dependency(indexes, enclosing, target)?;
                    }
                }
            }
        }
        ExprKind::Let { bindings, .. } => {
            for local in bindings {
                if local.slot >= local_count {
                    return Err(Error::msg("local binding slot exceeds owner local count"));
                }
                if let Some(entity) = binding_entity(maps, local.binding)? {
                    set_entity_owner(indexes, entity, enclosing)?;
                }
            }
        }
        ExprKind::MutableLocal { binding, slot, .. } => {
            if *slot >= local_count {
                return Err(Error::msg("mutable local slot exceeds owner local count"));
            }
            set_entity_owner(indexes, require_binding_entity(maps, *binding)?, enclosing)?;
        }
        ExprKind::Operation {
            resolved_signature, ..
        } => add_types_dependencies(
            program,
            maps,
            indexes,
            enclosing,
            std::iter::once(resolved_signature),
        )?,
        ExprKind::SetLocal { target, slot, .. } => {
            if *slot >= local_count {
                return Err(Error::msg("set-local slot exceeds owner local count"));
            }
            reference(*target)?;
        }
        ExprKind::ProductValue { product, .. } => {
            let target = product_entity(maps, *product)?;
            push_reference(indexes, node, target)?;
            push_dependency(indexes, enclosing, target)?;
            let (_, fields) = maps
                .products
                .get(index_of(product.raw(), "product")?)
                .ok_or_else(|| Error::msg("product identity is stale"))?;
            for field in fields {
                push_reference(indexes, node, *field)?;
                push_dependency(indexes, enclosing, *field)?;
            }
        }
        ExprKind::ProductField { product, field, .. }
        | ExprKind::WithProductField { product, field, .. } => {
            let target = product_entity(maps, *product)?;
            push_reference(indexes, node, target)?;
            push_dependency(indexes, enclosing, target)?;
            let field_entity = maps
                .products
                .get(index_of(product.raw(), "product")?)
                .and_then(|(_, fields)| fields.get(index_of(*field, "product field").ok()?))
                .copied()
                .ok_or_else(|| Error::msg("product field identity is stale"))?;
            push_reference(indexes, node, field_entity)?;
            push_dependency(indexes, enclosing, field_entity)?;
        }
        ExprKind::EnumValue {
            enum_id,
            variant,
            fields,
            ..
        } => {
            if let Some(target) = enum_entity(maps, *enum_id)? {
                push_reference(indexes, node, target)?;
                push_dependency(indexes, enclosing, target)?;
                let enum_index = enum_index(maps, *enum_id)?;
                let variant_index = enum_variant_index(maps, *enum_id, *variant)?;
                let (_, variants) = maps.enums[enum_index]
                    .as_ref()
                    .ok_or_else(|| Error::msg("user enum identity map is missing"))?;
                let (variant_entity, field_entities) = &variants[variant_index];
                if fields.len() != field_entities.len() {
                    return Err(Error::msg("enum value field identity map is stale"));
                }
                push_reference(indexes, node, *variant_entity)?;
                push_dependency(indexes, enclosing, *variant_entity)?;
                for field in field_entities {
                    push_reference(indexes, node, *field)?;
                    push_dependency(indexes, enclosing, *field)?;
                }
            }
        }
        ExprKind::EnumIsVariant {
            enum_id, variant, ..
        } => {
            if let Some(target) = enum_entity(maps, *enum_id)? {
                push_reference(indexes, node, target)?;
                push_dependency(indexes, enclosing, target)?;
                let enum_index = enum_index(maps, *enum_id)?;
                let variant_index = enum_variant_index(maps, *enum_id, *variant)?;
                if let Some((_, variants)) = &maps.enums[enum_index] {
                    push_reference(indexes, node, variants[variant_index].0)?;
                    push_dependency(indexes, enclosing, variants[variant_index].0)?;
                }
            }
        }
        ExprKind::EnumField {
            enum_id,
            variant,
            field,
            ..
        }
        | ExprKind::EnumUnwrap {
            enum_id,
            variant,
            field,
            ..
        } => {
            if let Some(target) = enum_entity(maps, *enum_id)? {
                push_reference(indexes, node, target)?;
                push_dependency(indexes, enclosing, target)?;
                let enum_index = enum_index(maps, *enum_id)?;
                if let Some((_, variants)) = &maps.enums[enum_index] {
                    let variant_index = enum_variant_index(maps, *enum_id, *variant)?;
                    let field_index = enum_field_index(maps, *enum_id, *variant, *field)?;
                    push_reference(indexes, node, variants[variant_index].0)?;
                    push_dependency(indexes, enclosing, variants[variant_index].0)?;
                    push_reference(indexes, node, variants[variant_index].1[field_index])?;
                    push_dependency(indexes, enclosing, variants[variant_index].1[field_index])?;
                }
            }
        }
        ExprKind::Match { plan, .. } | ExprKind::MatchUnreachable { plan } => {
            add_match_plan_relations(program, maps, indexes, node, enclosing, *plan)?;
        }
        _ => {}
    }
    Ok(())
}

fn add_match_plan_relations(
    program: &SemanticProgram,
    maps: &EntityMaps,
    indexes: &mut SnapshotIndexes,
    node: NodeId,
    enclosing: EntityId,
    id: crate::hir::MatchPlanId,
) -> Result<()> {
    let plan = program
        .match_plans
        .get(index_of(id.raw(), "match plan")?)
        .filter(|item| item.id == id)
        .ok_or_else(|| Error::msg("match plan identity is stale"))?;
    add_types_dependencies(
        program,
        maps,
        indexes,
        enclosing,
        std::iter::once(&plan.scrutinee.ty)
            .chain(std::iter::once(&plan.result_type))
            .chain(plan.arms.iter().map(|arm| &arm.body_type))
            .chain(plan.projections.iter().map(|item| &item.local.ty))
            .chain(plan.bindings.iter().map(|item| &item.local.ty)),
    )?;
    if let Some(entity) = binding_entity(maps, plan.scrutinee.binding)? {
        set_entity_owner(indexes, entity, enclosing)?;
    }
    for arm in &plan.arms {
        let mut patterns = Vec::new();
        reserve(&mut patterns, 1, "workspace match pattern work stack")?;
        patterns.push(&arm.pattern);
        while let Some(pattern) = patterns.pop() {
            let pattern_type = pattern.ty();
            add_types_dependencies(
                program,
                maps,
                indexes,
                enclosing,
                std::iter::once(&pattern_type),
            )?;
            match pattern {
                crate::hir::MatchPattern::Binding { local } => {
                    set_entity_owner(
                        indexes,
                        require_binding_entity(maps, local.binding)?,
                        enclosing,
                    )?;
                }
                crate::hir::MatchPattern::Variant {
                    enum_id,
                    variant,
                    fields,
                    ..
                } => {
                    if let Some(enum_entity) = enum_entity(maps, *enum_id)? {
                        let enum_index = enum_index(maps, *enum_id)?;
                        let variant_index = enum_variant_index(maps, *enum_id, *variant)?;
                        let (_, variants) = maps.enums[enum_index]
                            .as_ref()
                            .ok_or_else(|| Error::msg("user enum identity map is missing"))?;
                        let (variant_entity, field_entities) = &variants[variant_index];
                        push_reference(indexes, node, enum_entity)?;
                        push_dependency(indexes, enclosing, enum_entity)?;
                        push_reference(indexes, node, *variant_entity)?;
                        push_dependency(indexes, enclosing, *variant_entity)?;
                        for field in fields {
                            let field_index = index_of(field.field_index, "match enum field")?;
                            let field_entity = *field_entities
                                .get(field_index)
                                .ok_or_else(|| Error::msg("match enum field identity is stale"))?;
                            push_reference(indexes, node, field_entity)?;
                            push_dependency(indexes, enclosing, field_entity)?;
                            if let Some(local) = &field.projection {
                                if let Some(entity) = binding_entity(maps, local.binding)? {
                                    set_entity_owner(indexes, entity, enclosing)?;
                                }
                            }
                            patterns.push(&field.pattern);
                        }
                    }
                }
                crate::hir::MatchPattern::Product {
                    product, fields, ..
                } => {
                    let product_entity = product_entity(maps, *product)?;
                    push_reference(indexes, node, product_entity)?;
                    push_dependency(indexes, enclosing, product_entity)?;
                    let (_, field_entities) = maps
                        .products
                        .get(index_of(product.raw(), "match product")?)
                        .ok_or_else(|| Error::msg("match product identity is stale"))?;
                    for field in fields {
                        let field_entity = *field_entities
                            .get(index_of(field.field_index, "match product field")?)
                            .ok_or_else(|| Error::msg("match product field identity is stale"))?;
                        push_reference(indexes, node, field_entity)?;
                        push_dependency(indexes, enclosing, field_entity)?;
                        if let Some(local) = &field.projection {
                            if let Some(entity) = binding_entity(maps, local.binding)? {
                                set_entity_owner(indexes, entity, enclosing)?;
                            }
                        }
                        patterns.push(&field.pattern);
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn expression_children<'a>(
    program: &SemanticProgram,
    expression: &'a Expr,
    expected: Option<&Type>,
    return_type: &Type,
    _local_count: usize,
) -> Result<Vec<(&'a Expr, Option<Type>)>> {
    let control_result = expected
        .filter(|expected| **expected != Type::Never)
        .cloned()
        .unwrap_or_else(|| {
            if expression.ty == Type::Never {
                return_type.clone()
            } else {
                expression.ty.clone()
            }
        });
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
                let expected = (index + 1 == values.len()).then(|| control_result.clone());
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
            children.push((then_branch, Some(control_result.clone())));
            children.push((else_branch, Some(control_result.clone())));
        }
        ExprKind::Match {
            plan,
            scrutinee,
            arms,
        } => {
            let plan = program
                .match_plans
                .get(index_of(plan.raw(), "match plan")?)
                .filter(|item| item.id == *plan)
                .ok_or_else(|| Error::msg("semantic match plan identity is stale"))?;
            let additional = arms
                .len()
                .checked_add(1)
                .ok_or_else(|| Error::host("workspace match child count overflow"))?;
            reserve(&mut children, additional, "workspace match children")?;
            children.push((scrutinee, Some(plan.scrutinee.ty.clone())));
            for body in arms {
                children.push((body, Some(control_result.clone())));
            }
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
            children.push((body, Some(control_result.clone())));
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
            children.push((body, Some(control_result)));
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
    let mut substitutions = HashMap::new();
    if let Some(instantiation) = instantiation {
        substitutions
            .try_reserve(instantiation.substitutions.len())
            .map_err(|_| Error::host("workspace type substitution allocation failed"))?;
        for item in &instantiation.substitutions {
            substitutions.insert(item.parameter.as_str(), &item.ty);
        }
    }
    let mut resolved = Vec::new();
    reserve(
        &mut resolved,
        params.len(),
        "workspace resolved signature parameters",
    )?;
    for parameter in params {
        resolved.push(
            crate::generic_call::substitute_type(parameter, &substitutions).map_err(|error| {
                match error {
                    crate::generic_call::GenericCallError::Host(message) => Error::host(message),
                    other => Error::msg(format!("workspace call substitution failed: {other}")),
                }
            })?,
        );
    }
    Ok(Some(resolved))
}

fn function_return_type(program: &SemanticProgram, binding: BindingId) -> Result<&Type> {
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
    address: EntityAddress,
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
    reserve(
        &mut indexes.entity_addresses,
        1,
        "workspace entity addresses",
    )?;
    indexes.entity_addresses.push(address);
    Ok(id)
}

fn push_node(
    indexes: &mut SnapshotIndexes,
    owner: SemanticOwner,
    enclosing: EntityId,
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
    });
    reserve(
        &mut indexes.node_match_plans,
        1,
        "workspace match-plan node index",
    )?;
    indexes.node_match_plans.push(match &expression.kind {
        ExprKind::Match { plan, .. } => Some(*plan),
        _ => None,
    });
    reserve(
        &mut indexes.node_enclosing_entities,
        1,
        "workspace node enclosing-entity index",
    )?;
    indexes.node_enclosing_entities.push(enclosing);
    reserve(
        &mut indexes.node_actual_types,
        1,
        "workspace actual type index",
    )?;
    indexes.node_actual_types.push(expression.ty.clone());
    reserve(
        &mut indexes.node_expected_types,
        1,
        "workspace typed expectation index",
    )?;
    indexes.node_expected_types.push(expected.cloned());
    reserve(
        &mut indexes.node_operations,
        1,
        "workspace node operation index",
    )?;
    indexes.node_operations.push(match &expression.kind {
        ExprKind::Operation { operation, .. } => Some(*operation),
        _ => None,
    });
    reserve(&mut indexes.node_effects, 1, "workspace node effect index")?;
    indexes.node_effects.push(expression.effects);
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
        set_entity_owner(indexes, require_binding_entity(maps, *parameter)?, owner)?;
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

fn binding_entity(maps: &EntityMaps, binding: BindingId) -> Result<Option<EntityId>> {
    maps.bindings
        .get(index_of(binding.raw(), "binding")?)
        .copied()
        .ok_or_else(|| Error::msg("binding identity is stale"))
}

fn require_binding_entity(maps: &EntityMaps, binding: BindingId) -> Result<EntityId> {
    binding_entity(maps, binding)?
        .ok_or_else(|| Error::msg("compiler-owned binding has no program entity"))
}

fn product_entity(maps: &EntityMaps, product: lkjscript_core::ProductId) -> Result<EntityId> {
    maps.products
        .get(index_of(product.raw(), "product")?)
        .map(|item| item.0)
        .ok_or_else(|| Error::msg("product identity is stale"))
}

fn enum_index(maps: &EntityMaps, id: crate::hir::EnumId) -> Result<usize> {
    maps.enum_indices
        .get(&id)
        .copied()
        .ok_or_else(|| Error::msg("enum identity is stale"))
}

fn enum_variant_index(
    maps: &EntityMaps,
    enumeration: crate::hir::EnumId,
    variant: crate::hir::VariantId,
) -> Result<usize> {
    maps.variant_indices
        .get(&(enumeration, variant))
        .copied()
        .ok_or_else(|| Error::msg("enum variant identity is stale"))
}

fn enum_field_index(
    maps: &EntityMaps,
    enumeration: crate::hir::EnumId,
    variant: crate::hir::VariantId,
    field: crate::hir::VariantFieldId,
) -> Result<usize> {
    maps.enum_field_indices
        .get(&(enumeration, variant, field))
        .copied()
        .ok_or_else(|| Error::msg("enum field identity is stale"))
}

fn enum_entity(maps: &EntityMaps, id: crate::hir::EnumId) -> Result<Option<EntityId>> {
    maps.enums
        .get(enum_index(maps, id)?)
        .map(|item| item.as_ref().map(|item| item.0))
        .ok_or_else(|| Error::msg("enum identity map is stale"))
}

fn binding_kind(kind: &BindingKind) -> EntityKind {
    match kind {
        BindingKind::Parameter => EntityKind::Parameter,
        BindingKind::ImmutableLocal => EntityKind::ImmutableLocal,
        BindingKind::MatchTemporary => {
            unreachable!("match temporaries have no public workspace entity")
        }
        BindingKind::StaticBytesLocal => EntityKind::StaticBytesLocal,
        BindingKind::MutableLocal => EntityKind::MutableLocal,
        BindingKind::Function => EntityKind::Function,
        BindingKind::BuiltinOperation(_) => EntityKind::BuiltinOperation,
    }
}

fn node_kind(kind: &ExprKind) -> NodeKind {
    match kind {
        ExprKind::Hole => NodeKind::Hole,
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
        ExprKind::Match { .. } => NodeKind::Match,
        ExprKind::MatchUnreachable { .. } => NodeKind::MatchUnreachable,
        ExprKind::QuoteSymbol(_) => NodeKind::Symbol,
    }
}

fn finish_private_indexes(indexes: &mut SnapshotIndexes) -> Result<()> {
    if indexes.entity_addresses.len() != indexes.entities.len() {
        return Err(Error::msg("workspace entity address index is incomplete"));
    }
    if indexes.node_match_plans.len() != indexes.nodes.len() {
        return Err(Error::msg("workspace match-plan node index is incomplete"));
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
    let mut root_addresses: HashMap<EntityId, EntityAddress> = HashMap::new();
    let mut child_counts: HashMap<SemanticOwner, u64> = HashMap::new();
    let mut node_roots: HashMap<NodeId, EntityId> = HashMap::new();
    root_counts
        .try_reserve(indexes.entities.len())
        .map_err(|_| Error::host("workspace root address allocation failed"))?;
    root_addresses
        .try_reserve(indexes.entities.len())
        .map_err(|_| Error::host("workspace entity address lookup allocation failed"))?;
    for (entity, address) in indexes.entities.iter().zip(&indexes.entity_addresses) {
        root_addresses.insert(entity.id, *address);
    }
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
        #[cfg(test)]
        ROOT_ADDRESS_LOOKUPS.with(|count| count.set(count.get().saturating_add(1)));
        let root_address = root_addresses
            .get(&root)
            .copied()
            .ok_or_else(|| Error::msg("workspace node root entity is stale"))?;
        indexes.node_addresses.push(NodeAddress {
            root: root_address,
            preorder: *preorder,
        });
        *preorder = preorder
            .checked_add(1)
            .ok_or_else(|| Error::host("workspace root preorder exceeds u64"))?;
    }
    indexes.rebuild_maps()
}

fn index_of(raw: u64, kind: &str) -> Result<usize> {
    usize::try_from(raw).map_err(|_| Error::msg(format!("{kind} identity is not host-addressable")))
}

fn reserve<T>(values: &mut Vec<T>, additional: usize, context: &str) -> Result<()> {
    values
        .try_reserve(additional)
        .map_err(|_| Error::host(format!("{context} allocation failed")))
}
