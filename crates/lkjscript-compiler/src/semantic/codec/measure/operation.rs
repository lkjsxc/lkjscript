use crate::semantic::schema::{ExpressionCounts, ProtocolError, TransactionOperation};

pub(super) fn measure(
    operation: &TransactionOperation,
    strings: &mut u64,
    expression: &mut ExpressionCounts,
) -> Result<(), ProtocolError> {
    match operation {
        TransactionOperation::RenameDeclaration {
            declaration_key,
            entity_fingerprint,
            new_name,
        } => super::add(
            strings,
            &[
                declaration_key.len(),
                entity_fingerprint.len(),
                new_name.len(),
            ],
        )?,
        TransactionOperation::ReplaceExpression {
            declaration_key,
            entity_fingerprint,
            node_fingerprint,
            expression: replacement,
            ..
        } => {
            add_common(
                strings,
                declaration_key,
                entity_fingerprint,
                node_fingerprint,
            )?;
            replacement.measure(1, expression);
        }
        TransactionOperation::InsertHole {
            declaration_key,
            entity_fingerprint,
            node_fingerprint,
            hole_identity,
            goal,
            expected_type,
            ..
        }
        | TransactionOperation::RefineHole {
            declaration_key,
            entity_fingerprint,
            node_fingerprint,
            hole_identity,
            goal,
            expected_type,
            ..
        } => {
            add_common(
                strings,
                declaration_key,
                entity_fingerprint,
                node_fingerprint,
            )?;
            super::add(
                strings,
                &[
                    hole_identity.len(),
                    expected_type.len(),
                    goal.as_ref().map_or(0, String::len),
                ],
            )?;
        }
        TransactionOperation::FillHole {
            declaration_key,
            entity_fingerprint,
            node_fingerprint,
            hole_identity,
            expected_type,
            expression: replacement,
            ..
        } => {
            add_common(
                strings,
                declaration_key,
                entity_fingerprint,
                node_fingerprint,
            )?;
            super::add(strings, &[hole_identity.len(), expected_type.len()])?;
            replacement.measure(1, expression);
        }
        TransactionOperation::DeleteHole {
            declaration_key,
            entity_fingerprint,
            node_fingerprint,
            hole_identity,
            expected_type,
            ..
        } => {
            add_common(
                strings,
                declaration_key,
                entity_fingerprint,
                node_fingerprint,
            )?;
            super::add(strings, &[hole_identity.len(), expected_type.len()])?;
        }
    }
    Ok(())
}

fn add_common(
    total: &mut u64,
    declaration: &str,
    entity: &str,
    node: &str,
) -> Result<(), ProtocolError> {
    super::add(total, &[declaration.len(), entity.len(), node.len()])
}
