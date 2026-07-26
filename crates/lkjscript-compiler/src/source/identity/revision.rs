use std::collections::HashMap;

use crate::source::{
    RevisionId, SourceDiagnostic, SourceFile, SourceIdentity, SourceOrigin, SourceResult,
    SourceTreeIdentity,
};

use super::{append_framed, IdentityEncodingError};

pub(crate) fn source_identity(
    origin: &SourceOrigin,
    exact_source_len: u64,
    exact_source_sha256: [u8; 32],
) -> SourceResult<SourceIdentity> {
    let mut canonical = Vec::new();
    append_identity_header(&mut canonical, b"source-unit", origin)?;
    append_framed(&mut canonical, origin.logical_path().as_bytes())
        .map_err(|error| encoding_error(origin, "logical path", error))?;
    append_framed(&mut canonical, &exact_source_len.to_be_bytes())
        .map_err(|error| encoding_error(origin, "source length", error))?;
    append_framed(&mut canonical, &exact_source_sha256)
        .map_err(|error| encoding_error(origin, "source digest", error))?;
    Ok(SourceIdentity(lkjscript_core::sha256(&canonical)))
}

pub(crate) fn order_and_revision(
    files: &[SourceFile],
    root_origin: &SourceOrigin,
) -> SourceResult<(Vec<usize>, RevisionId)> {
    let mut canonical = Vec::new();
    append_identity_header(&mut canonical, b"revision", root_origin)?;
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
        append_framed(&mut canonical, file.origin.logical_path.as_bytes())
            .map_err(|error| encoding_error(&file.origin, "logical path", error))?;
        append_framed(&mut canonical, &file.identity.as_bytes())
            .map_err(|error| encoding_error(&file.origin, "source identity", error))?;
    }
    Ok((ordered, RevisionId(lkjscript_core::sha256(&canonical))))
}

pub(crate) fn tree_identity(
    root: &SourceOrigin,
    revision: RevisionId,
) -> SourceResult<SourceTreeIdentity> {
    let mut canonical = Vec::new();
    append_identity_header(&mut canonical, b"source-tree", root)?;
    append_framed(&mut canonical, root.logical_path.as_bytes())
        .map_err(|error| encoding_error(root, "root logical path", error))?;
    append_framed(&mut canonical, &revision.as_bytes())
        .map_err(|error| encoding_error(root, "revision", error))?;
    Ok(SourceTreeIdentity(lkjscript_core::sha256(&canonical)))
}

fn append_identity_header(
    output: &mut Vec<u8>,
    domain: &[u8],
    origin: &SourceOrigin,
) -> SourceResult<()> {
    append_framed(output, &lkjscript_contracts::SOURCE_DIGEST.as_bytes())
        .map_err(|error| encoding_error(origin, "source contract", error))?;
    append_framed(output, domain).map_err(|error| encoding_error(origin, "domain", error))
}

fn encoding_error(
    origin: &SourceOrigin,
    field: &str,
    error: IdentityEncodingError,
) -> SourceDiagnostic {
    SourceDiagnostic::generic(
        origin.clone(),
        format!("cannot encode source identity {field}: {error:?}"),
    )
}

fn validate_unique_origins(files: &[SourceFile], ordered: &[usize]) -> SourceResult<()> {
    let mut logical_origins: HashMap<String, SourceOrigin> = HashMap::new();
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
