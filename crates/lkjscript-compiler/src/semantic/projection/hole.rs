use crate::semantic::schema::SemanticNodeValue as Value;
use crate::source::{SourceNode, SyntaxKind};

pub(super) fn value(node: &SourceNode) -> Option<Value> {
    let identity = node
        .children
        .first()
        .and_then(|child| match &child.kind {
            SyntaxKind::Call { name } if name == "name" => child.children.first(),
            _ => None,
        })
        .and_then(|child| match &child.kind {
            SyntaxKind::Str { value } => Some(value.clone()),
            _ => None,
        })?;
    let goal = node
        .children
        .get(1)
        .and_then(|child| match &child.kind {
            SyntaxKind::Call { name } if name == "goal" => child.children.first(),
            _ => None,
        })
        .and_then(|child| match &child.kind {
            SyntaxKind::Str { value } => Some(value.clone()),
            _ => None,
        });
    Some(Value::TypedHole { identity, goal })
}
