use std::collections::HashSet;

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

fn checked_add(left: usize, right: usize) -> Result<usize> {
    left.checked_add(right)
        .ok_or_else(|| Error::msg("bytecode metadata byte size overflow"))
}
