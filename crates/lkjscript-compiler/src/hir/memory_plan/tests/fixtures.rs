use super::super::*;
use crate::hir;
use lkjscript_core::{Error, Result};

pub(super) fn origin() -> hir::SourceId {
    hir::SourceId::new(0)
}

pub(super) fn expression(ty: hir::Type, kind: hir::ExprKind) -> hir::Expr {
    hir::Expr {
        ty,
        effects: hir::EffectSet::PURE,
        origin: hir::Origin::Source(origin()),
        kind,
    }
}

pub(super) fn unit() -> hir::Expr {
    expression(hir::Type::Unit, hir::ExprKind::LitUnit)
}
pub(super) fn text(value: &str) -> hir::Expr {
    expression(hir::Type::Str, hir::ExprKind::LitStr(value.into()))
}
pub(super) fn bytes() -> hir::Expr {
    expression(hir::Type::Bytes, hir::ExprKind::LitBytes(vec![1]))
}
pub(super) fn fake(ty: hir::Type) -> hir::Expr {
    expression(ty, hir::ExprKind::LitUnit)
}

pub(super) fn program(
    result: hir::Type,
    body: hir::Expr,
    products: Vec<hir::ProductDefinition>,
    enums: Vec<hir::EnumDefinition>,
) -> hir::Program {
    hir::Program {
        sources: Vec::new(),
        bindings: Vec::new(),
        products,
        enums,
        traits: Vec::new(),
        implementations: Vec::new(),
        match_plans: Vec::new(),
        functions: Vec::new(),
        main: hir::Main {
            origin: hir::Origin::Source(origin()),
            params: Vec::new(),
            param_places: Vec::new(),
            param_types: Vec::new(),
            return_type: result,
            arity: 0,
            local_count: 0,
            body,
        },
        global_layout: Vec::new(),
    }
}

#[allow(clippy::expect_used)]
pub(super) fn product(id: u64, name: &str, fields: &[(&str, hir::Type)]) -> hir::ProductDefinition {
    let identity = lkjscript_contracts::sha256(&id.to_be_bytes());
    hir::ProductDefinition {
        id: hir::ProductId::new(id),
        identity,
        name: name.into(),
        origin: hir::Origin::Source(origin()),
        fields: fields
            .iter()
            .enumerate()
            .map(|(index, (name, ty))| {
                let source_order = u64::try_from(index).expect("fixture field order");
                hir::ProductField {
                    identity: crate::source::product_field_identity(identity, name, source_order)
                        .expect("fixture product field identity"),
                    source_order,
                    name: (*name).into(),
                    ty: ty.clone(),
                }
            })
            .collect(),
    }
}

pub(super) fn product_value(
    definition: &hir::ProductDefinition,
    fields: Vec<hir::Expr>,
) -> hir::Expr {
    expression(
        hir::Type::Product(definition.name.clone()),
        hir::ExprKind::ProductValue {
            product: definition.id,
            fields,
        },
    )
}

pub(super) fn enum_id(byte: u8) -> hir::EnumId {
    hir::EnumId::new([byte; 32])
}
pub(super) fn variant_id(byte: u8) -> hir::VariantId {
    hir::VariantId::new([byte; 32])
}
pub(super) fn field_id(byte: u8) -> hir::VariantFieldId {
    hir::VariantFieldId::new([byte; 32])
}

pub(super) fn enum_definition(
    id: u8,
    name: &str,
    parameters: &[&str],
    variants: Vec<(&str, Vec<hir::Type>)>,
) -> hir::EnumDefinition {
    let id_value = enum_id(id);
    let variants = variants
        .into_iter()
        .enumerate()
        .map(|(vi, (name, fields))| hir::EnumVariant {
            id: variant_id(id.wrapping_add(1 + u8::try_from(vi).unwrap_or(0))),
            name: name.into(),
            source_order: vi as u64,
            fields: fields
                .into_iter()
                .enumerate()
                .map(|(fi, ty)| hir::EnumVariantField {
                    id: field_id(id.wrapping_add(32 + u8::try_from(fi).unwrap_or(0))),
                    name: format!("field-{fi}"),
                    source_order: fi as u64,
                    ty,
                    indirect: false,
                })
                .collect(),
        })
        .collect();
    hir::EnumDefinition {
        id: id_value,
        name: name.into(),
        origin: hir::Origin::Source(origin()),
        type_parameters: parameters.iter().map(|item| (*item).into()).collect(),
        variants,
        layout: hir::EnumLayoutFacts {
            identity: hir::RuntimeLayoutId::new([id.wrapping_add(64); 32]),
            recursive: false,
        },
    }
}

pub(super) fn enum_type(definition: &hir::EnumDefinition, arguments: Vec<hir::Type>) -> hir::Type {
    hir::Type::Enum {
        id: definition.id,
        name: definition.name.clone(),
        arguments,
    }
}

pub(super) fn enum_value(
    definition: &hir::EnumDefinition,
    variant: usize,
    arguments: Vec<hir::Type>,
    fields: Vec<hir::Expr>,
) -> hir::Expr {
    expression(
        enum_type(definition, arguments),
        hir::ExprKind::EnumValue {
            enum_id: definition.id,
            variant: definition.variants[variant].id,
            layout: definition.layout.identity,
            fields,
        },
    )
}

pub(super) fn derive(program: &hir::Program) -> Result<HirMemoryPlan> {
    let plan = producer::derive(program)?;
    verifier::verify(program, &plan)?;
    Ok(plan)
}

pub(super) fn fact<'a>(plan: &'a HirMemoryPlan, ty: &MemoryType) -> Result<&'a MemoryTypeFact> {
    plan.type_facts
        .iter()
        .find(|fact| &fact.ty == ty)
        .ok_or_else(|| Error::msg("fixture type fact must exist"))
}

pub(super) fn verify_forged(program: &hir::Program, plan: &mut HirMemoryPlan) -> Result<u64> {
    plan.id = compute_plan_id(plan)?;
    verifier::verify(program, plan)
}
