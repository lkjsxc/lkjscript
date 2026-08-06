use std::collections::{HashSet, VecDeque};

use crate::{Chunk, Error, Result, MAX_PRODUCT_FIELDS};

pub(super) fn validate(chunk: &Chunk, mut metadata_bytes: usize) -> Result<usize> {
    let mut product_names = HashSet::with_capacity(chunk.products.len());
    for (index, product) in chunk.products.iter().enumerate() {
        if product.id.index() != index {
            return Err(Error::msg(format!(
                "product metadata index {index} has inconsistent ProductId {}",
                product.id.raw()
            )));
        }
        if !product.identity.is_resolved() {
            return Err(Error::msg(format!(
                "product metadata {index} has an unresolved runtime identity"
            )));
        }
        if product.region {
            let plan = chunk.memory_plan.ok_or_else(|| {
                Error::msg("region-product metadata lacks a canonical memory plan")
            })?;
            let expected = crate::runtime_product_contract_identity(plan, &product.name)?;
            if product.identity != expected {
                return Err(Error::msg(format!(
                    "product metadata {index} has a noncanonical region identity"
                )));
            }
        }
        metadata_bytes = checked_add(metadata_bytes, 33)?;
        if product.name.is_empty() {
            return Err(Error::msg(format!(
                "product metadata {index} has an empty name"
            )));
        }
        if !product_names.insert(product.name.as_str()) {
            return Err(Error::msg(format!(
                "duplicate product metadata name {}",
                product.name
            )));
        }
        if (product.region && product.region_fields.len() != product.fields.len())
            || (!product.region && !product.region_fields.is_empty())
        {
            return Err(Error::msg(format!(
                "product metadata {} has inconsistent region field routes",
                product.name
            )));
        }
        let region_bytes = product
            .region_fields
            .iter()
            .try_fold(0_usize, |bytes, route| {
                checked_add(
                    bytes,
                    if matches!(route, crate::RegionProductFieldKind::Product(_)) {
                        3
                    } else {
                        1
                    },
                )
            })?;
        metadata_bytes = checked_add(metadata_bytes, region_bytes)?;
        if product.fields.len() > MAX_PRODUCT_FIELDS {
            return Err(Error::msg(format!(
                "product metadata {} exceeds field limit {MAX_PRODUCT_FIELDS}",
                product.name
            )));
        }
        metadata_bytes = checked_add(metadata_bytes, product.name.len())?;
        let mut fields = HashSet::with_capacity(product.fields.len());
        for field in &product.fields {
            if field.is_empty() {
                return Err(Error::msg(format!(
                    "product metadata {} has an empty field name",
                    product.name
                )));
            }
            if !fields.insert(field.as_str()) {
                return Err(Error::msg(format!(
                    "product metadata {} has duplicate field {field}",
                    product.name
                )));
            }
            metadata_bytes = checked_add(metadata_bytes, field.len())?;
        }
    }

    validate_region_graph(chunk)?;

    for proto in std::iter::once(&chunk.main).chain(&chunk.protos) {
        for id in proto
            .parameter_region_products
            .iter()
            .copied()
            .flatten()
            .chain(proto.return_region_product)
        {
            let valid = chunk
                .products
                .get(id.index())
                .is_some_and(|product| product.id == id && product.region);
            if !valid {
                return Err(Error::msg(
                    "region-product signature references non-region product metadata",
                ));
            }
        }
    }

    let mut descriptors = HashSet::with_capacity(chunk.product_fields.len());
    for (index, field_ref) in chunk.product_fields.iter().copied().enumerate() {
        let product = chunk
            .products
            .get(field_ref.product.index())
            .ok_or_else(|| {
                Error::msg(format!(
                    "product field descriptor {index} has an unknown ProductId {}",
                    field_ref.product.raw()
                ))
            })?;
        if product.id != field_ref.product {
            return Err(Error::msg(format!(
                "product field descriptor {index} has inconsistent ProductId {}",
                field_ref.product.raw()
            )));
        }
        if usize::from(field_ref.field) >= product.fields.len() {
            return Err(Error::msg(format!(
                "product field descriptor {index} field {} is out of range",
                field_ref.field
            )));
        }
        if !descriptors.insert(field_ref) {
            return Err(Error::msg(format!(
                "duplicate product field descriptor at index {index}"
            )));
        }
    }
    Ok(metadata_bytes)
}

fn validate_region_graph(chunk: &Chunk) -> Result<()> {
    let mut edges = vec![Vec::new(); chunk.products.len()];
    let mut indegree = vec![0_usize; chunk.products.len()];
    let mut region_count = 0_usize;
    for product in chunk.products.iter().filter(|product| product.region) {
        region_count = region_count.saturating_add(1);
        for route in &product.region_fields {
            let crate::RegionProductFieldKind::Product(target) = route else {
                continue;
            };
            let valid = chunk
                .products
                .get(target.index())
                .is_some_and(|metadata| metadata.id == *target && metadata.region);
            if !valid {
                return Err(Error::msg(
                    "region-product field references non-region product metadata",
                ));
            }
            edges[product.id.index()].push(target.index());
            indegree[target.index()] = indegree[target.index()].saturating_add(1);
        }
    }
    let mut ready: VecDeque<_> = chunk
        .products
        .iter()
        .filter(|product| product.region && indegree[product.id.index()] == 0)
        .map(|product| product.id.index())
        .collect();
    let mut visited = 0_usize;
    while let Some(source) = ready.pop_front() {
        visited = visited.saturating_add(1);
        for target in &edges[source] {
            indegree[*target] = indegree[*target].saturating_sub(1);
            if indegree[*target] == 0 {
                ready.push_back(*target);
            }
        }
    }
    if visited != region_count {
        return Err(Error::msg(
            "region-product metadata dependency graph is cyclic",
        ));
    }
    Ok(())
}

fn checked_add(left: usize, right: usize) -> Result<usize> {
    left.checked_add(right)
        .ok_or_else(|| Error::host("bytecode metadata byte size overflow"))
}
