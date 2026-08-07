fn semantic_dag_list_len(value: &crate::SemanticDagSnapshot) -> Option<usize> {
    let mut id = value.root();
    let mut length = 0_usize;
    loop {
        let node = usize::try_from(id.get()).ok()?;
        match &value.nodes().get(node)?.payload {
            crate::SemanticDagPayload::EmptyList => return Some(length),
            crate::SemanticDagPayload::List { tail, .. } => {
                length = length.checked_add(1)?;
                id = *tail;
            }
            _ => return None,
        }
    }
}

fn semantic_dag_list_i64(
    value: &crate::SemanticDagSnapshot,
    requested: usize,
) -> Option<i64> {
    let mut id = value.root();
    let mut index = 0_usize;
    loop {
        let node = usize::try_from(id.get()).ok()?;
        match &value.nodes().get(node)?.payload {
            crate::SemanticDagPayload::EmptyList => return None,
            crate::SemanticDagPayload::List { head, tail } => {
                if index == requested {
                    let head = usize::try_from(head.get()).ok()?;
                    let head = value.nodes().get(head)?;
                    return match head.payload {
                        crate::SemanticDagPayload::Inline(InlineStructuralValue::I64(value)) => {
                            Some(value)
                        }
                        _ => None,
                    };
                }
                index = index.checked_add(1)?;
                id = *tail;
            }
            _ => return None,
        }
    }
}
