use std::collections::HashMap;

use crate::source::{
    DiagnosticCategory, RevisionId, SourceDiagnostic, SourceFile, SourceResult, SourceSpan,
    SEMANTIC_SOURCE_FOUNDATION_SCHEMA, SEMANTIC_SOURCE_FOUNDATION_SCHEMA_VERSION, SOURCE_EDITION,
};

use super::append_framed;

pub(crate) fn order_and_revision(files: &[SourceFile]) -> SourceResult<(Vec<usize>, RevisionId)> {
    let mut canonical = Vec::new();
    append_framed(&mut canonical, SEMANTIC_SOURCE_FOUNDATION_SCHEMA.as_bytes());
    append_framed(
        &mut canonical,
        &SEMANTIC_SOURCE_FOUNDATION_SCHEMA_VERSION.to_be_bytes(),
    );
    append_framed(&mut canonical, &SOURCE_EDITION.to_be_bytes());
    let mut ordered: Vec<usize> = (0..files.len()).collect();
    ordered.sort_by(|left, right| {
        files[*left]
            .origin
            .logical_path
            .cmp(&files[*right].origin.logical_path)
    });
    let mut logical_origins: HashMap<String, crate::source::SourceOrigin> = HashMap::new();
    for index in &ordered {
        let file = &files[*index];
        if let Some(first_origin) = logical_origins.get(&file.origin.logical_path) {
            return Err(SourceDiagnostic::new(
                "LKJ-SRC-LOAD",
                DiagnosticCategory::SourceLoading,
                format!(
                    "distinct source units have duplicate logical origin {}",
                    file.origin.logical_path
                ),
                file.origin.clone(),
                SourceSpan::zero(),
            )
            .with_related(
                "first source unit",
                first_origin.clone(),
                SourceSpan::zero(),
            ));
        }
        logical_origins.insert(file.origin.logical_path.clone(), file.origin.clone());
    }
    for index in &ordered {
        let file = &files[*index];
        append_framed(&mut canonical, file.origin.logical_path.as_bytes());
        append_framed(&mut canonical, &file.exact_source_len.to_be_bytes());
        append_framed(&mut canonical, &file.exact_source_sha256);
    }
    Ok((ordered, RevisionId(lkjscript_core::sha256(&canonical))))
}
