fn rewrite_structural_symbols(value: &mut SemanticValue, mapping: &[Option<u64>]) -> Result<()> {
    match &mut value.payload {
        SemanticPayload::Static(crate::StaticStructuralLeaf::Symbol(symbol)) => {
            *symbol = canonical_symbol(*symbol, mapping)?;
        }
        SemanticPayload::Product(fields)
        | SemanticPayload::Enum {
            active_payload: fields,
            ..
        } => {
            for field in fields {
                rewrite_structural_symbols(field, mapping)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn rewrite_symbol(value: &mut Value, mapping: &[Option<u64>]) -> Result<()> {
    let Some(old) = value.as_symbol() else {
        return Ok(());
    };
    *value = Value::from_symbol(canonical_symbol(old, mapping)?);
    Ok(())
}

fn canonical_symbol(old: u64, mapping: &[Option<u64>]) -> Result<u64> {
    mapping
        .get(usize::try_from(old).map_err(|_| Error::msg("owned symbol index exceeds host usize"))?)
        .copied()
        .flatten()
        .ok_or_else(|| Error::msg("owned symbol mapping is incomplete"))
}
