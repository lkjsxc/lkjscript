use crate::source::{SourceNode, SyntaxKind};

use super::{SemanticNodeKind, SemanticNodeValue};

pub(super) fn kind(
    kind: SemanticNodeKind,
    value: Option<&SemanticNodeValue>,
) -> Option<Result<SyntaxKind, String>> {
    match (kind, value) {
        (
            SemanticNodeKind::EditionMarker,
            Some(SemanticNodeValue::EditionIdentity { edition: 2 }),
        ) => Some(Ok(SyntaxKind::EditionMarker)),
        (
            SemanticNodeKind::EditionNumber,
            Some(SemanticNodeValue::EditionIdentity { edition: 2 }),
        ) => Some(Ok(SyntaxKind::I64 { value: 2 })),
        (SemanticNodeKind::EditionMarker | SemanticNodeKind::EditionNumber, _) => {
            Some(Err("semantic edition node has invalid identity".into()))
        }
        _ => None,
    }
}

pub(super) fn validate(
    kind: SemanticNodeKind,
    children: &[SourceNode],
    leading_trivia: &[String],
    before_close_trivia: &[String],
) -> Result<(), String> {
    match kind {
        SemanticNodeKind::EditionMarker => {
            if !before_close_trivia.is_empty() || !exact_number(children) {
                return Err("semantic edition marker must contain exact Edition 2 number".into());
            }
        }
        SemanticNodeKind::EditionNumber
            if !children.is_empty()
                || !leading_trivia.is_empty()
                || !before_close_trivia.is_empty() =>
        {
            return Err("semantic edition number cannot carry children or trivia".into());
        }
        _ => {}
    }
    Ok(())
}

fn exact_number(children: &[SourceNode]) -> bool {
    matches!(
        children,
        [SourceNode {
            kind: SyntaxKind::I64 { value: 2 },
            leading_trivia,
            before_close_trivia,
            children,
            ..
        }] if leading_trivia.is_empty() && before_close_trivia.is_empty() && children.is_empty()
    )
}
