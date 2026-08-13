use std::collections::BTreeSet;

use super::*;

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum EstimateKey {
    Product(u64),
    Enum(hir::EnumId),
}

pub(super) struct EstimateIndex<'a> {
    products: HashMap<hir::ProductId, &'a hir::ProductDefinition>,
    enums: HashMap<hir::EnumId, &'a hir::EnumDefinition>,
}

impl<'a> EstimateIndex<'a> {
    pub(super) fn new(program: &'a hir::Program) -> Result<Self> {
        let mut products = HashMap::new();
        let mut enums = HashMap::new();
        products
            .try_reserve(program.products.len())
            .map_err(|_| Error::host("value placement product index allocation failed"))?;
        enums
            .try_reserve(program.enums.len())
            .map_err(|_| Error::host("value placement enum index allocation failed"))?;
        for product in &program.products {
            if products.insert(product.id, product).is_some() {
                return Err(Error::msg("value placement product identity is duplicated"));
            }
        }
        for enumeration in &program.enums {
            if enums.insert(enumeration.id, enumeration).is_some() {
                return Err(Error::msg("value placement enum identity is duplicated"));
            }
        }
        Ok(Self { products, enums })
    }
}

pub(super) fn checked_estimate(index: &EstimateIndex<'_>, expression: &Expr) -> Result<(u64, u64)> {
    let mut active = BTreeSet::new();
    let (nodes, mut bytes) = estimate_type(index, &expression.ty, &mut active)?;
    match &expression.kind {
        ExprKind::LitStr(value) => bytes = checked_len(value.len())?,
        ExprKind::LitBytes(value) => bytes = checked_len(value.len())?,
        _ => {}
    }
    Ok((nodes, bytes))
}

fn estimate_type(
    index: &EstimateIndex<'_>,
    ty: &Type,
    active: &mut BTreeSet<EstimateKey>,
) -> Result<(u64, u64)> {
    crate::stack::grow(|| estimate_type_inner(index, ty, active))
}

fn estimate_type_inner(
    index: &EstimateIndex<'_>,
    ty: &Type,
    active: &mut BTreeSet<EstimateKey>,
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
        Type::Product(id) => estimate_product(index, *id, active),
        Type::Enum { id, .. } => estimate_enum(index, *id, active),
    }
}

fn estimate_product(
    index: &EstimateIndex<'_>,
    id: hir::ProductId,
    active: &mut BTreeSet<EstimateKey>,
) -> Result<(u64, u64)> {
    if !active.insert(EstimateKey::Product(id.raw())) {
        return Ok((0, 0));
    }
    let product = index
        .products
        .get(&id)
        .copied()
        .ok_or_else(|| Error::msg("value placement lost product estimate metadata"))?;
    let mut total = (1_u64, 0_u64);
    for field in &product.fields {
        total = add_estimate(total, estimate_type(index, &field.ty, active)?)?;
    }
    active.remove(&EstimateKey::Product(id.raw()));
    Ok(total)
}

fn estimate_enum(
    index: &EstimateIndex<'_>,
    id: hir::EnumId,
    active: &mut BTreeSet<EstimateKey>,
) -> Result<(u64, u64)> {
    let definition = index
        .enums
        .get(&id)
        .copied()
        .ok_or_else(|| Error::msg("value placement lost enum estimate metadata"))?;
    if !active.insert(EstimateKey::Enum(id)) {
        return Ok((0, 0));
    }
    let mut largest = (1_u64, 2_u64);
    for variant in &definition.variants {
        let mut estimate = (1_u64, 2_u64);
        for field in &variant.fields {
            estimate = add_estimate(estimate, estimate_type(index, &field.ty, active)?)?;
        }
        if estimate.0 > largest.0 || estimate.1 > largest.1 {
            largest = (largest.0.max(estimate.0), largest.1.max(estimate.1));
        }
    }
    active.remove(&EstimateKey::Enum(id));
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
