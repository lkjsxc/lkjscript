use crate::semantic::codec::error;
use crate::semantic::schema::{ProtocolError, ProtocolErrorCode};
use crate::semantic::transaction::ResolvedOperation;
use crate::source::{SourceFile, SourceNode};

pub(crate) fn delete(
    files: &mut [SourceFile],
    operation: &ResolvedOperation,
) -> Result<(), ProtocolError> {
    let ResolvedOperation::DeleteHole { owner, path, .. } = operation else {
        return Ok(());
    };
    let Some((&target, parent_path)) = path.split_last() else {
        return Err(error(
            ProtocolErrorCode::InvalidOperation,
            "cannot delete declaration root",
        ));
    };
    let owner = crate::semantic::transaction::node_mut(files, *owner)
        .ok_or_else(|| error(ProtocolErrorCode::UnknownNode, "hole owner disappeared"))?;
    let parent = descendant_mut(owner, parent_path)
        .ok_or_else(|| error(ProtocolErrorCode::UnknownNode, "hole parent disappeared"))?;
    if target >= parent.children.len() {
        return Err(error(
            ProtocolErrorCode::UnknownNode,
            "hole child disappeared",
        ));
    }
    parent.children.remove(target);
    Ok(())
}

fn descendant_mut<'a>(mut node: &'a mut SourceNode, path: &[usize]) -> Option<&'a mut SourceNode> {
    for index in path {
        node = node.children.get_mut(*index)?;
    }
    Some(node)
}
