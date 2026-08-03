use std::collections::BTreeSet;

use super::*;
use crate::hir::{Expr, ExprKind};

pub(super) fn checked_estimate(program: &hir::Program, expression: &Expr) -> Result<(u64, u64)> {
    let mut active = BTreeSet::new();
    let (nodes, mut bytes) = estimate_type(program, &expression.ty, &mut active)?;
    match &expression.kind {
        ExprKind::LitStr(value) => bytes = checked_len(value.len())?,
        ExprKind::LitBytes(value) => bytes = checked_len(value.len())?,
        _ => {}
    }
    Ok((nodes, bytes))
}

fn estimate_type(
    program: &hir::Program,
    ty: &Type,
    active: &mut BTreeSet<String>,
) -> Result<(u64, u64)> {
    match ty {
        Type::Unit | Type::Never => Ok((0, 0)),
        Type::Bool => Ok((0, 1)),
        Type::I64 | Type::F64 => Ok((0, 8)),
        Type::Capability(_) | Type::Resource(_) | Type::Fn { .. } | Type::Forall { .. } => {
            Ok((0, 16))
        }
        Type::Symbol | Type::ByteSlice | Type::ByteSliceMut => Ok((0, 16)),
        Type::Str | Type::Bytes | Type::Path | Type::ByteVector | Type::List(_) => Ok((1, 0)),
        Type::Param(_) => Ok((0, 0)),
        Type::Product(name) => estimate_product(program, name, active),
        Type::Enum { id, .. } => estimate_enum(program, *id, active),
    }
}

fn estimate_product(
    program: &hir::Program,
    name: &str,
    active: &mut BTreeSet<String>,
) -> Result<(u64, u64)> {
    if !active.insert(name.into()) {
        return Ok((0, 0));
    }
    let product = program
        .products
        .iter()
        .find(|product| product.name == name)
        .ok_or_else(|| Error::msg("value placement lost product estimate metadata"))?;
    let mut total = (1_u64, 0_u64);
    for field in &product.fields {
        total = add_estimate(total, estimate_type(program, &field.ty, active)?)?;
    }
    active.remove(name);
    Ok(total)
}

fn estimate_enum(
    program: &hir::Program,
    id: hir::EnumId,
    active: &mut BTreeSet<String>,
) -> Result<(u64, u64)> {
    let definition = program
        .enums
        .iter()
        .find(|definition| definition.id == id)
        .ok_or_else(|| Error::msg("value placement lost enum estimate metadata"))?;
    if !active.insert(definition.name.clone()) {
        return Ok((0, 0));
    }
    let mut largest = (1_u64, 2_u64);
    for variant in &definition.variants {
        let mut estimate = (1_u64, 2_u64);
        for field in &variant.fields {
            estimate = add_estimate(estimate, estimate_type(program, &field.ty, active)?)?;
        }
        if estimate.0 > largest.0 || estimate.1 > largest.1 {
            largest = (largest.0.max(estimate.0), largest.1.max(estimate.1));
        }
    }
    active.remove(&definition.name);
    Ok(largest)
}

fn add_estimate(left: (u64, u64), right: (u64, u64)) -> Result<(u64, u64)> {
    Ok((
        left.0
            .checked_add(right.0)
            .ok_or_else(|| Error::msg("value placement node estimate overflow"))?,
        left.1
            .checked_add(right.1)
            .ok_or_else(|| Error::msg("value placement byte estimate overflow"))?,
    ))
}

fn checked_len(value: usize) -> Result<u64> {
    u64::try_from(value).map_err(|_| Error::msg("value placement payload exceeds u64"))
}
