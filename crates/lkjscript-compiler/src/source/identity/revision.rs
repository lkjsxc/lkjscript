use std::collections::HashMap;

use crate::source::{
    DiagnosticCategory, RevisionId, SourceDiagnostic, SourceEdition, SourceFile, SourceIdentity,
    SourceOrigin, SourceResult, SourceSpan, SourceTreeIdentity, SEMANTIC_SOURCE_FOUNDATION_SCHEMA,
    SEMANTIC_SOURCE_FOUNDATION_SCHEMA_VERSION,
};

use super::append_framed;

pub(crate) fn source_identity(
    edition: SourceEdition,
    logical_path: &str,
    exact_source_len: u64,
    exact_source_sha256: [u8; 32],
) -> SourceIdentity {
    let mut canonical = Vec::new();
    append_identity_header(&mut canonical, b"source-unit", edition);
    append_framed(&mut canonical, logical_path.as_bytes());
    append_framed(&mut canonical, &exact_source_len.to_be_bytes());
    append_framed(&mut canonical, &exact_source_sha256);
    SourceIdentity(lkjscript_core::sha256(&canonical))
}

pub(crate) fn order_and_revision(
    files: &[SourceFile],
) -> SourceResult<(Vec<usize>, SourceEdition, RevisionId)> {
    let edition = files
        .first()
        .map_or(SourceEdition::Edition1, |file| file.edition);
    validate_closure_editions(files, edition)?;
    let mut canonical = Vec::new();
    append_identity_header(&mut canonical, b"revision", edition);
    let mut ordered: Vec<usize> = (0..files.len()).collect();
    ordered.sort_by(|left, right| {
        files[*left]
            .origin
            .logical_path
            .cmp(&files[*right].origin.logical_path)
    });
    validate_unique_origins(files, &ordered)?;
    for index in &ordered {
        let file = &files[*index];
        append_framed(&mut canonical, file.origin.logical_path.as_bytes());
        append_framed(&mut canonical, &file.identity.as_bytes());
    }
    Ok((
        ordered,
        edition,
        RevisionId(lkjscript_core::sha256(&canonical)),
    ))
}

pub(crate) fn tree_identity(
    edition: SourceEdition,
    root: &SourceOrigin,
    revision: RevisionId,
) -> SourceTreeIdentity {
    let mut canonical = Vec::new();
    append_identity_header(&mut canonical, b"source-tree", edition);
    append_framed(&mut canonical, root.logical_path.as_bytes());
    append_framed(&mut canonical, &revision.as_bytes());
    SourceTreeIdentity(lkjscript_core::sha256(&canonical))
}

fn append_identity_header(output: &mut Vec<u8>, domain: &[u8], edition: SourceEdition) {
    append_framed(output, SEMANTIC_SOURCE_FOUNDATION_SCHEMA.as_bytes());
    append_framed(
        output,
        &SEMANTIC_SOURCE_FOUNDATION_SCHEMA_VERSION.to_be_bytes(),
    );
    append_framed(output, domain);
    append_framed(output, &edition.number().to_be_bytes());
}

fn validate_closure_editions(files: &[SourceFile], expected: SourceEdition) -> SourceResult<()> {
    if let Some(file) = files.iter().find(|file| file.edition != expected) {
        let first = files.first().map(|file| file.origin.clone());
        let mut diagnostic = SourceDiagnostic::new(
            "LKJ-SRC-MIXED-EDITION",
            DiagnosticCategory::SourceLoading,
            "loaded source closure mixes Edition 1 and Edition 2",
            file.origin.clone(),
            SourceSpan::zero(),
        );
        if let Some(origin) = first {
            diagnostic = diagnostic.with_related(
                format!("closure edition is {}", expected.number()),
                origin,
                SourceSpan::zero(),
            );
        }
        return Err(diagnostic);
    }
    Ok(())
}

fn validate_unique_origins(files: &[SourceFile], ordered: &[usize]) -> SourceResult<()> {
    let mut logical_origins: HashMap<String, SourceOrigin> = HashMap::new();
    for index in ordered {
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
    Ok(())
}
