use std::path::PathBuf;

use crate::source::{
    identity, SourceFile, SourceOrigin, SourceResult, ValidatedSourceParts, ValidatedSourceTree,
};

pub(crate) fn finish_tree(
    root: PathBuf,
    root_origin: SourceOrigin,
    files: Vec<SourceFile>,
) -> SourceResult<ValidatedSourceTree> {
    let ordered = identity::order_files(&files, &root_origin)?;
    let declarations = identity::build_declarations(&files, &ordered)?;
    Ok(ValidatedSourceTree::from_authority(ValidatedSourceParts {
        root,
        #[cfg(test)]
        root_origin,
        files,
        declarations,
    }))
}
