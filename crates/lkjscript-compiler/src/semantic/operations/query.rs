use crate::semantic::codec::error;
use crate::semantic::schema::{
    FactRecord, NodeFacts, NodeQueryRecord, ProtocolError, ProtocolErrorCode,
};
use crate::source::{DeclarationKind, NodeKind, SourceNode, SyntaxKind, ValidatedSourceTree};

pub(crate) fn query(
    tree: &ValidatedSourceTree,
    index: u32,
) -> Result<NodeQueryRecord, ProtocolError> {
    let node = tree.nodes().get(index as usize).ok_or_else(|| {
        error(
            ProtocolErrorCode::UnknownNode,
            format!("unknown revision-scoped node index {index}"),
        )
    })?;
    let literal_type = match node.kind() {
        NodeKind::I64Literal => Some("I64"),
        NodeKind::F64Literal => Some("F64"),
        NodeKind::BoolLiteral => Some("Bool"),
        NodeKind::UnitLiteral => Some("Unit"),
        NodeKind::StringLiteral => Some("Str"),
        NodeKind::Symbol | NodeKind::Call => None,
    };
    let static_type = literal_type.map_or_else(
        || unavailable("the HIR does not retain a source-node correlation for this form"),
        available,
    );
    let effects = literal_type.map_or_else(
        || unavailable("effect facts are declaration HIR facts without source-node correlation"),
        |_| available("pure"),
    );
    let binding = binding_fact(tree, index, node);
    Ok(NodeQueryRecord {
        node: crate::semantic::tree::node_record(tree, node),
        facts: NodeFacts {
            binding,
            static_type,
            effects,
            ownership: unavailable("no ownership place or loan is correlated to this source node"),
            control_flow: unavailable("no control-flow relation is correlated to source nodes"),
            layout: unavailable("layout facts are not produced for semantic source nodes"),
            proof: unavailable("proof relations are not produced for this node"),
        },
    })
}

fn binding_fact(
    tree: &ValidatedSourceTree,
    index: u32,
    node: &crate::source::NodeSummary,
) -> FactRecord {
    if !matches!(node.kind(), NodeKind::Call | NodeKind::Symbol) {
        return unavailable("literal nodes do not resolve a binding");
    }
    let Some(name) = node.label() else {
        return unavailable("source node has no binding spelling");
    };
    let Some(owner) = crate::semantic::tree::containing_declaration(tree, node) else {
        return unavailable("source node has no containing declaration");
    };
    let nodes = crate::semantic::tree::source_nodes(tree);
    let Some(root) = nodes.get(owner.node().index() as usize) else {
        return unavailable("containing declaration source is unavailable");
    };
    let expression =
        crate::semantic::transaction::path_from_owner(tree, owner.node().index(), index)
            .ok()
            .is_some_and(|path| crate::semantic::transaction::is_expression_path(root, &path));
    if !expression || has_local_name(root, name) {
        return unavailable("the source position is structural or may resolve a lexical binding");
    }
    let Some(declaration) = tree.declarations().iter().find(|declaration| {
        declaration.kind() == DeclarationKind::Function && declaration.name() == name
    }) else {
        return unavailable("no source-closure function declaration resolves this spelling");
    };
    available(&format!("declaration:{}", declaration.key().to_hex()))
}

fn has_local_name(node: &SourceNode, target: &str) -> bool {
    if let SyntaxKind::Call { name } = &node.kind {
        if name == "params"
            && node
                .children
                .iter()
                .step_by(2)
                .any(|child| source_name(child) == Some(target))
        {
            return true;
        }
        if matches!(name.as_str(), "var" | "bind")
            && node
                .children
                .first()
                .and_then(|child| {
                    if name == "var" {
                        child.children.first()
                    } else {
                        Some(child)
                    }
                })
                .and_then(source_name)
                == Some(target)
        {
            return true;
        }
    }
    node.children
        .iter()
        .any(|child| has_local_name(child, target))
}

fn source_name(node: &SourceNode) -> Option<&str> {
    match &node.kind {
        SyntaxKind::Str { value } => Some(value),
        SyntaxKind::Symbol { name } => Some(name),
        _ => None,
    }
}

fn available(value: &str) -> FactRecord {
    FactRecord::Available {
        producer: "lkjscript-compiler-hir".to_string(),
        version: 1,
        certainty: "guaranteed".to_string(),
        value: value.to_string(),
    }
}

fn unavailable(reason: &str) -> FactRecord {
    FactRecord::Unavailable {
        reason: reason.to_string(),
    }
}
