use std::path::PathBuf;

use crate::source::{
    identity, SourceFile, SourceOrigin, SourceResult, ValidatedSourceParts, ValidatedSourceTree,
};

pub(crate) fn finish_tree(
    root: PathBuf,
    root_origin: SourceOrigin,
    files: Vec<SourceFile>,
) -> SourceResult<ValidatedSourceTree> {
    let (ordered, revision) = identity::order_and_revision(&files, &root_origin)?;
    let tree_identity = identity::tree_identity(&root_origin, revision)?;
    let (nodes, top_ids) = identity::flatten_files(&files, &ordered, revision)?;
    let declarations = identity::build_declarations(&files, &ordered, &top_ids)?;
    let origins = ordered
        .iter()
        .map(|index| files[*index].origin.clone())
        .collect();
    Ok(ValidatedSourceTree::from_authority(ValidatedSourceParts {
        identity: tree_identity,
        revision,
        root,
        root_origin,
        files,
        origins,
        declarations,
        nodes,
    }))
}
