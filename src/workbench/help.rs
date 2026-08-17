use crate::error::{ErrorCode, LkError, Result};
use crate::machine::schema_description;
use crate::machine_contract::{DraftFieldType, PayloadShapeKind};

/// Render concise authoring facts from the same executable description used by schema discovery.
pub fn authoring_help_cards() -> Result<String> {
    let schema = schema_description();
    let node_target = schema
        .transaction_variants
        .iter()
        .find(|variant| variant.name == DraftFieldType::NodeTarget.machine_name())
        .ok_or_else(|| contract_error("node-target variant"))?;

    let mut draft_targets = node_target.variants.iter().filter(|variant| {
        variant.payload.shape == PayloadShapeKind::Newtype
            && variant.payload.newtype.as_deref()
                == Some(DraftFieldType::DraftSymbol.type_expression())
    });
    let draft_target = draft_targets
        .next()
        .ok_or_else(|| contract_error("draft node target"))?;
    if draft_targets.next().is_some() {
        return Err(contract_error("unique draft node target"));
    }

    let mut existing_targets = node_target.variants.iter().filter(|variant| {
        variant.payload.shape == PayloadShapeKind::Newtype
            && variant.payload.newtype.as_deref() == Some(DraftFieldType::NodeId.type_expression())
    });
    let existing_target = existing_targets
        .next()
        .ok_or_else(|| contract_error("existing node target"))?;
    if existing_targets.next().is_some() {
        return Err(contract_error("unique existing node target"));
    }

    let primitive_types = schema
        .structured_authoring
        .type_variants
        .iter()
        .filter(|variant| variant.shape == PayloadShapeKind::Unit)
        .map(|variant| variant.name.as_str())
        .collect::<Vec<_>>();
    if primitive_types.is_empty() {
        return Err(contract_error("primitive type variants"));
    }
    let integer_type = primitive_types
        .iter()
        .copied()
        .find(|name| *name == DraftFieldType::I64.machine_name())
        .ok_or_else(|| contract_error("i64 type variant"))?;

    let mut nominal_types = schema
        .structured_authoring
        .type_variants
        .iter()
        .filter(|variant| {
            variant.shape == PayloadShapeKind::Newtype
                && variant.newtype == Some(DraftFieldType::NodeTarget)
        });
    let nominal_type = nominal_types
        .next()
        .ok_or_else(|| contract_error("nominal type variant"))?;
    if nominal_types.next().is_some() {
        return Err(contract_error("unique nominal type variant"));
    }

    let mut operation_results =
        schema
            .structured_authoring
            .value_variants
            .iter()
            .filter(|variant| {
                variant.shape == PayloadShapeKind::Record
                    && variant.fields.len() == 2
                    && variant.fields[0].field_type == DraftFieldType::NodeTarget
                    && variant.fields[1].field_type == DraftFieldType::U8
            });
    let operation_result = operation_results
        .next()
        .ok_or_else(|| contract_error("operation-result value variant"))?;
    if operation_results.next().is_some() {
        return Err(contract_error("unique operation-result value variant"));
    }

    let mut body_operations = schema.structured_authoring.records.iter().filter(|record| {
        record.fields.len() == 2
            && record.fields[0].field_type == DraftFieldType::DraftSymbol
            && record.fields[0].declares_symbol
            && record.fields[1].field_type == DraftFieldType::ExpressionKind
    });
    let body_operation = body_operations
        .next()
        .ok_or_else(|| contract_error("body-operation record"))?;
    if body_operations.next().is_some() {
        return Err(contract_error("unique body-operation record"));
    }

    Ok(format!(
        "Core authoring cards:\n\
  node target      ({} SYMBOL) | ({} NODE_OR_@ALIAS)\n\
  primitive type   {}\n\
  nominal type     ({} NODE_TARGET)\n\
  operation result ({} {{ {} NODE_TARGET {} N }})\n\
  body operation   {{ {} SYMBOL {} OPERATION_DRAFT }}\n\
Draft SYMBOL values are ordinary bare or quoted strings; @ spelling is reserved exclusively for aliases already present in the supplied packet. A create-purpose packet at revision zero therefore cannot alias nodes that the plan is about to draft. Unit type forms such as {integer_type} are bare values, not `({integer_type})`. Validate plans must omit idempotency_key. Apply may omit it or supply exactly {}.\n\
\n\
Use `lkjscript schema --root apply_transaction --root run --pretty` for the exact typed forms accepted inside operations, runtime values, and policies.",
        draft_target.name,
        existing_target.name,
        primitive_types.join(" | "),
        nominal_type.name,
        operation_result.name,
        operation_result.fields[0].name,
        operation_result.fields[1].name,
        body_operation.fields[0].name,
        body_operation.fields[1].name,
        schema.id_formats.idempotency_key,
    ))
}

fn contract_error(fact: &str) -> LkError {
    LkError::new(
        ErrorCode::ProtocolMalformed,
        format!("executable machine description lacks one {fact}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_cards_follow_the_executable_authoring_description() {
        let cards = authoring_help_cards().expect("active description is complete");
        assert!(cards.contains("(draft SYMBOL) | (existing NODE_OR_@ALIAS)"));
        assert!(cards.contains("unit | bool | i64 | bytes"));
        assert!(cards.contains("exactly 32 lowercase hexadecimal characters"));
    }
}
