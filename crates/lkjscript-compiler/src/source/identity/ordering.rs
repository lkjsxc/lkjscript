use std::collections::HashMap;

use crate::source::{SourceDiagnostic, SourceFile, SourceOrigin, SourceResult};

pub(crate) fn order_files(
    files: &[SourceFile],
    root_origin: &SourceOrigin,
) -> SourceResult<Vec<usize>> {
    let mut ordered: Vec<usize> = (0..files.len()).collect();
    ordered.sort_by(|left, right| {
        files[*left]
            .origin
            .logical_path
            .cmp(&files[*right].origin.logical_path)
    });
    validate_unique_origins(files, &ordered, root_origin)?;
    Ok(ordered)
}

fn validate_unique_origins(
    files: &[SourceFile],
    ordered: &[usize],
    root_origin: &SourceOrigin,
) -> SourceResult<()> {
    let mut logical_origins: HashMap<String, SourceOrigin> = HashMap::new();
    logical_origins.try_reserve(files.len()).map_err(|_| {
        SourceDiagnostic::host(root_origin.clone(), "source origin index allocation failed")
    })?;
    for index in ordered {
        let file = &files[*index];
        if let Some(first_origin) = logical_origins.get(&file.origin.logical_path) {
            return Err(SourceDiagnostic::new(
                "LKJ-SRC-LOAD",
                crate::source::DiagnosticCategory::SourceLoading,
                format!(
                    "distinct source units have duplicate logical origin {}",
                    file.origin.logical_path
                ),
                file.origin.clone(),
                crate::source::SourceSpan::zero(),
            )
            .with_related(
                "first source unit",
                first_origin.clone(),
                crate::source::SourceSpan::zero(),
            ));
        }
        logical_origins.insert(file.origin.logical_path.clone(), file.origin.clone());
    }
    Ok(())
}
