mod expected;

use crate::hir::Type;
use crate::semantic::codec::error;
use crate::semantic::schema::{ProtocolError, ProtocolErrorCode, TypeUnavailableReason};
use crate::source::{SourceNode, SyntaxKind, ValidatedSourceTree};
use expected::{declaration_return, expected_at};

pub(crate) struct HoleSite<'a> {
    pub tree: &'a ValidatedSourceTree,
    pub node: u64,
    pub owner_node: u64,
    pub declaration_key: String,
    pub local_identity: String,
    pub goal: Option<String>,
    pub source: &'a SourceNode,
    pub root: &'a SourceNode,
    pub path: Vec<usize>,
    pub return_type: Type,
    pub expected: Result<Type, TypeUnavailableReason>,
}

pub(crate) fn find(tree: &ValidatedSourceTree, index: u64) -> Result<HoleSite<'_>, ProtocolError> {
    let host_index = usize::try_from(index).map_err(|_| {
        error(
            ProtocolErrorCode::UnknownNode,
            format!("hole node {index} is not host-addressable"),
        )
    })?;
    let summary = tree.nodes().get(host_index).ok_or_else(|| {
        error(
            ProtocolErrorCode::UnknownNode,
            format!("unknown hole node {index}"),
        )
    })?;
    let owner = crate::semantic::tree::containing_declaration(tree, summary).ok_or_else(|| {
        error(
            ProtocolErrorCode::InvalidOperation,
            "hole has no containing declaration",
        )
    })?;
    let nodes = crate::semantic::tree::source_nodes(tree);
    let source = *nodes.get(host_index).ok_or_else(|| {
        error(
            ProtocolErrorCode::UnknownNode,
            "hole source node is unavailable",
        )
    })?;
    let (local_identity, goal) = parse_hole(source).map_err(|message| {
        error(
            ProtocolErrorCode::ValidationFailed,
            format!("invalid typed hole: {message}"),
        )
    })?;
    let owner_index = usize::try_from(owner.node().index()).map_err(|_| {
        error(
            ProtocolErrorCode::UnknownNode,
            "hole declaration node is not host-addressable",
        )
    })?;
    let root = *nodes.get(owner_index).ok_or_else(|| {
        error(
            ProtocolErrorCode::UnknownNode,
            "hole declaration source is unavailable",
        )
    })?;
    let path = crate::semantic::transaction::path_from_owner(tree, owner.node().index(), index)?;
    if !crate::semantic::transaction::is_expression_path(root, &path) {
        return Err(error(
            ProtocolErrorCode::InvalidOperation,
            "typed hole is not in an expression position",
        ));
    }
    let return_type = declaration_return(root).ok_or_else(|| {
        error(
            ProtocolErrorCode::ValidationFailed,
            "hole declaration has no exact return type",
        )
    })?;
    let expected = expected_at(root, &path, &return_type, tree);
    Ok(HoleSite {
        tree,
        node: index,
        owner_node: owner.node().index(),
        declaration_key: owner.key().to_hex(),
        local_identity,
        goal,
        source,
        root,
        path,
        return_type,
        expected,
    })
}

pub(crate) fn expected_for_node(
    tree: &ValidatedSourceTree,
    index: u64,
) -> Result<(String, Result<Type, TypeUnavailableReason>), ProtocolError> {
    let host_index = usize::try_from(index).map_err(|_| {
        error(
            ProtocolErrorCode::UnknownNode,
            format!("expression node {index} is not host-addressable"),
        )
    })?;
    let summary = tree.nodes().get(host_index).ok_or_else(|| {
        error(
            ProtocolErrorCode::UnknownNode,
            format!("unknown expression node {index}"),
        )
    })?;
    let owner = crate::semantic::tree::containing_declaration(tree, summary).ok_or_else(|| {
        error(
            ProtocolErrorCode::InvalidOperation,
            "expression has no containing declaration",
        )
    })?;
    let nodes = crate::semantic::tree::source_nodes(tree);
    let owner_index = usize::try_from(owner.node().index()).map_err(|_| {
        error(
            ProtocolErrorCode::UnknownNode,
            "expression declaration node is not host-addressable",
        )
    })?;
    let root = *nodes.get(owner_index).ok_or_else(|| {
        error(
            ProtocolErrorCode::UnknownNode,
            "expression declaration source is unavailable",
        )
    })?;
    let path = crate::semantic::transaction::path_from_owner(tree, owner.node().index(), index)?;
    if !crate::semantic::transaction::is_expression_path(root, &path) {
        return Err(error(
            ProtocolErrorCode::InvalidOperation,
            "target is not an expression",
        ));
    }
    let return_type = declaration_return(root).ok_or_else(|| {
        error(
            ProtocolErrorCode::ValidationFailed,
            "declaration has no exact return type",
        )
    })?;
    Ok((
        owner.key().to_hex(),
        expected_at(root, &path, &return_type, tree),
    ))
}

pub(super) fn parse_hole(node: &SourceNode) -> Result<(String, Option<String>), String> {
    if !super::types::call_is(node, "hole") {
        return Err("target is not a typed hole".into());
    }
    let ([identity] | [identity, _]) = node.children.as_slice() else {
        return Err("hole requires name/ identity and optional goal/".into());
    };
    let identity = match &identity.kind {
        SyntaxKind::Call { name } if name == "name" => identity.children.first(),
        _ => None,
    }
    .and_then(super::types::source_name)
    .ok_or("hole identity must be one name/ value")?;
    if !crate::source::is_source_identifier(identity) {
        return Err("hole identity is not a source identifier".into());
    }
    let goal = node
        .children
        .get(1)
        .map(|goal| {
            if !super::types::call_is(goal, "goal") || goal.children.len() != 1 {
                return Err("hole goal must contain exactly one string".to_string());
            }
            match &goal.children[0].kind {
                SyntaxKind::Str { value } => Ok(value.clone()),
                _ => Err("hole goal must be a string".into()),
            }
        })
        .transpose()?;
    Ok((identity.to_string(), goal))
}

pub(crate) fn deletion_legal(site: &HoleSite<'_>) -> bool {
    let Some((&target, parent_path)) = site.path.split_last() else {
        return false;
    };
    let mut parent = site.root;
    for index in parent_path {
        let Some(child) = parent.children.get(*index) else {
            return false;
        };
        parent = child;
    }
    match &parent.kind {
        SyntaxKind::Call { name } if name == "do" => parent.children.len() > 1,
        SyntaxKind::Call { name } if name == "while" => target > 0,
        _ => false,
    }
}
