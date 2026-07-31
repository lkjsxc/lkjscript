use super::super::*;
use super::fixtures::*;
use crate::hir;
use lkjscript_core::{ResourceKind, Result};

#[test]
fn products_fold_copy_immutable_affine_and_nested_fields() -> Result<()> {
    let copy = product(
        0,
        "copy-record",
        &[("count", hir::Type::I64), ("flag", hir::Type::Bool)],
    );
    let immutable = product(1, "immutable-record", &[("name", hir::Type::Str)]);
    let affine = product(
        2,
        "affine-record",
        &[
            ("payload", hir::Type::Bytes),
            ("vector", hir::Type::ByteVector),
            ("reader", hir::Type::Resource(ResourceKind::FileReader)),
        ],
    );
    let nested = product(
        3,
        "nested-record",
        &[
            ("copy", hir::Type::Product(copy.name.clone())),
            ("immutable", hir::Type::Product(immutable.name.clone())),
        ],
    );
    let nested_value = product_value(
        &nested,
        vec![
            product_value(
                &copy,
                vec![
                    expression(hir::Type::I64, hir::ExprKind::LitI64(1)),
                    expression(hir::Type::Bool, hir::ExprKind::LitBool(true)),
                ],
            ),
            product_value(&immutable, vec![text("name")]),
        ],
    );
    let hir_program = program(
        hir::Type::Product(nested.name.clone()),
        nested_value,
        vec![
            copy.clone(),
            immutable.clone(),
            affine.clone(),
            nested.clone(),
        ],
        Vec::new(),
    );
    let plan = derive(&hir_program)?;
    assert_eq!(
        fact(&plan, &MemoryType::Product(copy.name))?.mode,
        MemoryAggregateMode::Copy
    );
    assert_eq!(
        fact(&plan, &MemoryType::Product(immutable.name))?.mode,
        MemoryAggregateMode::ImmutableValue
    );
    assert_eq!(
        fact(&plan, &MemoryType::Product(nested.name))?.mode,
        MemoryAggregateMode::ImmutableValue
    );

    let affine_program = program(
        hir::Type::Product(affine.name.clone()),
        product_value(
            &affine,
            vec![
                bytes(),
                fake(hir::Type::ByteVector),
                fake(hir::Type::Resource(ResourceKind::FileReader)),
            ],
        ),
        vec![affine.clone()],
        Vec::new(),
    );
    let affine_plan = derive(&affine_program)?;
    let aggregate = fact(&affine_plan, &MemoryType::Product(affine.name))?;
    assert_eq!(aggregate.mode, MemoryAggregateMode::Affine);
    assert!(aggregate.contains_dynamic_owner);
    Ok(())
}

#[test]
fn generic_enum_folds_every_variant_but_destination_tracks_active_payload() -> Result<()> {
    let choice = enum_definition(
        10,
        "choice",
        &["t"],
        vec![
            ("none", Vec::new()),
            ("some", vec![hir::Type::Param("t".into())]),
        ],
    );
    let body = enum_value(&choice, 0, vec![hir::Type::Str], Vec::new());
    let ty = enum_type(&choice, vec![hir::Type::Str]);
    let hir_program = program(ty, body, Vec::new(), vec![choice.clone()]);
    let plan = derive(&hir_program)?;
    let memory_ty = MemoryType::Enum {
        id: choice.id.bytes(),
        name: choice.name.clone(),
        arguments: vec![MemoryType::String],
    };
    let enum_fact = fact(&plan, &memory_ty)?;
    assert_eq!(enum_fact.mode, MemoryAggregateMode::ImmutableValue);
    let destination = plan
        .destinations
        .first()
        .ok_or_else(|| lkjscript_core::Error::msg("enum destination is missing"))?;
    assert_eq!(destination.field_count, 0);
    assert_eq!(
        destination.active_payload.as_ref().map(|item| item.variant),
        Some(choice.variants[0].id.bytes())
    );
    let path = enum_fact
        .drop_path
        .and_then(|id| id.index())
        .and_then(|index| plan.drop_paths.get(index))
        .ok_or_else(|| lkjscript_core::Error::msg("enum drop path is missing"))?;
    assert_eq!(path.branches.len(), 2);

    let copy_program = program(
        enum_type(&choice, vec![hir::Type::Unit]),
        enum_value(&choice, 0, vec![hir::Type::Unit], Vec::new()),
        Vec::new(),
        vec![choice.clone()],
    );
    let copy_plan = derive(&copy_program)?;
    assert!(copy_plan.type_facts.iter().any(|item| matches!(item.ty,
        MemoryType::Enum { ref arguments, .. } if arguments == &[MemoryType::Unit])
        && item.mode == MemoryAggregateMode::Copy));
    let affine_program = program(
        enum_type(&choice, vec![hir::Type::ByteVector]),
        enum_value(&choice, 0, vec![hir::Type::ByteVector], Vec::new()),
        Vec::new(),
        vec![choice.clone()],
    );
    let affine_plan = derive(&affine_program)?;
    assert!(affine_plan.type_facts.iter().any(|item| matches!(item.ty,
        MemoryType::Enum { ref arguments, .. } if arguments == &[MemoryType::ByteVector])
        && item.mode == MemoryAggregateMode::Affine));
    Ok(())
}
