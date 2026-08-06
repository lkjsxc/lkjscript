fn structural_local_is_copy(chunk: &Chunk, kind: Kind) -> bool {
    let representation = match kind {
        Kind::StructuralOwner { representation, .. }
        | Kind::StructuralOwnerRef { representation, .. } => representation,
        _ => return false,
    };
    chunk
        .structural_representations
        .get_structural(representation)
        .and_then(|item| chunk.structural_types.get_structural(item.type_id))
        .is_some_and(|item| item.mode == crate::StructuralTypeMode::Copy)
}
