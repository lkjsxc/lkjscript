use std::path::PathBuf;

use crate::source::{identity, SourceFile, SourceOrigin, SourceResult, ValidatedSourceTree};

pub(crate) fn finish_tree(
    root: PathBuf,
    root_origin: SourceOrigin,
    files: Vec<SourceFile>,
) -> SourceResult<ValidatedSourceTree> {
    let (ordered, revision) = identity::order_and_revision(&files)?;
    let (nodes, top_ids) = identity::flatten_files(&files, &ordered, revision)?;
    let declarations = identity::build_declarations(&files, &ordered, &top_ids)?;
    let origins = ordered
        .iter()
        .map(|index| files[*index].origin.clone())
        .collect();
    Ok(ValidatedSourceTree::from_authority(
        revision,
        root,
        root_origin,
        files,
        origins,
        declarations,
        nodes,
    ))
}
