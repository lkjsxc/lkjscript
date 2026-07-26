use std::collections::HashSet;

use crate::{Chunk, Constant, Error, FunctionProto, Result, ValidationLimits, MAX_PRODUCT_FIELDS};

pub(super) fn validate_tables(chunk: &Chunk, limits: &ValidationLimits) -> Result<()> {
    let tables = [
        ("constants", chunk.constants.len()),
        ("prototypes", chunk.protos.len()),
        ("globals", chunk.global_names.len()),
        ("required capabilities", chunk.required_capabilities.len()),
        ("products", chunk.products.len()),
        ("product field descriptors", chunk.product_fields.len()),
        ("enums", chunk.enums.len()),
        ("enum constructors", chunk.enum_constructions.len()),
        ("enum variants", chunk.enum_variants.len()),
        ("enum fields", chunk.enum_fields.len()),
    ];
    for (name, length) in tables {
        if length > limits.max_table_entries {
            return Err(Error::msg(format!(
                "bytecode {name} table has {length} entries, limit {}",
                limits.max_table_entries
            )));
        }
    }
    super::entry_capabilities::validate(chunk)?;
    validate_proto_shape(&chunk.main, "main")?;

    let mut function_names = HashSet::with_capacity(chunk.protos.len());
    for proto in &chunk.protos {
        validate_proto_shape(proto, "prototype")?;
        if proto.name.is_empty() {
            return Err(Error::msg("bytecode prototype has an empty name"));
        }
        if !function_names.insert(proto.name.as_str()) {
            return Err(Error::msg(format!(
                "duplicate bytecode prototype name {}",
                proto.name
            )));
        }
    }

    let mut metadata_bytes = super::entry_capabilities::metadata_bytes(chunk)?;
    let mut encoded_bytes = chunk.main.code.len();
    for proto in &chunk.protos {
        metadata_bytes = checked_add(metadata_bytes, proto.name.len(), "metadata byte size")?;
        encoded_bytes = checked_add(encoded_bytes, proto.code.len(), "encoded byte size")?;
    }

    let mut global_names = HashSet::with_capacity(chunk.global_names.len());
    for name in &chunk.global_names {
        if name.is_empty() {
            return Err(Error::msg("bytecode global has an empty name"));
        }
        if !global_names.insert(name.as_str()) {
            return Err(Error::msg(format!("duplicate bytecode global name {name}")));
        }
        metadata_bytes = checked_add(metadata_bytes, name.len(), "metadata byte size")?;
    }

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
        metadata_bytes = checked_add(metadata_bytes, product.name.len(), "metadata byte size")?;
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
            metadata_bytes = checked_add(metadata_bytes, field.len(), "metadata byte size")?;
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

    metadata_bytes = checked_add(
        metadata_bytes,
        super::enum_shape::validate(chunk)?,
        "metadata byte size",
    )?;

    for (index, constant) in chunk.constants.iter().enumerate() {
        encoded_bytes = checked_add(encoded_bytes, 1, "encoded byte size")?;
        match constant {
            Constant::I64(_) | Constant::F64(_) => {
                encoded_bytes = checked_add(encoded_bytes, 8, "encoded byte size")?;
            }
            Constant::Str(text) | Constant::Symbol(text) => {
                if text.len() > limits.max_constant_data_bytes {
                    return Err(Error::msg(format!(
                        "constant {index} has {} data bytes, limit {}",
                        text.len(),
                        limits.max_constant_data_bytes
                    )));
                }
                encoded_bytes = checked_add(encoded_bytes, text.len(), "encoded byte size")?;
            }
            Constant::Proto(proto) => {
                if usize::try_from(*proto)
                    .ok()
                    .is_none_or(|proto| proto >= chunk.protos.len())
                {
                    return Err(Error::msg(format!(
                        "constant {index} references prototype {proto} out of range"
                    )));
                }
                encoded_bytes = checked_add(encoded_bytes, 4, "encoded byte size")?;
            }
        }
    }

    if metadata_bytes > limits.max_metadata_bytes {
        return Err(Error::msg(format!(
            "bytecode metadata has {metadata_bytes} bytes, limit {}",
            limits.max_metadata_bytes
        )));
    }
    if encoded_bytes > limits.max_encoded_bytes {
        return Err(Error::msg(format!(
            "encoded bytecode has {encoded_bytes} bytes, limit {}",
            limits.max_encoded_bytes
        )));
    }
    Ok(())
}

fn validate_proto_shape(proto: &FunctionProto, category: &str) -> Result<()> {
    if proto.arity > proto.locals {
        return Err(Error::msg(format!(
            "bytecode {category} {} has arity {} greater than local count {}",
            proto.name, proto.arity, proto.locals
        )));
    }
    Ok(())
}

fn checked_add(left: usize, right: usize, category: &str) -> Result<usize> {
    left.checked_add(right)
        .ok_or_else(|| Error::msg(format!("bytecode {category} overflow")))
}
