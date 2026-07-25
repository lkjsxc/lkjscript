use super::bounds::{limit, list, text, REFERENCE_ITEMS};
use super::model::{SemanticReference, SemanticWorkContext};

pub fn validate(value: &SemanticWorkContext, work: &mut usize) -> Result<(), String> {
    let reference_count = [
        value.target_entities.len(),
        value.completed_transactions.len(),
        value.diagnostics.len(),
        value.unresolved_holes.len(),
    ]
    .into_iter()
    .try_fold(0usize, |total, count| total.checked_add(count))
    .ok_or("semantic reference count overflow")?;
    if reference_count > REFERENCE_ITEMS {
        return Err(limit(
            "combined semantic references",
            REFERENCE_ITEMS,
            reference_count,
        ));
    }
    if let Some(session) = &value.session {
        text("semantic session schema", &session.schema, work)?;
        text("semantic session identity", &session.identity, work)?;
        text(
            "semantic session source revision",
            &session.source_revision,
            work,
        )?;
    }
    for (name, references) in [
        ("target entities", &value.target_entities),
        ("completed transactions", &value.completed_transactions),
        ("semantic diagnostics", &value.diagnostics),
        ("unresolved holes", &value.unresolved_holes),
    ] {
        list(name, references.len(), REFERENCE_ITEMS, work)?;
        for reference in references {
            semantic_reference(reference, work)?;
        }
    }
    Ok(())
}

fn semantic_reference(value: &SemanticReference, work: &mut usize) -> Result<(), String> {
    text("semantic reference schema", &value.schema, work)?;
    text("semantic source revision", &value.source_revision, work)?;
    text("semantic identity", &value.identity, work)
}
