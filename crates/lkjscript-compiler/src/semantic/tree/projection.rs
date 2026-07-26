use crate::semantic::schema::{SemanticNodeKind, SemanticNodeValue, TriviaRecord};
use crate::source::{SourceNode, ValidatedSourceTree};

#[derive(Clone)]
pub(super) struct Projection {
    pub(super) kind: SemanticNodeKind,
    pub(super) value: Option<SemanticNodeValue>,
    pub(super) trivia: Vec<TriviaRecord>,
}

pub(super) fn projections(tree: &ValidatedSourceTree) -> Vec<Projection> {
    let mut output = Vec::new();
    let mut files: Vec<_> = tree.files().iter().collect();
    files.sort_by(|a, b| a.origin.logical_path.cmp(&b.origin.logical_path));
    for file in files {
        for form in &file.syntax {
            project_node(form, None, None, 0, &mut output);
        }
    }
    output
}

fn project_node(
    node: &SourceNode,
    parent: Option<&SourceNode>,
    parent_kind: Option<SemanticNodeKind>,
    index: usize,
    output: &mut Vec<Projection>,
) {
    let (kind, value) = crate::semantic::projection::classify(node, parent, parent_kind, index);
    output.push(Projection {
        kind,
        value,
        trivia: crate::semantic::projection::trivia(node),
    });
    for (child_index, child) in node.children.iter().enumerate() {
        project_node(child, Some(node), Some(kind), child_index, output);
    }
}
