use super::*;
use crate::semantic::schema::SemanticNodeKind;

pub(super) fn enum_node_identity(
    tree: &ValidatedSourceTree,
    node: &NodeSummary,
    source: &SourceNode,
    projection: &Projection,
    source_nodes: &[&SourceNode],
) -> Option<String> {
    if projection.kind == SemanticNodeKind::TypeEnum {
        let crate::source::SyntaxKind::Call { name } = &source.kind else {
            return None;
        };
        return tree
            .declarations()
            .iter()
            .find(|declaration| {
                declaration.kind() == DeclarationKind::Enum && declaration.name() == name
            })
            .map(|declaration| declaration.key().to_hex());
    }
    let declaration = containing_declaration(tree, node)?;
    if declaration.kind() != DeclarationKind::Enum {
        return None;
    }
    match projection.kind {
        SemanticNodeKind::EnumDeclaration => Some(declaration.key().to_hex()),
        SemanticNodeKind::EnumVariant => {
            let name = declared_child_name(source)?;
            let id =
                crate::source::enum_member_identity(declaration.key().digest(), "variant", name);
            Some(hex(&id))
        }
        SemanticNodeKind::EnumVariantField => {
            let name = declared_child_name(source)?;
            let fields = tree.node(node.parent()?).ok()??;
            let variant = tree.node(fields.parent()?).ok()??;
            let variant_source = source_nodes.get(usize::try_from(variant.id().index()).ok()?)?;
            let variant_name = declared_child_name(variant_source)?;
            let variant_id = crate::source::enum_member_identity(
                declaration.key().digest(),
                "variant",
                variant_name,
            );
            let id = crate::source::enum_member_identity(variant_id, "field", name);
            Some(hex(&id))
        }
        _ => None,
    }
}

fn declared_child_name(node: &SourceNode) -> Option<&str> {
    let name = node.children.first()?;
    let child = name.children.first()?;
    match &child.kind {
        crate::source::SyntaxKind::Str { value } => Some(value),
        _ => None,
    }
}
